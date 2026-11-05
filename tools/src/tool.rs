//! Drawing tools as plain state machines: inputs in, edits on a [`Scene`] and a
//! preview out. No GPU, no UI; the UI turns pixels into rays and draws the
//! preview.

use inference::{
    ApertureMode, Axis, ElementRef, PickRay, SOFT_AXIS_APERTURE, Snap, SnapKind, SnapLock,
    SnapQuery, SnapSource, SnapWeights,
};
use kernel::{NodeId, Plane, Point3, SketchCurveKind, SketchId, SketchRegionId, Transform, Vec3};
use std::num::NonZeroU32;
use wangungis_scene::project::TileKey;

use crate::vcb::{self, Unit, Vcb};
use crate::{EditError, GUIDE_REACH_M, Hit, Movable, Scene};

/// Smallest length (m) a gesture counts as: below it a click means nothing.
const MIN_LEN: f64 = 1e-4;
/// A point this close to a plane (m) lies on it.
const ON_PLANE: f64 = 1e-6;
/// Pick cone (rad) when choosing what to push: narrow, so a corner does not
/// take the click away from the region or face under it.
const NARROW: f64 = 1e-6;
/// The smallest scale factor: smaller collapses geometry below kernel tolerance.
const MIN_SCALE: f64 = 1e-3;
/// Facets of an arc per full turn, and at least this many on any arc.
const ARC_SIDES_PER_TURN: f64 = 48.0;
const MIN_ARC_SIDES: usize = 4;
/// Cosine under which a direction counts as lying in a plane.
const IN_PLANE: f64 = 1e-6;

#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    Hover(PickRay),
    Click(PickRay),
    /// Shift + click: Select adds to or takes from what is picked. Any other
    /// tool takes it as a plain click.
    ClickAdd(PickRay),
    /// Text from the value box, on Enter.
    Typed(String),
    /// Toggles a lock to this axis.
    LockAxis(Axis),
    /// Line: toggles a lock square to the drawing's plane.
    LockNormal,
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
        /// Whole things picked, for grouping and deleting.
        nodes: Vec<(TileKey, NodeId)>,
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
    Rotate {
        turning: Option<Turning>,
        hover: Vec<(Point3, Point3)>,
        copy: bool,
        /// The copy just made and its angle (rad), for a typed count.
        last: Option<(Movable, Point3, Vec3, f64)>,
    },
    Scale {
        scaling: Option<Scaling>,
        hover: Vec<(Point3, Point3)>,
    },
    Erase {
        /// What a click would remove.
        hover: Vec<(Point3, Point3)>,
    },
    Round {
        kind: SketchCurveKind,
        sides: u32,
        center: Option<(Point3, Plane, TileKey)>,
        cursor: Option<Point3>,
    },
    Arc {
        start: Option<(Point3, Plane, TileKey)>,
        end: Option<Point3>,
        cursor: Option<Point3>,
    },
    Offset {
        armed: Option<Armed>,
        cursor: Option<Point3>,
        hover: Vec<(Point3, Point3)>,
    },
    Tape {
        /// The start, and the edge direction when it lies on an edge.
        from: Option<(Point3, Option<Vec3>)>,
        cursor: Option<Point3>,
    },
    Follow {
        profile: Option<Target>,
        outline: Vec<(Point3, Point3)>,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct Turning {
    what: Movable,
    center: Point3,
    axis: Vec3,
    /// Direction from the center that means angle zero, once clicked.
    reference: Option<Vec3>,
    angle: f64,
    lines: Vec<(Point3, Point3)>,
}

#[derive(Debug, Clone, PartialEq)]
struct Scaling {
    what: Movable,
    center: Point3,
    /// Where the cursor is read, fixed for the gesture so factors do not jump.
    plane: Plane,
    /// Distance from the center of the grabbed point.
    grip: f64,
    factor: f64,
    lines: Vec<(Point3, Point3)>,
}

#[derive(Debug, Clone, PartialEq)]
struct Moving {
    what: Movable,
    base: Point3,
    cursor: Option<Point3>,
    lock: Option<Axis>,
    lines: Vec<(Point3, Point3)>,
}

/// What the next segment of a Line is held to.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Lock {
    Axis(Axis),
    /// Square to the drawing's plane.
    Normal(Vec3),
    /// `degrees` away from `from`, which `name` describes, on `plane`. One way
    /// only: the angle chose the side.
    Angle {
        from: Vec3,
        name: &'static str,
        degrees: f64,
        dir: Vec3,
        plane: Plane,
    },
}

impl Lock {
    fn dir(self) -> Vec3 {
        match self {
            Lock::Axis(a) => a.unit(),
            Lock::Normal(n) => n,
            Lock::Angle { dir, .. } => dir,
        }
    }

    fn snap(self) -> SnapLock {
        match self {
            Lock::Axis(a) => SnapLock::Axis(a),
            _ => SnapLock::Direction(self.dir()),
        }
    }

    fn describe(self) -> String {
        match self {
            Lock::Axis(a) => format!("sumbu {}", axis_name(a)),
            Lock::Normal(_) => "tegak lurus bidang".into(),
            Lock::Angle { name, degrees, .. } => format!("{degrees}° dari {name}"),
        }
    }
}

