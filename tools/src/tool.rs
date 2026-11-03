//! Drawing tools as plain state machines: inputs in, edits on a [`Scene`] and a
//! preview out. No GPU, no UI; the UI turns pixels into rays and draws the
//! preview.

use inference::{
    ApertureMode, Axis, ElementRef, PickRay, SnapLock, SnapQuery, SnapSource, SnapWeights,
};
use kernel::{Plane, Point3, SketchId, SketchRegionId, Vec3};
use wangungis_scene::project::TileKey;

use crate::vcb::{self, Vcb};
use crate::{EditError, Hit, Scene};

/// Smallest length (m) a gesture counts as: below it a click means nothing.
const MIN_LEN: f64 = 1e-4;
/// A point this close to a plane (m) lies on it.
const ON_PLANE: f64 = 1e-6;
/// Pick cone (rad) when choosing what to push: narrow, so a corner does not
/// take the click away from the region or face under it.
const NARROW: f64 = 1e-6;

#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    Hover(PickRay),
    Click(PickRay),
    /// Text from the value box, on Enter.
    Typed(String),
    /// Toggles a lock to this axis.
    LockAxis(Axis),
    /// Esc.
    Cancel,
}

/// What the UI should draw over the model, project-local.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Preview {
    pub lines: Vec<(Point3, Point3)>,
    pub snap: Option<Hit>,
}

