//! WangunGIS spike for option A2: does Hew's `inference` crate (snapping +
//! face picking) stay interactive at city scale when used natively?
//! Usage: cargo run --release -- <model.hew> [queries]

use inference::{ApertureMode, InferenceScene, PickRay, SnapQuery, SnapWeights};
use kernel::{Point3, Transform, Vec3};
use std::time::Instant;

fn pct(v: &mut [f64], p: usize) -> f64 {
    v.sort_by(f64::total_cmp);
    v[(v.len() - 1) * p / 100]
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(1000);
    let doc = kernel::Document::load(&std::fs::read(&args[1]).unwrap()).unwrap();

    let t = Instant::now();
    let mut scene = InferenceScene::new();
    let (mut objects, mut lo, mut hi) = (0usize, [f64::MAX; 2], [f64::MIN; 2]);
    for oid in doc.live_object_ids() {
        if !doc.is_world_object(oid) { continue; }
        let obj = doc.object(oid).unwrap();
        for v in obj.vertices().values() {
            lo = [lo[0].min(v.position.x), lo[1].min(v.position.y)];
            hi = [hi[0].max(v.position.x), hi[1].max(v.position.y)];
        }
        scene.add_object(oid, obj, &Transform::IDENTITY);
        objects += 1;
    }
    let build = t.elapsed();

    // Deterministic pseudo-random targets across the scene, viewed from a drone-height camera.
    let mut seed = 0x9E3779B97F4A7C15u64;
    let mut rnd = || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; (seed >> 11) as f64 / (1u64 << 53) as f64 };
    let (mut snap_us, mut pick_us, mut hits, mut picks) = (Vec::new(), Vec::new(), 0, 0);
    for _ in 0..n {
        let target = Point3::new(lo[0] + rnd() * (hi[0] - lo[0]), lo[1] + rnd() * (hi[1] - lo[1]), 4.5);
        let eye = Point3::new(target.x - 60.0, target.y - 80.0, 120.0);
        let ray = PickRay { origin: eye, direction: target - eye };
        let q = SnapQuery {
            weights: SnapWeights::default(), ray, anchor: None, lock: None,
            aperture: 0.01, aperture_mode: ApertureMode::Cone, constraint_plane: None,
            soft_axis_aperture_scale: None, off_plane_points: false, see_through_points: false,
        };
        let t = Instant::now();
        hits += scene.resolve(&q).is_some() as usize;
        snap_us.push(t.elapsed().as_secs_f64() * 1e6);
        let t = Instant::now();
        picks += scene.pick_face(&ray).is_some() as usize;
        pick_us.push(t.elapsed().as_secs_f64() * 1e6);
    }
    let _ = Vec3::new(0.0, 0.0, 1.0);
    let first = snap_us[0];
    println!("first snap query {first:.0} us; max of the rest {:.0} us", snap_us[1..].iter().cloned().fold(0.0, f64::max));
    println!(
        "objects {objects}, index build {build:.2?}; {n} queries: snap resolve median {:.0} us / p95 {:.0} us / max {:.0} us ({hits} snapped); pick_face median {:.0} us / p95 {:.0} us ({picks} hit)",
        pct(&mut snap_us.clone(), 50), pct(&mut snap_us.clone(), 95), pct(&mut snap_us, 100),
        pct(&mut pick_us.clone(), 50), pct(&mut pick_us, 95)
    );
}
