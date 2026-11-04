//! Editing layer of WangunGIS: everything a tool needs, without GPU or UI.
//!
//! A [`Scene`] wraps a [`Project`] of kernel tiles and answers in project-local
//! coordinates. Hew's `InferenceScene` works per `Document` (object ids are only
//! unique there) and rebuilds whole after any change, which stalls on a dense
//! 1 km tile. So each tile is sharded into cells of [`CELL_M`], indexed only
//! when a ray first passes near them; results are merged across cells by the
//! same priority Hew uses inside one scene.

pub mod tool;
pub mod vcb;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::num::NonZeroU32;

use inference::{
    InferenceScene, PickRay, RANK_TIE_FRACTION, Snap, SnapKind, SnapQuery, SnapSource,
    rank_distance_bucket,
};
use kernel::{
    Document, KernelOp, NodeId, ObjectId, Plane, Point3, SketchId, SketchIslandId, SketchRegionId,
    Transform, Vec3,
};
use wangungis_scene::project::{
    Project, ProjectError, TileKey, horizontal_reach, node_points, node_visible,
};
use wangungis_scene::{KERNEL_FRAME_BOUND_M, PROJECT_BOUND_M, check_local};

/// Edge of an inference cell (m). An edit rebuilds only the cells a ray then
/// touches: ±110 objects each on a dense city, against ±2,800 in a whole tile.
pub const CELL_M: f64 = 200.0;

/// Why an edit was refused. The project is unchanged whenever one is returned.
#[derive(Debug, Clone, PartialEq)]
pub enum EditError {
    /// The result would reach `reach_m` from the center of `tile`.
    OutOfTile { tile: TileKey, reach_m: f64 },
    /// A point lies outside the project bound.
    OutOfProject,
    /// The kernel refused the operation.
    Kernel(String),
    /// Not supported yet (e.g. editing inside a component).
    Unsupported(&'static str),
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EditError::OutOfTile { reach_m, .. } => write!(
                f,
                "Ditolak: hasilnya {reach_m:.0} m dari pusat tile (batas {KERNEL_FRAME_BOUND_M:.0} m)"
            ),
            EditError::OutOfProject => write!(
                f,
                "Ditolak: di luar batas proyek (±{PROJECT_BOUND_M:.0} m dari origin)"
            ),
            EditError::Kernel(e) => write!(f, "Ditolak kernel: {e}"),
            EditError::Unsupported(what) => write!(f, "Belum didukung: {what}"),
        }
    }
}

impl From<ProjectError> for EditError {
    fn from(e: ProjectError) -> Self {
        match e {
            ProjectError::Origin(_) => EditError::OutOfProject,
            other => EditError::Kernel(format!("{other:?}")),
        }
    }
}

fn kernel_err<E: fmt::Debug>(e: E) -> EditError {
    EditError::Kernel(format!("{e:?}"))
}

/// A snap in project-local coordinates, and the tile whose geometry produced
/// it: `snap.source` and the sketch ids in `snap` are valid in that tile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    pub tile: TileKey,
    pub snap: Snap,
}

/// A face under a ray: `source` is valid in `tile`, `depth` is the distance
/// along the ray (m).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceHit {
    pub tile: TileKey,
    pub source: SnapSource,
    pub depth: f64,
}

/// What [`Scene::draw`] did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drawn {
    pub tile: TileKey,
    /// The free sketch written, if the drawing did not all land on faces.
    pub sketch: Option<SketchId>,
    /// A new region closed, on a sketch or as a split of a face.
    pub closed: bool,
}

/// What Move can pick up. Coordinates are tile-local.
#[derive(Debug, Clone, PartialEq)]
pub enum Movable {
    /// A whole top-level object, group or component instance.
    Node { tile: TileKey, node: NodeId },
    /// A connected drawing on a free sketch. `segments` outlives the id:
    /// drafting a copy rewrites the sketch.
    Island {
        tile: TileKey,
        sketch: SketchId,
        island: SketchIslandId,
        segments: Vec<(Point3, Point3)>,
    },
}

impl Movable {
    pub fn tile(&self) -> TileKey {
        match self {
            Movable::Node { tile, .. } | Movable::Island { tile, .. } => *tile,
        }
    }
}

/// Lines drawn for a picked object at most. Past this the preview is partial,
/// never the edit.
const MAX_PREVIEW_LINES: usize = 5_000;
/// A displacement this short (m) along a sketch's normal keeps it on its plane.
const IN_PLANE: f64 = 1e-6;

