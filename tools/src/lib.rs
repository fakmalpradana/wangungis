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
    Document, KernelOp, Material, NodeId, ObjectId, Plane, Point3, Rgba8, SketchId, SketchIslandId,
    SketchRegionId, Transform, Vec3,
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
    /// Geometry outside what is open for editing.
    OutOfScope,
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
            EditError::OutOfScope => write!(
                f,
                "Di luar yang sedang diedit. Isi grup atau komponen: klik dua kali untuk masuk. Esc keluar lagi."
            ),
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
    let text = format!("{e:?}");
    if text.contains("ExplodeSessionScope") {
        return EditError::OutOfScope;
    }
    EditError::Kernel(text)
}

fn tag_path(name: &str) -> Result<Vec<String>, EditError> {
    let path: Vec<String> = name.split('/').map(str::trim).map(str::to_owned).collect();
    if path.is_empty() || path.iter().any(String::is_empty) {
        Err(EditError::Unsupported("nama tag kosong"))
    } else {
        Ok(path)
    }
}

/// Whether `node` may be edited now: a direct member of the open session, or
/// with none open, top-level. What sits inside a group or component needs
/// entering first.
fn in_scope(doc: &Document, node: NodeId) -> bool {
    match doc.session_direct_members() {
        Some(members) => members.contains(&node),
        None => doc.node_parent(node).is_none(),
    }
}

/// [`in_scope`] for the solid a pick landed on.
fn source_in_scope(doc: &Document, source: SnapSource) -> bool {
    source.instance.is_none() && in_scope(doc, NodeId::Object(source.object))
}

/// Whether Hew's drafting takes `q` for the plane `p`. Its own test is private,
/// and this has to say the same: same-facing and coincident.
fn planes_coincide(p: &Plane, q: &Plane) -> bool {
    let anchor = Point3::ORIGIN + p.normal() * (-p.signed_distance(Point3::ORIGIN));
    (p.normal() - q.normal()).length() < kernel::tol::NORMAL_DIRECTION
        && q.signed_distance(anchor).abs() <= kernel::tol::PLANE_DIST
}

/// The solids on `plane` that a drawing may split.
fn drawable(doc: &Document, plane: Plane) -> Vec<ObjectId> {
    let mut objects = doc.drawable_objects_on_plane(plane, None);
    objects.retain(|o| in_scope(doc, NodeId::Object(*o)));
    objects
}

/// A snap in project-local coordinates, and the tile whose geometry produced
/// it: `snap.source` and the sketch ids in `snap` are valid in that tile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    pub tile: TileKey,
    pub snap: Snap,
}

/// A flat color for the Paint Bucket.
#[derive(Debug, Clone, PartialEq)]
pub struct Paint {
    pub name: String,
    pub color: Rgba8,
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

/// What the Eraser would remove at a snap.
#[derive(Debug, Clone, PartialEq)]
pub enum Erasable {
    /// A whole top-level solid: its edge is no drawing, so nothing less is valid.
    Node { tile: TileKey, node: NodeId },
    /// A drawn line, free or dividing a solid's face. Tile-local.
    Line {
        tile: TileKey,
        plane: Plane,
        a: Point3,
        b: Point3,
    },
}

/// How far (m) either way from its origin a guide line is drawn and snapped to.
pub(crate) const GUIDE_REACH_M: f64 = 200.0;
/// Half-size (m) of the cross drawn for a guide point.
const GUIDE_CROSS_M: f64 = 0.25;

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
    guides: Vec<kernel::GuideId>,
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
    /// Edits in order: their tile, and how many Hew steps each took.
    undo: Vec<(TileKey, u32)>,
    redo: Vec<(TileKey, u32)>,
    dirty: BTreeSet<TileKey>,
    /// The drawing that belongs to the open session. Hew does not say for a
    /// group, except by closing it on a copy, so this is kept across hovers.
    open_sketches: Vec<SketchId>,
}

/// The sketches an open session owns. Costs a copy of the tile for a group.
// ponytail: once per edit in a session; incremental if big tiles feel it.
fn open_sketches(doc: &Document) -> Vec<SketchId> {
    // Innermost only: closing the frames around a group too would pack it away.
    if let Some(group) = doc.group_session_group() {
        let mut closed = doc.clone();
        return match closed.close_innermost_session() {
            Ok(_) => closed.group_member_sketches(group),
            Err(_) => Vec::new(),
        };
    }
    doc.explode_session_sketches().unwrap_or_default()
}

fn offset(p: Point3, c: [f64; 3]) -> Point3 {
    Point3::new(p.x - c[0], p.y - c[1], p.z - c[2])
}

fn onset(p: Point3, c: [f64; 3]) -> Point3 {
    Point3::new(p.x + c[0], p.y + c[1], p.z + c[2])
}

/// The plane of the two faces `edge` divides, when they lie on one plane facing
/// the same way: such an edge is drawing, any other holds the solid together.
fn dividing_plane(object: &kernel::Object, edge: kernel::EdgeId) -> Option<Plane> {
    let he = object.half_edges().values().find(|h| h.edge == edge)?;
    let twin = &object.half_edges()[he.twin?];
    let (f, g) = (
        object.loops()[he.loop_id].face,
        object.loops()[twin.loop_id].face,
    );
    let (p, q) = (object.faces()[f].plane, object.faces()[g].plane);
    let same = f != g
        && p.normal().dot(q.normal()) > 1.0 - IN_PLANE
        && p.signed_distance(q.point()).abs() <= IN_PLANE;
    same.then_some(p)
}

