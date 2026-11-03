//! The editing layer end to end, without GPU or UI: the smoke flow (draw,
//! extrude, push/pull, undo/redo, save), snaps and occlusion across tile
//! boundaries, and refusals that must leave the project untouched.

use inference::{ApertureMode, ElementRef, PickRay, SnapKind, SnapQuery, SnapWeights};
use kernel::{Plane, Point3, SketchId, Vec3};
use proptest::prelude::*;
use wangungis_scene::project::{Project, TileKey};
use wangungis_scene::{KERNEL_FRAME_BOUND_M, ProjectOrigin};
use wangungis_tools::{EditError, Scene};

const MM: f64 = 1e-3;
const UTM49S: ProjectOrigin = ProjectOrigin {
    epsg: 32749,
    origin: [430_000.123, 9_150_000.456, 12.5],
};

fn scene() -> Scene {
    Scene::new(Project::new(Some(UTM49S)).unwrap())
}

fn ground() -> Plane {
    Plane::from_point_normal(Point3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0)).unwrap()
}

fn rect_segments(x: f64, y: f64, w: f64, d: f64) -> Vec<(Point3, Point3)> {
    let p = |a: f64, b: f64| Point3::new(x + a, y + b, 0.0);
    let r = [p(0.0, 0.0), p(w, 0.0), p(w, d), p(0.0, d)];
    (0..4).map(|k| (r[k], r[(k + 1) % 4])).collect()
}

/// Draws a rectangle and extrudes it `h` up, as two undo steps.
fn add_box(s: &mut Scene, x: f64, y: f64, w: f64, d: f64, h: f64) -> TileKey {
    let (key, sk) = s.draw(None, ground(), &rect_segments(x, y, w, d)).unwrap();
    let region = regions(s, key, sk)[0];
    s.extrude(key, sk, region, h).unwrap();
    key
}

fn regions(s: &Scene, key: TileKey, sk: SketchId) -> Vec<kernel::SketchRegionId> {
    s.project().tiles[&key].extrudable_regions(sk).unwrap()
}

type Bounds = ([f64; 3], [f64; 3]);

/// Project-local bounds of every world-object vertex across all tiles, and the
/// object count.
fn solids(p: &Project) -> (usize, Option<Bounds>) {
    let mut n = 0;
    let mut b: Option<Bounds> = None;
    for (key, doc) in &p.tiles {
        let c = key.center();
        for o in doc.visible_object_ids() {
            n += 1;
            for v in doc.object(o).unwrap().vertices().values() {
                let q = [
                    v.position.x + c[0],
                    v.position.y + c[1],
                    v.position.z + c[2],
                ];
                let (lo, hi) = b.get_or_insert((q, q));
                for k in 0..3 {
                    lo[k] = lo[k].min(q[k]);
                    hi[k] = hi[k].max(q[k]);
                }
            }
        }
    }
    (n, b)
}

fn assert_box(p: &Project, lo: [f64; 3], hi: [f64; 3]) {
    let (n, b) = solids(p);
    assert_eq!(n, 1, "object count");
    let (got_lo, got_hi) = b.expect("a solid");
    for k in 0..3 {
        assert!(
            (got_lo[k] - lo[k]).abs() <= MM && (got_hi[k] - hi[k]).abs() <= MM,
            "bounds {got_lo:?}..{got_hi:?}, want {lo:?}..{hi:?}"
        );
    }
}

fn sketch_edges(p: &Project) -> usize {
    p.tiles
        .values()
        .map(|d| {
            d.sketch_ids()
                .iter()
                .filter_map(|s| d.sketch(*s))
                .map(|s| s.edges().len())
                .sum::<usize>()
        })
        .sum()
}

