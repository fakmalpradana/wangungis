//! The editing layer end to end, without GPU or UI: the smoke flow (draw,
//! extrude, push/pull, undo/redo, save), snaps and occlusion across tile
//! boundaries, and refusals that must leave the project untouched.

use inference::{ApertureMode, ElementRef, PickRay, SnapKind, SnapQuery, SnapWeights};
use kernel::{Plane, Point3, SketchId, Transform, Vec3};
use proptest::prelude::*;
use std::num::NonZeroU32;
use wangungis_scene::project::{Project, TileKey};
use wangungis_scene::{KERNEL_FRAME_BOUND_M, ProjectOrigin};
use wangungis_tools::{EditError, Erasable, Movable, Scene};

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

/// A rectangle on the ground, in the tile of its first corner.
fn draw_rect(s: &mut Scene, x: f64, y: f64, w: f64, d: f64) -> (TileKey, SketchId) {
    let drawn = s.draw(None, ground(), &rect_segments(x, y, w, d)).unwrap();
    (drawn.tile, drawn.sketch.unwrap())
}

/// Draws a rectangle and extrudes it `h` up, as two undo steps.
fn add_box(s: &mut Scene, x: f64, y: f64, w: f64, d: f64, h: f64) -> TileKey {
    let (key, sk) = draw_rect(s, x, y, w, d);
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
    let (key, sk) = draw_rect(&mut s, 0.0, 0.0, 5.0, 4.0);
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
    let first = s.draw(None, ground(), &segs[..1]).unwrap();
    assert!(!first.closed);
    let key = first.tile;
    let mut last = first;
    for seg in &segs[1..] {
        last = s.draw(Some(key), ground(), &[*seg]).unwrap();
        assert_eq!(last.tile, key);
    }
    assert!(last.closed, "the closing segment makes the region");
    let sk = last.sketch.unwrap();
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

/// Undo, then a refused edit: the refused edit must not come back through redo.
#[test]
fn refused_edit_cannot_return_through_redo() {
    let mut s = scene();
    add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    let wall = |s: &mut Scene| {
        s.pick_face(&PickRay {
            origin: Point3::new(100.0, 2.0, 1.5),
            direction: Vec3::new(-1.0, 0.0, 0.0),
        })
        .unwrap()
    };
    let w = wall(&mut s);
    s.push_pull(w.tile, w.source, 1.0).unwrap();
    s.undo().unwrap();
    assert!(s.can_redo());
    let w = wall(&mut s);
    assert!(matches!(
        s.push_pull(w.tile, w.source, 2_000.0),
        Err(EditError::OutOfTile { .. })
    ));
    assert_eq!(
        s.redo().unwrap(),
        None,
        "redo must not re-apply the refusal"
    );
    assert_box(s.project(), [0.0, 0.0, 0.0], [5.0, 4.0, 3.0]);
    // Undo still walks back through the real edits.
    s.undo().unwrap();
    assert_eq!(solids(s.project()).0, 0);
}

/// A refused first segment leaves no sketch and no new tile.
#[test]
fn refused_drawing_leaves_no_sketch_or_tile() {
    let mut s = scene();
    let far = Point3::new(3_000.0, 0.0, 0.0);
    assert!(s.draw(None, ground(), &[(far, far)]).is_err());
    assert_eq!(s.project().tiles.len(), 1, "no tile created");
    assert!(s.project().tiles[&TileKey::ORIGIN].sketch_ids().is_empty());
    assert!(!s.can_undo());
    // A refused drawing in a tile that does not exist yet does not create it.
    let elsewhere = TileKey { i: 5, j: 5 };
    let (a, b) = (
        Point3::new(5_000.0, 5_000.0, 0.0),
        Point3::new(5_000.0, 5_000.0, 2.0),
    );
    assert!(s.draw(Some(elsewhere), ground(), &[(a, b)]).is_err());
    assert_eq!(s.project().tiles.len(), 1);
}

/// Hovering beside a corner, on the far side of the cursor along the face: the
/// corner is farther than the face under the cursor but not hidden by it.
#[test]
fn corner_beyond_the_cursor_is_not_hidden_by_its_own_face() {
    let mut s = scene();
    add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    let hit = s
        .snap(&query([-20.0, -30.0, 40.0], [4.9, 3.9, 3.0]))
        .expect("a snap");
    assert_eq!(hit.snap.kind, SnapKind::Endpoint, "{hit:?}");
    let miss = (hit.snap.position - Point3::new(5.0, 4.0, 3.0)).length();
    assert!(miss <= MM, "{:?}", hit.snap.position);
}

/// A nearly level ray from inside a cell, low over a drawing 50 m away.
#[test]
fn low_level_ray_from_inside_a_cell_finds_the_drawing() {
    let mut s = scene();
    s.draw(None, ground(), &rect_segments(50.0, 0.0, 4.0, 4.0))
        .unwrap();
    s.draw(None, ground(), &rect_segments(0.0, 60.0, 4.0, 4.0))
        .unwrap();
    let hit = s
        .snap(&query([1.0, 1.0, 0.05], [50.0, 0.0, 0.0]))
        .expect("a snap");
    assert_eq!(hit.snap.kind, SnapKind::Endpoint);
}

/// A lock projects real geometry onto the line; the bare line of an empty scene
/// must not win over it.
#[test]
fn locked_snap_prefers_geometry_over_the_bare_line() {
    let mut s = scene();
    add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    let hit = s
        .snap(&SnapQuery {
            anchor: Some(Point3::new(0.0, -5.0, 3.0)),
            lock: Some(inference::SnapLock::Axis(inference::Axis::X)),
            ..query([-20.0, -30.0, 40.0], [5.0, 4.0, 3.0])
        })
        .expect("a snap");
    assert_eq!(hit.snap.kind, SnapKind::Endpoint, "{hit:?}");
    let p = hit.snap.position;
    assert!((p.x - 5.0).abs() <= MM && (p.y + 5.0).abs() <= MM, "{p:?}");
    let from = hit.snap.projected_from.expect("projected");
    assert!(
        (from - Point3::new(5.0, 4.0, 3.0)).length() <= MM,
        "{from:?}"
    );
}

fn instance_scene(hide: bool) -> Scene {
    let mut doc = kernel::Document::new();
    let p = |a: f64, b: f64| Point3::new(a, b, 0.0);
    let ring = [
        p(1000.0, 0.0),
        p(1005.0, 0.0),
        p(1005.0, 4.0),
        p(1000.0, 4.0),
    ];
    let sk = doc.add_sketch(Plane::from_polygon(&ring).unwrap());
    let sketch = doc.sketch_mut(sk).unwrap();
    for k in 0..4 {
        sketch.add_segment(ring[k], ring[(k + 1) % 4]).unwrap();
    }
    let r = doc.extrudable_regions(sk).unwrap();
    doc.extrude_region(sk, r[0], 3.0).unwrap();
    let node = doc.top_level_nodes()[0];
    let (_, inst, _) = doc.make_component(&[node]).unwrap();
    if hide {
        doc.set_node_user_hidden(kernel::NodeId::Instance(inst), true);
    }
    // Reaches 1 km from the document origin, so it stays one tile.
    let project = Project::from_document(doc).unwrap();
    assert_eq!(project.tiles.len(), 1);
    Scene::new(project)
}

/// Component instances snap and pick like solids; hidden ones do not.
#[test]
fn instances_snap_and_pick_unless_hidden() {
    let eye = [990.0, -20.0, 25.0];
    let corner = [1000.0, 0.0, 3.0];
    let mut s = instance_scene(false);
    let hit = s.snap(&query(eye, corner)).expect("a snap");
    assert_eq!(hit.snap.kind, SnapKind::Endpoint);
    assert!(hit.snap.source.expect("source").instance.is_some());
    let down = PickRay {
        origin: Point3::new(1002.0, 2.0, 50.0),
        direction: Vec3::new(0.0, 0.0, -1.0),
    };
    assert!(s.pick_face(&down).is_some());
    // Editing inside a component is refused, not misapplied.
    let f = s.pick_face(&down).unwrap();
    assert!(matches!(
        s.push_pull(f.tile, f.source, 1.0),
        Err(EditError::OutOfScope)
    ));

    let mut s = instance_scene(true);
    assert_ne!(
        s.snap(&query(eye, corner)).map(|h| h.snap.kind),
        Some(SnapKind::Endpoint)
    );
    assert!(s.pick_face(&down).is_none());
}

/// Drawing over an existing edge changes nothing, so it is no undo step.
#[test]
fn segment_on_an_existing_edge_is_no_undo_step() {
    let mut s = scene();
    let segs = rect_segments(0.0, 0.0, 5.0, 4.0);
    let key = s.draw(None, ground(), &segs).unwrap().tile;
    s.draw(Some(key), ground(), &segs[..1]).unwrap();
    assert_eq!(s.undo().unwrap(), Some(key));
    assert_eq!(
        sketch_edges(s.project()),
        0,
        "one undo reverts the rectangle"
    );
    assert_eq!(s.undo().unwrap(), None);
}

/// The project axis behind a wall must not beat the wall under the cursor.
#[test]
fn an_axis_behind_a_wall_does_not_win() {
    let mut s = scene();
    add_box(&mut s, -5.0, -10.0, 15.0, 5.0, 3.0);
    let hit = s
        .snap(&query([2.0, -40.0, 1.0], [2.0, 0.0, 0.0]))
        .expect("the wall");
    assert_ne!(hit.snap.kind, SnapKind::OnAxis, "{hit:?}");
    assert!(hit.snap.position.y <= -9.9, "{:?}", hit.snap.position);
}

/// A lock with no anchor means nothing to Hew: geometry still ranks normally.
#[test]
fn lock_without_anchor_ranks_normally() {
    let mut s = scene();
    add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    let hit = s
        .snap(&SnapQuery {
            lock: Some(inference::SnapLock::Axis(inference::Axis::X)),
            ..query([-20.0, -30.0, 40.0], [5.0, 4.0, 3.0])
        })
        .expect("a snap");
    assert_eq!(hit.snap.kind, SnapKind::Endpoint, "{hit:?}");
}

/// What a tag hides cannot be snapped to or picked.
#[test]
fn tag_hidden_objects_do_not_snap() {
    let eye = [-20.0, -30.0, 40.0];
    let corner = [5.0, 4.0, 3.0];
    let tagged = |hide: bool| {
        let mut doc = kernel::Document::new();
        let p = |a: f64, b: f64| Point3::new(a, b, 0.0);
        let ring = [p(0.0, 0.0), p(5.0, 0.0), p(5.0, 4.0), p(0.0, 4.0)];
        let sk = doc.add_sketch(Plane::from_polygon(&ring).unwrap());
        let sketch = doc.sketch_mut(sk).unwrap();
        for k in 0..4 {
            sketch.add_segment(ring[k], ring[(k + 1) % 4]).unwrap();
        }
        let r = doc.extrudable_regions(sk).unwrap();
        doc.extrude_region(sk, r[0], 3.0).unwrap();
        let node = doc.top_level_nodes()[0];
        let tag = vec!["Lantai".to_string()];
        doc.add_node_tag(node, tag.clone()).unwrap();
        doc.set_tag_hidden(tag, hide);
        Scene::new(Project::from_document(doc).unwrap())
    };
    let hit = tagged(false).snap(&query(eye, corner)).map(|h| h.snap.kind);
    assert_eq!(hit, Some(SnapKind::Endpoint));
    let mut hidden = tagged(true);
    assert_ne!(hidden.snap(&query(eye, corner)).map(|h| h.snap.kind), hit);
    let down = PickRay {
        origin: Point3::new(2.5, 2.0, 50.0),
        direction: Vec3::new(0.0, 0.0, -1.0),
    };
    assert!(hidden.pick_face(&down).is_none());
}

/// An edit undone in a new tile leaves a blank document: not saved, not counted,
/// and redo still brings the edit back.
#[test]
fn blank_tile_after_undo_is_not_saved() {
    let mut s = scene();
    add_box(&mut s, 1_000.0, 0.0, 5.0, 4.0, 3.0);
    assert!(s.project().single().is_none());
    s.undo().unwrap();
    s.undo().unwrap();
    assert_eq!(s.project().tiles.len(), 2, "kept in memory for redo");
    assert_eq!(s.project().occupied().count(), 1);
    assert!(s.project().single().is_some(), "saves as a plain .hew");
    let reopened = Project::open(&s.project().save()).unwrap();
    assert_eq!(reopened.tiles.len(), 1);
    s.redo().unwrap();
    s.redo().unwrap();
    assert_eq!(s.project().occupied().count(), 2);
    assert_eq!(Project::open(&s.project().save()).unwrap().tiles.len(), 2);
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

fn top_plane(z: f64) -> Plane {
    Plane::from_point_normal(Point3::new(0.0, 0.0, z), Vec3::new(0.0, 0.0, 1.0)).unwrap()
}

fn ring_at(z: f64, x: f64, y: f64, w: f64, d: f64) -> Vec<(Point3, Point3)> {
    let p = |a: f64, b: f64| Point3::new(x + a, y + b, z);
    let r = [p(0.0, 0.0), p(w, 0.0), p(w, d), p(0.0, d)];
    (0..4).map(|k| (r[k], r[(k + 1) % 4])).collect()
}

/// The only object of a one-box project, with its id.
fn the_object(s: &Scene, key: TileKey) -> (kernel::ObjectId, &kernel::Object) {
    let doc = &s.project().tiles[&key];
    let id = doc.visible_object_ids()[0];
    (id, doc.object(id).unwrap())
}

/// The face straight below (`x`, `y`, `z` + 50 m).
fn face_at(s: &mut Scene, key: TileKey, x: f64, y: f64, z: f64) -> inference::SnapSource {
    let found = s
        .pick_face(&PickRay {
            origin: Point3::new(x, y, z + 50.0),
            direction: Vec3::new(0.0, 0.0, -1.0),
        })
        .expect("a face under the ray");
    assert_eq!(found.tile, key);
    assert!(matches!(found.source.element, ElementRef::Face(_)));
    found.source
}

/// Drawing on a face splits it, as one undo step, and the new face can be
/// pulled on its own: this is auto-split (#14).
#[test]
fn a_shape_on_a_face_splits_it() {
    let mut s = scene();
    let key = add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    assert_eq!(the_object(&s, key).1.faces().len(), 6);

    let drawn = s
        .draw(Some(key), top_plane(3.0), &ring_at(3.0, 1.0, 1.0, 2.0, 2.0))
        .unwrap();
    assert!(drawn.closed, "the loop is a new face");
    let (_, obj) = the_object(&s, key);
    assert_eq!(obj.faces().len(), 7, "the top face was split");
    assert!(sketch_edges(s.project()) == 0, "nothing stayed a sketch");

    // Pull the inner face up: a tower on the box.
    let inner = face_at(&mut s, key, 2.0, 2.0, 3.0);
    s.push_pull(key, inner, 2.0).unwrap();
    let (_, b) = solids(s.project());
    let (lo, hi) = b.unwrap();
    assert!((hi[2] - 5.0).abs() < MM && lo[2].abs() < MM);
    kernel_valid(&s, key);

    // Undo the pull, then the split: back to a plain box.
    s.undo().unwrap();
    s.undo().unwrap();
    assert_eq!(the_object(&s, key).1.faces().len(), 6);
    s.redo().unwrap();
    assert_eq!(the_object(&s, key).1.faces().len(), 7);
    kernel_valid(&s, key);
}

/// The box with a door of the v0.1 exit task: a rectangle on a wall touching
/// the floor edge, pushed through the whole box.
#[test]
fn a_door_goes_through_the_wall() {
    let mut s = scene();
    let key = add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    // The wall y = 0 faces -y.
    let wall =
        Plane::from_point_normal(Point3::new(0.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0)).unwrap();
    let p = |x: f64, z: f64| Point3::new(x, 0.0, z);
    let door = [p(2.0, 0.0), p(3.0, 0.0), p(3.0, 2.0), p(2.0, 2.0)];
    let segs: Vec<_> = (0..4).map(|k| (door[k], door[(k + 1) % 4])).collect();
    assert!(s.draw(Some(key), wall, &segs).unwrap().closed);
    kernel_valid(&s, key);

    let face = s
        .pick_face(&PickRay {
            origin: Point3::new(2.5, -10.0, 1.0),
            direction: Vec3::new(0.0, 1.0, 0.0),
        })
        .unwrap();
    s.push_pull(key, face.source, -4.0).unwrap();
    kernel_valid(&s, key);
    // Looking through the opening, the ray passes the box.
    assert!(
        s.pick_face(&PickRay {
            origin: Point3::new(2.5, -10.0, 1.0),
            direction: Vec3::new(0.0, 1.0, 0.0),
        })
        .is_none(),
        "the door opens right through"
    );
}

/// Separate strokes on one plane join, whichever order they come in.
#[test]
fn strokes_on_a_plane_join_up() {
    let mut s = scene();
    let segs = rect_segments(0.0, 0.0, 5.0, 4.0);
    let d0 = s.draw(None, ground(), &segs[..2]).unwrap();
    let d1 = s.draw(Some(d0.tile), ground(), &segs[2..]).unwrap();
    assert!(d1.closed, "a later stroke closes the earlier one");
    // A line across the region divides it.
    let across = (Point3::new(2.0, 0.0, 0.0), Point3::new(2.0, 4.0, 0.0));
    let d2 = s.draw(Some(d0.tile), ground(), &[across]).unwrap();
    assert!(d2.closed);
    let sk = d2.sketch.unwrap();
    assert_eq!(regions(&s, d0.tile, sk).len(), 2);
}

/// Random rectangles on a box's top keep the kernel's invariants, undo included.
fn kernel_valid(s: &Scene, key: TileKey) {
    let doc = &s.project().tiles[&key];
    for id in doc.visible_object_ids() {
        doc.object(id).unwrap().validate().unwrap();
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    #[test]
    fn drawing_on_a_face_never_breaks_it(
        x in 0.1f64..4.0, y in 0.1f64..3.0, w in 0.1f64..0.9, d in 0.1f64..0.9, pull in -2.5f64..2.5,
    ) {
        let mut s = scene();
        let key = add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
        let drawn = s.draw(Some(key), top_plane(3.0), &ring_at(3.0, x, y, w, d)).unwrap();
        prop_assert!(drawn.closed);
        kernel_valid(&s, key);
        let inner = face_at(&mut s, key, x + w / 2.0, y + d / 2.0, 3.0);
        if pull.abs() > 0.01 {
            s.push_pull(key, inner, pull).unwrap();
            kernel_valid(&s, key);
            s.undo().unwrap();
        }
        s.undo().unwrap();
        kernel_valid(&s, key);
        prop_assert_eq!(the_object(&s, key).1.faces().len(), 6);
    }
}

fn the_box(s: &mut Scene, x: f64, y: f64, z: f64) -> Movable {
    let hit = s
        .snap(&query([x, y, z + 50.0], [x, y, z]))
        .expect("something below");
    s.movable_at(&hit).expect("movable")
}

const NOUGHT: Vec3 = Vec3::new(0.0, 0.0, 0.0);

/// Moves bake into the geometry and undo as one step; a door moves along.
#[test]
fn move_translates_a_whole_solid() {
    let mut s = scene();
    add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    let m = the_box(&mut s, 2.0, 2.0, 3.0);
    s.transform(&m, &Transform::translation(Vec3::new(10.0, -2.0, 1.0)))
        .unwrap();
    let (n, b) = solids(s.project());
    let (lo, hi) = b.unwrap();
    assert_eq!(n, 1);
    for (got, want) in [lo, hi]
        .iter()
        .flatten()
        .zip([10.0, -2.0, 1.0, 15.0, 2.0, 4.0])
    {
        assert!((got - want).abs() < MM, "{lo:?} {hi:?}");
    }
    s.undo().unwrap();
    let (lo, _) = solids(s.project()).1.unwrap();
    assert!(lo[0].abs() < MM);
}

/// The v0.1 exit task, on the layer below the UI.
#[test]
fn array_of_ten_is_one_step_and_saves_identically() {
    let mut s = scene();
    add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    let m = the_box(&mut s, 2.0, 2.0, 3.0);
    let step = Transform::translation(Vec3::new(7.0, 0.0, 0.0));

    // The first copy, then the count: still one undo step.
    s.copy(&m, &step, NonZeroU32::MIN).unwrap();
    assert_eq!(solids(s.project()).0, 2);
    s.respace_copy(&m, &step, NonZeroU32::new(10).unwrap())
        .unwrap();
    let (n, b) = solids(s.project());
    assert_eq!(n, 11, "the source and ten copies");
    let (_, hi) = b.unwrap();
    assert!((hi[0] - 75.0).abs() < MM, "{hi:?}");

    let bytes = s.project().save();
    let reopened = Project::open(&bytes).unwrap();
    assert_eq!(solids(&reopened).0, 11);
    assert_eq!(reopened.save(), bytes, "save, load, save is byte-identical");

    s.undo().unwrap();
    assert_eq!(solids(s.project()).0, 1, "one undo removes the whole array");
    s.redo().unwrap();
    assert_eq!(solids(s.project()).0, 11);
}

#[test]
fn moves_and_arrays_past_the_tile_are_refused_whole() {
    let mut s = scene();
    add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    let m = the_box(&mut s, 2.0, 2.0, 3.0);
    let before = s.project().save();
    let far = Transform::translation(Vec3::new(KERNEL_FRAME_BOUND_M, 0.0, 0.0));
    assert!(matches!(
        s.transform(&m, &far),
        Err(EditError::OutOfTile { .. })
    ));
    assert!(matches!(
        s.copy(&m, &far, NonZeroU32::MIN),
        Err(EditError::OutOfTile { .. })
    ));
    // The last copy would be the first past the bound.
    let step = Transform::translation(Vec3::new(200.0, 0.0, 0.0));
    assert!(matches!(
        s.copy(&m, &step, NonZeroU32::new(9).unwrap()),
        Err(EditError::OutOfTile { .. })
    ));
    assert_eq!(s.project().save(), before);

    // A refused respacing keeps the first copy.
    s.copy(&m, &step, NonZeroU32::MIN).unwrap();
    assert!(
        s.respace_copy(&m, &step, NonZeroU32::new(9).unwrap())
            .is_err()
    );
    assert_eq!(solids(s.project()).0, 2);
    assert!(s.can_undo() && !s.can_redo());
}

/// Moving past a tile's edge keeps the solid in its tile, and it still saves
/// and reopens in place.
#[test]
fn moving_over_a_tile_border_keeps_the_geometry_right() {
    let mut s = scene();
    add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    let m = the_box(&mut s, 2.0, 2.0, 3.0);
    s.transform(&m, &Transform::translation(Vec3::new(600.0, 0.0, 0.0)))
        .unwrap();
    let (_, hi) = solids(s.project()).1.unwrap();
    assert!((hi[0] - 605.0).abs() < MM);
    let reopened = Project::open(&s.project().save()).unwrap();
    let (_, hi2) = solids(&reopened).1.unwrap();
    assert!((hi2[0] - 605.0).abs() < MM);
}

/// Drawing moves as a connected piece; copies on its plane join it as one step.
#[test]
fn drawing_moves_and_copies_as_an_island() {
    let mut s = scene();
    let (key, sk) = draw_rect(&mut s, 0.0, 0.0, 2.0, 2.0);
    draw_rect(&mut s, 50.0, 0.0, 2.0, 2.0);
    let hit = s
        .snap(&query([1.0, 1.0, 50.0], [1.0, 1.0, 0.0]))
        .expect("region");
    let m = s.movable_at(&hit).expect("island");
    assert!(matches!(m, Movable::Island { .. }));
    assert_eq!(s.movable_lines(&m).len(), 4);

    let undo_to = |s: &mut Scene| s.undo().unwrap();
    s.transform(&m, &Transform::translation(Vec3::new(10.0, 0.0, 0.0)))
        .unwrap();
    let doc = &s.project().tiles[&key];
    let at = |doc: &kernel::Document, x: f64| {
        doc.sketch_ids()
            .iter()
            .filter_map(|i| doc.sketch(*i))
            .flat_map(|k| {
                k.vertices()
                    .values()
                    .map(|v| v.position.x)
                    .collect::<Vec<_>>()
            })
            .any(|v| (v - x).abs() < MM)
    };
    assert!(at(doc, 12.0) && at(doc, 50.0));
    undo_to(&mut s);

    let edges_before = sketch_edges(s.project());
    s.copy(
        &m,
        &Transform::translation(Vec3::new(5.0, 0.0, 0.0)),
        NonZeroU32::new(3).unwrap(),
    )
    .unwrap();
    assert_eq!(sketch_edges(s.project()), edges_before + 12);
    undo_to(&mut s);
    assert_eq!(
        sketch_edges(s.project()),
        edges_before,
        "the array is one step"
    );
    let _ = sk;

    // Up off its plane: a separate drawing, and the source stays.
    s.copy(
        &m,
        &Transform::translation(Vec3::new(0.0, 0.0, 3.0)),
        NonZeroU32::MIN,
    )
    .unwrap();
    assert_eq!(sketch_edges(s.project()), edges_before + 4);
    undo_to(&mut s);
    s.transform(&m, &Transform::translation(Vec3::new(0.0, 0.0, 3.0)))
        .unwrap();
    assert_eq!(sketch_edges(s.project()), edges_before);
    undo_to(&mut s);
    assert_eq!(sketch_edges(s.project()), edges_before);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    /// Random displacements and counts: every solid stays valid and in place,
    /// wherever the source sits, and undo restores the file byte for byte.
    #[test]
    fn arrays_keep_every_solid_valid(
        x in -400.0f64..400.0, y in -400.0f64..400.0,
        dx in -20.0f64..20.0, dy in 6.0f64..20.0, dz in -3.0f64..3.0, n in 1u32..12,
    ) {
        let mut s = scene();
        let key = add_box(&mut s, x, y, 5.0, 4.0, 3.0);
        let before = s.project().save();
        let m = the_box(&mut s, x + 2.0, y + 2.0, 3.0);
        let step = Transform::translation(Vec3::new(dx, dy, dz));
        s.copy(&m, &step, NonZeroU32::new(n).unwrap()).unwrap();
        kernel_valid(&s, key);
        let (count, b) = solids(s.project());
        prop_assert_eq!(count, n as usize + 1);
        let (_, hi) = b.unwrap();
        let top = 3.0 + (dz * f64::from(n)).max(0.0);
        prop_assert!((hi[2] - top).abs() < MM, "{:?} vs {}", hi, top);
        s.undo().unwrap();
        prop_assert_eq!(&s.project().save(), &before);
        s.transform(&m, &step).unwrap();
        kernel_valid(&s, key);
        s.undo().unwrap();
        prop_assert_eq!(&s.project().save(), &before);
    }
}

#[test]
fn a_stale_pick_is_refused() {
    let mut s = scene();
    add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    let m = the_box(&mut s, 2.0, 2.0, 3.0);
    s.undo().unwrap();
    s.undo().unwrap();
    assert!(
        s.transform(&m, &Transform::translation(NOUGHT)).is_err(),
        "the solid is gone"
    );
}

fn erasable(s: &mut Scene, from: [f64; 3], at: [f64; 3]) -> Erasable {
    let hit = s.snap(&query(from, at)).expect("a snap");
    s.erasable_at(&hit)
        .unwrap_or_else(|| panic!("nothing to erase at {hit:?}"))
}

/// Erasing the line that divides a face merges it back; a real edge takes the
/// whole solid. Each is one undo step.
#[test]
fn eraser_removes_lines_and_whole_solids() {
    let mut s = scene();
    let key = add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    // A line right across the top divides it.
    let (a, b) = (Point3::new(2.0, 0.0, 3.0), Point3::new(2.0, 4.0, 3.0));
    s.draw(Some(key), top_plane(3.0), &[(a, b)]).unwrap();
    assert_eq!(the_object(&s, key).1.faces().len(), 7);

    let e = erasable(&mut s, [2.0, 2.0, 40.0], [2.0, 2.0, 3.0]);
    assert!(matches!(e, Erasable::Line { .. }), "{e:?}");
    s.erase(&e).unwrap();
    assert_eq!(the_object(&s, key).1.faces().len(), 6, "the faces merged");
    kernel_valid(&s, key);
    s.undo().unwrap();
    assert_eq!(the_object(&s, key).1.faces().len(), 7);
    s.redo().unwrap();

    // A real edge of the box: the box goes, and comes back on undo.
    let e = erasable(&mut s, [2.0, -30.0, 30.0], [2.5, 0.0, 3.0]);
    assert!(matches!(e, Erasable::Node { .. }), "{e:?}");
    assert_eq!(s.erasable_lines(&e).len(), 12);
    s.erase(&e).unwrap();
    assert_eq!(solids(s.project()).0, 0);
    s.undo().unwrap();
    assert_eq!(solids(s.project()).0, 1);
}

/// A drawn line goes alone, opening the loop it closed.
#[test]
fn eraser_opens_a_drawn_loop() {
    let mut s = scene();
    let (key, sk) = draw_rect(&mut s, 0.0, 0.0, 5.0, 4.0);
    assert_eq!(regions(&s, key, sk).len(), 1);
    let e = erasable(&mut s, [2.5, -20.0, 30.0], [2.5, 0.0, 0.0]);
    s.erase(&e).unwrap();
    assert_eq!(sketch_edges(s.project()), 3);
    let live = s.project().tiles[&key].sketch_ids()[0];
    assert_eq!(regions(&s, key, live).len(), 0);
    s.undo().unwrap();
    assert_eq!(sketch_edges(s.project()), 4);
}

/// A face holds no edge to erase; a click on it does nothing.
#[test]
fn eraser_ignores_faces() {
    let mut s = scene();
    add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    let hit = s.snap(&query([2.5, 2.0, 40.0], [2.5, 2.0, 3.0])).unwrap();
    assert_eq!(hit.snap.kind, SnapKind::OnFace);
    assert_eq!(s.erasable_at(&hit), None);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    /// A shape drawn on a face and erased line by line leaves a valid solid, and
    /// undoing it all restores the file.
    #[test]
    fn erasing_drawing_keeps_the_solid_valid(
        x in 0.5f64..2.5, y in 0.5f64..1.5, w in 1.5f64..2.5, d in 1.0f64..2.0,
    ) {
        let mut s = scene();
        let key = add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
        s.draw(Some(key), top_plane(3.0), &ring_at(3.0, x, y, w, d)).unwrap();
        let drawn = s.project().save();
        let e = erasable(&mut s, [x + w / 2.0, y - 20.0, 30.0], [x + w / 2.0, y, 3.0]);
        s.erase(&e).unwrap();
        kernel_valid(&s, key);
        s.undo().unwrap();
        prop_assert_eq!(&s.project().save(), &drawn);
    }
}

/// Turn and scale about a far pivot: the tile frame must not leak into either.
#[test]
fn transforms_are_project_local_in_any_tile() {
    let mut s = scene();
    let key = add_box(&mut s, 2_000.0, 3_000.0, 5.0, 4.0, 3.0);
    assert_ne!(key, TileKey::ORIGIN);
    let m = the_box(&mut s, 2_002.0, 3_002.0, 3.0);

    // A quarter turn about the box's own corner.
    let to = Vec3::new(2_000.0, 3_000.0, 0.0);
    let spin = Transform::translation(-to)
        .then(&Transform::rotation(Vec3::new(0.0, 0.0, 1.0), std::f64::consts::FRAC_PI_2).unwrap())
        .then(&Transform::translation(to));
    s.transform(&m, &spin).unwrap();
    let (lo, hi) = solids(s.project()).1.unwrap();
    for (got, want) in [lo, hi]
        .iter()
        .flatten()
        .zip([1_996.0, 3_000.0, 0.0, 2_000.0, 3_005.0, 3.0])
    {
        assert!((got - want).abs() < MM, "{lo:?} {hi:?}");
    }
    kernel_valid(&s, key);
    s.undo().unwrap();

    // Double about the same corner.
    let grow = Transform::translation(-to)
        .then(&Transform::uniform_scale(2.0))
        .then(&Transform::translation(to));
    s.transform(&m, &grow).unwrap();
    let (lo, hi) = solids(s.project()).1.unwrap();
    for (got, want) in [lo, hi]
        .iter()
        .flatten()
        .zip([2_000.0, 3_000.0, 0.0, 2_010.0, 3_008.0, 6.0])
    {
        assert!((got - want).abs() < MM, "{lo:?} {hi:?}");
    }
    kernel_valid(&s, key);
    assert_eq!(s.movable_center(&m).map(|c| c.x.round()), Some(2_005.0));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    /// Radial arrays and scales keep solids valid and undo to the same file.
    #[test]
    fn turning_and_scaling_keep_solids_valid(
        x in -300.0f64..300.0, y in -300.0f64..300.0,
        angle in 0.05f64..3.0, n in 1u32..8, factor in 0.2f64..3.0,
    ) {
        let mut s = scene();
        let key = add_box(&mut s, x, y, 5.0, 4.0, 3.0);
        let before = s.project().save();
        let m = the_box(&mut s, x + 2.0, y + 2.0, 3.0);
        let to = Vec3::new(x, y, 0.0);
        let spin = Transform::translation(-to)
            .then(&Transform::rotation(Vec3::new(0.0, 0.0, 1.0), angle).unwrap())
            .then(&Transform::translation(to));
        s.copy(&m, &spin, NonZeroU32::new(n).unwrap()).unwrap();
        kernel_valid(&s, key);
        prop_assert_eq!(solids(s.project()).0, n as usize + 1);
        s.undo().unwrap();
        prop_assert_eq!(&s.project().save(), &before);

        let grow = Transform::translation(-to)
            .then(&Transform::uniform_scale(factor))
            .then(&Transform::translation(to));
        s.transform(&m, &grow).unwrap();
        kernel_valid(&s, key);
        let (lo, hi) = solids(s.project()).1.unwrap();
        prop_assert!((hi[2] - lo[2] - 3.0 * factor).abs() < MM);
        s.undo().unwrap();
        prop_assert_eq!(&s.project().save(), &before);
    }
}

/// A real edge on a face that carries drawing is still a real edge.
#[test]
fn a_boundary_edge_of_a_drawn_face_takes_the_solid() {
    let mut s = scene();
    let key = add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    s.draw(Some(key), top_plane(3.0), &ring_at(3.0, 1.0, 1.0, 2.0, 2.0))
        .unwrap();
    let e = erasable(&mut s, [4.0, -30.0, 30.0], [4.0, 0.0, 3.0]);
    assert!(matches!(e, Erasable::Node { .. }), "{e:?}");
    s.erase(&e).unwrap();
    assert_eq!(solids(s.project()).0, 0);
}

/// Copying drawing off its plane is one project step, whatever the count, and
/// respacing it twice leaves only the last array.
#[test]
fn off_plane_arrays_are_one_step_and_respace_cleanly() {
    let mut s = scene();
    draw_rect(&mut s, 0.0, 0.0, 2.0, 2.0);
    let base = sketch_edges(s.project());
    let hit = s.snap(&query([1.0, 1.0, 50.0], [1.0, 1.0, 0.0])).unwrap();
    let m = s.movable_at(&hit).unwrap();
    let up = Transform::translation(Vec3::new(0.0, 0.0, 3.0));

    s.copy(&m, &up, NonZeroU32::MIN).unwrap();
    s.respace_copy(&m, &up, NonZeroU32::new(3).unwrap())
        .unwrap();
    assert_eq!(sketch_edges(s.project()), base + 12);
    s.respace_copy(&m, &up, NonZeroU32::new(2).unwrap())
        .unwrap();
    assert_eq!(
        sketch_edges(s.project()),
        base + 8,
        "only the last array stays"
    );
    s.undo().unwrap();
    assert_eq!(sketch_edges(s.project()), base, "one step");
    s.redo().unwrap();
    assert_eq!(sketch_edges(s.project()), base + 8);
    s.undo().unwrap();
    assert_eq!(s.undo().unwrap(), Some(TileKey::ORIGIN));
    assert_eq!(sketch_edges(s.project()), 0);
}

/// A copy lying exactly on existing drawing changes nothing, so it is no step:
/// the project's undo stack must not drift from the tile's.
#[test]
fn a_copy_onto_itself_is_no_step() {
    let mut s = scene();
    draw_rect(&mut s, 0.0, 0.0, 2.0, 2.0);
    draw_rect(&mut s, 5.0, 0.0, 2.0, 2.0);
    let hit = s.snap(&query([1.0, 1.0, 50.0], [1.0, 1.0, 0.0])).unwrap();
    let m = s.movable_at(&hit).unwrap();
    let over = Transform::translation(Vec3::new(5.0, 0.0, 0.0));
    s.copy(&m, &over, NonZeroU32::MIN).unwrap();
    assert!(s.undo().unwrap().is_some());
    assert!(s.undo().unwrap().is_some());
    assert_eq!(s.undo().unwrap(), None, "two drawings, two steps");
}

fn nodes(s: &Scene, key: TileKey) -> Vec<kernel::NodeId> {
    s.project().tiles[&key].top_level_nodes()
}

/// A group moves as one, survives a save, and dissolves back.
#[test]
fn groups_move_as_one_and_dissolve() {
    let mut s = scene();
    let key = add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    add_box(&mut s, 10.0, 0.0, 5.0, 4.0, 3.0);
    let both = nodes(&s, key);
    assert_eq!(both.len(), 2);
    s.group(key, &both).unwrap();
    assert_eq!(nodes(&s, key).len(), 1);
    assert_eq!(solids(s.project()).0, 2, "the geometry is untouched");

    let m = the_box(&mut s, 2.0, 2.0, 3.0);
    assert!(
        matches!(
            m,
            Movable::Node {
                node: kernel::NodeId::Group(_),
                ..
            }
        ),
        "{m:?}"
    );
    assert_eq!(s.movable_lines(&m).len(), 24, "both boxes follow");
    s.transform(&m, &Transform::translation(Vec3::new(0.0, 20.0, 0.0)))
        .unwrap();
    let (lo, hi) = solids(s.project()).1.unwrap();
    assert!(
        (lo[1] - 20.0).abs() < MM && (hi[0] - 15.0).abs() < MM,
        "{lo:?} {hi:?}"
    );

    let bytes = s.project().save();
    let reopened = Project::open(&bytes).unwrap();
    assert_eq!(
        reopened.tiles[&key].top_level_nodes().len(),
        1,
        "the group is saved"
    );
    assert_eq!(reopened.save(), bytes);

    let group = nodes(&s, key)[0];
    s.dissolve(key, &[group]).unwrap();
    assert_eq!(nodes(&s, key).len(), 2);
    s.undo().unwrap();
    assert_eq!(nodes(&s, key).len(), 1);
    let g = nodes(&s, key);
    assert!(matches!(s.dissolve(key, &g), Ok(())));
    let plain = nodes(&s, key)[0];
    assert!(
        s.dissolve(key, &[plain]).is_err(),
        "a plain object has nothing to dissolve"
    );
}

/// Instances of a component share geometry; exploding one frees it.
#[test]
fn component_copies_are_instances() {
    let mut s = scene();
    let key = add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    let one = nodes(&s, key);
    s.make_component(key, &one).unwrap();
    assert!(matches!(nodes(&s, key)[0], kernel::NodeId::Instance(_)));

    let m = the_box(&mut s, 2.0, 2.0, 3.0);
    let step = Transform::translation(Vec3::new(8.0, 0.0, 0.0));
    s.copy(&m, &step, NonZeroU32::new(3).unwrap()).unwrap();
    let doc = &s.project().tiles[&key];
    assert_eq!(doc.top_level_nodes().len(), 4);
    assert!(
        doc.top_level_nodes()
            .iter()
            .all(|n| matches!(n, kernel::NodeId::Instance(_)))
    );
    assert_eq!(
        doc.visible_object_ids().len(),
        0,
        "no copy of the geometry in the world"
    );

    // Every instance snaps and picks where it stands.
    let far = the_box(&mut s, 26.0, 2.0, 3.0);
    assert_ne!(far, m);

    let n = nodes(&s, key)[1];
    s.dissolve(key, &[n]).unwrap();
    assert_eq!(s.project().tiles[&key].visible_object_ids().len(), 1);
    s.undo().unwrap();

    let bytes = s.project().save();
    assert_eq!(Project::open(&bytes).unwrap().save(), bytes);
}

#[test]
fn deleting_nodes_is_one_step() {
    let mut s = scene();
    let key = add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    add_box(&mut s, 10.0, 0.0, 5.0, 4.0, 3.0);
    let both = nodes(&s, key);
    s.delete_nodes(key, &both).unwrap();
    assert_eq!(solids(s.project()).0, 0);
    s.undo().unwrap();
    assert_eq!(solids(s.project()).0, 2);
}

/// Dissolving several at once is one step, and skips what has nothing to dissolve.
#[test]
fn dissolving_several_is_one_step() {
    let mut s = scene();
    let key = add_box(&mut s, 0.0, 0.0, 5.0, 4.0, 3.0);
    add_box(&mut s, 10.0, 0.0, 5.0, 4.0, 3.0);
    add_box(&mut s, 20.0, 0.0, 5.0, 4.0, 3.0);
    let all = nodes(&s, key);
    s.group(key, &all[..1]).unwrap();
    let now = nodes(&s, key);
    let second = *now
        .iter()
        .find(|n| matches!(n, kernel::NodeId::Object(_)))
        .unwrap();
    s.make_component(key, &[second]).unwrap();
    let mixed = nodes(&s, key);
    assert_eq!(mixed.len(), 3);
    s.dissolve(key, &mixed).unwrap();
    assert!(
        nodes(&s, key)
            .iter()
            .all(|n| matches!(n, kernel::NodeId::Object(_)))
    );
    s.undo().unwrap();
    assert_eq!(nodes(&s, key), mixed, "one undo restores them all");
}