/// One shard of a tile: what it holds, tile-local bounds to reject rays cheaply,
/// and its index once a ray has needed it.
#[derive(Default)]
struct Cell {
    nodes: Vec<NodeId>,
    sketches: Vec<SketchId>,
    lo: Option<[f64; 3]>,
    hi: [f64; 3],
    scene: Option<InferenceScene>,
}

impl Cell {
    fn extend(&mut self, points: impl IntoIterator<Item = Point3>) {
        for p in points {
            let q = [p.x, p.y, p.z];
            match &mut self.lo {
                None => {
                    self.lo = Some(q);
                    self.hi = q;
                }
                Some(lo) => {
                    for k in 0..3 {
                        lo[k] = lo[k].min(q[k]);
                        self.hi[k] = self.hi[k].max(q[k]);
                    }
                }
            }
        }
    }

    /// Whether the tile-local unit ray passes near the bounds: within the
    /// height range, close enough in plan view for a pick cone of `cone_tan`
    /// (tangent of its half-angle) to reach, measured at the bounds' far side.
    fn near(&self, origin: Point3, dir: Vec3, cone_tan: f64) -> bool {
        let Some(lo) = self.lo else { return false };
        let hi = self.hi;
        let c = [
            (lo[0] + hi[0]) / 2.0,
            (lo[1] + hi[1]) / 2.0,
            (lo[2] + hi[2]) / 2.0,
        ];
        let half = (hi[0] - lo[0]).hypot(hi[1] - lo[1]).hypot(hi[2] - lo[2]) / 2.0;
        let dist = (origin.x - c[0])
            .hypot(origin.y - c[1])
            .hypot(origin.z - c[2]);
        let m = cone_tan * (dist + half) + 1e-3;
        // Ray span inside the height range.
        let (mut t0, mut t1) = (0.0f64, 1e7f64);
        if dir.z.abs() > 1e-12 {
            let a = (lo[2] - m - origin.z) / dir.z;
            let b = (hi[2] + m - origin.z) / dir.z;
            t0 = t0.max(a.min(b));
            t1 = t1.min(a.max(b));
        } else if origin.z < lo[2] - m || origin.z > hi[2] + m {
            return false;
        }
        if t0 > t1 {
            return false;
        }
        // Closest approach of that span to the center, in plan view.
        let d2 = dir.x * dir.x + dir.y * dir.y;
        let (vx, vy) = (c[0] - origin.x, c[1] - origin.y);
        let t = if d2 > 0.0 {
            ((vx * dir.x + vy * dir.y) / d2).clamp(t0, t1)
        } else {
            t0
        };
        let gap = (vx - t * dir.x).hypot(vy - t * dir.y);
        gap <= (hi[0] - lo[0]).hypot(hi[1] - lo[1]) / 2.0 + m
    }
}

/// A tile's cells, by index on the [`CELL_M`] grid of the tile frame.
type Cells = BTreeMap<(i32, i32), Cell>;

pub struct Scene {
    project: Project,
    /// Dropped when the tile changes; rebuilt on the next ray near it.
    cells: BTreeMap<TileKey, Cells>,
    /// No geometry: answers pure-direction snaps (project axes, axis locks).
    base: InferenceScene,
    undo: Vec<TileKey>,
    redo: Vec<TileKey>,
    dirty: BTreeSet<TileKey>,
}

fn offset(p: Point3, c: [f64; 3]) -> Point3 {
    Point3::new(p.x - c[0], p.y - c[1], p.z - c[2])
}

fn onset(p: Point3, c: [f64; 3]) -> Point3 {
    Point3::new(p.x + c[0], p.y + c[1], p.z + c[2])
}

fn shifted_plane(plane: Plane, c: [f64; 3]) -> Result<Plane, kernel::MathError> {
    Plane::from_point_normal(offset(plane.point(), c), plane.normal())
}

/// The cell holding `points`, by the center of their plan bounds, with its
/// bounds grown to cover them. `None` for no points.
fn place(cells: &mut Cells, points: Vec<Point3>) -> Option<&mut Cell> {
    let (first, rest) = points.split_first()?;
    let (mut lo, mut hi) = ([first.x, first.y], [first.x, first.y]);
    for p in rest {
        lo = [lo[0].min(p.x), lo[1].min(p.y)];
        hi = [hi[0].max(p.x), hi[1].max(p.y)];
    }
    let at = |k: usize| ((lo[k] + hi[k]) / 2.0 / CELL_M).floor() as i32;
    let cell = cells.entry((at(0), at(1))).or_default();
    cell.extend(points);
    Some(cell)
}

