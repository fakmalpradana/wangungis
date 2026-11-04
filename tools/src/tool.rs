//! Drawing tools as plain state machines: inputs in, edits on a [`Scene`] and a
//! preview out. No GPU, no UI; the UI turns pixels into rays and draws the
//! preview.

use inference::{
    ApertureMode, Axis, ElementRef, PickRay, SnapKind, SnapLock, SnapQuery, SnapSource, SnapWeights,
};
use kernel::{Plane, Point3, SketchId, SketchRegionId, Vec3};
use std::num::NonZeroU32;
use wangungis_scene::project::TileKey;

use crate::vcb::{self, Vcb};
use crate::{EditError, Hit, Movable, Scene};

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
    /// Move: copy instead of moving, or back.
    ToggleCopy,
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
        first: Option<(Point3, Plane, TileKey)>,
        cursor: Option<Point3>,
    },
    PushPull {
        armed: Option<Armed>,
        /// Outline of what a click would pick.
        hover: Vec<(Point3, Point3)>,
    },
    Move {
        moving: Option<Moving>,
        /// Edges of what a click would pick up.
        hover: Vec<(Point3, Point3)>,
        copy: bool,
        /// The copy just made, which a typed count turns into an array.
        last: Option<(Movable, Vec3)>,
    },
    Erase {
        /// What a click would remove.
        hover: Vec<(Point3, Point3)>,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct Moving {
    what: Movable,
    base: Point3,
    cursor: Option<Point3>,
    lock: Option<Axis>,
    lines: Vec<(Point3, Point3)>,
}

#[derive(Debug, Clone, PartialEq)]
struct Chain {
    anchor: Point3,
    plane: Plane,
    tile: TileKey,
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

/// "What is under the cursor", for a click that picks rather than draws: a corner
/// does not reach past the aperture, so edges stay reachable next to it.
fn pick_query(ray: PickRay, aperture: f64) -> SnapQuery {
    SnapQuery {
        weights: SnapWeights::standard().for_picking(),
        ..query(ray, aperture)
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
    if denom <= 1e-9 {
        return None;
    }
    let t = (uv * v.dot(w) - along.dot(w)) / denom;
    // The nearest point must be in front of the eye.
    let s = v.dot(w) + uv * t;
    (s >= 0.0).then_some(t)
}

/// In-plane directions: world X projected onto a near-horizontal plane,
/// otherwise horizontal and upward along the plane.
fn plane_axes(plane: &Plane) -> (Vec3, Vec3) {
    let n = plane.normal();
    let u = if n.z.abs() > 0.999 {
        let x = Vec3::new(1.0, 0.0, 0.0);
        (x - n * n.dot(x)).normalized()
    } else {
        Vec3::new(0.0, 0.0, 1.0).cross(n).normalized()
    }
    .expect("n is unit and not parallel to the reference");
    (u, n.cross(u))
}

/// `p` moved onto `plane`: snaps land within rounding of it, and the kernel
/// takes only exact sketch points.
fn on_plane(p: Point3, plane: &Plane) -> Point3 {
    p - plane.normal() * plane.signed_distance(p)
}

/// The snap for where a drawing starts. The Z axis does not count: a click on the
/// ground that happens to be in line with it would start the drawing in mid-air.
fn start_snap(scene: &mut Scene, ray: PickRay, aperture: f64) -> Option<Hit> {
    scene
        .snap(&query(ray, aperture))
        .filter(|h| h.snap.kind != SnapKind::OnAxis || h.snap.position.z.abs() <= ON_PLANE)
}

/// A first click: the point, the plane the drawing will lie on, and the tile
/// holding what it joins. The face under the cursor gives the plane; any other
/// snap a horizontal plane through it; nothing at all the ground.
fn first_point(scene: &mut Scene, ray: PickRay, aperture: f64) -> Option<(Point3, Plane, TileKey)> {
    let own_tile = |p: Point3| TileKey::of([p.x, p.y]);
    let Some(hit) = start_snap(scene, ray, aperture) else {
        return ray_on_plane(&ray, &ground()).map(|p| (p, ground(), own_tile(p)));
    };
    let p = hit.snap.position;
    // A pure direction snap carries no geometry, so no tile of its own.
    let tile = match hit.snap.kind {
        SnapKind::Endpoint
        | SnapKind::Midpoint
        | SnapKind::Center
        | SnapKind::OnEdge
        | SnapKind::OnFace => hit.tile,
        _ => own_tile(p),
    };
    let on_face = hit.snap.source.and_then(|s| match s.element {
        ElementRef::Face(_) if s.instance.is_none() => scene.face_shape(hit.tile, s),
        _ => None,
    });
    let on_region = hit
        .snap
        .sketch_region_source
        .and_then(|(sk, r)| scene.region_shape(hit.tile, sk, r));
    // A corner or edge outranks the face it bounds: the face under the ray
    // still gives the plane when the snap lies on it.
    let plane = match on_face.or(on_region) {
        Some((plane, _)) => plane,
        None => {
            let beneath = scene
                .pick_face(&ray)
                .and_then(|f| scene.face_shape(f.tile, f.source))
                .map(|shape| shape.0)
                .filter(|plane| plane.signed_distance(p).abs() <= ON_PLANE);
            match beneath {
                Some(plane) => plane,
                None => Plane::from_point_normal(p, Vec3::new(0.0, 0.0, 1.0)).ok()?,
            }
        }
    };
    Some((on_plane(p, &plane), plane, tile))
}

/// A later point of a gesture, kept on `plane`. `anchor` turns on axis inference
/// from that point; a rectangle passes none, as a corner pulled onto an axis
/// through the first corner would leave it no area.
fn next_point(
    scene: &mut Scene,
    ray: PickRay,
    anchor: Option<Point3>,
    plane: &Plane,
    lock: Option<Axis>,
    aperture: f64,
) -> Option<(Point3, Option<Hit>)> {
    let q = SnapQuery {
        anchor,
        lock: lock.map(SnapLock::Axis),
        constraint_plane: Some(*plane),
        ..query(ray, aperture)
    };
    let hit = scene.snap(&q);
    if let Some(h) = hit
        && plane.signed_distance(h.snap.position).abs() <= ON_PLANE
    {
        return Some((on_plane(h.snap.position, plane), hit));
    }
    let point = match (lock, anchor) {
        (Some(axis), Some(a)) => {
            let u = axis.unit();
            on_plane(a + u * along_line(&ray, a, u)?, plane)
        }
        _ => ray_on_plane(&ray, plane)?,
    };
    Some((point, None))
}

/// Where a moved thing's base point lands: a snap, the locked axis, or the
/// horizontal plane through the base. It never snaps to itself: its faces and
/// corners travel with it, and would pull it towards the eye.
// ponytail: a snap on itself hides whatever was behind it for that frame. Taking
// the object out of its cell's index (`remove_object`) would fix that.
fn move_point(
    scene: &mut Scene,
    ray: PickRay,
    m: &Moving,
    aperture: f64,
) -> Option<(Point3, Option<Hit>)> {
    let q = SnapQuery {
        anchor: Some(m.base),
        lock: m.lock.map(SnapLock::Axis),
        ..query(ray, aperture)
    };
    let hit = scene
        .snap(&q)
        .filter(|h| scene.movable_at(h).as_ref() != Some(&m.what));
    if let Some(h) = hit {
        return Some((h.snap.position, hit));
    }
    if let Some(axis) = m.lock {
        let u = axis.unit();
        return Some((m.base + u * along_line(&ray, m.base, u)?, None));
    }
    let plane = Plane::from_point_normal(m.base, Vec3::new(0.0, 0.0, 1.0)).ok()?;
    ray_on_plane(&ray, &plane).map(|p| (p, None))
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

    pub fn move_tool(aperture: f64) -> Self {
        Self::with(
            State::Move {
                moving: None,
                hover: Vec::new(),
                copy: false,
                last: None,
            },
            aperture,
        )
    }

    pub fn eraser(aperture: f64) -> Self {
        Self::with(State::Erase { hover: Vec::new() }, aperture)
    }

    /// Whether the next move copies.
    pub fn copying(&self) -> bool {
        matches!(self.state, State::Move { copy: true, .. })
    }

    /// Whether `other` is the same tool, whatever state each is in.
    pub fn same_kind(&self, other: &Tool) -> bool {
        std::mem::discriminant(&self.state) == std::mem::discriminant(&other.state)
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
            State::Move { moving, .. } => moving.is_some(),
            State::Erase { .. } => false,
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
            State::Move { .. } => Self::move_tool(aperture),
            State::Erase { .. } => Self::eraser(aperture),
        };
    }

    pub fn handle(&mut self, scene: &mut Scene, input: Input) -> Result<(), EditError> {
        if input == Input::Cancel {
            self.reset();
            return Ok(());
        }
        let hovering = matches!(input, Input::Hover(_));
        match &self.state {
            State::Select { .. } => self.handle_select(scene, input),
            State::Line { .. } => self.handle_line(scene, input),
            State::Rectangle { .. } => self.handle_rectangle(scene, input),
            State::PushPull { .. } => self.handle_push_pull(scene, input),
            State::Move { .. } => self.handle_move(scene, input),
            State::Erase { .. } => self.handle_erase(scene, input),
        }?;
        if !hovering {
            // A click or typed value ends what the marker described.
            self.preview.snap = None;
        }
        self.refresh_lines();
        Ok(())
    }

    fn handle_select(&mut self, scene: &mut Scene, input: Input) -> Result<(), EditError> {
        match input {
            Input::Hover(ray) => self.preview.snap = scene.snap(&pick_query(ray, self.aperture)),
            Input::Click(ray) => {
                let hit = scene.snap(&pick_query(ray, self.aperture));
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
            Input::Typed(_) | Input::LockAxis(_) | Input::Cancel | Input::ToggleCopy => {}
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
                    let next = next_point(scene, ray, Some(c.anchor), &c.plane, c.lock, aperture);
                    c.cursor = next.map(|n| n.0);
                    self.preview.snap = next.and_then(|n| n.1);
                }
                None => self.preview.snap = start_snap(scene, ray, aperture),
            },
            Input::Click(ray) => match chain {
                None => {
                    if let Some((anchor, plane, tile)) = first_point(scene, ray, aperture) {
                        *chain = Some(Chain {
                            anchor,
                            plane,
                            tile,
                            cursor: None,
                            lock: None,
                        });
                    }
                }
                Some(c) => {
                    if let Some((end, _)) =
                        next_point(scene, ray, Some(c.anchor), &c.plane, c.lock, aperture)
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
                let end = on_plane(c.anchor + dir * len, &c.plane);
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
            Input::Cancel | Input::ToggleCopy => {}
        }
        Ok(())
    }

    /// Adds one segment; `true` when it closed a loop, which ends the chain.
    fn add_segment(scene: &mut Scene, c: &mut Chain, end: Point3) -> Result<bool, EditError> {
        if (end - c.anchor).length() < MIN_LEN {
            return Ok(false);
        }
        let drawn = scene.draw(Some(c.tile), c.plane, &[(c.anchor, end)])?;
        c.anchor = end;
        c.cursor = None;
        Ok(drawn.closed)
    }

    fn handle_rectangle(&mut self, scene: &mut Scene, input: Input) -> Result<(), EditError> {
        let aperture = self.aperture;
        let State::Rectangle { first, cursor } = &mut self.state else {
            unreachable!("dispatched on state")
        };
        match input {
            Input::Hover(ray) => match first {
                Some((_, plane, _)) => {
                    let next = next_point(scene, ray, None, plane, None, aperture);
                    *cursor = next.map(|n| n.0);
                    self.preview.snap = next.and_then(|n| n.1);
                }
                None => self.preview.snap = start_snap(scene, ray, aperture),
            },
            Input::Click(ray) => match first {
                None => {
                    *first = first_point(scene, ray, aperture);
                }
                Some((a, plane, tile)) => {
                    if let Some((b, _)) = next_point(scene, ray, None, plane, None, aperture) {
                        let (w, d) = spans(*a, b, plane);
                        if Self::draw_rectangle(scene, *a, plane, *tile, w, d)? {
                            *first = None;
                            *cursor = None;
                        }
                    }
                }
            },
            Input::Typed(text) => {
                let Some((a, plane, tile)) = first else {
                    return Err(needs("klik sudut pertama dulu"));
                };
                let Some(Vcb::Two(w, d)) = vcb::parse(&text) else {
                    return Err(needs("persegi panjang butuh dua ukuran, mis. 5;4"));
                };
                // Typed sizes go toward the cursor's quadrant, as in the preview.
                let (cw, cd) = cursor.map_or((1.0, 1.0), |b| spans(*a, b, plane));
                // `<` rather than signum: -0.0 has a negative sign.
                let side = |v: f64| if v < 0.0 { -1.0 } else { 1.0 };
                let (w, d) = (w * side(cw), d * side(cd));
                if Self::draw_rectangle(scene, *a, plane, *tile, w, d)? {
                    *first = None;
                    *cursor = None;
                }
            }
            Input::LockAxis(_) | Input::Cancel | Input::ToggleCopy => {}
        }
        Ok(())
    }

    /// `false` when the rectangle has no area, which is not an error: the
    /// gesture simply stays open.
    fn draw_rectangle(
        scene: &mut Scene,
        a: Point3,
        plane: &Plane,
        tile: TileKey,
        w: f64,
        d: f64,
    ) -> Result<bool, EditError> {
        if w.abs() < MIN_LEN || d.abs() < MIN_LEN {
            return Ok(false);
        }
        scene.draw(Some(tile), *plane, &ring(&rectangle(a, plane, w, d)))?;
        Ok(true)
    }

    fn handle_erase(&mut self, scene: &mut Scene, input: Input) -> Result<(), EditError> {
        let aperture = self.aperture;
        let State::Erase { hover } = &mut self.state else {
            unreachable!("dispatched on state")
        };
        match input {
            Input::Hover(ray) => {
                let hit = scene.snap(&pick_query(ray, aperture));
                self.preview.snap = hit;
                *hover = hit
                    .and_then(|h| scene.erasable_at(&h))
                    .map_or_else(Vec::new, |e| scene.erasable_lines(&e));
            }
            Input::Click(ray) => {
                let target = scene
                    .snap(&pick_query(ray, aperture))
                    .and_then(|h| scene.erasable_at(&h));
                if let Some(e) = target {
                    scene.erase(&e)?;
                    hover.clear();
                }
            }
            Input::Typed(_) | Input::LockAxis(_) | Input::Cancel | Input::ToggleCopy => {}
        }
        Ok(())
    }

    fn handle_move(&mut self, scene: &mut Scene, input: Input) -> Result<(), EditError> {
        let aperture = self.aperture;
        let State::Move {
            moving,
            hover,
            copy,
            last,
        } = &mut self.state
        else {
            unreachable!("dispatched on state")
        };
        match input {
            Input::Hover(ray) => match moving {
                Some(m) => {
                    let next = move_point(scene, ray, m, aperture);
                    m.cursor = next.map(|n| n.0);
                    self.preview.snap = next.and_then(|n| n.1);
                }
                None => {
                    let hit = scene.snap(&query(ray, aperture));
                    self.preview.snap = hit;
                    *hover = hit
                        .and_then(|h| scene.movable_at(&h))
                        .map_or_else(Vec::new, |m| scene.movable_lines(&m));
                }
            },
            Input::Click(ray) => match moving {
                None => {
                    if let Some(hit) = scene.snap(&query(ray, aperture))
                        && let Some(what) = scene.movable_at(&hit)
                    {
                        *moving = Some(Moving {
                            lines: scene.movable_lines(&what),
                            what,
                            base: hit.snap.position,
                            cursor: None,
                            lock: None,
                        });
                        *last = None;
                        hover.clear();
                    }
                }
                Some(m) => {
                    if let Some((p, _)) = move_point(scene, ray, m, aperture) {
                        let delta = p - m.base;
                        if Self::finish_move(scene, m, delta, *copy, last)? {
                            *moving = None;
                            *copy = false;
                        }
                    }
                }
            },
            Input::Typed(text) => match (&*moving, vcb::parse(&text)) {
                (Some(m), Some(Vcb::One(len))) => {
                    let cursor = m.cursor.ok_or(needs("arahkan kursor dulu"))?;
                    let dir = (cursor - m.base)
                        .normalized()
                        .map_err(|_| needs("arahkan kursor dulu"))?;
                    let m = moving.as_mut().expect("matched above");
                    if Self::finish_move(scene, m, dir * len, *copy, last)? {
                        *moving = None;
                        *copy = false;
                    }
                }
                (None, Some(Vcb::Copies(n))) => {
                    let (what, step) = last.as_ref().ok_or(needs("salin sesuatu dulu"))?;
                    scene.respace_copy(what, *step, n)?;
                }
                (None, Some(Vcb::Divide(n))) => {
                    let (what, step) = last.as_ref().ok_or(needs("salin sesuatu dulu"))?;
                    let part = *step / f64::from(n.get());
                    scene.respace_copy(what, part, n)?;
                }
                _ => return Err(needs("Move butuh satu jarak; setelah menyalin, x5 atau /5")),
            },
            Input::LockAxis(axis) => {
                if let Some(m) = moving {
                    m.lock = if m.lock == Some(axis) {
                        None
                    } else {
                        Some(axis)
                    };
                }
            }
            Input::ToggleCopy => *copy = !*copy,
            Input::Cancel => {}
        }
        Ok(())
    }

    /// `false` for a displacement too short to mean anything: the gesture stays
    /// open.
    fn finish_move(
        scene: &mut Scene,
        m: &Moving,
        delta: Vec3,
        copy: bool,
        last: &mut Option<(Movable, Vec3)>,
    ) -> Result<bool, EditError> {
        if delta.length() < MIN_LEN {
            return Ok(false);
        }
        if copy {
            scene.copy(&m.what, delta, NonZeroU32::MIN)?;
            *last = Some((m.what.clone(), delta));
        } else {
            scene.translate(&m.what, delta)?;
        }
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
            Input::LockAxis(_) | Input::Cancel | Input::ToggleCopy => {}
        }
        Ok(())
    }

    /// Picks the region or solid face under the ray: whichever is nearer.
    /// Regions come through a narrow snap; faces through a pick, so a ray right
    /// on an edge or an axis still finds the face.
    fn arm(scene: &mut Scene, ray: PickRay) -> Option<Armed> {
        let dir = ray.direction.normalized().ok()?;
        let region = scene.snap(&query(ray, NARROW)).and_then(|h| {
            let (sketch, region) = h.snap.sketch_region_source?;
            let shape = scene.region_shape(h.tile, sketch, region)?;
            let target = Target::Region {
                tile: h.tile,
                sketch,
                region,
            };
            let at = h.snap.position;
            Some(((at - ray.origin).dot(dir), target, at, shape))
        });
        let face = scene.pick_face(&ray).and_then(|f| {
            let shape = scene.face_shape(f.tile, f.source)?;
            let target = Target::Solid {
                tile: f.tile,
                source: f.source,
            };
            Some((f.depth, target, ray.origin + dir * f.depth, shape))
        });
        let (_, target, anchor, (plane, outline)) = match (region, face) {
            (Some(r), Some(f)) => {
                if r.0 <= f.0 {
                    r
                } else {
                    f
                }
            }
            (r, f) => r.or(f)?,
        };
        Some(Armed {
            target,
            anchor,
            normal: plane.normal(),
            outline,
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
                (Some((a, plane, _)), Some(b)) => {
                    let (w, d) = spans(*a, *b, plane);
                    ring(&rectangle(*a, plane, w, d))
                }
                _ => Vec::new(),
            },
            State::Erase { hover } => hover.clone(),
            State::Move { moving, hover, .. } => match moving {
                None => hover.clone(),
                Some(m) => {
                    let d = m.cursor.map_or(Vec3::new(0.0, 0.0, 0.0), |c| c - m.base);
                    let mut lines: Vec<_> = m.lines.iter().map(|(a, b)| (*a + d, *b + d)).collect();
                    lines.extend(m.cursor.map(|c| (m.base, c)));
                    lines
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plane_axes_stay_in_the_plane_when_nearly_flat() {
        for n in [
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.03, 0.02, 0.999),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.3, -0.5, 0.2),
        ] {
            let plane = Plane::from_point_normal(Point3::ORIGIN, n).unwrap();
            let (u, v) = plane_axes(&plane);
            let m = plane.normal();
            for d in [m.dot(u), m.dot(v), u.dot(v)] {
                assert!(d.abs() < 1e-12, "normal {n:?}: {d}");
            }
            assert!((u.length() - 1.0).abs() < 1e-12 && (v.length() - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn along_line_needs_the_point_in_front_of_the_eye() {
        let at = Point3::new(0.0, 0.0, 0.0);
        let up = Vec3::new(0.0, 0.0, 1.0);
        let toward = PickRay {
            origin: Point3::new(10.0, 0.0, 5.0),
            direction: Vec3::new(-1.0, 0.0, 0.0),
        };
        let t = along_line(&toward, at, up).expect("in front");
        assert!((t - 5.0).abs() < 1e-9, "{t}");
        let away = PickRay {
            direction: Vec3::new(1.0, 0.0, 0.0),
            ..toward
        };
        assert_eq!(along_line(&away, at, up), None);
        let parallel = PickRay {
            direction: up,
            ..toward
        };
        assert_eq!(along_line(&parallel, at, up), None);
    }
}