fn query(eye: [f64; 3], at: [f64; 3]) -> SnapQuery {
    let origin = Point3::new(eye[0], eye[1], eye[2]);
    SnapQuery {
        ray: PickRay {
            origin,
            direction: Point3::new(at[0], at[1], at[2]) - origin,
        },
        anchor: None,
        lock: None,
        aperture: 0.01,
        aperture_mode: ApertureMode::Cone,
        constraint_plane: None,
        weights: SnapWeights::default(),
        soft_axis_aperture_scale: None,
        off_plane_points: false,
        see_through_points: false,
    }
}

/// The uji asap: 5 x 4 x 3 m box, push/pull, undo all the way, redo all the
/// way, save and reopen.
#[test]
fn smoke_box_undo_redo_save() {
    let mut s = scene();
    assert!(!s.can_undo());
    let (key, sk) = s
        .draw(None, ground(), &rect_segments(0.0, 0.0, 5.0, 4.0))
        .unwrap();
    assert_eq!(key, TileKey::ORIGIN);
    assert_eq!(
        regions(&s, key, sk).len(),
        1,
        "closed rectangle is a region"
    );
    assert_eq!(s.take_dirty().into_iter().collect::<Vec<_>>(), [key]);
    let region = regions(&s, key, sk)[0];
    s.extrude(key, sk, region, 3.0).unwrap();
    assert_box(s.project(), [0.0, 0.0, 0.0], [5.0, 4.0, 3.0]);

    // Pull the top up 1 m, picking it with a ray straight down.
    let hit = s
        .pick_face(&PickRay {
            origin: Point3::new(2.5, 2.0, 50.0),
            direction: Vec3::new(0.0, 0.0, -1.0),
        })
        .expect("top face");
    assert!((hit.depth - 47.0).abs() <= MM, "depth {}", hit.depth);
    assert!(matches!(hit.source.element, ElementRef::Face(_)));
    s.push_pull(hit.tile, hit.source, 1.0).unwrap();
    assert_box(s.project(), [0.0, 0.0, 0.0], [5.0, 4.0, 4.0]);

    assert_eq!(s.undo().unwrap(), Some(key));
    assert_box(s.project(), [0.0, 0.0, 0.0], [5.0, 4.0, 3.0]);
    assert_eq!(s.undo().unwrap(), Some(key));
    assert_eq!(solids(s.project()).0, 0, "undo removes the solid");
    assert_eq!(sketch_edges(s.project()), 4, "the rectangle is back");
    assert_eq!(s.undo().unwrap(), Some(key));
    assert_eq!(sketch_edges(s.project()), 0, "undo removes the drawing");
    assert_eq!(
        s.undo().unwrap(),
        None,
        "nothing older than the user's edits"
    );

    s.redo().unwrap();
    assert_eq!(sketch_edges(s.project()), 4);
    s.redo().unwrap();
    s.redo().unwrap();
    assert_eq!(s.redo().unwrap(), None);
    assert_box(s.project(), [0.0, 0.0, 0.0], [5.0, 4.0, 4.0]);

    let back = Project::open(&s.project().save()).unwrap();
    assert_eq!(back.origin, Some(UTM49S));
    assert_box(&back, [0.0, 0.0, 0.0], [5.0, 4.0, 4.0]);
}

/// A new edit after undo drops the redo branch.
#[test]
fn new_edit_clears_redo() {
    let mut s = scene();
    add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    s.undo().unwrap();
    assert!(s.can_redo());
    s.draw(None, ground(), &rect_segments(20.0, 0.0, 2.0, 2.0))
        .unwrap();
    assert!(!s.can_redo());
    assert_eq!(s.redo().unwrap(), None);
}

/// Line by line on one sketch: each segment is its own undo step, and the
/// closing segment makes the region.
#[test]
fn line_chain_closes_into_a_region() {
    let mut s = scene();
    let segs = rect_segments(1.0, 1.0, 5.0, 4.0);
    let (key, sk) = s.draw(None, ground(), &segs[..1]).unwrap();
    for seg in &segs[1..] {
        assert_eq!(
            s.draw(Some((key, sk)), ground(), &[*seg]).unwrap(),
            (key, sk)
        );
    }
    assert_eq!(regions(&s, key, sk).len(), 1);
    s.undo().unwrap();
    assert_eq!(regions(&s, key, sk).len(), 0, "undo reopens the loop");
    assert_eq!(sketch_edges(s.project()), 3);
}

