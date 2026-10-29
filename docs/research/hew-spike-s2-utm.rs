//! WangunGIS spike S2 (ADR-000 criterion #3): push/pull precision at large
//! UTM coordinates vs. at the origin. Not part of Hew; local experiment only.
//! Run: cargo test -p kernel --test zz_wangungis_utm_spike -- --nocapture

use kernel::{Document, FaceId, KernelOp, ObjectId, Plane, Point3, Vec3, tol};

mod face_arrangement_support;
use face_arrangement_support::draw_loop;

fn p(x: f64, y: f64, z: f64) -> Point3 {
    Point3::new(x, y, z)
}

/// Footprint corner at local (u, v) for a box rotated by `rot` radians about (x0, y0).
fn at(x0: f64, y0: f64, rot: f64, u: f64, v: f64, z: f64) -> Point3 {
    let (s, c) = rot.sin_cos();
    p(x0 + u * c - v * s, y0 + u * s + v * c, z)
}

fn make_box(doc: &mut Document, x0: f64, y0: f64, rot: f64, w: f64, d: f64, h: f64) -> ObjectId {
    let plane = Plane::from_polygon(&[p(x0, y0, 0.0), p(x0 + 1.0, y0, 0.0), p(x0, y0 + 1.0, 0.0)])
        .expect("plane");
    let s = doc.add_sketch(plane);
    {
        let sk = doc.sketch_mut(s).expect("sketch");
        let c = [at(x0, y0, rot, 0.0, 0.0, 0.0), at(x0, y0, rot, w, 0.0, 0.0), at(x0, y0, rot, w, d, 0.0), at(x0, y0, rot, 0.0, d, 0.0)];
        for k in 0..4 {
            sk.add_segment(c[k], c[(k + 1) % 4]).expect("segment");
        }
    }
    let regions = doc.extrudable_regions(s).expect("regions");
    doc.extrude_region(s, regions[0], h).expect("extrude").0
}

/// The face of `obj` with outward normal `dir` whose plane passes through `on`.
fn face_with(doc: &Document, obj: ObjectId, dir: Vec3, on: Point3) -> Option<FaceId> {
    let o = doc.object(obj)?;
    o.faces()
        .iter()
        .filter(|(_, f)| f.plane.normal().approx_eq(dir, tol::NORMAL_DIRECTION))
        .min_by(|a, b| {
            let da = a.1.plane.signed_distance(on).abs();
            let db = b.1.plane.signed_distance(on).abs();
            da.total_cmp(&db)
        })
        .map(|(id, _)| id)
}

fn positions(doc: &Document, obj: ObjectId) -> Vec<Point3> {
    doc.object(obj).expect("object").vertices().values().map(|v| v.position).collect()
}

/// Distance from `v` to the nearest value in `set`.
fn off(v: f64, set: &[f64]) -> f64 {
    set.iter().map(|s| (v - s).abs()).fold(f64::INFINITY, f64::min)
}

struct Run {
    label: &'static str,
    ok: usize,
    failed: usize,
    first_err: Option<String>,
    max_err: f64,
    imprint: String,
    undo_err: f64,
}

