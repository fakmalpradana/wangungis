//! WangunGIS spike: does per-tile snapping (3x3 neighbourhood, merged by
//! (SnapKind, angular distance)) agree with one global InferenceScene?
//! Usage: cargo run --release --bin tiled_agree -- <model.hew> [queries]

use inference::{ApertureMode, InferenceScene, PickRay, Snap, SnapQuery, SnapWeights};
use kernel::{Point3, Transform};
use std::collections::BTreeMap;

const TILE: f64 = 100.0;
const OCCLUDE: bool = true;
const MODE_METRIC: bool = false;

fn q(ray: PickRay) -> SnapQuery {
    SnapQuery {
        weights: SnapWeights::default(), ray, anchor: None, lock: None, aperture: 0.01,
        aperture_mode: ApertureMode::Cone, constraint_plane: None, soft_axis_aperture_scale: None,
        off_plane_points: false, see_through_points: false,
    }
}

fn angle(ray: &PickRay, p: Point3) -> f64 {
    let d = ray.direction;
    let v = p - ray.origin;
    let c = (d.x * v.x + d.y * v.y + d.z * v.z) / ((d.x * d.x + d.y * d.y + d.z * d.z).sqrt() * (v.x * v.x + v.y * v.y + v.z * v.z).sqrt());
    c.clamp(-1.0, 1.0).acos()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(2000);
    let doc = kernel::Document::load(&std::fs::read(&args[1]).unwrap()).unwrap();
    let mut global = InferenceScene::new();
    let mut tiles: BTreeMap<(i64, i64), InferenceScene> = BTreeMap::new();
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for oid in doc.live_object_ids() {
        if !doc.is_world_object(oid) { continue; }
        let obj = doc.object(oid).unwrap();
        let p = obj.vertices().values().next().unwrap().position;
        for v in obj.vertices().values() {
            lo = [lo[0].min(v.position.x), lo[1].min(v.position.y)];
            hi = [hi[0].max(v.position.x), hi[1].max(v.position.y)];
        }
        global.add_object(oid, obj, &Transform::IDENTITY);
        let key = ((p.x / TILE).floor() as i64, (p.y / TILE).floor() as i64);
        tiles.entry(key).or_insert_with(InferenceScene::new).add_object(oid, obj, &Transform::IDENTITY);
    }
    let mut seed = 0x2545F4914F6CDD1Du64;
    let mut rnd = || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; (seed >> 11) as f64 / (1u64 << 53) as f64 };
    let (mut same, mut same_kind, mut both_none, mut diff) = (0, 0, 0, Vec::new());
    for _ in 0..n {
        let target = Point3::new(lo[0] + rnd() * (hi[0] - lo[0]), lo[1] + rnd() * (hi[1] - lo[1]), 4.5);
        let eye = Point3::new(target.x - 60.0 * rnd(), target.y - 80.0 * rnd(), 60.0 + 60.0 * rnd());
        let ray = PickRay { origin: eye, direction: target - eye };
        let g: Option<Snap> = global.resolve(&q(ray));
        // Tile under the cursor's ground hit + its 8 neighbours.
        let (ti, tj) = ((target.x / TILE).floor() as i64, (target.y / TILE).floor() as i64);
        let mut best: Option<Snap> = None;
        let near: Vec<&InferenceScene> = (-1..=1).flat_map(|di| (-1..=1).map(move |dj| (ti + di, tj + dj)))
            .filter_map(|k| tiles.get(&k)).collect();
        // Nearest face hit across the neighbourhood: a candidate behind it is
        // occluded by geometry in ANOTHER tile, which that tile's scene cannot see.
        let hit_t = near.iter().filter_map(|sc| sc.pick_face(&ray).map(|(_, t)| t)).fold(f64::INFINITY, f64::min);
        let dd = ray.direction.x * ray.direction.x + ray.direction.y * ray.direction.y + ray.direction.z * ray.direction.z;
        let param = |p: Point3| { let v = p - ray.origin; (v.x * ray.direction.x + v.y * ray.direction.y + v.z * ray.direction.z) / dd };
        let len = dd.sqrt();
        for sc in &near {
            if let Some(s) = sc.resolve(&q(ray)) {
                // pick_face's t is tried both as a ray parameter and as a metric distance.
                let behind = if MODE_METRIC { param(s.position) * len > hit_t + 1e-6 } else { param(s.position) > hit_t + 1e-9 };
                if OCCLUDE && behind { continue; }
                let key = |x: &Snap| (x.kind.rank_group(), angle(&ray, x.position), x.kind);
                let better = match &best { None => true, Some(b) => key(&s) < key(b) };
                if better { best = Some(s); }
            }
        }
        match (&g, &best) {
            (None, None) => { both_none += 1; same += 1; }
            (Some(a), Some(b)) if a.kind == b.kind && a.position.approx_eq(b.position, 1e-6) => { same += 1; same_kind += 1; }
            (Some(a), Some(b)) if a.kind == b.kind => { same_kind += 1; diff.push(format!("{:?} pos differs by {:.3} m", a.kind, (a.position - b.position).length())); }
            _ => diff.push(format!("global {:?} vs tiled {:?}", g.as_ref().map(|s| s.kind), best.as_ref().map(|s| s.kind))),
        }
    }
    println!("{n} queries: identical {same} ({:.1}%), same kind {same_kind}, both none {both_none}, differ {}", 100.0 * same as f64 / n as f64, diff.len());
    for d in diff.iter().take(6) { println!("  {d}"); }
}