/// What Select or Push/Pull has picked.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    Solid {
        tile: TileKey,
        source: SnapSource,
    },
    Region {
        tile: TileKey,
        sketch: SketchId,
        region: SketchRegionId,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct Armed {
    target: Target,
    anchor: Point3,
    normal: Vec3,
    outline: Vec<Point3>,
    distance: f64,
}

#[derive(Debug, Clone, PartialEq)]
enum State {
    Select {
        selected: Option<Target>,
        outline: Vec<(Point3, Point3)>,
    },
    Line {
        chain: Option<Chain>,
    },
    Rectangle {
        first: Option<(Point3, Plane)>,
        cursor: Option<Point3>,
    },
    PushPull {
        armed: Option<Armed>,
        /// Outline of what a click would pick.
        hover: Vec<(Point3, Point3)>,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct Chain {
    anchor: Point3,
    plane: Plane,
    sketch: Option<(TileKey, SketchId)>,
    cursor: Option<Point3>,
    lock: Option<Axis>,
}

/// The active tool. Errors leave the tool where it was, so the user can retry.
#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    state: State,
    aperture: f64,
    preview: Preview,
}

fn query(ray: PickRay, aperture: f64) -> SnapQuery {
    SnapQuery {
        ray,
        anchor: None,
        lock: None,
        aperture,
        aperture_mode: ApertureMode::Cone,
        constraint_plane: None,
        weights: SnapWeights::default(),
        soft_axis_aperture_scale: None,
        off_plane_points: false,
        see_through_points: false,
    }
}

fn ground() -> Plane {
    Plane::from_point_normal(Point3::ORIGIN, Vec3::new(0.0, 0.0, 1.0)).expect("unit normal")
}

fn ray_on_plane(ray: &PickRay, plane: &Plane) -> Option<Point3> {
    let dir = ray.direction.normalized().ok()?;
    let denom = plane.normal().dot(dir);
    if denom.abs() < 1e-9 {
        return None;
    }
    let t = -plane.signed_distance(ray.origin) / denom;
    (t > 0.0).then(|| ray.origin + dir * t)
}

/// The point on the line (`at`, unit `along`) nearest the ray, as its parameter.
fn along_line(ray: &PickRay, at: Point3, along: Vec3) -> Option<f64> {
    let v = ray.direction.normalized().ok()?;
    let w = at - ray.origin;
    let uv = along.dot(v);
    let denom = 1.0 - uv * uv;
    (denom > 1e-9).then(|| (uv * v.dot(w) - along.dot(w)) / denom)
}

/// In-plane directions: world X and Y on a horizontal plane, otherwise
/// horizontal and upward along the plane.
fn plane_axes(plane: &Plane) -> (Vec3, Vec3) {
    let n = plane.normal();
    let u = if n.z.abs() > 0.999 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::new(0.0, 0.0, 1.0)
            .cross(n)
            .normalized()
            .expect("n is not vertical")
    };
    (u, n.cross(u))
}

/// A first click: the point and the plane the drawing will lie on. The face
/// under the cursor gives the plane; any other snap a horizontal plane through
/// it; nothing at all the ground.
fn first_point(
    scene: &mut Scene,
    ray: PickRay,
    aperture: f64,
) -> Option<(Point3, Plane, Option<Hit>)> {
    let Some(hit) = scene.snap(&query(ray, aperture)) else {
        return ray_on_plane(&ray, &ground()).map(|p| (p, ground(), None));
    };
    let p = hit.snap.position;
    let on_face = hit.snap.source.and_then(|s| match s.element {
        ElementRef::Face(_) if s.instance.is_none() => scene.face_shape(hit.tile, s),
        _ => None,
    });
    let on_region = hit
        .snap
        .sketch_region_source
        .and_then(|(sk, r)| scene.region_shape(hit.tile, sk, r));
    let plane = match on_face.or(on_region) {
        Some((plane, _)) => plane,
        None => Plane::from_point_normal(p, Vec3::new(0.0, 0.0, 1.0)).ok()?,
    };
    Some((p, plane, Some(hit)))
}

/// A later point of a gesture, kept on `plane`.
fn next_point(
    scene: &mut Scene,
    ray: PickRay,
    anchor: Point3,
    plane: &Plane,
    lock: Option<Axis>,
    aperture: f64,
) -> Option<(Point3, Option<Hit>)> {
    let q = SnapQuery {
        anchor: Some(anchor),
        lock: lock.map(SnapLock::Axis),
        constraint_plane: Some(*plane),
        ..query(ray, aperture)
    };
    let hit = scene.snap(&q);
    if let Some(h) = hit
        && plane.signed_distance(h.snap.position).abs() <= ON_PLANE
    {
        return Some((h.snap.position, hit));
    }
    let point = match lock {
        Some(axis) => {
            let u = axis.unit();
            anchor + u * along_line(&ray, anchor, u)?
        }
        None => ray_on_plane(&ray, plane)?,
    };
    Some((point, None))
}

fn ring(points: &[Point3]) -> Vec<(Point3, Point3)> {
    (0..points.len())
        .map(|k| (points[k], points[(k + 1) % points.len()]))
        .collect()
}

fn rectangle(a: Point3, plane: &Plane, w: f64, d: f64) -> Vec<Point3> {
    let (u, v) = plane_axes(plane);
    vec![a, a + u * w, a + u * w + v * d, a + v * d]
}

/// Width and depth from `a` to `b` along the plane's axes.
fn spans(a: Point3, b: Point3, plane: &Plane) -> (f64, f64) {
    let (u, v) = plane_axes(plane);
    ((b - a).dot(u), (b - a).dot(v))
}

fn needs(what: &'static str) -> EditError {
    EditError::Unsupported(what)
}

impl Tool {
    /// `aperture`: half-angle (rad) of the pick cone, from the UI's pixel reach.
    fn with(state: State, aperture: f64) -> Self {
        Tool {
            state,
            aperture,
            preview: Preview::default(),
        }
    }

    pub fn select(aperture: f64) -> Self {
        Self::with(
            State::Select {
                selected: None,
                outline: Vec::new(),
            },
            aperture,
        )
    }

    pub fn line(aperture: f64) -> Self {
        Self::with(State::Line { chain: None }, aperture)
    }

    pub fn rectangle(aperture: f64) -> Self {
        Self::with(
            State::Rectangle {
                first: None,
                cursor: None,
            },
            aperture,
        )
    }

    pub fn push_pull(aperture: f64) -> Self {
        Self::with(
            State::PushPull {
                armed: None,
                hover: Vec::new(),
            },
            aperture,
        )
    }

    pub fn set_aperture(&mut self, aperture: f64) {
        self.aperture = aperture;
    }

    pub fn preview(&self) -> &Preview {
        &self.preview
    }

    /// What Select has picked.
    pub fn selected(&self) -> Option<Target> {
        match &self.state {
            State::Select { selected, .. } => *selected,
            _ => None,
        }
    }

    /// True while a gesture is under way, so Esc means "cancel it".
    pub fn busy(&self) -> bool {
        match &self.state {
            State::Select { selected, .. } => selected.is_some(),
            State::Line { chain } => chain.is_some(),
            State::Rectangle { first, .. } => first.is_some(),
            State::PushPull { armed, .. } => armed.is_some(),
        }
    }

    /// Back to the start of the tool, dropping stale ids after undo or redo.
    pub fn reset(&mut self) {
        let aperture = self.aperture;
        *self = match self.state {
            State::Select { .. } => Self::select(aperture),
            State::Line { .. } => Self::line(aperture),
            State::Rectangle { .. } => Self::rectangle(aperture),
            State::PushPull { .. } => Self::push_pull(aperture),
        };
    }

    pub fn handle(&mut self, scene: &mut Scene, input: Input) -> Result<(), EditError> {
        if input == Input::Cancel {
            self.reset();
            return Ok(());
        }
        match &self.state {
            State::Select { .. } => self.handle_select(scene, input),
            State::Line { .. } => self.handle_line(scene, input),
            State::Rectangle { .. } => self.handle_rectangle(scene, input),
            State::PushPull { .. } => self.handle_push_pull(scene, input),
        }?;
        self.refresh_lines();
        Ok(())
    }

    fn handle_select(&mut self, scene: &mut Scene, input: Input) -> Result<(), EditError> {
        match input {
            Input::Hover(ray) => self.preview.snap = scene.snap(&query(ray, self.aperture)),
            Input::Click(ray) => {
                let hit = scene.snap(&query(ray, self.aperture));
                let picked = hit.and_then(|h| match (h.snap.source, h.snap.sketch_region_source) {
                    (Some(source), _) => Some(Target::Solid {
                        tile: h.tile,
                        source,
                    }),
                    (None, Some((sketch, region))) => Some(Target::Region {
                        tile: h.tile,
                        sketch,
                        region,
                    }),
                    _ => None,
                });
                let lines = picked.map_or_else(Vec::new, |t| scene.outline(t));
                if let State::Select { selected, outline } = &mut self.state {
                    *selected = picked;
                    *outline = lines;
                }
            }
            Input::Typed(_) | Input::LockAxis(_) | Input::Cancel => {}
        }
        Ok(())
    }

    fn handle_line(&mut self, scene: &mut Scene, input: Input) -> Result<(), EditError> {
        let aperture = self.aperture;
        let State::Line { chain } = &mut self.state else {
            unreachable!("dispatched on state")
        };
        match input {
            Input::Hover(ray) => match chain {
                Some(c) => {
                    let next = next_point(scene, ray, c.anchor, &c.plane, c.lock, aperture);
                    c.cursor = next.map(|n| n.0);
                    self.preview.snap = next.and_then(|n| n.1);
                }
                None => self.preview.snap = scene.snap(&query(ray, aperture)),
            },
            Input::Click(ray) => match chain {
                None => {
                    if let Some((anchor, plane, _)) = first_point(scene, ray, aperture) {
                        *chain = Some(Chain {
                            anchor,
                            plane,
                            sketch: None,
                            cursor: None,
                            lock: None,
                        });
                    }
                }
                Some(c) => {
                    if let Some((end, _)) =
                        next_point(scene, ray, c.anchor, &c.plane, c.lock, aperture)
                        && Self::add_segment(scene, c, end)?
                    {
                        *chain = None;
                    }
                }
            },
            Input::Typed(text) => {
                let Some(c) = chain else {
                    return Err(needs("klik titik awal dulu"));
                };
                let Some(Vcb::One(len)) = vcb::parse(&text) else {
                    return Err(needs("garis butuh satu panjang, mis. 2,5"));
                };
                let cursor = c.cursor.ok_or(needs("arahkan kursor dulu"))?;
                let dir = (cursor - c.anchor)
                    .normalized()
                    .map_err(|_| needs("arahkan kursor dulu"))?;
                let end = c.anchor + dir * len;
                if Self::add_segment(scene, c, end)? {
                    *chain = None;
                }
            }
            Input::LockAxis(axis) => {
                let Some(c) = chain else { return Ok(()) };
                if c.plane.normal().dot(axis.unit()).abs() > 1e-6 {
                    return Err(needs("sumbu itu tidak sebidang dengan gambar"));
                }
                c.lock = if c.lock == Some(axis) {
                    None
                } else {
                    Some(axis)
                };
            }
            Input::Cancel => {}
        }
        Ok(())
    }

    /// Adds one segment; `true` when it closed a loop, which ends the chain.
    fn add_segment(scene: &mut Scene, c: &mut Chain, end: Point3) -> Result<bool, EditError> {
        if (end - c.anchor).length() < MIN_LEN {
            return Ok(false);
        }
        let regions = |s: &Scene, t: Option<(TileKey, SketchId)>| {
            t.map_or(0, |(k, id)| s.region_count(k, id))
        };
        let before = regions(scene, c.sketch);
        let drawn = scene.draw(c.sketch, c.plane, &[(c.anchor, end)])?;
        c.sketch = Some(drawn);
        c.anchor = end;
        c.cursor = None;
        Ok(regions(scene, c.sketch) > before)
    }

    fn handle_rectangle(&mut self, scene: &mut Scene, input: Input) -> Result<(), EditError> {
        let aperture = self.aperture;
        let State::Rectangle { first, cursor } = &mut self.state else {
            unreachable!("dispatched on state")
        };
        match input {
            Input::Hover(ray) => match first {
                Some((a, plane)) => {
                    let next = next_point(scene, ray, *a, plane, None, aperture);
                    *cursor = next.map(|n| n.0);
                    self.preview.snap = next.and_then(|n| n.1);
                }
                None => self.preview.snap = scene.snap(&query(ray, aperture)),
            },
            Input::Click(ray) => match first {
                None => {
                    if let Some((p, plane, _)) = first_point(scene, ray, aperture) {
                        *first = Some((p, plane));
                    }
                }
                Some((a, plane)) => {
                    if let Some((b, _)) = next_point(scene, ray, *a, plane, None, aperture) {
                        let (w, d) = spans(*a, b, plane);
                        if Self::draw_rectangle(scene, *a, plane, w, d)? {
                            *first = None;
                            *cursor = None;
                        }
                    }
                }
            },
            Input::Typed(text) => {
                let Some((a, plane)) = first else {
                    return Err(needs("klik sudut pertama dulu"));
                };
                let Some(Vcb::Two(w, d)) = vcb::parse(&text) else {
                    return Err(needs("persegi panjang butuh dua ukuran, mis. 5;4"));
                };
                // Typed sizes go toward the cursor's quadrant, as in the preview.
                let (cw, cd) = cursor.map_or((1.0, 1.0), |b| spans(*a, b, plane));
                let (w, d) = (w * cw.signum(), d * cd.signum());
                if Self::draw_rectangle(scene, *a, plane, w, d)? {
                    *first = None;
                    *cursor = None;
                }
            }
            Input::LockAxis(_) | Input::Cancel => {}
        }
        Ok(())
    }

    /// `false` when the rectangle has no area, which is not an error: the
    /// gesture simply stays open.
    fn draw_rectangle(
        scene: &mut Scene,
        a: Point3,
        plane: &Plane,
        w: f64,
        d: f64,
    ) -> Result<bool, EditError> {
        if w.abs() < MIN_LEN || d.abs() < MIN_LEN {
            return Ok(false);
        }
        scene.draw(None, *plane, &ring(&rectangle(a, plane, w, d)))?;
        Ok(true)
    }

    fn handle_push_pull(&mut self, scene: &mut Scene, input: Input) -> Result<(), EditError> {
        let aperture = self.aperture;
        let State::PushPull { armed, hover } = &mut self.state else {
            unreachable!("dispatched on state")
        };
        match input {
            Input::Hover(ray) => match armed {
                Some(a) => {
                    if let Some(s) = along_line(&ray, a.anchor, a.normal) {
                        a.distance = s;
                    }
                }
                None => {
                    self.preview.snap = scene.snap(&query(ray, aperture));
                    *hover = Self::arm(scene, ray).map_or_else(Vec::new, |a| ring(&a.outline));
                }
            },
            Input::Click(ray) => match armed {
                None => {
                    *armed = Self::arm(scene, ray);
                    hover.clear();
                }
                Some(a) => {
                    if a.distance.abs() >= MIN_LEN {
                        Self::commit_push_pull(scene, a.target, a.distance)?;
                        *armed = None;
                    }
                }
            },
            Input::Typed(text) => {
                let Some(a) = armed else {
                    return Err(needs("klik face atau region dulu"));
                };
                let Some(Vcb::One(d)) = vcb::parse(&text) else {
                    return Err(needs("Push/Pull butuh satu jarak, mis. 3"));
                };
                if d.abs() < MIN_LEN {
                    return Err(needs("jarak tidak boleh nol"));
                }
                Self::commit_push_pull(scene, a.target, d)?;
                *armed = None;
            }
            Input::LockAxis(_) | Input::Cancel => {}
        }
        Ok(())
    }

    /// Picks the region or solid face under the ray: whichever is nearer.
    fn arm(scene: &mut Scene, ray: PickRay) -> Option<Armed> {
        let hit = scene.snap(&query(ray, NARROW))?;
        let (target, shape) = if let Some((sketch, region)) = hit.snap.sketch_region_source {
            (
                Target::Region {
                    tile: hit.tile,
                    sketch,
                    region,
                },
                scene.region_shape(hit.tile, sketch, region)?,
            )
        } else {
            let source = hit.snap.source?;
            if !matches!(source.element, ElementRef::Face(_)) {
                return None;
            }
            (
                Target::Solid {
                    tile: hit.tile,
                    source,
                },
                scene.face_shape(hit.tile, source)?,
            )
        };
        Some(Armed {
            target,
            anchor: hit.snap.position,
            normal: shape.0.normal(),
            outline: shape.1,
            distance: 0.0,
        })
    }

    fn commit_push_pull(scene: &mut Scene, target: Target, d: f64) -> Result<(), EditError> {
        match target {
            Target::Region {
                tile,
                sketch,
                region,
            } => scene.extrude(tile, sketch, region, d).map(|_| ()),
            Target::Solid { tile, source } => scene.push_pull(tile, source, d),
        }
    }

    /// Rebuilds the preview lines from the state.
    fn refresh_lines(&mut self) {
        self.preview.lines = match &self.state {
            State::Select { outline, .. } => outline.clone(),
            State::Line { chain } => chain
                .iter()
                .filter_map(|c| c.cursor.map(|p| (c.anchor, p)))
                .collect(),
            State::Rectangle { first, cursor } => match (first, cursor) {
                (Some((a, plane)), Some(b)) => {
                    let (w, d) = spans(*a, *b, plane);
                    ring(&rectangle(*a, plane, w, d))
                }
                _ => Vec::new(),
            },
            State::PushPull { armed, hover } => armed.as_ref().map_or_else(
                || hover.clone(),
                |a| {
                    let moved: Vec<Point3> = a
                        .outline
                        .iter()
                        .map(|p| *p + a.normal * a.distance)
                        .collect();
                    let mut lines = ring(&moved);
                    lines.extend(a.outline.iter().copied().zip(moved.iter().copied()));
                    lines
                },
            ),
        };
    }
}