/// Undo walks back across tiles in edit order.
#[test]
fn undo_order_follows_edits_across_tiles() {
    let mut s = scene();
    let one = TileKey { i: 1, j: 0 };
    add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    assert_eq!(add_box(&mut s, 1_000.0, 0.0, 5.0, 4.0, 3.0), one);
    assert_eq!(solids(s.project()).0, 2);
    let order: Vec<_> = (0..4).map(|_| s.undo().unwrap().unwrap()).collect();
    assert_eq!(order, [one, one, TileKey::ORIGIN, TileKey::ORIGIN]);
    assert_eq!(solids(s.project()).0, 0);
    let order: Vec<_> = (0..4).map(|_| s.redo().unwrap().unwrap()).collect();
    assert_eq!(order, [TileKey::ORIGIN, TileKey::ORIGIN, one, one]);
    assert_eq!(solids(s.project()).0, 2);
}

/// Snaps come back in project-local coordinates from the right tile, right
/// across the boundary at x = 500.
#[test]
fn snaps_cross_tile_boundaries() {
    let mut s = scene();
    assert_eq!(add_box(&mut s, 495.0, 0.0, 4.0, 4.0, 3.0), TileKey::ORIGIN);
    let one = add_box(&mut s, 501.0, 0.0, 4.0, 4.0, 3.0);
    assert_eq!(one, TileKey { i: 1, j: 0 });
    let eye = [500.0, -30.0, 25.0];
    for (at, tile, kind) in [
        ([499.0, 0.0, 3.0], TileKey::ORIGIN, SnapKind::Endpoint),
        ([501.0, 0.0, 3.0], one, SnapKind::Endpoint),
        ([503.0, 0.0, 3.0], one, SnapKind::Midpoint),
    ] {
        let hit = s.snap(&query(eye, at)).expect("a snap");
        assert_eq!((hit.tile, hit.snap.kind), (tile, kind), "at {at:?}");
        let p = hit.snap.position;
        let miss = (p - Point3::new(at[0], at[1], at[2])).length();
        assert!(miss <= MM, "at {at:?}: snapped to {p:?}");
    }
}

/// A corner in the next tile, hidden behind a wall in this tile, must not win
/// even though Endpoint outranks OnFace.
#[test]
fn a_wall_in_one_tile_hides_a_corner_in_the_next() {
    let mut s = scene();
    assert_eq!(
        add_box(&mut s, 490.0, -10.0, 5.0, 20.0, 10.0),
        TileKey::ORIGIN
    );
    let one = add_box(&mut s, 510.0, 0.0, 5.0, 4.0, 3.0);
    assert_eq!(one, TileKey { i: 1, j: 0 });
    let eye = [400.0, 2.0, 1.5];
    let at = [510.0, 0.0, 3.0];
    let hit = s.snap(&query(eye, at)).expect("the wall");
    assert_eq!(hit.tile, TileKey::ORIGIN, "saw through the wall: {hit:?}");
    // With see-through on, the corner is what the cursor aims at.
    let through = s
        .snap(&SnapQuery {
            see_through_points: true,
            ..query(eye, at)
        })
        .unwrap();
    assert_eq!((through.tile, through.snap.kind), (one, SnapKind::Endpoint));
}

