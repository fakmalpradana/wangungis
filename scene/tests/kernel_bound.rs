//! K6 gate (ADR-004): Hew's kernel, with its 1e-9 m tolerances, must accept
//! and stay exact (≤ 1 mm) for typical building operations anywhere inside one
//! kernel frame, i.e. within KERNEL_FRAME_BOUND_M of its origin. Spike S2 only swept z-rotated boxes;
//! this adds tilted (roof) planes, circles, and booleans.
//!
//! `far_rotated_boolean_is_refused` records why the frame is small: Hew's
//! boolean refuses rotated geometry from ~2.8 km out. If Hew fixes that, the
//! test fails and the bound can be revisited.
//!
//! Every operation that the kernel refuses here is a failure: inside the bound
//! a user must never see "operation refused" for ordinary geometry.

use kernel::sketch::CurveGeom;
use kernel::{BooleanOp, Document, KernelOp, ObjectId, Plane, Point3, Vec3, tol};
use proptest::prelude::*;
use wangungis_scene::KERNEL_FRAME_BOUND_M as BOUND;

const MM: f64 = 1e-3;

fn p(x: f64, y: f64, z: f64) -> Point3 {
    Point3::new(x, y, z)
}

fn dot(a: Vec3, b: Vec3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn verts(doc: &Document, o: ObjectId) -> Vec<Point3> {
    doc.object(o)
        .expect("object")
        .vertices()
        .values()
        .map(|v| v.position)
        .collect()
}

/// Closed polygon sketched on `plane` and extruded `h` along its normal.
fn extrude_polygon(doc: &mut Document, plane: Plane, ring: &[Point3], h: f64) -> ObjectId {
    let sk = doc.add_sketch(plane);
    let s = doc.sketch_mut(sk).expect("sketch");
    for k in 0..ring.len() {
        s.add_segment(ring[k], ring[(k + 1) % ring.len()])
            .expect("segment");
    }
    let regions = doc.extrudable_regions(sk).expect("regions");
    assert_eq!(regions.len(), 1, "one closed region");
    doc.extrude_region(sk, regions[0], h).expect("extrude").0
}

/// Local frame of a footprint at (ox, oy, 0) rotated `rot` about z.
struct Frame {
    o: Point3,
    u: Vec3,
    v: Vec3,
}

impl Frame {
    fn new(ox: f64, oy: f64, rot: f64) -> Self {
        let (s, c) = rot.sin_cos();
        Frame {
            o: p(ox, oy, 0.0),
            u: Vec3::new(c, s, 0.0),
            v: Vec3::new(-s, c, 0.0),
        }
    }
    fn at(&self, a: f64, b: f64, z: f64) -> Point3 {
        self.o + self.u * a + self.v * b + Vec3::new(0.0, 0.0, z)
    }
    /// (a, b, z) of `q` in this frame.
    fn local(&self, q: Point3) -> [f64; 3] {
        let d = q - self.o;
        [dot(d, self.u), dot(d, self.v), d.z]
    }
}

fn ground(f: &Frame) -> Plane {
    Plane::from_polygon(&[
        f.at(0.0, 0.0, 0.0),
        f.at(1.0, 0.0, 0.0),
        f.at(0.0, 1.0, 0.0),
    ])
    .expect("plane")
}

#[allow(clippy::too_many_arguments)]
fn box_at(
    doc: &mut Document,
    f: &Frame,
    a0: f64,
    b0: f64,
    w: f64,
    d: f64,
    z0: f64,
    h: f64,
) -> ObjectId {
    let plane = Plane::from_polygon(&[
        f.at(a0, b0, z0),
        f.at(a0 + 1.0, b0, z0),
        f.at(a0, b0 + 1.0, z0),
    ])
    .expect("plane");
    let ring = [
        f.at(a0, b0, z0),
        f.at(a0 + w, b0, z0),
        f.at(a0 + w, b0 + d, z0),
        f.at(a0, b0 + d, z0),
    ];
    extrude_polygon(doc, plane, &ring, h)
}

/// The face of `o` whose outward normal is ±`n` and whose plane is farthest
/// from `from` (the extruded cap, not the sketch face).
fn far_cap(doc: &Document, o: ObjectId, n: Vec3, from: Point3) -> kernel::FaceId {
    doc.object(o)
        .expect("object")
        .faces()
        .iter()
        .filter(|(_, f)| {
            let fnrm = f.plane.normal();
            fnrm.approx_eq(n, tol::NORMAL_DIRECTION) || fnrm.approx_eq(-n, tol::NORMAL_DIRECTION)
        })
        .max_by(|a, b| {
            a.1.plane
                .signed_distance(from)
                .abs()
                .total_cmp(&b.1.plane.signed_distance(from).abs())
        })
        .map(|(id, _)| id)
        .expect("cap face")
}

/// A footprint corner inside one kernel frame: a point of the disk of radius
/// BOUND - 40 m (shapes are at most 30 m wide), with the rim weighted in.
fn corner() -> impl Strategy<Value = (f64, f64)> {
    let r = BOUND - 40.0;
    let radius = prop_oneof![3 => 0.0..r, 1 => Just(r)];
    (radius, 0.0..std::f64::consts::TAU).prop_map(|(d, a)| (d * a.cos(), d * a.sin()))
}

#[cfg(not(debug_assertions))]
fn pocket_box(ox: f64, oy: f64, rot: f64) -> Result<usize, kernel::DocumentError> {
    let f = Frame::new(ox, oy, rot);
    let mut doc = Document::new();
    let body = box_at(&mut doc, &f, 0.0, 0.0, 6.0, 6.0, 0.0, 3.0);
    let tool = box_at(&mut doc, &f, 0.6, 0.6, 2.4, 2.4, 0.6, 3.0);
    let (r, _) = doc.boolean(BooleanOp::Subtract, body, tool)?;
    Ok(verts(&doc, r).len())
}

/// Characterization of the Hew limit behind KERNEL_FRAME_BOUND_M (release
/// only: in debug builds the kernel's invariant check panics before the typed
/// refusal). Measured 2026-11-01 on Hew 612c1c1 with random boxes and
/// rotations: 0/3000 refused up to 2.5 km, 20/3000 at 2.79 km, 47/3000 at
/// 3 km; with this fixed box, 63/72 rotations at 10 km.
#[cfg(not(debug_assertions))]
#[test]
fn far_rotated_boolean_is_refused() {
    let rots = (0..72).map(|k| (k as f64 * 5.0).to_radians() + 1e-4);
    let refused = rots
        .clone()
        .filter(|&r| pocket_box(0.0, -10_000.0, r).is_err())
        .count();
    assert!(
        refused > 0,
        "Hew booleans no longer refuse at 10 km: revisit KERNEL_FRAME_BOUND_M (ADR-004)"
    );
    let near = rots
        .filter(|&r| pocket_box(0.0, -BOUND, r).is_err())
        .count();
    assert_eq!(near, 0, "refusals inside the kernel frame");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Roof-like slab on a plane pitched 15–60°: extrude, then push/pull the
    /// pitched cap. The slab must keep 8 vertices and its thickness must be
    /// exactly h + d.
    #[test]
    fn tilted_plane_push_pull(
        (ox, oy) in corner(), rot in 0.0..std::f64::consts::TAU,
        pitch_deg in 15.0..60.0f64, w in 2.0..20.0f64, d in 2.0..15.0f64,
        h in 0.05..0.5f64, dist in 0.01..2.0f64,
    ) {
        let f = Frame::new(ox, oy, rot);
        let (sp, cp) = pitch_deg.to_radians().sin_cos();
        // Rafter direction: climbs along v with the given pitch.
        let rafter = f.v * cp + Vec3::new(0.0, 0.0, sp);
        let base = f.at(0.0, 0.0, 3.0);
        let ring = [base, base + f.u * w, base + f.u * w + rafter * d, base + rafter * d];
        let plane = Plane::from_polygon(&ring).expect("pitched plane");
        let n = plane.normal();
        let mut doc = Document::new();
        let o = extrude_polygon(&mut doc, plane, &ring, h);
        let cap = far_cap(&doc, o, n, base);
        doc.apply_object_op(o, KernelOp::PushPull { face: cap, distance: dist })
            .map_err(|e| TestCaseError::fail(format!("push/pull refused at ({ox:.1}, {oy:.1}): {e:?}")))?;
        let vs = verts(&doc, o);
        prop_assert_eq!(vs.len(), 8);
        let proj: Vec<f64> = vs.iter().map(|q| dot(*q - base, n)).collect();
        let span = proj.iter().cloned().fold(f64::NEG_INFINITY, f64::max) - proj.iter().cloned().fold(f64::INFINITY, f64::min);
        prop_assert!((span - (h + dist)).abs() <= MM, "thickness {span} vs {}", h + dist);
    }

    /// Faceted circle (48 segments) extruded, then its top pushed up. Every
    /// vertex must stay on the circle and on z ∈ {0, h + d}.
    #[test]
    fn circle_extrude_push_pull(
        (ox, oy) in corner(), r in 0.5..10.0f64, h in 1.0..20.0f64, dist in 0.01..3.0f64,
    ) {
        let f = Frame::new(ox, oy, 0.0);
        let c = f.at(0.0, 0.0, 0.0);
        let mut doc = Document::new();
        let sk = doc.add_sketch(ground(&f));
        let s = doc.sketch_mut(sk).expect("sketch");
        s.begin_curve_with(CurveGeom { center: c, radius: r }).expect("curve");
        let n = 48;
        let ring: Vec<Point3> = (0..n)
            .map(|i| {
                let a = std::f64::consts::TAU * i as f64 / n as f64;
                c + Vec3::new(r * a.cos(), r * a.sin(), 0.0)
            })
            .collect();
        for i in 0..n {
            s.add_segment(ring[i], ring[(i + 1) % n]).expect("segment");
        }
        s.end_curve();
        let regions = doc.extrudable_regions(sk).expect("regions");
        let (o, _) = doc.extrude_region(sk, regions[0], h)
            .map_err(|e| TestCaseError::fail(format!("circle extrude refused at ({ox:.1}, {oy:.1}): {e:?}")))?;
        let top = far_cap(&doc, o, Vec3::new(0.0, 0.0, 1.0), c);
        doc.apply_object_op(o, KernelOp::PushPull { face: top, distance: dist })
            .map_err(|e| TestCaseError::fail(format!("push/pull refused at ({ox:.1}, {oy:.1}): {e:?}")))?;
        let vs = verts(&doc, o);
        prop_assert_eq!(vs.len(), 2 * n);
        for q in vs {
            let [a, b, z] = f.local(q);
            prop_assert!(((a * a + b * b).sqrt() - r).abs() <= MM, "radius off at {q:?}");
            prop_assert!(z.abs() <= MM || (z - (h + dist)).abs() <= MM, "z off at {q:?}");
        }
    }

    /// Boolean subtract: a pocket cut down from the top of a rotated box. The
    /// result must have the 16 expected corners (box + pocket), each ≤ 1 mm off.
    #[test]
    fn boolean_pocket(
        (ox, oy) in corner(), rot in 0.0..std::f64::consts::TAU,
        w in 6.0..30.0f64, d in 6.0..30.0f64, h in 3.0..20.0f64,
        fa in 0.1..0.4f64, fb in 0.1..0.4f64, fz in 0.2..0.8f64,
    ) {
        let f = Frame::new(ox, oy, rot);
        let mut doc = Document::new();
        let body = box_at(&mut doc, &f, 0.0, 0.0, w, d, 0.0, h);
        let (a0, b0, z0) = (fa * w, fb * d, fz * h);
        let (pw, pd) = (w * 0.4, d * 0.4);
        let tool = box_at(&mut doc, &f, a0, b0, pw, pd, z0, h);
        let (res, _) = doc.boolean(BooleanOp::Subtract, body, tool)
            .map_err(|e| TestCaseError::fail(format!("boolean refused at ({ox:.1}, {oy:.1}): {e:?}")))?;
        let vs = verts(&doc, res);
        prop_assert_eq!(vs.len(), 16);
        let near = |x: f64, set: &[f64]| set.iter().any(|s| (x - s).abs() <= MM);
        for q in vs {
            let [a, b, z] = f.local(q);
            prop_assert!(near(a, &[0.0, w, a0, a0 + pw]), "a off at {q:?}");
            prop_assert!(near(b, &[0.0, d, b0, b0 + pd]), "b off at {q:?}");
            prop_assert!(near(z, &[0.0, h, z0]), "z off at {q:?}");
        }
    }
}