fn run(label: &'static str, x0: f64, y0: f64, rot: f64) -> Run {
    let (w, d, h) = (5.0, 4.0, 3.0);
    let mut doc = Document::new();
    let obj = make_box(&mut doc, x0, y0, rot, w, d, h);
    let start = positions(&doc, obj);
    let (s, c) = rot.sin_cos();
    let east_dir = Vec3::new(c, s, 0.0);

    // Exact bookkeeping in units of 0.5 mm (every distance is a multiple of it).
    let (mut top_k, mut east_k): (i64, i64) = (0, 0);
    let (mut top, mut east) = (h, w);
    let mut steps = 0usize;
    let (mut ok, mut failed, mut first_err) = (0, 0, None);
    for i in 0..100usize {
        // Distances 1..17 mm, alternating pull (+) and a smaller push (-).
        let k = (((i * 37) % 17) + 1) as i64 * 2; // 1..17 mm in 0.5 mm units
        let dk = if i % 2 == 0 { k } else { -k / 2 };
        let dist = dk as f64 * 0.0005;
        let (dir, on) = if i % 3 == 0 {
            (east_dir, at(x0, y0, rot, east, d / 2.0, top / 2.0))
        } else {
            (Vec3::new(0.0, 0.0, 1.0), at(x0, y0, rot, w / 2.0, d / 2.0, top))
        };
        let face = face_with(&doc, obj, dir, on).expect("face");
        match doc.apply_object_op(obj, KernelOp::PushPull { face, distance: dist }) {
            Ok(_) => {
                ok += 1;
                steps += 1;
                if i % 3 == 0 {
                    east_k += dk;
                    east = w + east_k as f64 * 0.0005;
                } else {
                    top_k += dk;
                    top = h + top_k as f64 * 0.0005;
                }
            }
            Err(e) => {
                failed += 1;
                first_err.get_or_insert_with(|| format!("op {i}: {e:?}"));
            }
        }
    }

    // Error against the exact expected box, measured in the box's local frame.
    let us = [0.0, east];
    let vs = [0.0, d];
    let zs = [0.0, top];
    // Bijective match: the box must still have exactly 8 vertices, one per
    // expected corner of {0,east} x {0,d} x {0,top}; error = max Euclidean
    // distance (local frame) from each corner to its vertex.
    let local: Vec<[f64; 3]> = positions(&doc, obj)
        .iter()
        .map(|q| {
            let (dx, dy) = (q.x - x0, q.y - y0);
            [dx * c + dy * s, -dx * s + dy * c, q.z]
        })
        .collect();
    let mut max_err = if local.len() == 8 { 0.0 } else { f64::INFINITY };
    let mut used = [false; 8];
    for &u in &us {
        for &v in &vs {
            for &z in &zs {
                let (k, dist) = local
                    .iter()
                    .enumerate()
                    .map(|(k, q)| (k, ((q[0] - u).powi(2) + (q[1] - v).powi(2) + (q[2] - z).powi(2)).sqrt()))
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .unwrap();
                if k < 8 && !used[k] { used[k] = true; } else { max_err = f64::INFINITY; }
                max_err = max_err.max(dist);
            }
        }
    }

    // Imprint a 1x1 m rectangle on the top face, then raise it 0.5 m (boss).
    let top_face = face_with(&doc, obj, Vec3::new(0.0, 0.0, 1.0), at(x0, y0, rot, 1.0, 1.0, top))
        .expect("top face");
    let r = [
        at(x0, y0, rot, 1.0, 1.0, top),
        at(x0, y0, rot, 2.0, 1.0, top),
        at(x0, y0, rot, 2.0, 2.0, top),
        at(x0, y0, rot, 1.0, 2.0, top),
    ];
    let imprint = match draw_loop(&mut doc, None, obj, top_face, &r, None) {
        Err(e) => format!("imprint FAILED: {e:?}"),
        Ok(_) => {
            steps += 1;
            let sub = doc.object(obj).and_then(|o| {
                o.faces()
                    .iter()
                    .filter(|(_, f)| f.plane.normal().approx_eq(Vec3::new(0.0, 0.0, 1.0), tol::NORMAL_DIRECTION))
                    .find(|(id, _)| o.is_flat_sub_face(*id))
                    .map(|(id, _)| id)
            });
            match sub {
                None => "imprint ok, no sub-face found".into(),
                Some(sf) => match doc.apply_object_op(obj, KernelOp::ExtrudeSubFace { sub_face: sf, distance: 0.5 }) {
                    Ok(_) => {
                        steps += 1;
                        "imprint + boss ok".into()
                    }
                    Err(e) => format!("boss FAILED: {e:?}"),
                },
            }
        }
    };

    let mut undo_fail = None;
    for _ in 0..steps {
        if let Err(e) = doc.undo() {
            undo_fail = Some(format!("{e:?}"));
            break;
        }
    }
    if let Some(e) = undo_fail {
        first_err.get_or_insert(format!("undo: {e}"));
    }
    let undo_err = match doc.object(obj) {
        None => f64::NAN,
        Some(_) => {
            let mut a = positions(&doc, obj);
            let mut b = start.clone();
            let key = |q: &Point3| (q.x, q.y, q.z);
            a.sort_by(|m, n| key(m).partial_cmp(&key(n)).unwrap());
            b.sort_by(|m, n| key(m).partial_cmp(&key(n)).unwrap());
            if a.len() != b.len() {
                f64::INFINITY
            } else {
                a.iter().zip(&b).map(|(m, n)| (*m - *n).length()).fold(0.0, f64::max)
            }
        }
    };

    Run { label, ok, failed, first_err, max_err, imprint, undo_err }
}

fn row(r: Run) {
    println!(
        "ROW | {} | {} | {} | {:.3e} | {} | {:.3e} | {} |",
        r.label, r.ok, r.failed, r.max_err, r.imprint, r.undo_err,
        r.first_err.as_deref().unwrap_or("-")
    );
}

const R30: f64 = std::f64::consts::FRAC_PI_6;

#[test] fn a_origin_aligned() { row(run("origin, axis-aligned", 0.0, 0.0, 0.0)) }
#[test] fn b_utm_aligned() { row(run("UTM (430000, 9150000), axis-aligned", 430_000.0, 9_150_000.0, 0.0)) }
#[test] fn c_utm_frac_aligned() { row(run("UTM + fraction, axis-aligned", 430_000.123, 9_150_000.456, 0.0)) }
#[test] fn d_origin_rot() { row(run("origin, rotated 30deg", 0.0, 0.0, R30)) }
#[test] fn e_east_rot() { row(run("easting only (430000, 0), rotated 30deg", 430_000.0, 0.0, R30)) }
#[test] fn f_north_rot() { row(run("northing only (0, 9150000), rotated 30deg", 0.0, 9_150_000.0, R30)) }
#[test] fn g_utm_frac_rot() { row(run("UTM + fraction, rotated 30deg", 430_000.123, 9_150_000.456, R30)) }
#[test] fn h_local_100m_rot() { row(run("local (100, 200), rotated 30deg", 100.0, 200.0, R30)) }
#[test] fn i_10km_rot() { row(run("10 km (10000, 10000), rotated 30deg", 10_000.0, 10_000.0, R30)) }