#[test]
fn snaps_update_after_edit_and_undo() {
    let mut s = scene();
    let eye = [2.0, -20.0, 15.0];
    let at = [5.0, 0.0, 3.0];
    assert_ne!(
        s.snap(&query(eye, at)).map(|h| h.snap.kind),
        Some(SnapKind::Endpoint)
    );
    add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    assert_eq!(
        s.snap(&query(eye, at)).unwrap().snap.kind,
        SnapKind::Endpoint
    );
    s.undo().unwrap();
    assert_ne!(
        s.snap(&query(eye, at)).map(|h| h.snap.kind),
        Some(SnapKind::Endpoint),
        "stale index after undo"
    );
}

fn untouched(s: &Scene) {
    assert!(!s.can_undo(), "refusal recorded an undo step");
    assert_eq!(sketch_edges(s.project()), 0);
    assert_eq!(solids(s.project()).0, 0);
}

#[test]
fn drawing_past_the_tile_or_project_is_refused() {
    let mut s = scene();
    let a = Point3::new(0.0, 0.0, 0.0);
    let far = Point3::new(KERNEL_FRAME_BOUND_M + 100.0, 0.0, 0.0);
    assert!(matches!(
        s.draw(None, ground(), &[(a, far)]),
        Err(EditError::OutOfTile { .. })
    ));
    untouched(&s);
    let out = Point3::new(60_000.0, 0.0, 0.0);
    assert_eq!(
        s.draw(None, ground(), &[(out, Point3::new(60_010.0, 0.0, 0.0))]),
        Err(EditError::OutOfProject)
    );
    untouched(&s);
}

/// A batch failing halfway leaves nothing: no edges, no undo step.
#[test]
fn failed_batch_rolls_back() {
    let mut s = scene();
    let a = Point3::new(0.0, 0.0, 0.0);
    let b = Point3::new(5.0, 0.0, 0.0);
    assert!(matches!(
        s.draw(None, ground(), &[(a, b), (b, b)]),
        Err(EditError::Kernel(_))
    ));
    untouched(&s);
    let off = Point3::new(5.0, 5.0, 2.0);
    assert!(matches!(
        s.draw(None, ground(), &[(a, b), (b, off)]),
        Err(EditError::Kernel(_))
    ));
    untouched(&s);
}

#[test]
fn pulling_past_the_tile_is_undone() {
    let mut s = scene();
    add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    // The east wall, picked along -x.
    let wall = s
        .pick_face(&PickRay {
            origin: Point3::new(100.0, 2.0, 1.5),
            direction: Vec3::new(-1.0, 0.0, 0.0),
        })
        .unwrap();
    let steps = |s: &mut Scene| {
        let mut n = 0;
        while s.undo().unwrap().is_some() {
            n += 1;
        }
        for _ in 0..n {
            s.redo().unwrap();
        }
        n
    };
    let before = steps(&mut s);
    assert!(matches!(
        s.push_pull(wall.tile, wall.source, 2_000.0),
        Err(EditError::OutOfTile { .. })
    ));
    assert_box(s.project(), [0.0, 0.0, 0.0], [5.0, 4.0, 3.0]);
    assert_eq!(steps(&mut s), before, "refused pull left an undo step");
    assert_eq!(
        s.push_pull(wall.tile, wall.source, f64::NAN),
        Err(EditError::OutOfProject)
    );
    // Within the bound the same pull works.
    s.push_pull(wall.tile, wall.source, 1.0).unwrap();
    assert_box(s.project(), [0.0, 0.0, 0.0], [6.0, 4.0, 3.0]);
}

/// Pushing a face past everything removes all material: refused, not corrupted.
#[test]
fn pushing_a_box_away_entirely_is_refused() {
    let mut s = scene();
    add_box(&mut s, 20.0, 0.0, 2.0, 2.0, 3.0);
    let top = s
        .pick_face(&PickRay {
            origin: Point3::new(21.0, 1.0, 50.0),
            direction: Vec3::new(0.0, 0.0, -1.0),
        })
        .unwrap();
    let before = solids(s.project());
    assert!(s.push_pull(top.tile, top.source, -3.5).is_err());
    assert_eq!(solids(s.project()), before);
}