fn axis_name(axis: Axis) -> &'static str {
    match axis {
        Axis::X => "X",
        Axis::Y => "Y",
        Axis::Z => "Z",
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Chain {
    anchor: Point3,
    plane: Plane,
    tile: TileKey,
    cursor: Option<Point3>,
    /// Where the pointer was last, for the side of a typed angle.
    ray: Option<PickRay>,
    /// Holds the next segment only.
    lock: Option<Lock>,
    /// Nothing drawn yet, so the plane may still follow the cursor.
    open: bool,
    /// The edge last passed over, or else the segment just drawn: what the
    /// next segment may run parallel or square to. Unit length.
    reference: Option<Vec3>,
}

impl Chain {
    /// Where the pointer puts the end of the next segment, for hover and click alike.
    fn target(
        &mut self,
        scene: &mut Scene,
        ray: PickRay,
        aperture: f64,
    ) -> Option<(Point3, Option<Hit>)> {
        let plane = self.next_plane();
        let (point, hit) = next_point(scene, ray, Some(self.anchor), &plane, self.lock, aperture)?;
        if let Some(dir) = hit.as_ref().and_then(|h| scene.edge_direction(h)) {
            self.reference = Some(dir);
        }
        Some(self.infer(point, hit, &plane))
    }

    /// Pulls a free point onto the line through the anchor parallel or square
    /// to the reference, within Hew's own angle for inferring an axis. Hew
    /// declares these two snap kinds and never produces them. Anything the
    /// pointer really is on wins, as does a lock.
    fn infer(&self, point: Point3, hit: Option<Hit>, plane: &Plane) -> (Point3, Option<Hit>) {
        let free = hit.as_ref().is_none_or(|h| h.snap.kind == SnapKind::OnFace);
        let n = plane.normal();
        let Some(parallel) = self
            .reference
            .filter(|r| free && self.lock.is_none() && r.dot(n).abs() <= IN_PLANE)
        else {
            return (point, hit);
        };
        let span = point - self.anchor;
        let Ok(seen) = span.normalized() else {
            return (point, hit);
        };
        for (dir, kind) in [
            (parallel, SnapKind::Parallel),
            (parallel.cross(n), SnapKind::Perpendicular),
        ] {
            if seen.dot(dir).abs() >= SOFT_AXIS_APERTURE.cos() {
                let position = self.anchor + dir * span.dot(dir);
                let snap = Snap {
                    position,
                    kind,
                    source: None,
                    sketch_source: None,
                    sketch_region_source: None,
                    sketch_curve_source: None,
                    direction: Some(dir),
                    projected_from: None,
                    crossed_edge: None,
                };
                let tile = self.tile;
                return (position, Some(Hit { tile, snap }));
            }
        }
        (point, hit)
    }

    /// The plane the next segment lies on.
    fn next_plane(&self) -> Plane {
        match self.lock {
            Some(Lock::Angle { plane, .. }) => plane,
            Some(lock) => plane_along(self.anchor, lock.dir(), &self.plane, self.ray.as_ref()),
            None => self.plane,
        }
    }

    /// `p`, held to the side an angle chose: behind the anchor it means no length.
    fn ahead(&self, p: Point3) -> Point3 {
        match self.lock {
            Some(Lock::Angle { dir, .. }) if (p - self.anchor).dot(dir) < 0.0 => self.anchor,
            _ => p,
        }
    }

    /// Locks the next segment `degrees` away from the locked direction, or from
    /// the plane's first axis, on the side of it where the pointer is. A negative
    /// angle takes the other side. Typing again starts from the same direction.
    /// Needs the pointer to have moved since the last segment, to tell the side.
    fn aim(&mut self, degrees: f64) -> Result<(), EditError> {
        let ray = self.ray.ok_or(needs("arahkan kursor dulu"))?;
        let (from, name) = match self.lock {
            Some(Lock::Axis(a)) => (a.unit(), axis_name(a)),
            Some(Lock::Normal(n)) => (n, "tegak lurus bidang"),
            Some(Lock::Angle { from, name, .. }) => (from, name),
            None => {
                let name = if self.plane.normal().z.abs() > 0.999 {
                    "X"
                } else {
                    "horizontal"
                };
                (plane_axes(&self.plane).0, name)
            }
        };
        let plane = plane_along(self.anchor, from, &self.plane, Some(&ray));
        let side = plane.normal().cross(from);
        let seen = ray_on_plane(&ray, &plane).map(|p| p - self.anchor);
        let sign = |v: Vec3| {
            if seen.is_some_and(|d| d.dot(v) < 0.0) {
                -1.0
            } else {
                1.0
            }
        };
        let angle = degrees.to_radians();
        self.lock = Some(Lock::Angle {
            from,
            name,
            degrees,
            dir: from * (sign(from) * angle.cos()) + side * (sign(side) * angle.sin()),
            plane,
        });
        self.cursor = None;
        Ok(())
    }
}

/// The active tool. Errors leave the tool where it was, so the user can retry.
#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    state: State,
    aperture: f64,
    /// Bare typed lengths are in this.
    unit: Unit,
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

/// A plane through `anchor` holding `along`: `plane` itself when it can, otherwise
/// the axis plane that faces the eye most. Axis planes, so that lines locked one
/// after another can still close into a region.
fn plane_along(anchor: Point3, along: Vec3, plane: &Plane, ray: Option<&PickRay>) -> Plane {
    if plane.normal().dot(along).abs() <= IN_PLANE {
        return *plane;
    }
    let view = ray
        .and_then(|r| r.direction.normalized().ok())
        .unwrap_or(Vec3::new(0.0, 0.0, 1.0));
    let (u, v) = plane_axes(plane);
    [
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        u,
        v,
    ]
    .into_iter()
    .filter(|n| n.dot(along).abs() <= IN_PLANE)
    .max_by(|a, b| a.dot(view).abs().total_cmp(&b.dot(view).abs()))
    .and_then(|n| Plane::from_point_normal(anchor, n).ok())
    .unwrap_or(*plane)
}

/// `p` moved onto `plane`: snaps land within rounding of it, and the kernel
/// takes only exact sketch points.
fn on_plane(p: Point3, plane: &Plane) -> Point3 {
    p - plane.normal() * plane.signed_distance(p)
}

/// What the Tape spans from its start `a` to the cursor. From an edge it measures
/// square to the edge, as that is the distance of the guide it leaves; along the
/// edge itself it is the plain distance.
fn tape_span(a: Point3, along: Option<Vec3>, cursor: Point3) -> Vec3 {
    let d = cursor - a;
    match along {
        Some(u) if (d - u * d.dot(u)).length() >= MIN_LEN => d - u * d.dot(u),
        _ => d,
    }
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

/// The face under the cursor that holds `anchor`. A first click on an edge or
/// corner cannot tell which of its faces is meant; where the cursor goes next can.
fn plane_under(scene: &mut Scene, ray: &PickRay, anchor: Point3) -> Option<Plane> {
    let face = scene.pick_face(ray)?;
    let (plane, _) = scene.face_shape(face.tile, face.source)?;
    (plane.signed_distance(anchor).abs() <= ON_PLANE).then_some(plane)
}

/// A later point of a gesture, kept on `plane`. `anchor` turns on axis inference
/// from that point; a rectangle passes none, as a corner pulled onto an axis
/// through the first corner would leave it no area.
fn next_point(
    scene: &mut Scene,
    ray: PickRay,
    anchor: Option<Point3>,
    plane: &Plane,
    lock: Option<Lock>,
    aperture: f64,
) -> Option<(Point3, Option<Hit>)> {
    let q = SnapQuery {
        anchor,
        lock: lock.map(Lock::snap),
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
        (Some(lock), Some(a)) => {
            let u = lock.dir();
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

/// The plane to read a scaling cursor on: horizontal through `center`, or the
/// vertical plane facing the eye when looking along the horizontal.
fn scale_plane(ray: &PickRay, center: Point3) -> Option<Plane> {
    let dir = ray.direction.normalized().ok()?;
    let normal = if dir.z.abs() > 0.1 {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        Vec3::new(dir.x, dir.y, 0.0).normalized().ok()?
    };
    Plane::from_point_normal(center, normal).ok()
}

/// `sides` points around `center` on `plane`, the first towards `toward`.
fn round_points(
    center: Point3,
    plane: &Plane,
    radius: f64,
    sides: u32,
    toward: Option<Point3>,
) -> Vec<Point3> {
    let (u, v) = plane_axes(plane);
    let (cx, cy) = toward.map_or((1.0, 0.0), |p| ((p - center).dot(u), (p - center).dot(v)));
    let start = if cx == 0.0 && cy == 0.0 {
        0.0
    } else {
        cy.atan2(cx)
    };
    (0..sides)
        .map(|k| {
            let a = start + std::f64::consts::TAU * f64::from(k) / f64::from(sides);
            on_plane(
                center + u * (radius * a.cos()) + v * (radius * a.sin()),
                plane,
            )
        })
        .collect()
}

/// An arc from `a` to `b` through the point `sagitta` off the chord's middle, on
/// the side of `plane.normal() x (b - a)`: its points and center.
fn arc_points(a: Point3, b: Point3, sagitta: f64, plane: &Plane) -> Option<(Vec<Point3>, Point3)> {
    let chord = b - a;
    let half = chord.length() / 2.0;
    if half < MIN_LEN || sagitta.abs() < MIN_LEN {
        return None;
    }
    let side = plane.normal().cross(chord).normalized().ok()?;
    let apex = a + chord * 0.5 + side * sagitta;
    let radius = (half * half + sagitta * sagitta) / (2.0 * sagitta.abs());
    let center = apex - side * (sagitta.signum() * radius);
    // The inscribed angle at the apex is half the central angle on each side.
    let sweep = 4.0 * sagitta.abs().atan2(half);
    // Even, so the apex is a vertex and can be snapped to.
    let half_turns = sweep / std::f64::consts::TAU * ARC_SIDES_PER_TURN / 2.0;
    let n = ((half_turns - 1e-9).ceil() as usize * 2).max(MIN_ARC_SIDES);
    let from = a - center;
    let across = plane.normal().cross(from);
    // Which way round passes the apex.
    let turn = if (apex - center).dot(across) >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let mut points: Vec<Point3> = (0..=n)
        .map(|k| {
            let t = sweep * k as f64 / n as f64;
            on_plane(center + from * t.cos() + across * (turn * t.sin()), plane)
        })
        .collect();
    points[0] = a;
    *points.last_mut()? = b;
    Some((points, center))
}

/// Whether `p` lies inside the ring, in the plane's axes.
fn inside_ring(p: Point3, ring: &[Point3], plane: &Plane) -> bool {
    let (u, v) = plane_axes(plane);
    let at = |q: Point3| ((q - p).dot(u), (q - p).dot(v));
    let mut inside = false;
    for k in 0..ring.len() {
        let (x0, y0) = at(ring[k]);
        let (x1, y1) = at(ring[(k + 1) % ring.len()]);
        if (y0 > 0.0) != (y1 > 0.0) && x0 + (0.0 - y0) * (x1 - x0) / (y1 - y0) > 0.0 {
            inside = !inside;
        }
    }
    inside
}

/// How far `p` is from the ring's edges.
fn ring_distance(p: Point3, ring: &[Point3]) -> f64 {
    (0..ring.len())
        .map(|k| {
            let (a, b) = (ring[k], ring[(k + 1) % ring.len()]);
            let ab = b - a;
            let t = ((p - a).dot(ab) / ab.dot(ab).max(1e-18)).clamp(0.0, 1.0);
            (a + ab * t - p).length()
        })
        .fold(f64::INFINITY, f64::min)
}

/// A rotation of `angle` (rad) about the line through `center` along `axis`.
fn rotation_about(center: Point3, axis: Vec3, angle: f64) -> Result<Transform, EditError> {
    let to = center - Point3::ORIGIN;
    let spin = Transform::rotation(axis, angle).map_err(|_| needs("sumbu putar tidak sah"))?;
    Ok(Transform::translation(-to)
        .then(&spin)
        .then(&Transform::translation(to)))
}

fn scaling_about(center: Point3, factor: f64) -> Transform {
    let to = center - Point3::ORIGIN;
    Transform::translation(-to)
        .then(&Transform::uniform_scale(factor))
        .then(&Transform::translation(to))
}

/// The cursor's direction from `center`, in the plane normal to `axis`.
fn direction_in_plane(ray: &PickRay, center: Point3, axis: Vec3) -> Option<Vec3> {
    let plane = Plane::from_point_normal(center, axis).ok()?;
    (ray_on_plane(ray, &plane)? - center).normalized().ok()
}

/// Rounds to a multiple of 15 degrees within 1.5 degrees of one.
fn detent(angle: f64) -> f64 {
    let step = 15f64.to_radians();
    let near = (angle / step).round() * step;
    if (angle - near).abs() <= 1.5f64.to_radians() {
        near
    } else {
        angle
    }
}

/// `count` from a typed array mark: `Copies` keeps the step, `Divide` splits it.
fn typed_array(v: Vcb) -> Option<(NonZeroU32, f64)> {
    match v {
        Vcb::Copies(n) => Some((n, 1.0)),
        Vcb::Divide(n) => Some((n, 1.0 / f64::from(n.get()))),
        _ => None,
    }
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
            unit: Unit::default(),
            preview: Preview::default(),
        }
    }

    pub fn select(aperture: f64) -> Self {
        Self::with(
            State::Select {
                selected: None,
                outline: Vec::new(),
                nodes: Vec::new(),
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

    pub fn rotate(aperture: f64) -> Self {
        Self::with(
            State::Rotate {
                turning: None,
                hover: Vec::new(),
                copy: false,
                last: None,
            },
            aperture,
        )
    }

    pub fn scale(aperture: f64) -> Self {
        Self::with(
            State::Scale {
                scaling: None,
                hover: Vec::new(),
            },
            aperture,
        )
    }

    pub fn circle(aperture: f64) -> Self {
        Self::with(
            State::Round {
                kind: SketchCurveKind::Circle,
                sides: 48,
                center: None,
                cursor: None,
            },
            aperture,
        )
    }

    pub fn polygon(aperture: f64) -> Self {
        Self::with(
            State::Round {
                kind: SketchCurveKind::Polygon,
                sides: 6,
                center: None,
                cursor: None,
            },
            aperture,
        )
    }

    pub fn arc(aperture: f64) -> Self {
        Self::with(
            State::Arc {
                start: None,
                end: None,
                cursor: None,
            },
            aperture,
        )
    }

    pub fn offset(aperture: f64) -> Self {
        Self::with(
            State::Offset {
                armed: None,
                cursor: None,
                hover: Vec::new(),
            },
            aperture,
        )
    }

    pub fn tape(aperture: f64) -> Self {
        Self::with(
            State::Tape {
                from: None,
                cursor: None,
            },
            aperture,
        )
    }

    pub fn follow_me(aperture: f64) -> Self {
        Self::with(
            State::Follow {
                profile: None,
                outline: Vec::new(),
            },
            aperture,
        )
    }

    /// What the Tape reads now, in meters.
    pub fn measure(&self) -> Option<f64> {
        match &self.state {
            State::Tape {
                from: Some((a, along)),
                cursor: Some(b),
            } => Some(tape_span(*a, *along, *b).length()),
            _ => None,
        }
    }

    /// What the tool waits for now, in words for the status bar.
    pub fn hint(&self) -> String {
        let s = match &self.state {
            State::Select { .. } => "Klik benda untuk memilih. Shift + klik menambah.",
            State::Line { chain: None } => "Klik titik awal.",
            State::Line { chain: Some(c) } => {
                return match c.lock {
                    Some(l) => format!(
                        "Terkunci {}. Klik atau ketik panjang. {}",
                        l.describe(),
                        if matches!(l, Lock::Angle { .. }) {
                            "Panah mengganti kunci."
                        } else {
                            "Panah yang sama melepas kunci."
                        }
                    ),
                    None => "Klik titik berikut atau ketik panjang. Panah kanan/kiri/atas: sumbu X/Y/Z, bawah: tegak lurus bidang. 30d: miring 30°. Esc: selesai.".into(),
                };
            }
            State::Rectangle { first: None, .. } => "Klik sudut pertama.",
            State::Rectangle { .. } => "Klik sudut lawan atau ketik ukuran, mis. 5;4.",
            State::PushPull { armed: None, .. } => "Klik region atau face.",
            State::PushPull { .. } => "Klik tinggi atau ketik jarak.",
            State::Move { moving: None, .. } => "Klik benda. C: salin.",
            State::Move { .. } => "Klik tujuan atau ketik jarak. Panah: kunci sumbu.",
            State::Rotate { turning: None, .. } => "Klik benda: titik itu jadi poros.",
            State::Rotate {
                turning: Some(t), ..
            } if t.reference.is_none() => "Klik arah nol. Panah: sumbu putar.",
            State::Rotate { .. } => "Klik atau ketik sudut.",
            State::Scale { scaling: None, .. } => "Klik benda.",
            State::Scale { .. } => "Gerakkan kursor lalu klik, atau ketik faktor.",
            State::Erase { .. } => "Klik tepi untuk menghapus.",
            State::Round { center: None, .. } => "Klik pusat. 8s: jumlah sisi.",
            State::Round { .. } => "Klik atau ketik jari-jari.",
            State::Arc { start: None, .. } => "Klik ujung pertama.",
            State::Arc { end: None, .. } => "Klik ujung kedua.",
            State::Arc { .. } => "Klik atau ketik tinggi lengkung.",
            State::Offset { armed: None, .. } => "Klik region atau face.",
            State::Offset { .. } => "Klik atau ketik jarak.",
            State::Tape { from: None, .. } => {
                "Klik tepi untuk garis panduan, atau titik untuk titik panduan."
            }
            State::Tape { .. } => "Gerakkan kursor menjauh, lalu klik atau ketik jarak.",
            State::Follow { profile: None, .. } => "Klik region atau face sebagai penampang.",
            State::Follow { .. } => "Klik garis sebagai jalur.",
        };
        s.into()
    }

    /// Whether a copying toggle means anything to this tool, so C can mean
    /// Circle everywhere else.
    pub fn can_copy(&self) -> bool {
        matches!(self.state, State::Move { .. } | State::Rotate { .. })
    }

    pub fn eraser(aperture: f64) -> Self {
        Self::with(State::Erase { hover: Vec::new() }, aperture)
    }

    /// Whether the next move copies.
    pub fn copying(&self) -> bool {
        matches!(
            self.state,
            State::Move { copy: true, .. } | State::Rotate { copy: true, .. }
        )
    }

    /// Whether `other` is the same tool, whatever state each is in.
    pub fn same_kind(&self, other: &Tool) -> bool {
        match (&self.state, &other.state) {
            (State::Round { kind: a, .. }, State::Round { kind: b, .. }) => a == b,
            (a, b) => std::mem::discriminant(a) == std::mem::discriminant(b),
        }
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
            State::Select {
                selected, nodes, ..
            } => selected.is_some() || !nodes.is_empty(),
            State::Line { chain } => chain.is_some(),
            State::Rectangle { first, .. } => first.is_some(),
            State::PushPull { armed, .. } => armed.is_some(),
            State::Move { moving, .. } => moving.is_some(),
            State::Rotate { turning, .. } => turning.is_some(),
            State::Scale { scaling, .. } => scaling.is_some(),
            State::Erase { .. } => false,
            State::Round { center, .. } => center.is_some(),
            State::Arc { start, .. } => start.is_some(),
            State::Offset { armed, .. } => armed.is_some(),
            State::Tape { from, .. } => from.is_some(),
            State::Follow { profile, .. } => profile.is_some(),
        }
    }

    pub fn set_unit(&mut self, unit: Unit) {
        self.unit = unit;
    }

    /// Back to the start of the tool, dropping stale ids after undo or redo.
    pub fn reset(&mut self) {
        let aperture = self.aperture;
        let unit = self.unit;
        *self = match self.state {
            State::Select { .. } => Self::select(aperture),
            State::Line { .. } => Self::line(aperture),
            State::Rectangle { .. } => Self::rectangle(aperture),
            State::PushPull { .. } => Self::push_pull(aperture),
            State::Move { .. } => Self::move_tool(aperture),
            State::Rotate { .. } => Self::rotate(aperture),
            State::Scale { .. } => Self::scale(aperture),
            State::Erase { .. } => Self::eraser(aperture),
            State::Round {
                kind: SketchCurveKind::Polygon,
                ..
            } => Self::polygon(aperture),
            State::Round { .. } => Self::circle(aperture),
            State::Arc { .. } => Self::arc(aperture),
            State::Offset { .. } => Self::offset(aperture),
            State::Tape { .. } => Self::tape(aperture),
            State::Follow { .. } => Self::follow_me(aperture),
        };
        self.unit = unit;
    }

    /// The whole things Select has picked: top-level nodes, all in one tile.
    pub fn picked_nodes(&self) -> Option<(TileKey, Vec<NodeId>)> {
        let State::Select { nodes, .. } = &self.state else {
            return None;
        };
        let tile = nodes.first()?.0;
        Some((tile, nodes.iter().map(|n| n.1).collect()))
    }

    pub fn handle(&mut self, scene: &mut Scene, input: Input) -> Result<(), EditError> {
        let input = match input {
            Input::ClickAdd(ray) if !matches!(self.state, State::Select { .. }) => {
                Input::Click(ray)
            }
            other => other,
        };
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
            State::Rotate { .. } => self.handle_rotate(scene, input),
            State::Scale { .. } => self.handle_scale(scene, input),
            State::Erase { .. } => self.handle_erase(scene, input),
            State::Round { .. } => self.handle_round(scene, input),
            State::Arc { .. } => self.handle_arc(scene, input),
            State::Offset { .. } => self.handle_offset(scene, input),
            State::Tape { .. } => self.handle_tape(scene, input),
            State::Follow { .. } => self.handle_follow(scene, input),
        }?;
        if !hovering {
            // A click or typed value ends what the marker described.
            self.preview.snap = None;
        }
        self.refresh_lines();
        Ok(())
    }

    fn handle_select(&mut self, scene: &mut Scene, input: Input) -> Result<(), EditError> {
        let aperture = self.aperture;
        let (ray, adding) = match input {
            Input::Hover(ray) => {
                self.preview.snap = scene.snap(&pick_query(ray, aperture));
                return Ok(());
            }
            Input::Click(ray) => (ray, false),
            Input::ClickAdd(ray) => (ray, true),
            _ => return Ok(()),
        };
        let hit = scene.snap(&pick_query(ray, aperture));
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
        let node = hit
            .and_then(|h| scene.movable_at(&h))
            .and_then(|m| match m {
                Movable::Node { tile, node } => Some((tile, node)),
                Movable::Island { .. } => None,
            });
        let State::Select {
            selected,
            outline,
            nodes,
        } = &mut self.state
        else {
            unreachable!("dispatched on state")
        };
        if !adding {
            nodes.clear();
        }
        if let Some(n) = node {
            // Shift on what is picked takes it back. A pick in another tile starts
            // over: Hew groups within one document.
            if nodes.first().is_some_and(|f| f.0 != n.0) {
                nodes.clear();
            }
            if let Some(at) = nodes.iter().position(|x| *x == n) {
                nodes.remove(at);
            } else {
                nodes.push(n);
            }
        }
        // A plain click shows the face or edge under it. With Shift, or with several
        // picked, the outline is what is held: what Shift took back must not stay lit.
        *selected = if adding && nodes.is_empty() {
            None
        } else {
            picked
        };
        *outline = if adding || nodes.len() > 1 {
            nodes
                .iter()
                .flat_map(|(tile, node)| {
                    scene.movable_lines(&Movable::Node {
                        tile: *tile,
                        node: *node,
                    })
                })
                .collect()
        } else {
            picked.map_or_else(Vec::new, |t| scene.outline(t))
        };
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
                    Self::follow_cursor(scene, c, ray);
                    let next = c.target(scene, ray, aperture);
                    c.cursor = next.map(|n| c.ahead(n.0));
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
                            ray: None,
                            lock: None,
                            open: true,
                            reference: None,
                        });
                    }
                }
                Some(c) => {
                    Self::follow_cursor(scene, c, ray);
                    if let Some((end, _)) = c.target(scene, ray, aperture)
                        && Self::add_segment(scene, c, c.ahead(end))?
                    {
                        *chain = None;
                    }
                }
            },
            Input::Typed(text) => {
                let Some(c) = chain else {
                    return Err(needs("klik titik awal dulu"));
                };
                let len = match vcb::parse_in(&text, self.unit) {
                    Some(Vcb::One(len)) => len,
                    Some(Vcb::Angle(degrees)) => return c.aim(degrees),
                    _ => return Err(needs("garis butuh panjang (2,5) atau sudut (30d)")),
                };
                let dir = match c.lock {
                    Some(Lock::Angle { dir, .. }) => dir,
                    _ => {
                        let cursor = c.cursor.ok_or(needs("arahkan kursor dulu"))?;
                        (cursor - c.anchor)
                            .normalized()
                            .map_err(|_| needs("arahkan kursor dulu"))?
                    }
                };
                let end = on_plane(c.anchor + dir * len, &c.next_plane());
                if Self::add_segment(scene, c, end)? {
                    *chain = None;
                }
            }
            Input::LockAxis(axis) => {
                let Some(c) = chain else { return Ok(()) };
                c.lock = if c.lock == Some(Lock::Axis(axis)) {
                    None
                } else {
                    Some(Lock::Axis(axis))
                };
                // It was computed without this lock: a typed length must not use it.
                c.cursor = None;
            }
            Input::LockNormal => {
                let Some(c) = chain else { return Ok(()) };
                c.lock = if matches!(c.lock, Some(Lock::Normal(_))) {
                    None
                } else {
                    Some(Lock::Normal(c.plane.normal()))
                };
                c.cursor = None;
            }
            Input::ClickAdd(_) | Input::Cancel | Input::ToggleCopy => {}
        }
        Ok(())
    }

    /// Notes where the pointer is. Before the first segment, and with nothing
    /// locked, the plane also follows it.
    fn follow_cursor(scene: &mut Scene, c: &mut Chain, ray: PickRay) {
        c.ray = Some(ray);
        if c.open
            && c.lock.is_none()
            && let Some(p) = plane_under(scene, &ray, c.anchor)
        {
            c.plane = p;
        }
    }

    /// Adds one segment; `true` when it closed a loop, which ends the chain.
    fn add_segment(scene: &mut Scene, c: &mut Chain, end: Point3) -> Result<bool, EditError> {
        if (end - c.anchor).length() < MIN_LEN {
            return Ok(false);
        }
        let plane = c.next_plane();
        let drawn = scene.draw(Some(c.tile), plane, &[(c.anchor, end)])?;
        c.plane = plane;
        c.open = false;
        c.reference = (end - c.anchor).normalized().ok();
        c.anchor = end;
        c.cursor = None;
        c.ray = None;
        c.lock = None;
        Ok(drawn.closed)
    }

    fn handle_rectangle(&mut self, scene: &mut Scene, input: Input) -> Result<(), EditError> {
        let aperture = self.aperture;
        let State::Rectangle { first, cursor } = &mut self.state else {
            unreachable!("dispatched on state")
        };
        match input {
            Input::Hover(ray) => match first {
                Some((a, plane, _)) => {
                    if let Some(p) = plane_under(scene, &ray, *a) {
                        *plane = p;
                    }
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
                    if let Some(p) = plane_under(scene, &ray, *a) {
                        *plane = p;
                    }
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
                let Some(Vcb::Two(w, d)) = vcb::parse_in(&text, self.unit) else {
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
            Input::ClickAdd(_)
            | Input::LockAxis(_)
            | Input::LockNormal
            | Input::Cancel
            | Input::ToggleCopy => {}
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

    fn handle_rotate(&mut self, scene: &mut Scene, input: Input) -> Result<(), EditError> {
        let aperture = self.aperture;
        let State::Rotate {
            turning,
            hover,
            copy,
            last,
        } = &mut self.state
        else {
            unreachable!("dispatched on state")
        };
        let mut finish = |scene: &mut Scene, t: &Turning, angle: f64, copy: bool| {
            let spin = rotation_about(t.center, t.axis, angle)?;
            if copy {
                scene.copy(&t.what, &spin, NonZeroU32::MIN)?;
                *last = Some((t.what.clone(), t.center, t.axis, angle));
            } else {
                scene.transform(&t.what, &spin)?;
            }
            Ok::<(), EditError>(())
        };
        match input {
            Input::Hover(ray) => match turning {
                Some(t) => {
                    if let Some(r) = t.reference
                        && let Some(v) = direction_in_plane(&ray, t.center, t.axis)
                    {
                        t.angle = detent(t.axis.dot(r.cross(v)).atan2(r.dot(v)));
                    }
                }
                None => {
                    let hit = scene.snap(&query(ray, aperture));
                    self.preview.snap = hit;
                    *hover = hit
                        .and_then(|h| scene.movable_at(&h))
                        .map_or_else(Vec::new, |m| scene.movable_lines(&m));
                }
            },
            Input::Click(ray) => match turning {
                None => {
                    if let Some(hit) = scene.snap(&query(ray, aperture))
                        && let Some(what) = scene.movable_at(&hit)
                    {
                        *turning = Some(Turning {
                            lines: scene.movable_lines(&what),
                            what,
                            center: hit.snap.position,
                            axis: Vec3::new(0.0, 0.0, 1.0),
                            reference: None,
                            angle: 0.0,
                        });
                        *last = None;
                        hover.clear();
                    }
                }
                Some(t) if t.reference.is_none() => {
                    t.reference = direction_in_plane(&ray, t.center, t.axis);
                }
                Some(t) => {
                    if let (Some(r), Some(v)) =
                        (t.reference, direction_in_plane(&ray, t.center, t.axis))
                    {
                        t.angle = detent(t.axis.dot(r.cross(v)).atan2(r.dot(v)));
                    }
                    if t.angle.abs() > 1e-9 {
                        let snapshot = t.clone();
                        finish(scene, &snapshot, snapshot.angle, *copy)?;
                        *turning = None;
                        *copy = false;
                    }
                }
            },
            Input::Typed(text) => match (&*turning, vcb::parse(&text)) {
                (Some(t), Some(Vcb::One(degrees) | Vcb::Angle(degrees))) => {
                    if degrees.abs() < 1e-6 {
                        return Err(needs("sudut nol tidak mengubah apa pun"));
                    }
                    let snapshot = t.clone();
                    finish(scene, &snapshot, degrees.to_radians(), *copy)?;
                    *turning = None;
                    *copy = false;
                }
                (None, Some(v @ (Vcb::Copies(_) | Vcb::Divide(_)))) => {
                    let (what, center, axis, angle) =
                        last.as_ref().ok_or(needs("salin sesuatu dulu"))?;
                    let (n, part) = typed_array(v).expect("matched above");
                    let step = rotation_about(*center, *axis, angle * part)?;
                    scene.respace_copy(what, &step, n)?;
                }
                _ => {
                    return Err(needs(
                        "putar butuh sudut dalam derajat; setelah menyalin, x5",
                    ));
                }
            },
            Input::LockAxis(axis) => {
                if let Some(t) = turning {
                    t.axis = axis.unit();
                    t.reference = None;
                    t.angle = 0.0;
                }
            }
            Input::ToggleCopy => *copy = !*copy,
            Input::ClickAdd(_) | Input::LockNormal | Input::Cancel => {}
        }
        Ok(())
    }

    fn handle_scale(&mut self, scene: &mut Scene, input: Input) -> Result<(), EditError> {
        let aperture = self.aperture;
        let State::Scale { scaling, hover } = &mut self.state else {
            unreachable!("dispatched on state")
        };
        match input {
            Input::Hover(ray) => match scaling {
                Some(s) => {
                    if let Some(p) = ray_on_plane(&ray, &s.plane) {
                        s.factor = ((p - s.center).length() / s.grip).max(MIN_SCALE);
                    }
                }
                None => {
                    let hit = scene.snap(&query(ray, aperture));
                    self.preview.snap = hit;
                    *hover = hit
                        .and_then(|h| scene.movable_at(&h))
                        .map_or_else(Vec::new, |m| scene.movable_lines(&m));
                }
            },
            Input::Click(ray) => match scaling {
                None => {
                    if let Some(hit) = scene.snap(&query(ray, aperture))
                        && let Some(what) = scene.movable_at(&hit)
                        && let Some(center) = scene.movable_center(&what)
                        && let Some(plane) = scale_plane(&ray, center)
                    {
                        let p = ray_on_plane(&ray, &plane).unwrap_or(hit.snap.position);
                        let grip = (p - center).length();
                        if grip > MIN_LEN {
                            *scaling = Some(Scaling {
                                lines: scene.movable_lines(&what),
                                what,
                                center,
                                plane,
                                grip,
                                factor: 1.0,
                            });
                            hover.clear();
                        }
                    }
                }
                Some(s) => {
                    if let Some(p) = ray_on_plane(&ray, &s.plane) {
                        s.factor = ((p - s.center).length() / s.grip).max(MIN_SCALE);
                    }
                    if (s.factor - 1.0).abs() > 1e-6 {
                        scene.transform(&s.what, &scaling_about(s.center, s.factor))?;
                        *scaling = None;
                    }
                }
            },
            Input::Typed(text) => {
                let Some(s) = scaling else {
                    return Err(needs("klik benda dulu"));
                };
                let Some(Vcb::One(factor)) = vcb::parse(&text) else {
                    return Err(needs("skala butuh satu faktor, mis. 2 atau 0,5"));
                };
                if factor < MIN_SCALE {
                    return Err(needs("faktor skala harus positif"));
                }
                if (factor - 1.0).abs() < 1e-6 {
                    return Err(needs("faktor 1 tidak mengubah apa pun"));
                }
                scene.transform(&s.what, &scaling_about(s.center, factor))?;
                *scaling = None;
            }
            Input::ClickAdd(_)
            | Input::LockAxis(_)
            | Input::LockNormal
            | Input::ToggleCopy
            | Input::Cancel => {}
        }
        Ok(())
    }

    fn handle_round(&mut self, scene: &mut Scene, input: Input) -> Result<(), EditError> {
        let aperture = self.aperture;
        let State::Round {
            kind,
            sides,
            center,
            cursor,
        } = &mut self.state
        else {
            unreachable!("dispatched on state")
        };
        let lowest = if *kind == SketchCurveKind::Circle {
            kernel::MIN_CIRCLE_SEGMENTS as u32
        } else {
            3
        };
        let mut radius = None;
        match input {
            Input::Hover(ray) => match center {
                Some((c, plane, _)) => {
                    let next = next_point(scene, ray, Some(*c), plane, None, aperture);
                    *cursor = next.map(|n| n.0);
                    self.preview.snap = next.and_then(|n| n.1);
                }
                None => self.preview.snap = start_snap(scene, ray, aperture),
            },
            Input::Click(ray) => match center {
                None => *center = first_point(scene, ray, aperture),
                Some((c, plane, _)) => {
                    if let Some((p, _)) = next_point(scene, ray, Some(*c), plane, None, aperture) {
                        *cursor = Some(p);
                        radius = Some((p - *c).length());
                    }
                }
            },
            Input::Typed(text) => match vcb::parse_in(&text, self.unit) {
                Some(Vcb::Sides(n)) => {
                    if !(lowest..=kernel::MAX_CIRCLE_SEGMENTS as u32).contains(&n) {
                        return Err(needs("jumlah sisi di luar batas (lingkaran minimal 24)"));
                    }
                    *sides = n;
                }
                Some(Vcb::One(r)) if center.is_some() && r > 0.0 => radius = Some(r),
                _ => {
                    return Err(needs(
                        "klik pusat, lalu ketik jari-jari, atau 8s untuk sisi",
                    ));
                }
            },
            Input::ClickAdd(_)
            | Input::LockAxis(_)
            | Input::LockNormal
            | Input::Cancel
            | Input::ToggleCopy => {}
        }
        if let (Some(r), Some((c, plane, tile))) = (radius, *center)
            && r >= MIN_LEN
        {
            let points = round_points(c, &plane, r, *sides, *cursor);
            scene.draw_curve(tile, plane, &points, true, c, *kind)?;
            *center = None;
            *cursor = None;
        }
        Ok(())
    }

    fn handle_arc(&mut self, scene: &mut Scene, input: Input) -> Result<(), EditError> {
        let aperture = self.aperture;
        let State::Arc { start, end, cursor } = &mut self.state else {
            unreachable!("dispatched on state")
        };
        let bulge = |a: Point3, b: Point3, plane: &Plane, p: Point3| {
            let side = plane.normal().cross(b - a).normalized().ok()?;
            Some((p - (a + (b - a) * 0.5)).dot(side))
        };
        let mut sagitta = None;
        match input {
            Input::Hover(ray) => match start {
                Some((a, plane, _)) => {
                    let anchor = if end.is_some() { None } else { Some(*a) };
                    let next = next_point(scene, ray, anchor, plane, None, aperture);
                    *cursor = next.map(|n| n.0);
                    self.preview.snap = next.and_then(|n| n.1);
                }
                None => self.preview.snap = start_snap(scene, ray, aperture),
            },
            Input::Click(ray) => match (&*start, *end) {
                (None, _) => *start = first_point(scene, ray, aperture),
                (Some((a, plane, _)), None) => {
                    if let Some((p, _)) = next_point(scene, ray, Some(*a), plane, None, aperture)
                        && (p - *a).length() >= MIN_LEN
                    {
                        *end = Some(p);
                    }
                }
                (Some((a, plane, _)), Some(b)) => {
                    if let Some((p, _)) = next_point(scene, ray, None, plane, None, aperture) {
                        sagitta = bulge(*a, b, plane, p);
                    }
                }
            },
            Input::Typed(text) => {
                let (Some((a, plane, _)), Some(b)) = (&*start, *end) else {
                    return Err(needs("klik dua ujung busur dulu"));
                };
                let Some(Vcb::One(s)) = vcb::parse_in(&text, self.unit) else {
                    return Err(needs("busur butuh satu tinggi lengkung"));
                };
                // Typed heights go to the cursor's side, as in the preview.
                let side = cursor
                    .and_then(|p| bulge(*a, b, plane, p))
                    .filter(|v| *v < 0.0)
                    .map_or(1.0, |_| -1.0);
                sagitta = Some(s * side);
            }
            Input::ClickAdd(_)
            | Input::LockAxis(_)
            | Input::LockNormal
            | Input::Cancel
            | Input::ToggleCopy => {}
        }
        if let (Some(s), Some((a, plane, tile)), Some(b)) = (sagitta, *start, *end)
            && let Some((points, center)) = arc_points(a, b, s, &plane)
        {
            scene.draw_curve(tile, plane, &points, false, center, SketchCurveKind::Circle)?;
            *start = None;
            *end = None;
            *cursor = None;
        }
        Ok(())
    }

    fn handle_offset(&mut self, scene: &mut Scene, input: Input) -> Result<(), EditError> {
        let aperture = self.aperture;
        let State::Offset {
            armed,
            cursor,
            hover,
        } = &mut self.state
        else {
            unreachable!("dispatched on state")
        };
        // Signed distance towards the cursor: outward when it is outside.
        let toward = |a: &Armed, p: Point3| {
            let plane = Plane::from_point_normal(a.anchor, a.normal).ok()?;
            let d = ring_distance(p, &a.outline);
            Some(if inside_ring(p, &a.outline, &plane) {
                -d
            } else {
                d
            })
        };
        let mut distance = None;
        match input {
            Input::Hover(ray) => match armed {
                Some(a) => {
                    let plane = Plane::from_point_normal(a.anchor, a.normal).ok();
                    *cursor = plane.and_then(|pl| ray_on_plane(&ray, &pl));
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
                    let plane = Plane::from_point_normal(a.anchor, a.normal).ok();
                    let p = plane.and_then(|pl| ray_on_plane(&ray, &pl));
                    distance = p.and_then(|p| toward(a, p));
                }
            },
            Input::Typed(text) => {
                let Some(a) = armed else {
                    return Err(needs("klik region atau face dulu"));
                };
                let Some(Vcb::One(d)) = vcb::parse_in(&text, self.unit) else {
                    return Err(needs("offset butuh satu jarak"));
                };
                // A plain number goes the way the cursor is; a sign flips that.
                let side = cursor
                    .and_then(|p| toward(a, p))
                    .filter(|v| *v < 0.0)
                    .map_or(1.0, |_| -1.0);
                distance = Some(d * side);
            }
            Input::ClickAdd(_)
            | Input::LockAxis(_)
            | Input::LockNormal
            | Input::Cancel
            | Input::ToggleCopy => {}
        }
        if let (Some(d), Some(a)) = (distance, armed.as_ref())
            && d.abs() >= MIN_LEN
        {
            let tile = match a.target {
                Target::Solid { tile, .. } | Target::Region { tile, .. } => tile,
            };
            let plane = Plane::from_point_normal(a.anchor, a.normal)
                .map_err(|_| needs("bidang tidak sah"))?;
            scene.offset(tile, plane, a.anchor, d)?;
            *armed = None;
            *cursor = None;
        }
        Ok(())
    }

    fn handle_tape(&mut self, scene: &mut Scene, input: Input) -> Result<(), EditError> {
        let aperture = self.aperture;
        let State::Tape { from, cursor } = &mut self.state else {
            unreachable!("dispatched on state")
        };
        let point = |scene: &mut Scene, ray: PickRay, a: Point3| {
            let q = SnapQuery {
                anchor: Some(a),
                ..query(ray, aperture)
            };
            let hit = scene.snap(&q);
            let p = hit.map(|h| h.snap.position).or_else(|| {
                let plane = Plane::from_point_normal(a, Vec3::new(0.0, 0.0, 1.0)).ok()?;
                ray_on_plane(&ray, &plane)
            });
            (p, hit)
        };
        let mut to = None;
        match input {
            Input::Hover(ray) => match from {
                Some((a, _)) => {
                    let (p, hit) = point(scene, ray, *a);
                    *cursor = p;
                    self.preview.snap = hit;
                }
                None => self.preview.snap = scene.snap(&query(ray, aperture)),
            },
            Input::Click(ray) => match from {
                Some((a, _)) => to = point(scene, ray, *a).0,
                None => {
                    let hit = scene.snap(&query(ray, aperture));
                    let start = hit
                        .map(|h| h.snap.position)
                        .or_else(|| ray_on_plane(&ray, &ground()));
                    *from = start.map(|p| (p, hit.and_then(|h| scene.edge_direction(&h))));
                }
            },
            Input::Typed(text) => {
                let (Some((a, along)), Some(c)) = (&*from, *cursor) else {
                    return Err(needs("klik titik awal lalu arahkan kursor dulu"));
                };
                let Some(Vcb::One(len)) = vcb::parse_in(&text, self.unit) else {
                    return Err(needs("pita ukur butuh satu jarak"));
                };
                if len.abs() < MIN_LEN {
                    return Err(needs("jarak pita ukur tidak boleh nol"));
                }
                let dir = tape_span(*a, *along, c)
                    .normalized()
                    .map_err(|_| needs("arahkan kursor dulu"))?;
                to = Some(*a + dir * len);
            }
            Input::ClickAdd(_)
            | Input::LockAxis(_)
            | Input::LockNormal
            | Input::Cancel
            | Input::ToggleCopy => {}
        }
        if let (Some(b), Some((a, along))) = (to, *from)
            && (b - a).length() >= MIN_LEN
        {
            match along {
                // From an edge: a parallel line. Measured along that edge it
                // would be the edge's own line, so a point instead.
                Some(u) if (b - a).cross(u).length() > MIN_LEN => scene.add_guide_line(b, u)?,
                _ => scene.add_guide_point(b)?,
            }
            *from = None;
            *cursor = None;
        }
        Ok(())
    }

    fn handle_follow(&mut self, scene: &mut Scene, input: Input) -> Result<(), EditError> {
        let aperture = self.aperture;
        let State::Follow { profile, outline } = &mut self.state else {
            unreachable!("dispatched on state")
        };
        match input {
            Input::Hover(ray) => {
                self.preview.snap = scene.snap(&pick_query(ray, aperture));
            }
            Input::Click(ray) => {
                let hit = scene.snap(&pick_query(ray, aperture));
                match *profile {
                    None => {
                        if let Some(armed) = Self::arm(scene, ray) {
                            *profile = Some(armed.target);
                            *outline = scene.outline(armed.target);
                        } else if hit.is_some() {
                            return Err(needs("klik region atau face sebagai penampang"));
                        }
                    }
                    Some(target) => {
                        let Some(h) = hit else { return Ok(()) };
                        let Some(path) = h.snap.sketch_source else {
                            return Err(needs("klik garis sebagai jalur"));
                        };
                        let tile = match target {
                            Target::Region { tile, .. } | Target::Solid { tile, .. } => tile,
                        };
                        if h.tile != tile {
                            return Err(needs("penampang dan jalur harus di tile yang sama"));
                        }
                        match target {
                            Target::Region { sketch, region, .. } => {
                                scene.follow_me(tile, (sketch, region), path)?
                            }
                            Target::Solid { source, .. } => {
                                scene.follow_me_face(tile, source, path)?
                            }
                        }
                        *profile = None;
                        outline.clear();
                    }
                }
            }
            Input::ClickAdd(_)
            | Input::Typed(_)
            | Input::LockAxis(_)
            | Input::LockNormal
            | Input::Cancel
            | Input::ToggleCopy => {}
        }
        Ok(())
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
            Input::Typed(_)
            | Input::ClickAdd(_)
            | Input::LockAxis(_)
            | Input::LockNormal
            | Input::Cancel
            | Input::ToggleCopy => {}
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
            Input::Typed(text) => match (&*moving, vcb::parse_in(&text, self.unit)) {
                (Some(m), Some(Vcb::One(len))) => {
                    let cursor = m.cursor.ok_or(needs("arahkan kursor dulu"))?;
                    let toward = cursor - m.base;
                    // Under a lock the cursor only says which way along the axis:
                    // it may predate the lock.
                    let dir = match m.lock {
                        Some(axis) if toward.dot(axis.unit()) < 0.0 => axis.unit() * -1.0,
                        Some(axis) => axis.unit(),
                        None => toward
                            .normalized()
                            .map_err(|_| needs("arahkan kursor dulu"))?,
                    };
                    let m = moving.as_mut().expect("matched above");
                    if Self::finish_move(scene, m, dir * len, *copy, last)? {
                        *moving = None;
                        *copy = false;
                    }
                }
                (None, Some(v @ (Vcb::Copies(_) | Vcb::Divide(_)))) => {
                    let (what, step) = last.as_ref().ok_or(needs("salin sesuatu dulu"))?;
                    let (n, part) = typed_array(v).expect("matched above");
                    scene.respace_copy(what, &Transform::translation(*step * part), n)?;
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
            Input::ClickAdd(_) | Input::LockNormal | Input::Cancel => {}
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
            scene.copy(&m.what, &Transform::translation(delta), NonZeroU32::MIN)?;
            *last = Some((m.what.clone(), delta));
        } else {
            scene.transform(&m.what, &Transform::translation(delta))?;
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
                let Some(Vcb::One(d)) = vcb::parse_in(&text, self.unit) else {
                    return Err(needs("Push/Pull butuh satu jarak, mis. 3"));
                };
                if d.abs() < MIN_LEN {
                    return Err(needs("jarak tidak boleh nol"));
                }
                Self::commit_push_pull(scene, a.target, d)?;
                *armed = None;
            }
            Input::ClickAdd(_)
            | Input::LockAxis(_)
            | Input::LockNormal
            | Input::Cancel
            | Input::ToggleCopy => {}
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
            State::Tape { from, cursor } => match (from, cursor) {
                (Some((a, along)), Some(b)) => {
                    let span = tape_span(*a, *along, *b);
                    let end = *a + span;
                    let mut lines = vec![(*a, end)];
                    // The guide it would leave.
                    if let Some(u) = along
                        && span.cross(*u).length() > MIN_LEN
                    {
                        lines.push((end - *u * GUIDE_REACH_M, end + *u * GUIDE_REACH_M));
                    }
                    lines
                }
                _ => Vec::new(),
            },
            State::Follow { outline, .. } => outline.clone(),
            State::Round {
                sides,
                center,
                cursor,
                ..
            } => match (center, cursor) {
                (Some((c, plane, _)), Some(p)) => ring(&round_points(
                    *c,
                    plane,
                    (*p - *c).length(),
                    *sides,
                    *cursor,
                )),
                _ => Vec::new(),
            },
            State::Arc { start, end, cursor } => match (start, end, cursor) {
                (Some((a, _, _)), None, Some(p)) => vec![(*a, *p)],
                (Some((a, plane, _)), Some(b), Some(p)) => plane
                    .normal()
                    .cross(*b - *a)
                    .normalized()
                    .ok()
                    .and_then(|side| {
                        arc_points(*a, *b, (*p - (*a + (*b - *a) * 0.5)).dot(side), plane)
                    })
                    .map_or_else(
                        || vec![(*a, *b)],
                        |(pts, _)| pts.windows(2).map(|w| (w[0], w[1])).collect(),
                    ),
                _ => Vec::new(),
            },
            State::Offset { armed, hover, .. } => armed
                .as_ref()
                .map_or_else(|| hover.clone(), |a| ring(&a.outline)),
            State::Rotate { turning, hover, .. } => match turning {
                None => hover.clone(),
                Some(t) => rotation_about(t.center, t.axis, t.angle).map_or_else(
                    |_| t.lines.clone(),
                    |r| {
                        t.lines
                            .iter()
                            .map(|(a, b)| (r.apply_point(*a), r.apply_point(*b)))
                            .collect()
                    },
                ),
            },
            State::Scale { scaling, hover } => match scaling {
                None => hover.clone(),
                Some(s) => {
                    let t = scaling_about(s.center, s.factor);
                    s.lines
                        .iter()
                        .map(|(a, b)| (t.apply_point(*a), t.apply_point(*b)))
                        .collect()
                }
            },
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
