//! WangunGIS spike: mitigate Hew `inference`'s full-rebuild-after-edit stall
//! WITHOUT changing Hew, by sharding the scene into one InferenceScene per
//! 100 m tile. An edit dirties only its tile; the cursor query hits only the
//! tile under the cursor (plus neighbours in a real editor).
//! Usage: cargo run --release --bin tiled -- <model.hew>

use inference::{ApertureMode, InferenceScene, PickRay, SnapQuery, SnapWeights};
use kernel::{KernelOp, Point3, Transform, Vec3, tol};
use std::collections::BTreeMap;
use std::time::Instant;

const TILE: f64 = 100.0;

fn tile_of(p: Point3) -> (i64, i64) {
    ((p.x / TILE).floor() as i64, (p.y / TILE).floor() as i64)
}

fn query(scene: &InferenceScene, target: Point3) -> std::time::Duration {
    let eye = Point3::new(target.x - 60.0, target.y - 80.0, 120.0);
    let t = Instant::now();
    let _ = scene.resolve(&SnapQuery {
        weights: SnapWeights::default(), ray: PickRay { origin: eye, direction: target - eye },
        anchor: None, lock: None, aperture: 0.01, aperture_mode: ApertureMode::Cone,
        constraint_plane: None, soft_axis_aperture_scale: None, off_plane_points: false, see_through_points: false,
    });
    t.elapsed()
}

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let mut doc = kernel::Document::load(&std::fs::read(path).unwrap()).unwrap();
    let ids: Vec<_> = doc.live_object_ids().into_iter().filter(|o| doc.is_world_object(*o)).collect();

    let t = Instant::now();
    let mut tiles: BTreeMap<(i64, i64), InferenceScene> = BTreeMap::new();
    let mut home = BTreeMap::new();
    for &oid in &ids {
        let obj = doc.object(oid).unwrap();
        let first = obj.vertices().values().next().unwrap().position;
        let key = tile_of(first);
        tiles.entry(key).or_insert_with(InferenceScene::new).add_object(oid, obj, &Transform::IDENTITY);
        home.insert(oid, key);
    }
    let build = t.elapsed();
    // Warm every tile's lazy index once (what a background thread would do after load).
    let t = Instant::now();
    for (&(i, j), scene) in &tiles {
        let _ = query(scene, Point3::new((i as f64 + 0.5) * TILE, (j as f64 + 0.5) * TILE, 4.5));
    }
    let warm = t.elapsed();
    let per_tile = ids.len() / tiles.len();

    // Edit one Object, re-register it in its tile only, then query that tile.
    let victim = ids[ids.len() / 2];
    let top = doc.object(victim).unwrap().faces().iter()
        .find(|(_, f)| f.plane.normal().approx_eq(Vec3::new(0.0, 0.0, 1.0), tol::NORMAL_DIRECTION))
        .map(|(id, _)| id).unwrap();
    doc.apply_object_op(victim, KernelOp::PushPull { face: top, distance: 2.0 }).unwrap();
    let key = home[&victim];
    let scene = tiles.get_mut(&key).unwrap();
    let t = Instant::now();
    scene.remove_object(victim);
    scene.add_object(victim, doc.object(victim).unwrap(), &Transform::IDENTITY);
    let reg = t.elapsed();
    let target = doc.object(victim).unwrap().vertices().values().next().unwrap().position;
    let after = query(scene, Point3::new(target.x, target.y, 4.5));
    let next = query(scene, Point3::new(target.x + 1.0, target.y, 4.5));
    println!(
        "objects {}, tiles {} (~{per_tile} objects/tile), build {build:.2?}, warm all tiles {warm:.2?}; edit -> re-register {reg:.2?}, first snap after edit {after:.2?}, next {next:.2?}",
        ids.len(), tiles.len()
    );
}