/// `t`, given in project-local coordinates, as seen from a tile centered at `c`.
fn tile_transform(t: &Transform, c: [f64; 3]) -> Transform {
    let to = Vec3::new(c[0], c[1], c[2]);
    Transform::translation(to)
        .then(t)
        .then(&Transform::translation(-to))
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
    for id in doc.guide_ids() {
        if !doc.guide_shown(id) {
            continue;
        }
        let Some(guide) = doc.guide(id) else { continue };
        if let Some(cell) = place(&mut cells, guide_extent(guide)) {
            cell.guides.push(id);
        }
    }
    cells
}

/// The two ends of what is drawn and snapped to for `guide`.
fn guide_extent(guide: &kernel::Guide) -> Vec<Point3> {
    match guide {
        kernel::Guide::Line {
            origin, direction, ..
        } => vec![
            *origin - *direction * GUIDE_REACH_M,
            *origin + *direction * GUIDE_REACH_M,
        ],
        kernel::Guide::Point { position } => vec![*position],
    }
}

/// Everything drawn connected to `path`, as a Follow Me route.
fn follow_route(
    doc: &Document,
    path: (SketchId, kernel::SketchEdgeId),
) -> Result<kernel::FollowMePath, EditError> {
    let sk = doc
        .sketch(path.0)
        .ok_or(EditError::Kernel("sketch hilang".into()))?;
    let island = sk
        .island_of_edge(path.1)
        .and_then(|i| sk.islands().get(i))
        .ok_or(EditError::Kernel("garis hilang".into()))?;
    Ok(kernel::FollowMePath::SketchEdges {
        sketch: path.0,
        edges: island.edges.clone(),
    })
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
    for &id in &cell.guides {
        if let Some(guide) = doc.guide(id) {
            s.add_guide(id, guide);
        }
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
            open_sketches: Vec::new(),
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    /// Tiles changed since the last call, for the renderer to rebuild.
    pub fn take_dirty(&mut self) -> BTreeSet<TileKey> {
        std::mem::take(&mut self.dirty)
    }

    /// Every tag in the project, sorted by its slash-separated path. Walks every
    /// node: call it when something changed, not every frame.
    pub fn tags(&self) -> Vec<(String, bool)> {
        let mut tags = BTreeMap::<Vec<String>, bool>::new();
        for doc in self.project.tiles.values() {
            for (path, hidden) in doc.tag_meta() {
                tags.entry(path.to_vec())
                    .and_modify(|h| *h |= hidden)
                    .or_insert(hidden);
            }
            // Repeats are harmless, and a dedup here would be quadratic in a city.
            let mut nodes: Vec<NodeId> = doc
                .live_object_ids()
                .into_iter()
                .map(NodeId::Object)
                .chain(doc.group_ids().into_iter().map(NodeId::Group))
                .chain(doc.instance_ids().into_iter().map(NodeId::Instance))
                .collect();
            for component in doc.component_ids() {
                nodes.extend(doc.def_member_nodes(component).unwrap_or_default());
            }
            let mut at = 0;
            while at < nodes.len() {
                if let NodeId::Group(group) = nodes[at] {
                    nodes.extend(doc.group_members(group).unwrap_or_default());
                }
                at += 1;
            }
            for node in nodes {
                for path in doc.node_tags(node) {
                    let hidden = doc.tag_hidden(path);
                    tags.entry(path.clone())
                        .and_modify(|h| *h |= hidden)
                        .or_insert(hidden);
                }
            }
        }
        tags.into_iter()
            .map(|(path, hidden)| (path.join("/"), hidden))
            .collect()
    }

    /// Sets tag visibility in every tile. This is view state, not undoable.
    pub fn set_tag_hidden(&mut self, name: &str, hidden: bool) -> Result<(), EditError> {
        let path = tag_path(name)?;
        let tiles: Vec<_> = self.project.tiles.keys().copied().collect();
        for tile in tiles {
            self.project
                .tiles
                .get_mut(&tile)
                .expect("listed tile")
                .set_tag_hidden(path.clone(), hidden);
            self.touch(tile);
        }
        Ok(())
    }

    /// Gives one tag to selected nodes as one project undo step.
    pub fn tag_nodes(
        &mut self,
        tile: TileKey,
        nodes: &[NodeId],
        name: &str,
    ) -> Result<(), EditError> {
        self.edit_node_tag(tile, nodes, name, true)
    }

    /// Takes one tag from selected nodes as one project undo step.
    pub fn untag_nodes(
        &mut self,
        tile: TileKey,
        nodes: &[NodeId],
        name: &str,
    ) -> Result<(), EditError> {
        self.edit_node_tag(tile, nodes, name, false)
    }

    fn edit_node_tag(
        &mut self,
        tile: TileKey,
        nodes: &[NodeId],
        name: &str,
        add: bool,
    ) -> Result<(), EditError> {
        self.check_nodes(tile, nodes)?;
        let path = tag_path(name)?;
        let doc = self.project.tile_mut(tile)?;
        let mut made = 0;
        for &node in nodes {
            let has = doc.node_tags(node).contains(&path);
            let result = if add {
                doc.add_node_tag(node, path.clone())
            } else {
                doc.remove_node_tag(node, &path)
            };
            if let Err(e) = result {
                for _ in 0..made {
                    doc.undo().map_err(kernel_err)?;
                }
                self.redo.clear();
                return Err(kernel_err(e));
            }
            if has != add {
                made += 1;
            }
        }
        if made > 0 {
            self.record_steps(tile, made);
        }
        Ok(())
    }

    /// Renames a tag across every tile, one project undo step per changed tile.
    pub fn rename_tag(&mut self, old: &str, new: &str) -> Result<(), EditError> {
        if self.session().is_some() {
            return Err(EditError::OutOfScope);
        }
        let old = tag_path(old)?;
        let new = tag_path(new)?;
        self.edit_project_tags(|doc| doc.rename_tag(&old, new.clone()))
    }

    /// Deletes a tag across every tile, one project undo step per changed tile.
    pub fn delete_tag(&mut self, name: &str) -> Result<(), EditError> {
        if self.session().is_some() {
            return Err(EditError::OutOfScope);
        }
        let path = tag_path(name)?;
        self.edit_project_tags(|doc| doc.delete_tag(&path))
    }

    fn edit_project_tags(
        &mut self,
        mut edit: impl FnMut(&mut Document) -> Result<kernel::DocChange, kernel::DocumentError>,
    ) -> Result<(), EditError> {
        let tiles: Vec<_> = self.project.tiles.keys().copied().collect();
        let mut changed = Vec::new();
        for tile in tiles {
            let doc = self.project.tiles.get_mut(&tile).expect("listed tile");
            let before = doc.save();
            if let Err(e) = edit(doc) {
                for tile in changed.into_iter().rev() {
                    self.project
                        .tiles
                        .get_mut(&tile)
                        .expect("changed tile")
                        .undo()
                        .map_err(kernel_err)?;
                }
                self.redo.clear();
                return Err(kernel_err(e));
            }
            if doc.save() != before {
                changed.push(tile);
            }
        }
        for tile in changed {
            self.record(tile);
        }
        Ok(())
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

    /// The nearest solid face under a project-local ray, whatever it belongs to.
    pub fn pick_face(&mut self, ray: &PickRay) -> Option<FaceHit> {
        let unit = PickRay {
            origin: ray.origin,
            direction: ray.direction.normalized().ok()?,
        };
        self.nearest_face(&unit)
    }

    /// [`Scene::pick_face`], when that face may be edited now. A face that may
    /// not still hides what is behind it.
    pub fn editable_face(&mut self, ray: &PickRay) -> Option<FaceHit> {
        let hit = self.pick_face(ray)?;
        self.check_tile(hit.tile).ok()?;
        let doc = self.project.tiles.get(&hit.tile)?;
        source_in_scope(doc, hit.source).then_some(hit)
    }

    /// The open editing session: its tile, and what was entered, outermost first.
    pub fn session(&self) -> Option<(TileKey, Vec<NodeId>)> {
        self.project.tiles.iter().find_map(|(key, doc)| {
            let stack = doc.session_stack();
            (!stack.is_empty()).then_some((*key, stack))
        })
    }

    /// A name for `node`, for telling the user what is open.
    pub fn node_name(&self, tile: TileKey, node: NodeId) -> String {
        let doc = self.project.tiles.get(&tile);
        let named = doc.and_then(|doc| match node {
            NodeId::Object(o) => doc.object_name(o),
            NodeId::Group(g) => doc.group_name(g),
            NodeId::Instance(i) => doc
                .instance_name(i)
                .or_else(|| doc.component_name(doc.instance_def(i)?)),
        });
        let kind = match node {
            NodeId::Object(_) => "Objek",
            NodeId::Group(_) => "Grup",
            NodeId::Instance(_) => "Komponen",
        };
        named.map_or_else(|| kind.into(), Into::into)
    }

    /// Opens a group or component for editing in place, as one undo step. Its
    /// members become editable and everything else does not, until
    /// [`Scene::leave`]. Hew hides the other instances of a component meanwhile.
    pub fn enter(&mut self, tile: TileKey, node: NodeId) -> Result<(), EditError> {
        self.check_tile(tile)?;
        let doc = self.project.tile_mut(tile)?;
        if !in_scope(doc, node) {
            return Err(EditError::OutOfScope);
        }
        match node {
            NodeId::Group(g) => doc.open_group_session(g),
            NodeId::Instance(i) => doc.open_explode_session(i),
            NodeId::Object(_) => {
                return Err(EditError::Unsupported("objek biasa tidak punya isi"));
            }
        }
        .map_err(kernel_err)?;
        self.record(tile);
        Ok(())
    }

    /// Closes the innermost open group or component, as one undo step. `false`
    /// when none is open.
    pub fn leave(&mut self) -> Result<bool, EditError> {
        let Some((tile, _)) = self.session() else {
            return Ok(false);
        };
        let doc = self.project.tile_mut(tile)?;
        doc.close_innermost_session().map_err(kernel_err)?;
        self.record(tile);
        Ok(true)
    }

    /// Whether a drawing may be edited now: the open session's own, or with
    /// none open, one that sits in no group.
    pub fn sketch_editable(&self, key: TileKey, sketch: SketchId) -> bool {
        if self.check_tile(key).is_err() {
            return false;
        }
        self.project.tiles.get(&key).is_some_and(|doc| {
            if let Some(owner) = doc.sketch_owner_object(sketch) {
                in_scope(doc, NodeId::Object(owner))
            } else if doc.session_stack().is_empty() {
                doc.sketch_owner_group(sketch).is_none()
            } else {
                self.open_sketches.contains(&sketch)
            }
        })
    }

    /// Hew's drafting merges every sketch on a plane into one, a group's own among
    /// them, and closing a session folds in only sketches it created. So a drawing
    /// on a plane that holds one out of scope would land on the wrong side of the
    /// group, and could change it. Refused.
    // ponytail: a separate sketch on that plane, if this gets in the way.
    fn check_open_plane(&self, key: TileKey, plane: Plane) -> Result<(), EditError> {
        let Some(doc) = self.project.tiles.get(&key) else {
            return Ok(());
        };
        let taken = doc.visible_sketch_ids().into_iter().any(|id| {
            !self.sketch_editable(key, id)
                && doc
                    .sketch(id)
                    .is_some_and(|sk| planes_coincide(&plane, &sk.plane()))
        });
        if taken {
            return Err(EditError::Unsupported(
                "menggambar pada bidang yang berisi gambar di sisi lain batas grup",
            ));
        }
        Ok(())
    }

    fn check_sketch(&self, key: TileKey, sketch: SketchId) -> Result<(), EditError> {
        if self.sketch_editable(key, sketch) {
            Ok(())
        } else {
            Err(EditError::OutOfScope)
        }
    }

    /// Refuses edits in any tile but the one with the open session.
    fn check_tile(&self, key: TileKey) -> Result<(), EditError> {
        match self.session() {
            Some((open, _)) if open != key => Err(EditError::OutOfScope),
            _ => Ok(()),
        }
    }

    /// Refuses nodes outside the open scope.
    fn check_nodes(&self, key: TileKey, nodes: &[NodeId]) -> Result<(), EditError> {
        self.check_tile(key)?;
        match self.project.tiles.get(&key) {
            Some(doc) if !nodes.iter().all(|n| in_scope(doc, *n)) => Err(EditError::OutOfScope),
            _ => Ok(()),
        }
    }

    /// Refuses a solid outside the open scope.
    fn check_source(&self, key: TileKey, source: SnapSource) -> Result<(), EditError> {
        match self.project.tiles.get(&key) {
            Some(doc) if !source_in_scope(doc, source) => Err(EditError::OutOfScope),
            _ => Ok(()),
        }
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
        self.record_steps(key, 1);
    }

    /// Records `steps` Hew steps on `key` as one project undo step.
    fn record_steps(&mut self, key: TileKey, steps: u32) {
        let hidden: Vec<Vec<String>> = self
            .project
            .tiles
            .values()
            .flat_map(|doc| {
                doc.tag_meta()
                    .filter(|(_, hidden)| *hidden)
                    .map(|(p, _)| p.to_vec())
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if !hidden.is_empty() {
            let doc = self.project.tiles.get_mut(&key).expect("edited tile");
            for path in hidden {
                if !doc.tag_hidden(&path) {
                    doc.set_tag_hidden(path, true);
                }
            }
        }
        self.undo.push((key, steps));
        self.redo.clear();
        self.touch(key);
    }

    fn touch(&mut self, key: TileKey) {
        self.cells.remove(&key);
        self.dirty.insert(key);
        self.open_sketches = self
            .session()
            .and_then(|(open, _)| self.project.tiles.get(&open))
            .map(open_sketches)
            .unwrap_or_default();
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
        self.run_draft(key, plane, "draw", |sk| {
            let mut closed = false;
            for (a, b) in &local {
                closed |= !sk.add_segment(*a, *b)?.regions_created.is_empty();
            }
            Ok(closed)
        })
    }

    /// One draft on tile `key` as one undo step. `plane` is tile-local; `edit`
    /// answers whether a region closed.
    fn run_draft(
        &mut self,
        key: TileKey,
        plane: Plane,
        label: &str,
        edit: impl FnOnce(&mut kernel::Sketch) -> Result<bool, kernel::SketchError>,
    ) -> Result<Drawn, EditError> {
        let existed = self.project.tiles.contains_key(&key);
        self.check_tile(key)?;
        self.check_open_plane(key, plane)?;
        let doc = self.project.tile_mut(key)?;
        let objects = drawable(doc, plane);
        let mut closed = false;
        let drafted = doc.draft(plane, None, &objects, label, |sk| {
            closed = edit(sk)?;
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

    /// Draws a circle, polygon or arc (`points`, project-local, in order; closed
    /// back to the first when `closed`) on the circle at `center`, as one curve
    /// and one undo step.
    pub fn draw_curve(
        &mut self,
        tile: TileKey,
        plane: Plane,
        points: &[Point3],
        closed: bool,
        center: Point3,
        kind: kernel::SketchCurveKind,
    ) -> Result<Drawn, EditError> {
        let c = tile.center();
        let radius = (points.first().copied().unwrap_or(center) - center).length();
        let mut local = Vec::with_capacity(points.len());
        for p in points {
            check_local([p.x, p.y, p.z]).map_err(|_| EditError::OutOfProject)?;
            let reach_m = (p.x - c[0]).hypot(p.y - c[1]);
            if reach_m > KERNEL_FRAME_BOUND_M {
                return Err(EditError::OutOfTile { tile, reach_m });
            }
            local.push(offset(*p, c));
        }
        let plane = shifted_plane(plane, c).map_err(kernel_err)?;
        let geom = kernel::CurveGeom {
            center: offset(center, c),
            radius,
        };
        self.run_draft(tile, plane, "curve", |sk| {
            sk.begin_curve_with_kind(geom, kind)?;
            let mut made = false;
            let n = local.len();
            let spans = if closed { n } else { n.saturating_sub(1) };
            let mut result = Ok(());
            for k in 0..spans {
                match sk.add_segment(local[k], local[(k + 1) % n]) {
                    Ok(added) => made |= !added.regions_created.is_empty(),
                    Err(e) => {
                        result = Err(e);
                        break;
                    }
                }
            }
            sk.end_curve();
            result.map(|()| made)
        })
    }

    /// Offsets the region or face holding `at` by `distance`, as one undo step.
    /// Positive is outward. Project-local.
    pub fn offset(
        &mut self,
        tile: TileKey,
        plane: Plane,
        at: Point3,
        distance: f64,
    ) -> Result<Drawn, EditError> {
        Self::check_distance(distance)?;
        let c = tile.center();
        let plane = shifted_plane(plane, c).map_err(kernel_err)?;
        let at = offset(at, c);
        let at = at - plane.normal() * plane.signed_distance(at);
        self.run_draft(tile, plane, "offset", |sk| {
            let mut best = None;
            for (id, r) in sk.regions().iter() {
                if !r.faceless && sk.region_contains_point(id, at)? {
                    let area = sk.region_area(id)?;
                    if best.is_none_or(|(_, b)| area < b) {
                        best = Some((id, area));
                    }
                }
            }
            let (region, _) = best.ok_or(kernel::SketchError::UnknownRegion)?;
            sk.offset_region(region, distance)?;
            Ok(true)
        })
    }

    /// What Move would pick up at `hit`: the top-level node of a solid, or the
    /// connected drawing of a sketch. `None` over nothing movable.
    pub fn movable_at(&self, hit: &Hit) -> Option<Movable> {
        self.check_tile(hit.tile).ok()?;
        let doc = self.project.tiles.get(&hit.tile)?;
        let tile = hit.tile;
        if let Some(source) = hit.snap.source {
            let node = doc
                .top_level_nodes()
                .into_iter()
                .find(|n| match source.instance {
                    Some(inst) => doc.leaf_instances_under(*n).contains(&inst),
                    None => doc.leaf_objects_under(*n).contains(&source.object),
                })
                .filter(|n| in_scope(doc, *n))?;
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
            .filter(|s| self.sketch_editable(tile, *s))
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

    /// The center of `m`'s bounds, project-local: the pivot for scaling.
    pub fn movable_center(&self, m: &Movable) -> Option<Point3> {
        let doc = self.project.tiles.get(&m.tile())?;
        let points: Vec<Point3> = match m {
            Movable::Node { node, .. } => node_points(doc, *node),
            Movable::Island { segments, .. } => {
                segments.iter().flat_map(|(a, b)| [*a, *b]).collect()
            }
        };
        let (first, rest) = points.split_first()?;
        let (mut lo, mut hi) = (*first, *first);
        for p in rest {
            lo = Point3::new(lo.x.min(p.x), lo.y.min(p.y), lo.z.min(p.z));
            hi = Point3::new(hi.x.max(p.x), hi.y.max(p.y), hi.z.max(p.z));
        }
        let mid = Point3::new(
            (lo.x + hi.x) / 2.0,
            (lo.y + hi.y) / 2.0,
            (lo.z + hi.z) / 2.0,
        );
        Some(onset(mid, m.tile().center()))
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

    /// What the Eraser would remove at `hit`: only edges count, as faces belong
    /// to their edges. `None` over a face, a corner, or nothing.
    pub fn erasable_at(&self, hit: &Hit) -> Option<Erasable> {
        self.check_tile(hit.tile).ok()?;
        let tile = hit.tile;
        let doc = self.project.tiles.get(&tile)?;
        if let Some((sketch, edge)) = hit.snap.sketch_source {
            if !self.sketch_editable(tile, sketch) {
                return None;
            }
            let sk = doc.sketch(sketch)?;
            let e = sk.edges().get(edge)?;
            return Some(Erasable::Line {
                tile,
                plane: sk.plane(),
                a: sk.vertices()[e.from].position,
                b: sk.vertices()[e.to].position,
            });
        }
        let source = hit.snap.source?;
        let inference::ElementRef::Edge(edge) = source.element else {
            return None;
        };
        let node = |doc: &Document| {
            doc.top_level_nodes()
                .into_iter()
                .find(|n| match source.instance {
                    Some(inst) => doc.leaf_instances_under(*n).contains(&inst),
                    None => doc.leaf_objects_under(*n).contains(&source.object),
                })
        };
        if source_in_scope(doc, source) {
            let object = doc.object(source.object)?;
            let (a, b) = object.edge_endpoints(edge)?;
            if let Some(plane) = dividing_plane(object, edge) {
                return Some(Erasable::Line { tile, plane, a, b });
            }
        }
        Some(Erasable::Node {
            tile,
            node: node(doc).filter(|n| in_scope(doc, *n))?,
        })
    }

    /// Lines to highlight what would be erased, project-local.
    pub fn erasable_lines(&self, e: &Erasable) -> Vec<(Point3, Point3)> {
        match e {
            Erasable::Node { tile, node } => self.movable_lines(&Movable::Node {
                tile: *tile,
                node: *node,
            }),
            Erasable::Line { tile, a, b, .. } => {
                let c = tile.center();
                vec![(onset(*a, c), onset(*b, c))]
            }
        }
    }

    /// Removes `e` as one undo step. A line's faces fall with it: a loop it
    /// closed opens, and on a solid the two faces it divided merge.
    pub fn erase(&mut self, e: &Erasable) -> Result<(), EditError> {
        match e {
            Erasable::Node { tile, node } => {
                self.check_nodes(*tile, &[*node])?;
                let doc = self.project.tile_mut(*tile)?;
                doc.delete_node(*node).map_err(kernel_err)?;
                self.record(*tile);
            }
            Erasable::Line { tile, plane, a, b } => {
                self.check_tile(*tile)?;
                self.check_open_plane(*tile, *plane)?;
                let doc = self.project.tile_mut(*tile)?;
                let objects = drawable(doc, *plane);
                let (report, _) = doc
                    .draft(*plane, None, &objects, "erase", |sk| {
                        // Drafting may have split it where other drawing meets it.
                        let on = |p: Point3| {
                            let (u, v) = (*b - *a, p - *a);
                            let t = (v.dot(u) / u.dot(u)).clamp(0.0, 1.0);
                            (v - u * t).length() <= IN_PLANE
                        };
                        let pieces: Vec<_> = sk
                            .edges()
                            .iter()
                            .filter(|(_, e)| {
                                let (p, q) =
                                    (sk.vertices()[e.from].position, sk.vertices()[e.to].position);
                                on(p) && on(q)
                            })
                            .map(|(id, _)| id)
                            .collect();
                        for piece in pieces {
                            sk.remove_edge(piece)?;
                        }
                        Ok(())
                    })
                    .map_err(kernel_err)?;
                if !report.objects.is_empty() || !report.sketches.is_empty() {
                    self.record(*tile);
                }
            }
        }
        Ok(())
    }

    /// Refuses a transform whose `count`th power carries `m` past the tile or
    /// project bound, or any power on the way. `step` is project-local.
    fn check_moved(&self, m: &Movable, step: &Transform, count: u32) -> Result<(), EditError> {
        let key = m.tile();
        match m {
            Movable::Node { node, .. } => self.check_nodes(key, &[*node])?,
            Movable::Island { sketch, .. } => self.check_sketch(key, *sketch)?,
        }
        let doc = self
            .project
            .tiles
            .get(&key)
            .ok_or(EditError::Kernel("tile hilang".into()))?;
        let mut points: Vec<Point3> = match m {
            Movable::Node { node, .. } => node_points(doc, *node),
            Movable::Island { segments, .. } => {
                segments.iter().flat_map(|(a, b)| [*a, *b]).collect()
            }
        };
        let c = key.center();
        let local = tile_transform(step, c);
        for _ in 0..count {
            for p in &mut points {
                *p = local.apply_point(*p);
                let reach_m = p.x.hypot(p.y);
                if reach_m > KERNEL_FRAME_BOUND_M {
                    return Err(EditError::OutOfTile { tile: key, reach_m });
                }
                let q = onset(*p, c);
                check_local([q.x, q.y, q.z]).map_err(|_| EditError::OutOfProject)?;
            }
        }
        Ok(())
    }

    /// Applies `t` (project-local) to `m` as one undo step. It stays in its
    /// tile, up to the kernel frame bound.
    pub fn transform(&mut self, m: &Movable, t: &Transform) -> Result<(), EditError> {
        self.check_moved(m, t, 1)?;
        self.check_tile(m.tile())?;
        let local = tile_transform(t, m.tile().center());
        let doc = self.project.tile_mut(m.tile())?;
        match m {
            Movable::Node { node, .. } => doc
                .transform_selection(&[*node], &[], &local)
                .map_err(kernel_err)?,
            Movable::Island { sketch, island, .. } => doc
                .transform_sketch_island(*sketch, *island, &local)
                .map_err(kernel_err)?,
        };
        self.record(m.tile());
        Ok(())
    }

    /// Makes `count` copies of `m`, each `step` (project-local) past the last.
    /// One undo step.
    pub fn copy(
        &mut self,
        m: &Movable,
        step: &Transform,
        count: NonZeroU32,
    ) -> Result<(), EditError> {
        self.check_moved(m, step, count.get())?;
        let key = m.tile();
        let local = tile_transform(step, key.center());
        self.check_tile(key)?;
        let doc = self.project.tile_mut(key)?;
        match m {
            Movable::Node { node, .. } => {
                doc.duplicate_nodes_array(&[*node], &local, count)
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
                // Every copy of every point on the plane: the kernel can draft them.
                let stays = {
                    let mut placed: Vec<Point3> =
                        segments.iter().flat_map(|(a, b)| [*a, *b]).collect();
                    (0..count.get()).all(|_| {
                        placed.iter_mut().all(|p| {
                            *p = local.apply_point(*p);
                            plane.signed_distance(*p).abs() <= IN_PLANE
                        })
                    })
                };
                if stays {
                    // One draft: the copies join the drawing and undo together.
                    self.run_draft(key, plane, "copy", |sk| {
                        let mut placed = segments.clone();
                        for _ in 0..count.get() {
                            for (a, b) in &mut placed {
                                *a = local.apply_point(*a);
                                *b = local.apply_point(*b);
                                sk.add_segment(*a, *b)?;
                            }
                        }
                        Ok(false)
                    })?;
                } else {
                    // Hew takes one step per copy: the project counts them as one.
                    let mut t = Transform::IDENTITY;
                    let mut made = 0;
                    for _ in 0..count.get() {
                        t = t.then(&local);
                        if let Err(e) = doc.copy_sketch_islands(*sketch, &[*island], &t) {
                            for _ in 0..made {
                                doc.undo().map_err(kernel_err)?;
                            }
                            return Err(kernel_err(e));
                        }
                        made += 1;
                    }
                    self.record_steps(key, made);
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
        step: &Transform,
        count: NonZeroU32,
    ) -> Result<(), EditError> {
        self.check_moved(m, step, count.get())?;
        self.undo()?;
        let made = self.copy(m, step, count);
        if made.is_err() {
            self.redo()?;
        }
        made
    }

    /// Adds a construction point, as one undo step.
    pub fn add_guide_point(&mut self, at: Point3) -> Result<(), EditError> {
        let (tile, local) = self.guide_home(at)?;
        self.check_tile(tile)?;
        self.project
            .tile_mut(tile)?
            .add_guide_point(local)
            .map_err(kernel_err)?;
        self.record(tile);
        Ok(())
    }

    /// Adds a construction line through `at`, as one undo step.
    pub fn add_guide_line(&mut self, at: Point3, direction: Vec3) -> Result<(), EditError> {
        let (tile, local) = self.guide_home(at)?;
        self.check_tile(tile)?;
        self.project
            .tile_mut(tile)?
            .add_guide_line(local, direction)
            .map_err(kernel_err)?;
        self.record(tile);
        Ok(())
    }

    fn guide_home(&self, at: Point3) -> Result<(TileKey, Point3), EditError> {
        check_local([at.x, at.y, at.z]).map_err(|_| EditError::OutOfProject)?;
        let tile = TileKey::of([at.x, at.y]);
        Ok((tile, offset(at, tile.center())))
    }

    /// Removes every construction line and point, one undo step per tile.
    pub fn delete_guides(&mut self) -> Result<(), EditError> {
        // Guides of every tile go, and only the session's tile may change.
        if self.session().is_some() {
            return Err(EditError::OutOfScope);
        }
        let tiles: Vec<TileKey> = self
            .project
            .tiles
            .iter()
            .filter(|(_, d)| !d.guide_ids().is_empty())
            .map(|(k, _)| *k)
            .collect();
        for tile in tiles {
            self.project
                .tile_mut(tile)?
                .delete_all_guides()
                .map_err(kernel_err)?;
            self.record(tile);
        }
        Ok(())
    }

    /// Lines to draw for the construction geometry, project-local.
    pub fn guide_lines(&self) -> Vec<(Point3, Point3)> {
        let mut out = Vec::new();
        for (key, doc) in &self.project.tiles {
            let c = key.center();
            for id in doc.guide_ids() {
                if !doc.guide_shown(id) {
                    continue;
                }
                let Some(guide) = doc.guide(id) else { continue };
                match guide {
                    kernel::Guide::Line { .. } => {
                        let ends = guide_extent(guide);
                        out.push((onset(ends[0], c), onset(ends[1], c)));
                    }
                    kernel::Guide::Point { position } => {
                        let p = onset(*position, c);
                        for u in [
                            Vec3::new(GUIDE_CROSS_M, 0.0, 0.0),
                            Vec3::new(0.0, GUIDE_CROSS_M, 0.0),
                            Vec3::new(0.0, 0.0, GUIDE_CROSS_M),
                        ] {
                            out.push((p - u, p + u));
                        }
                    }
                }
            }
        }
        out
    }

    /// The direction of the edge under `hit`, when it lies on one: solid or
    /// drawn. `None` on a corner, a face or a guide.
    pub fn edge_direction(&self, hit: &Hit) -> Option<Vec3> {
        let doc = self.project.tiles.get(&hit.tile)?;
        let span = if let Some((sketch, edge)) = hit.snap.sketch_source {
            let sk = doc.sketch(sketch)?;
            let e = sk.edges().get(edge)?;
            sk.vertices()[e.to].position - sk.vertices()[e.from].position
        } else {
            let source = hit.snap.source?;
            let inference::ElementRef::Edge(edge) = source.element else {
                return None;
            };
            let (a, b) = doc.object(source.object)?.edge_endpoints(edge)?;
            match source.instance {
                None => b - a,
                Some(inst) => {
                    let pose = doc.instance_pose(inst)?;
                    let def = doc.instance_def(inst)?;
                    let placements = doc.expanded_def_placements(def);
                    let mut placed = placements.iter().filter(|(m, _)| *m == source.object);
                    let (_, local) = placed.next()?;
                    // A leaf placed twice in its definition: the snap does not say
                    // which placement, so the direction is not known.
                    if placed.next().is_some() {
                        return None;
                    }
                    local.then(&pose).apply_vector(b - a)
                }
            }
        };
        span.normalized().ok()
    }

    /// Sweeps a region along the drawing connected to `path_edge`, into a solid.
    /// Both must lie in `tile`. One undo step.
    pub fn follow_me(
        &mut self,
        tile: TileKey,
        profile: (SketchId, SketchRegionId),
        path: (SketchId, kernel::SketchEdgeId),
    ) -> Result<(), EditError> {
        self.check_tile(tile)?;
        self.check_sketch(tile, profile.0)?;
        self.check_sketch(tile, path.0)?;
        let doc = self.project.tile_mut(tile)?;
        let route = follow_route(doc, path)?;
        doc.follow_me(profile.0, profile.1, &route)
            .map_err(kernel_err)?;
        self.guard_reach(tile)?;
        self.record(tile);
        Ok(())
    }

    /// Sweeps a solid's face along the drawing connected to `path_edge`, merging
    /// the result with that solid when they touch. One undo step.
    pub fn follow_me_face(
        &mut self,
        tile: TileKey,
        source: SnapSource,
        path: (SketchId, kernel::SketchEdgeId),
    ) -> Result<(), EditError> {
        self.check_source(tile, source)?;
        self.check_sketch(tile, path.0)?;
        let inference::ElementRef::Face(face) = source.element else {
            return Err(EditError::Unsupported("follow me selain pada face"));
        };
        self.check_tile(tile)?;
        let doc = self.project.tile_mut(tile)?;
        let route = follow_route(doc, path)?;
        doc.follow_me_face(source.object, face, &route, None)
            .map_err(kernel_err)?;
        self.guard_reach(tile)?;
        self.record(tile);
        Ok(())
    }

    /// Groups `nodes`, top-level nodes of one tile, as one undo step.
    pub fn group(&mut self, tile: TileKey, nodes: &[NodeId]) -> Result<(), EditError> {
        self.check_nodes(tile, nodes)?;
        self.project
            .tile_mut(tile)?
            .group_nodes(nodes)
            .map_err(kernel_err)?;
        self.record(tile);
        Ok(())
    }

    /// Turns `nodes`, top-level nodes of one tile, into one instance of a new
    /// component, as one undo step.
    pub fn make_component(&mut self, tile: TileKey, nodes: &[NodeId]) -> Result<(), EditError> {
        self.check_nodes(tile, nodes)?;
        self.project
            .tile_mut(tile)?
            .make_component(nodes)
            .map_err(kernel_err)?;
        self.record(tile);
        Ok(())
    }

    /// Dissolves the groups and explodes the component instances among `nodes`
    /// back into what they hold, as one undo step. Plain objects are left alone;
    /// with nothing else, that is refused. On refusal nothing changes.
    pub fn dissolve(&mut self, tile: TileKey, nodes: &[NodeId]) -> Result<(), EditError> {
        self.check_nodes(tile, nodes)?;
        let doc = self.project.tile_mut(tile)?;
        let mut made = 0;
        for node in nodes {
            let done = match *node {
                NodeId::Group(g) => doc.ungroup(g).map(|_| ()),
                NodeId::Instance(i) => doc.explode_instance(i).map(|_| ()),
                NodeId::Object(_) => continue,
            };
            if let Err(e) = done {
                for _ in 0..made {
                    doc.undo().map_err(kernel_err)?;
                }
                return Err(kernel_err(e));
            }
            made += 1;
        }
        if made == 0 {
            return Err(EditError::Unsupported(
                "yang dipilih bukan grup atau komponen",
            ));
        }
        self.record_steps(tile, made);
        Ok(())
    }

    /// Deletes top-level `nodes` of one tile, as one undo step.
    pub fn delete_nodes(&mut self, tile: TileKey, nodes: &[NodeId]) -> Result<(), EditError> {
        self.check_nodes(tile, nodes)?;
        self.project
            .tile_mut(tile)?
            .delete_selection(nodes)
            .map_err(kernel_err)?;
        self.record(tile);
        Ok(())
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
        self.check_sketch(key, sketch)?;
        self.check_tile(key)?;
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
        self.check_source(key, source)?;
        let inference::ElementRef::Face(face) = source.element else {
            return Err(EditError::Unsupported("push/pull selain pada face"));
        };
        self.check_tile(key)?;
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

    /// Extrudes a solid's face into a new solid, leaving this one as it is. The
    /// two share the face, which Hew's freely overlapping solids allow.
    pub fn extrude_face_new(
        &mut self,
        key: TileKey,
        source: SnapSource,
        distance: f64,
    ) -> Result<ObjectId, EditError> {
        Self::check_distance(distance)?;
        self.check_source(key, source)?;
        let inference::ElementRef::Face(face) = source.element else {
            return Err(EditError::Unsupported("push/pull selain pada face"));
        };
        self.check_tile(key)?;
        let doc = self.project.tile_mut(key)?;
        let (object, _) = doc
            .extrude_face_as_new_object(source.object, face, distance)
            .map_err(kernel_err)?;
        self.guard_reach(key)?;
        self.record(key);
        Ok(object)
    }

    /// Paints a solid's face, or takes the paint off with `None`. One undo step,
    /// and none when the face already looks that way. A material belongs to one
    /// document, so each tile holds its own of the same color. It stays in the
    /// tile's palette after an undo, as Hew does not undo palette additions.
    pub fn paint(
        &mut self,
        key: TileKey,
        source: SnapSource,
        paint: Option<&Paint>,
    ) -> Result<(), EditError> {
        self.check_source(key, source)?;
        let inference::ElementRef::Face(face) = source.element else {
            return Err(EditError::Unsupported("mengecat selain face"));
        };
        self.check_tile(key)?;
        let doc = self.project.tile_mut(key)?;
        let material = paint.map(|p| {
            let same = doc
                .materials()
                .iter()
                .find(|(_, m)| m.color == p.color && !m.has_texture())
                .map(|(id, _)| id);
            same.unwrap_or_else(|| doc.add_material(Material::solid(p.name.clone(), p.color)))
        });
        if doc.face_material(source.object, face) == material {
            return Ok(());
        }
        doc.paint_face(source.object, face, material)
            .map_err(kernel_err)?;
        self.record(key);
        Ok(())
    }

    /// Undoes the latest edit, in whichever tile it happened.
    pub fn undo(&mut self) -> Result<Option<TileKey>, EditError> {
        let Some((key, steps)) = self.undo.last().copied() else {
            return Ok(None);
        };
        let doc = self.project.tile_mut(key)?;
        for _ in 0..steps {
            doc.undo().map_err(kernel_err)?;
        }
        self.undo.pop();
        self.redo.push((key, steps));
        self.touch(key);
        Ok(Some(key))
    }

    /// Redoes the latest undone edit.
    pub fn redo(&mut self) -> Result<Option<TileKey>, EditError> {
        let Some((key, steps)) = self.redo.last().copied() else {
            return Ok(None);
        };
        let doc = self.project.tile_mut(key)?;
        for _ in 0..steps {
            doc.redo().map_err(kernel_err)?;
        }
        self.redo.pop();
        self.undo.push((key, steps));
        self.touch(key);
        Ok(Some(key))
    }
}