/// A long object is indexed in the cell of its center but found from its far
/// end, hundreds of meters into other cells.
#[test]
fn long_object_is_found_far_from_its_cell() {
    let mut s = scene();
    add_box(&mut s, -480.0, 0.0, 960.0, 10.0, 5.0);
    let eye = [470.0, -30.0, 25.0];
    let hit = s.snap(&query(eye, [480.0, 0.0, 5.0])).expect("far corner");
    assert_eq!(hit.snap.kind, SnapKind::Endpoint);
    let miss = (hit.snap.position - Point3::new(480.0, 0.0, 5.0)).length();
    assert!(miss <= MM, "{:?}", hit.snap.position);
}

/// No geometry at all: axis locks still answer, through the base scene.
#[test]
fn axis_lock_works_in_an_empty_project() {
    let mut s = scene();
    let hit = s
        .snap(&SnapQuery {
            anchor: Some(Point3::new(10.0, 10.0, 0.0)),
            lock: Some(inference::SnapLock::Axis(inference::Axis::X)),
            ..query([20.0, -30.0, 25.0], [20.0, 10.0, 0.0])
        })
        .expect("locked snap");
    let p = hit.snap.position;
    assert!((p.y - 10.0).abs() <= MM && p.z.abs() <= MM, "{p:?}");
    assert!((p.x - 20.0).abs() <= 1.0, "{p:?}");
}

/// Every object health-checks as a solid, in every tile.
fn all_solid(p: &Project) -> bool {
    p.tiles
        .values()
        .all(|d| d.health_report(None).rows.is_empty())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    /// A grid of boxes over a whole tile: aiming at any corner, from any side,
    /// finds that corner, wherever cells and tile edges fall.
    #[test]
    fn corners_are_found_across_cells(
        gx in 0usize..12, gy in 0usize..12, cx in 0usize..2, cy in 0usize..2,
        ex in -60.0f64..60.0, ey in -60.0f64..60.0,
    ) {
        let mut s = scene();
        for i in 0..12 {
            for j in 0..12 {
                add_box(&mut s, -600.0 + i as f64 * 100.0, -600.0 + j as f64 * 100.0, 10.0, 10.0, 4.0);
            }
        }
        let c = [
            -600.0 + gx as f64 * 100.0 + cx as f64 * 10.0,
            -600.0 + gy as f64 * 100.0 + cy as f64 * 10.0,
            4.0,
        ];
        let hit = s.snap(&query([c[0] + ex, c[1] + ey, 90.0], c)).expect("a snap");
        prop_assert_eq!(hit.snap.kind, SnapKind::Endpoint);
        let miss = (hit.snap.position - Point3::new(c[0], c[1], c[2])).length();
        prop_assert!(miss <= MM, "{:?} vs {:?}", hit.snap.position, c);
    }

    /// Boxes anywhere in ±5 km, including across tile boundaries, land where
    /// asked in the tile of their first corner, and survive undo + redo + save.
    #[test]
    fn boxes_anywhere_stay_exact(
        x in -5_000.0f64..5_000.0,
        y in -5_000.0f64..5_000.0,
        w in 0.5f64..80.0,
        d in 0.5f64..80.0,
        h in 0.5f64..40.0,
    ) {
        let mut s = scene();
        let key = add_box(&mut s, x, y, w, d, h);
        prop_assert_eq!(key, TileKey::of([x, y]));
        prop_assert!(all_solid(s.project()));
        let (lo, hi) = ([x, y, 0.0], [x + w, y + d, h]);
        assert_box(s.project(), lo, hi);
        s.undo().unwrap();
        s.undo().unwrap();
        prop_assert_eq!(solids(s.project()).0, 0);
        s.redo().unwrap();
        s.redo().unwrap();
        assert_box(s.project(), lo, hi);
        assert_box(&Project::open(&s.project().save()).unwrap(), lo, hi);
    }
}