/// Sorts a tile's visible nodes and sketches into cells. Nothing is indexed yet.
fn partition(doc: &Document) -> Cells {
    let mut cells = Cells::new();
    for node in doc.top_level_nodes() {
        if node_visible(doc, node)
            && let Some(cell) = place(&mut cells, node_points(doc, node))
        {
            cell.nodes.push(node);
        }
    }
    for id in doc.visible_sketch_ids() {
        let Some(sk) = doc.sketch(id) else { continue };
        let points = sk
            .edges()
            .values()
            .flat_map(|e| [e.from, e.to])
            .map(|v| sk.vertices()[v].position)
            .collect();
        if let Some(cell) = place(&mut cells, points) {
            cell.sketches.push(id);
        }
    }
    cells
}

/// The inference over one cell. Project axes are the base scene's job.
fn build_cell(doc: &Document, cell: &Cell) -> InferenceScene {
    let mut s = InferenceScene::new();
    s.set_axes_enabled(false);
    for &node in &cell.nodes {
        for o in doc.leaf_objects_under(node) {
            if let Some(obj) = doc.object(o)
                && node_visible(doc, NodeId::Object(o))
            {
                s.add_object(o, obj, &Transform::IDENTITY);
            }
        }
        for inst in doc.leaf_instances_under(node) {
            let (Some(def), Some(pose)) = (doc.instance_def(inst), doc.instance_pose(inst)) else {
                continue;
            };
            if !node_visible(doc, NodeId::Instance(inst)) {
                continue;
            }
            let shown = doc.expanded_def_placements_shown(def);
            for ((m, local), shown) in doc.expanded_def_placements(def).into_iter().zip(shown) {
                if shown && let Some(obj) = doc.object(m) {
                    if !s.has_def_member(m) {
                        s.set_def_member(m, obj);
                    }
                    s.add_placement(inst, m, &local.then(&pose));
                }
            }
        }
    }
    for &id in &cell.sketches {
        let Some(sk) = doc.sketch(id) else { continue };
        let pos = |v: kernel::SketchVertexId| sk.vertices()[v].position;
        let edges: Vec<_> = sk
            .edges()
            .iter()
            .map(|(e, edge)| (e, edge.curve, pos(edge.from), pos(edge.to)))
            .collect();
        s.add_sketch_edges(id, &edges);
        let mut seen = BTreeSet::new();
        let verts: Vec<_> = sk
            .edges()
            .values()
            .flat_map(|e| [e.from, e.to])
            .filter(|v| seen.insert(*v))
            .map(|v| (v, pos(v)))
            .collect();
        s.add_sketch_vertices(id, &verts);
        let faces: Vec<_> = sk
            .regions()
            .iter()
            .filter(|(_, r)| !r.faceless)
            .map(|(rid, r)| inference::SketchRegionFace {
                region: rid,
                plane: sk.plane(),
                boundary: r.outer.iter().map(|v| pos(*v)).collect(),
                holes: r
                    .holes
                    .iter()
                    .map(|h| h.iter().map(|v| pos(*v)).collect())
                    .collect(),
            })
            .collect();
        s.add_sketch_faces(id, &faces);
    }
    s
}

impl Scene {
    pub fn new(project: Project) -> Self {
        Scene {
            project,
            cells: BTreeMap::new(),
            base: InferenceScene::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            dirty: BTreeSet::new(),
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    /// Tiles changed since the last call, for the renderer to rebuild.
    pub fn take_dirty(&mut self) -> BTreeSet<TileKey> {
        std::mem::take(&mut self.dirty)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Tiles whose geometry can lie near the ray in plan view.
    fn tiles_near(&self, ray: &PickRay, cone_tan: f64) -> Vec<TileKey> {
        let o = [ray.origin.x, ray.origin.y];
        let d = [ray.direction.x, ray.direction.y];
        let dd = d[0] * d[0] + d[1] * d[1];
        self.project
            .tiles
            .keys()
            .copied()
            .filter(|key| {
                let c = key.center();
                let v = [c[0] - o[0], c[1] - o[1]];
                let t = if dd > 0.0 {
                    ((v[0] * d[0] + v[1] * d[1]) / dd).max(0.0)
                } else {
                    0.0
                };
                let gap = (v[0] - t * d[0]).hypot(v[1] - t * d[1]);
                let far = v[0].hypot(v[1]).hypot(ray.origin.z) + KERNEL_FRAME_BOUND_M;
                gap <= KERNEL_FRAME_BOUND_M + cone_tan * far
            })
            .collect()
    }

    /// Runs `visit` on every cell near the unit ray, indexing it first if
    /// needed. `visit` gets the tile and the cell's index.
    fn for_cells_near(
        &mut self,
        ray: &PickRay,
        cone_tan: f64,
        mut visit: impl FnMut(TileKey, &InferenceScene),
    ) {
        for key in self.tiles_near(ray, cone_tan) {
            let doc = &self.project.tiles[&key];
            let origin = offset(ray.origin, key.center());
            let cells = self.cells.entry(key).or_insert_with(|| partition(doc));
            for cell in cells.values_mut() {
                if cell.near(origin, ray.direction, cone_tan) {
                    if cell.scene.is_none() {
                        cell.scene = Some(build_cell(doc, cell));
                    }
                    visit(key, cell.scene.as_ref().expect("just built"));
                }
            }
        }
    }

    /// Hew's snap resolve across tiles. `query` is project-local, cone aperture.
    ///
    /// Each cell's answer is already right about that cell; what crosses cells
    /// is the choice between answers and occlusion. Answers rank as Hew ranks
    /// inside one scene (group, then weighted angle in ties of one pixel, then
    /// kind, then depth),
    /// except that Hew's "reach never steals" guard is not visible from outside.
    pub fn snap(&mut self, query: &SnapQuery) -> Option<Hit> {
        let dir = query.ray.direction.normalized().ok()?;
        let ray = PickRay {
            origin: query.ray.origin,
            direction: dir,
        };
        let reach = SnapKind::ALL
            .iter()
            .map(|k| query.weights.reach(*k, 1.0))
            .fold(1.0, f64::max);
        // Past 1.5 rad the cone is nearly a half-space: rule nothing out.
        let half_angle = query.aperture * reach;
        let cone_tan = if half_angle < 1.5 {
            half_angle.tan()
        } else {
            f64::INFINITY
        };

        let mut found: Vec<(u8, f64, f64, Hit, Point3)> = Vec::new();
        // Hew ignores a lock without an anchor.
        let locked = query.lock.is_some() && query.anchor.is_some();
        let mut offer = |key: TileKey, mut snap: Snap| {
            let c = key.center();
            snap.position = onset(snap.position, c);
            snap.projected_from = snap.projected_from.map(|p| onset(p, c));
            // Under a lock the position is on the line; what the cursor aimed
            // at is where it was projected from.
            let aim = snap.projected_from.unwrap_or(snap.position);
            let to = aim - ray.origin;
            let depth = to.dot(dir);
            let angle = to.cross(dir).length().atan2(depth) / query.weights.weight(snap.kind);
            // Every scene answers a lock with its bare line when it has nothing:
            // that answer must not beat real geometry from another cell.
            let rank = if locked && snap.projected_from.is_none() {
                u8::MAX
            } else {
                snap.kind.rank_group()
            };
            found.push((rank, angle, depth, Hit { tile: key, snap }, aim));
        };
        let unit = SnapQuery { ray, ..*query };
        if let Some(snap) = self.base.resolve(&unit) {
            offer(TileKey::ORIGIN, snap);
        }
        self.for_cells_near(&ray, cone_tan, |key, scene| {
            let c = key.center();
            let plane = query
                .constraint_plane
                .and_then(|p| shifted_plane(p, c).ok());
            let local = SnapQuery {
                ray: PickRay {
                    origin: offset(ray.origin, c),
                    direction: dir,
                },
                anchor: query.anchor.map(|a| offset(a, c)),
                constraint_plane: plane,
                ..*query
            };
            if let Some(snap) = scene.resolve(&local) {
                offer(key, snap);
            }
        });
        found.sort_by(|a, b| {
            let bucket = |d: f64| rank_distance_bucket(d, query.aperture * RANK_TIE_FRACTION);
            a.0.cmp(&b.0)
                .then(bucket(a.1).cmp(&bucket(b.1)))
                .then(a.3.snap.kind.cmp(&b.3.snap.kind))
                .then(a.2.total_cmp(&b.2))
        });
        for (rank, _, _, hit, aim) in found {
            let free =
                query.see_through_points || rank == u8::MAX || !self.is_occluded(ray.origin, aim);
            if free {
                return Some(hit);
            }
        }
        None
    }

    /// Whether a solid face in any cell stands between `eye` and `point`. Each
    /// cell judges its own faces too, but a face in another cell it cannot see.
    // ponytail: sketch regions do not occlude across cells; they are flat
    // drawings on the way to becoming solids.
    fn is_occluded(&mut self, eye: Point3, point: Point3) -> bool {
        let to = point - eye;
        let dist = to.length();
        let Ok(direction) = to.normalized() else {
            return false;
        };
        let ray = PickRay {
            origin: eye,
            direction,
        };
        self.nearest_face(&ray)
            .is_some_and(|f| f.depth < dist - (1e-6 * dist).max(1e-3))
    }

    fn nearest_face(&mut self, ray: &PickRay) -> Option<FaceHit> {
        let mut best: Option<FaceHit> = None;
        self.for_cells_near(ray, 0.0, |key, scene| {
            let local = PickRay {
                origin: offset(ray.origin, key.center()),
                direction: ray.direction,
            };
            if let Some((source, depth)) = scene.pick_face(&local)
                && best.is_none_or(|b| depth < b.depth)
            {
                best = Some(FaceHit {
                    tile: key,
                    source,
                    depth,
                });
            }
        });
        best
    }

    /// The nearest solid face under a project-local ray.
    pub fn pick_face(&mut self, ray: &PickRay) -> Option<FaceHit> {
        let unit = PickRay {
            origin: ray.origin,
            direction: ray.direction.normalized().ok()?,
        };
        self.nearest_face(&unit)
    }

    /// A solid face's plane and outer boundary, project-local. `None` for a
    /// stale id, a non-face element, or a face inside a component.
    pub fn face_shape(&self, key: TileKey, source: SnapSource) -> Option<(Plane, Vec<Point3>)> {
        let inference::ElementRef::Face(face) = source.element else {
            return None;
        };
        if source.instance.is_some() {
            return None;
        }
        let object = self.project.tiles.get(&key)?.object(source.object)?;
        let f = object.faces().get(face)?;
        let c = key.center();
        let up = |p: Point3| Point3::new(p.x + c[0], p.y + c[1], p.z + c[2]);
        let plane = Plane::from_point_normal(up(f.plane.point()), f.plane.normal()).ok()?;
        Some((plane, object.loop_positions(f.outer_loop).map(up).collect()))
    }

    /// A sketch region's plane and outer boundary, project-local.
    pub fn region_shape(
        &self,
        key: TileKey,
        sketch: SketchId,
        region: SketchRegionId,
    ) -> Option<(Plane, Vec<Point3>)> {
        let sk = self.project.tiles.get(&key)?.sketch(sketch)?;
        let r = sk.regions().get(region)?;
        let c = key.center();
        let up = |p: Point3| Point3::new(p.x + c[0], p.y + c[1], p.z + c[2]);
        let plane = Plane::from_point_normal(up(sk.plane().point()), sk.plane().normal()).ok()?;
        Some((
            plane,
            r.outer
                .iter()
                .map(|v| up(sk.vertices()[*v].position))
                .collect(),
        ))
    }

    /// How many extrudable regions `sketch` has.
    pub fn region_count(&self, key: TileKey, sketch: SketchId) -> usize {
        self.project
            .tiles
            .get(&key)
            .and_then(|d| d.extrudable_regions(sketch).ok())
            .map_or(0, |r| r.len())
    }

    /// Lines to highlight a picked target, project-local.
    pub fn outline(&self, target: tool::Target) -> Vec<(Point3, Point3)> {
        let closed = |shape: Option<(Plane, Vec<Point3>)>| {
            shape.map_or_else(Vec::new, |(_, p)| {
                (0..p.len()).map(|k| (p[k], p[(k + 1) % p.len()])).collect()
            })
        };
        match target {
            tool::Target::Region {
                tile,
                sketch,
                region,
            } => closed(self.region_shape(tile, sketch, region)),
            tool::Target::Solid { tile, source } => match source.element {
                inference::ElementRef::Face(_) => closed(self.face_shape(tile, source)),
                inference::ElementRef::Edge(edge) if source.instance.is_none() => {
                    let c = tile.center();
                    let up = |p: Point3| Point3::new(p.x + c[0], p.y + c[1], p.z + c[2]);
                    self.project
                        .tiles
                        .get(&tile)
                        .and_then(|d| d.object(source.object))
                        .and_then(|o| o.edge_endpoints(edge))
                        .map_or_else(Vec::new, |(a, b)| vec![(up(a), up(b))])
                }
                _ => Vec::new(),
            },
        }
    }

    /// Records a finished edit on `key` as one project undo step.
    fn record(&mut self, key: TileKey) {
        self.undo.push(key);
        self.redo.clear();
        self.touch(key);
    }

    fn touch(&mut self, key: TileKey) {
        self.cells.remove(&key);
        self.dirty.insert(key);
    }

    /// Draws `segments` (project-local, on `plane`) as ONE undo step through
    /// Hew's drafting engine: whatever is drawn on that plane joins up, so a
    /// shape on a solid's face splits the face, and lines meeting free drawing
    /// connect to it. `tile`: where the plane's drawing lives, or `None` for
    /// the tile of the first point. Refused edits leave nothing behind.
    pub fn draw(
        &mut self,
        tile: Option<TileKey>,
        plane: Plane,
        segments: &[(Point3, Point3)],
    ) -> Result<Drawn, EditError> {
        let first = segments
            .first()
            .ok_or(EditError::Unsupported("gambar kosong"))?
            .0;
        let key = tile.unwrap_or_else(|| TileKey::of([first.x, first.y]));
        let c = key.center();
        let mut local = Vec::with_capacity(segments.len());
        for (a, b) in segments {
            for p in [a, b] {
                check_local([p.x, p.y, p.z]).map_err(|_| EditError::OutOfProject)?;
                let reach_m = (p.x - c[0]).hypot(p.y - c[1]);
                if reach_m > KERNEL_FRAME_BOUND_M {
                    return Err(EditError::OutOfTile { tile: key, reach_m });
                }
            }
            local.push((offset(*a, c), offset(*b, c)));
        }
        let plane = shifted_plane(plane, c).map_err(kernel_err)?;
        let existed = self.project.tiles.contains_key(&key);
        let doc = self.project.tile_mut(key)?;
        let objects = doc.drawable_objects_on_plane(plane, None);
        let mut closed = false;
        let drafted = doc.draft(plane, None, &objects, "draw", |sk| {
            for (a, b) in &local {
                closed |= !sk.add_segment(*a, *b)?.regions_created.is_empty();
            }
            Ok(())
        });
        let report = match drafted {
            Ok((report, _)) => report,
            Err(e) => {
                if !existed {
                    self.project.tiles.remove(&key);
                }
                return Err(kernel_err(e));
            }
        };
        // Segments lying on existing edges change nothing: Hew records no step,
        // so neither may the project.
        if !report.objects.is_empty() || !report.sketches.is_empty() {
            self.record(key);
        }
        Ok(Drawn {
            tile: key,
            sketch: report.sketches.last().copied(),
            closed,
        })
    }

    /// What Move would pick up at `hit`: the top-level node of a solid, or the
    /// connected drawing of a sketch. `None` over nothing movable.
    pub fn movable_at(&self, hit: &Hit) -> Option<Movable> {
        let doc = self.project.tiles.get(&hit.tile)?;
        let tile = hit.tile;
        if let Some(source) = hit.snap.source {
            let node = doc
                .top_level_nodes()
                .into_iter()
                .find(|n| match source.instance {
                    Some(inst) => doc.leaf_instances_under(*n).contains(&inst),
                    None => doc.leaf_objects_under(*n).contains(&source.object),
                })?;
            return Some(Movable::Node { tile, node });
        }
        let at = offset(hit.snap.position, tile.center());
        let on_sketch = |sketch: SketchId| -> Option<kernel::SketchEdgeId> {
            let sk = doc.sketch(sketch)?;
            if let Some((_, edge)) = hit.snap.sketch_source.filter(|s| s.0 == sketch) {
                return Some(edge);
            }
            let corner = match hit.snap.sketch_region_source.filter(|s| s.0 == sketch) {
                Some((_, region)) => *sk.regions().get(region)?.outer.first()?,
                // A corner comes with no provenance: find it by position.
                None if hit.snap.kind == SnapKind::Endpoint => {
                    sk.vertices()
                        .iter()
                        .find(|(_, v)| (v.position - at).length() <= IN_PLANE)?
                        .0
                }
                None => return None,
            };
            sk.edges()
                .iter()
                .find(|(_, e)| e.from == corner || e.to == corner)
                .map(|(id, _)| id)
        };
        let (sketch, edge) = doc
            .visible_sketch_ids()
            .into_iter()
            .find_map(|s| on_sketch(s).map(|e| (s, e)))?;
        let sk = doc.sketch(sketch)?;
        let island = sk.island_of_edge(edge)?;
        let segments = sk.islands().get(island)?.edges.iter().map(|e| {
            let e = &sk.edges()[*e];
            (sk.vertices()[e.from].position, sk.vertices()[e.to].position)
        });
        Some(Movable::Island {
            tile,
            sketch,
            island,
            segments: segments.collect(),
        })
    }

    /// The edges of `m`, project-local, for the preview.
    pub fn movable_lines(&self, m: &Movable) -> Vec<(Point3, Point3)> {
        let c = m.tile().center();
        let Some(doc) = self.project.tiles.get(&m.tile()) else {
            return Vec::new();
        };
        let mut lines = Vec::new();
        match m {
            Movable::Island { segments, .. } => lines.extend(segments.iter().copied()),
            Movable::Node { node, .. } => {
                let mut push = |o: ObjectId, t: Transform| {
                    let Some(obj) = doc.object(o) else { return };
                    lines.extend(obj.edges().keys().filter_map(|e| {
                        let (a, b) = obj.edge_endpoints(e)?;
                        Some((t.apply_point(a), t.apply_point(b)))
                    }));
                };
                for o in doc.leaf_objects_under(*node) {
                    push(o, Transform::IDENTITY);
                }
                for inst in doc.leaf_instances_under(*node) {
                    let (Some(def), Some(pose)) = (doc.instance_def(inst), doc.instance_pose(inst))
                    else {
                        continue;
                    };
                    for (member, local) in doc.expanded_def_placements(def) {
                        push(member, local.then(&pose));
                    }
                }
            }
        }
        lines.truncate(MAX_PREVIEW_LINES);
        lines
            .into_iter()
            .map(|(a, b)| (onset(a, c), onset(b, c)))
            .collect()
    }

    /// Refuses a displacement carrying `m` past the tile or project bound.
    fn check_moved(&self, m: &Movable, delta: Vec3) -> Result<(), EditError> {
        let key = m.tile();
        let doc = self
            .project
            .tiles
            .get(&key)
            .ok_or(EditError::Kernel("tile hilang".into()))?;
        let points: Vec<Point3> = match m {
            Movable::Node { node, .. } => node_points(doc, *node),
            Movable::Island { segments, .. } => {
                segments.iter().flat_map(|(a, b)| [*a, *b]).collect()
            }
        };
        let c = key.center();
        for p in points {
            let q = p + delta;
            let reach_m = q.x.hypot(q.y);
            if reach_m > KERNEL_FRAME_BOUND_M {
                return Err(EditError::OutOfTile { tile: key, reach_m });
            }
            let local = onset(q, c);
            check_local([local.x, local.y, local.z]).map_err(|_| EditError::OutOfProject)?;
        }
        Ok(())
    }

    /// Moves `m` by `delta` as one undo step. It stays in its tile, up to the
    /// kernel frame bound.
    pub fn translate(&mut self, m: &Movable, delta: Vec3) -> Result<(), EditError> {
        self.check_moved(m, delta)?;
        let t = Transform::translation(delta);
        let doc = self.project.tile_mut(m.tile())?;
        match m {
            Movable::Node { node, .. } => doc
                .transform_selection(&[*node], &[], &t)
                .map_err(kernel_err)?,
            Movable::Island { sketch, island, .. } => doc
                .transform_sketch_island(*sketch, *island, &t)
                .map_err(kernel_err)?,
        };
        self.record(m.tile());
        Ok(())
    }

    /// Makes `count` copies of `m`, each `step` past the last. One undo step,
    /// except drawing copied off its plane: one per copy.
    pub fn copy(&mut self, m: &Movable, step: Vec3, count: NonZeroU32) -> Result<(), EditError> {
        self.check_moved(m, step * f64::from(count.get()))?;
        let key = m.tile();
        let doc = self.project.tile_mut(key)?;
        match m {
            Movable::Node { node, .. } => {
                doc.duplicate_nodes_array(&[*node], &Transform::translation(step), count)
                    .map_err(kernel_err)?;
                self.record(key);
            }
            Movable::Island {
                sketch,
                island,
                segments,
                ..
            } => {
                let plane = doc
                    .sketch(*sketch)
                    .ok_or(EditError::Kernel("sketch hilang".into()))?
                    .plane();
                if plane.normal().dot(step).abs() <= IN_PLANE {
                    // One draft: the copies join the drawing and undo together.
                    let objects = doc.drawable_objects_on_plane(plane, None);
                    doc.draft(plane, None, &objects, "copy", |sk| {
                        for k in 1..=count.get() {
                            let d = step * f64::from(k);
                            for (a, b) in segments {
                                sk.add_segment(*a + d, *b + d)?;
                            }
                        }
                        Ok(())
                    })
                    .map_err(kernel_err)?;
                    self.record(key);
                } else {
                    for k in 1..=count.get() {
                        let t = Transform::translation(step * f64::from(k));
                        let doc = self.project.tile_mut(key)?;
                        doc.copy_sketch_islands(*sketch, &[*island], &t)
                            .map_err(kernel_err)?;
                        self.record(key);
                    }
                }
            }
        }
        Ok(())
    }

    /// Replaces the copy just made (the last undo step) by `count` copies at
    /// `step`, so copy-then-array is a single step. On refusal the first copy
    /// stays as it was.
    pub fn respace_copy(
        &mut self,
        m: &Movable,
        step: Vec3,
        count: NonZeroU32,
    ) -> Result<(), EditError> {
        self.check_moved(m, step * f64::from(count.get()))?;
        self.undo()?;
        let made = self.copy(m, step, count);
        if made.is_err() {
            self.redo()?;
        }
        made
    }

    /// Undoes the op just applied when it pushed the tile past the kernel
    /// frame bound.
    fn guard_reach(&mut self, key: TileKey) -> Result<(), EditError> {
        let doc = self.project.tile_mut(key)?;
        let reach_m = horizontal_reach(doc);
        if reach_m > KERNEL_FRAME_BOUND_M {
            doc.undo().map_err(kernel_err)?;
            // The op wiped the document's redo and the undo left itself there:
            // redoing from here would bring the refused edit back.
            self.redo.clear();
            return Err(EditError::OutOfTile { tile: key, reach_m });
        }
        Ok(())
    }

    fn check_distance(distance: f64) -> Result<(), EditError> {
        if distance.is_finite() && distance.abs() <= PROJECT_BOUND_M {
            Ok(())
        } else {
            Err(EditError::OutOfProject)
        }
    }

    /// Extrudes a closed sketch region along the sketch normal.
    pub fn extrude(
        &mut self,
        key: TileKey,
        sketch: SketchId,
        region: SketchRegionId,
        distance: f64,
    ) -> Result<ObjectId, EditError> {
        Self::check_distance(distance)?;
        let doc = self.project.tile_mut(key)?;
        let (object, _) = doc
            .extrude_region(sketch, region, distance)
            .map_err(kernel_err)?;
        self.guard_reach(key)?;
        self.record(key);
        Ok(object)
    }

    /// Pushes or pulls a solid's face. A push past the far wall cuts through.
    pub fn push_pull(
        &mut self,
        key: TileKey,
        source: SnapSource,
        distance: f64,
    ) -> Result<(), EditError> {
        Self::check_distance(distance)?;
        if source.instance.is_some() {
            return Err(EditError::Unsupported("mengedit isi komponen"));
        }
        let inference::ElementRef::Face(face) = source.element else {
            return Err(EditError::Unsupported("push/pull selain pada face"));
        };
        let doc = self.project.tile_mut(key)?;
        let object = doc
            .object(source.object)
            .ok_or(EditError::Kernel("objek hilang".into()))?;
        if object.push_pull_overshoots(face, distance) {
            doc.push_pull_through(source.object, face, distance)
                .map_err(kernel_err)?;
        } else {
            doc.apply_object_op(source.object, KernelOp::PushPull { face, distance })
                .map_err(kernel_err)?;
        }
        self.guard_reach(key)?;
        self.record(key);
        Ok(())
    }

    /// Undoes the latest edit, in whichever tile it happened.
    pub fn undo(&mut self) -> Result<Option<TileKey>, EditError> {
        let Some(key) = self.undo.last().copied() else {
            return Ok(None);
        };
        self.project.tile_mut(key)?.undo().map_err(kernel_err)?;
        self.undo.pop();
        self.redo.push(key);
        self.touch(key);
        Ok(Some(key))
    }

    /// Redoes the latest undone edit.
    pub fn redo(&mut self) -> Result<Option<TileKey>, EditError> {
        let Some(key) = self.redo.last().copied() else {
            return Ok(None);
        };
        self.project.tile_mut(key)?.redo().map_err(kernel_err)?;
        self.redo.pop();
        self.undo.push(key);
        self.touch(key);
        Ok(Some(key))
    }
}
