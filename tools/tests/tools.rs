//! Tools driven by scripted input, the way the UI will drive them: rays and
//! typed text in, geometry out. This is the uji asap without a GPU.

use inference::{Axis, PickRay};
use kernel::{Point3, Vec3};
use proptest::prelude::*;
use wangungis_scene::ProjectOrigin;
use wangungis_scene::project::Project;
use wangungis_tools::tool::{Input, Target, Tool};
use wangungis_tools::{EditError, Scene};

const MM: f64 = 1e-3;
const APERTURE: f64 = 0.01;

fn scene() -> Scene {
    let origin = ProjectOrigin {
        epsg: 32749,
        origin: [430_000.123, 9_150_000.456, 12.5],
    };
    Scene::new(Project::new(Some(origin)).unwrap())
}

/// A ray from a drone-height camera through `at`.
fn ray(at: [f64; 3]) -> PickRay {
    let origin = Point3::new(at[0] - 15.0, at[1] - 25.0, at[2] + 40.0);
    PickRay {
        origin,
        direction: Point3::new(at[0], at[1], at[2]) - origin,
    }
}

fn click(t: &mut Tool, s: &mut Scene, at: [f64; 3]) {
    t.handle(s, Input::Click(ray(at))).unwrap();
}

fn hover(t: &mut Tool, s: &mut Scene, at: [f64; 3]) {
    t.handle(s, Input::Hover(ray(at))).unwrap();
}

fn typed(t: &mut Tool, s: &mut Scene, text: &str) -> Result<(), EditError> {
    t.handle(s, Input::Typed(text.into()))
}

fn bounds(s: &Scene) -> Vec<([f64; 3], [f64; 3])> {
    let mut out = Vec::new();
    for (key, doc) in &s.project().tiles {
        let c = key.center();
        for o in doc.visible_object_ids() {
            let mut lo = [f64::INFINITY; 3];
            let mut hi = [f64::NEG_INFINITY; 3];
            for v in doc.object(o).unwrap().vertices().values() {
                let q = [
                    v.position.x + c[0],
                    v.position.y + c[1],
                    v.position.z + c[2],
                ];
                for k in 0..3 {
                    lo[k] = lo[k].min(q[k]);
                    hi[k] = hi[k].max(q[k]);
                }
            }
            out.push((lo, hi));
        }
    }
    out
}

fn assert_box(s: &Scene, lo: [f64; 3], hi: [f64; 3]) {
    let all = bounds(s);
    assert!(
        all.iter()
            .any(|(l, h)| (0..3).all(|k| (l[k] - lo[k]).abs() <= MM && (h[k] - hi[k]).abs() <= MM)),
        "no box {lo:?}..{hi:?} in {all:?}"
    );
}

fn edges(s: &Scene) -> usize {
    s.project()
        .tiles
        .values()
        .map(|d| {
            d.sketch_ids()
                .iter()
                .filter_map(|i| d.sketch(*i))
                .map(|k| k.edges().len())
                .sum::<usize>()
        })
        .sum()
}

/// Solid faces whose outer loop lies wholly where `on` says, tile-local.
fn faces_where(s: &Scene, on: impl Fn(Point3) -> bool) -> usize {
    let mut n = 0;
    for doc in s.project().tiles.values() {
        for id in doc.visible_object_ids() {
            let o = doc.object(id).unwrap();
            n += o
                .faces()
                .values()
                .filter(|f| o.loop_positions(f.outer_loop).all(&on))
                .count();
        }
    }
    n
}

/// Draws the 5 x 4 m rectangle at (10, 10) with typed sizes.
fn typed_rectangle(s: &mut Scene) {
    let mut r = Tool::rectangle(APERTURE);
    click(&mut r, s, [10.0, 10.0, 0.0]);
    assert!(r.busy());
    hover(&mut r, s, [12.0, 12.0, 0.0]);
    typed(&mut r, s, "5;4").unwrap();
    assert!(!r.busy(), "typed size finishes the rectangle");
}

/// The uji asap: 5 x 4 x 3 m by typing, undo, redo, save, reopen.
#[test]
fn smoke_flow_with_typed_numbers() {
    let mut s = scene();
    typed_rectangle(&mut s);
    assert_eq!(edges(&s), 4);

    let mut p = Tool::push_pull(APERTURE);
    hover(&mut p, &mut s, [12.5, 12.0, 0.0]);
    assert_eq!(
        p.preview().lines.len(),
        4,
        "outline of what a click would pick"
    );
    hover(&mut p, &mut s, [100.0, 100.0, 0.0]);
    assert!(p.preview().lines.is_empty());
    click(&mut p, &mut s, [12.5, 12.0, 0.0]);
    assert!(p.busy(), "the region is picked");
    typed(&mut p, &mut s, "3").unwrap();
    assert!(!p.busy());
    assert_box(&s, [10.0, 10.0, 0.0], [15.0, 14.0, 3.0]);

    s.undo().unwrap();
    assert!(bounds(&s).is_empty());
    s.undo().unwrap();
    assert_eq!(edges(&s), 0);
    s.redo().unwrap();
    s.redo().unwrap();
    assert_box(&s, [10.0, 10.0, 0.0], [15.0, 14.0, 3.0]);

    let back = Scene::new(Project::open(&s.project().save()).unwrap());
    assert_box(&back, [10.0, 10.0, 0.0], [15.0, 14.0, 3.0]);
}

#[test]
fn rectangle_by_two_clicks_and_toward_the_cursor() {
    let mut s = scene();
    let mut r = Tool::rectangle(APERTURE);
    click(&mut r, &mut s, [10.0, 10.0, 0.0]);
    hover(&mut r, &mut s, [13.0, 12.0, 0.0]);
    assert_eq!(r.preview().lines.len(), 4, "rubber band");
    click(&mut r, &mut s, [13.0, 12.0, 0.0]);
    assert_eq!(edges(&s), 4);
    assert!(r.preview().lines.is_empty());

    // Typed sizes follow the cursor's quadrant.
    click(&mut r, &mut s, [50.0, 50.0, 0.0]);
    hover(&mut r, &mut s, [48.0, 47.0, 0.0]);
    typed(&mut r, &mut s, "5;4").unwrap();
    let mut p = Tool::push_pull(APERTURE);
    click(&mut p, &mut s, [47.0, 48.0, 0.0]);
    typed(&mut p, &mut s, "1").unwrap();
    assert_box(&s, [45.0, 46.0, 0.0], [50.0, 50.0, 1.0]);
}

/// A click on a box's top draws on the top: the plane comes from the face.
#[test]
fn drawing_on_a_face_stays_on_it() {
    let mut s = scene();
    typed_rectangle(&mut s);
    let mut p = Tool::push_pull(APERTURE);
    click(&mut p, &mut s, [12.5, 12.0, 0.0]);
    typed(&mut p, &mut s, "3").unwrap();

    let mut r = Tool::rectangle(APERTURE);
    click(&mut r, &mut s, [11.0, 11.0, 3.0]);
    hover(&mut r, &mut s, [12.0, 12.0, 3.0]);
    typed(&mut r, &mut s, "1;1").unwrap();
    assert_eq!(
        faces_where(&s, |p| (p.z - 3.0).abs() <= MM),
        2,
        "top not split"
    );
    assert_eq!(edges(&s), 0, "nothing stayed a sketch");
}

#[test]
fn push_pull_on_a_solid_face_by_typing_and_by_cursor() {
    let mut s = scene();
    typed_rectangle(&mut s);
    let mut p = Tool::push_pull(APERTURE);
    click(&mut p, &mut s, [12.5, 12.0, 0.0]);
    typed(&mut p, &mut s, "3").unwrap();

    // Negative: the top goes down.
    click(&mut p, &mut s, [12.5, 12.0, 3.0]);
    typed(&mut p, &mut s, "-1").unwrap();
    assert_box(&s, [10.0, 10.0, 0.0], [15.0, 14.0, 2.0]);

    // By cursor: the top follows the ray's closest point on its normal. The
    // ray is from the side, aimed straight at height 5.
    click(&mut p, &mut s, [12.5, 12.0, 2.0]);
    let side = PickRay {
        origin: Point3::new(12.5, -40.0, 5.0),
        direction: Vec3::new(0.0, 1.0, 0.0),
    };
    p.handle(&mut s, Input::Hover(side)).unwrap();
    assert!(!p.preview().lines.is_empty(), "outline preview");
    p.handle(&mut s, Input::Click(side)).unwrap();
    assert_box(&s, [10.0, 10.0, 0.0], [15.0, 14.0, 5.0]);
}

#[test]
fn line_chain_closes_a_loop_and_ends() {
    let mut s = scene();
    let mut l = Tool::line(APERTURE);
    click(&mut l, &mut s, [10.0, 10.0, 0.0]);
    for at in [[15.0, 10.0, 0.0], [15.0, 14.0, 0.0], [10.0, 14.0, 0.0]] {
        hover(&mut l, &mut s, at);
        assert_eq!(l.preview().lines.len(), 1, "rubber band");
        click(&mut l, &mut s, at);
        assert!(l.busy(), "chain goes on");
    }
    assert_eq!(edges(&s), 3);
    click(&mut l, &mut s, [10.0, 10.0, 0.0]);
    assert_eq!(edges(&s), 4);
    assert!(!l.busy(), "closing the loop ends the chain");
    let mut p = Tool::push_pull(APERTURE);
    click(&mut p, &mut s, [12.5, 12.0, 0.0]);
    typed(&mut p, &mut s, "3").unwrap();
    assert_box(&s, [10.0, 10.0, 0.0], [15.0, 14.0, 3.0]);
}

#[test]
fn typed_length_goes_toward_the_cursor() {
    let mut s = scene();
    let mut l = Tool::line(APERTURE);
    assert!(typed(&mut l, &mut s, "3").is_err(), "no start point yet");
    click(&mut l, &mut s, [10.0, 10.0, 0.0]);
    assert!(typed(&mut l, &mut s, "3").is_err(), "no direction yet");
    hover(&mut l, &mut s, [20.0, 20.0, 0.0]);
    typed(&mut l, &mut s, "2,5").unwrap();
    assert_eq!(edges(&s), 1);
    let doc = s.project().tiles.values().next().unwrap();
    let sk = doc.sketch(doc.sketch_ids()[0]).unwrap();
    let e = sk.edges().values().next().unwrap();
    let len = (sk.vertices()[e.from].position - sk.vertices()[e.to].position).length();
    assert!((len - 2.5).abs() <= MM, "length {len}");
    assert!(typed(&mut l, &mut s, "abc").is_err());
    assert!(typed(&mut l, &mut s, "1;2").is_err());
    assert!(l.busy(), "bad input keeps the gesture");
}

#[test]
fn axis_lock_keeps_the_line_on_the_axis() {
    let mut s = scene();
    let mut l = Tool::line(APERTURE);
    click(&mut l, &mut s, [10.0, 10.0, 0.0]);
    l.handle(&mut s, Input::LockAxis(Axis::X)).unwrap();
    click(&mut l, &mut s, [20.0, 17.0, 0.0]);
    let doc = s.project().tiles.values().next().unwrap();
    let sk = doc.sketch(doc.sketch_ids()[0]).unwrap();
    let e = sk.edges().values().next().unwrap();
    let (a, b) = (sk.vertices()[e.from].position, sk.vertices()[e.to].position);
    assert!(
        (a.y - 10.0).abs() <= MM && (b.y - 10.0).abs() <= MM,
        "{a:?} {b:?}"
    );
    assert!((a.x - b.x).abs() > 1.0, "{a:?} {b:?}");
    // Z is not on the ground plane: refused, and the gesture survives.
    assert!(l.handle(&mut s, Input::LockAxis(Axis::Z)).is_err());
    assert!(l.busy());
}

/// A later click snaps to an existing corner: the second rectangle shares it.
#[test]
fn clicks_snap_to_existing_corners() {
    let mut s = scene();
    typed_rectangle(&mut s);
    let mut r = Tool::rectangle(APERTURE);
    click(&mut r, &mut s, [15.2, 14.2, 0.0]);
    hover(&mut r, &mut s, [18.0, 18.0, 0.0]);
    typed(&mut r, &mut s, "2;2").unwrap();
    let mut p = Tool::push_pull(APERTURE);
    click(&mut p, &mut s, [16.0, 15.0, 0.0]);
    typed(&mut p, &mut s, "1").unwrap();
    assert_box(&s, [15.0, 14.0, 0.0], [17.0, 16.0, 1.0]);
}

#[test]
fn escape_leaves_nothing_behind() {
    let mut s = scene();
    let mut r = Tool::rectangle(APERTURE);
    click(&mut r, &mut s, [10.0, 10.0, 0.0]);
    hover(&mut r, &mut s, [12.0, 12.0, 0.0]);
    r.handle(&mut s, Input::Cancel).unwrap();
    assert!(!r.busy() && r.preview().lines.is_empty());
    assert!(!s.can_undo() && edges(&s) == 0);

    typed_rectangle(&mut s);
    let steps = 1;
    let mut p = Tool::push_pull(APERTURE);
    click(&mut p, &mut s, [12.5, 12.0, 0.0]);
    p.handle(&mut s, Input::Cancel).unwrap();
    assert!(bounds(&s).is_empty());
    for _ in 0..steps {
        s.undo().unwrap();
    }
    assert!(!s.can_undo(), "cancel recorded a step");
}

/// A shallow drag must not collapse onto the axis through the first corner.
#[test]
fn shallow_rectangle_keeps_its_area() {
    let mut s = scene();
    let mut r = Tool::rectangle(APERTURE);
    click(&mut r, &mut s, [10.0, 10.0, 0.0]);
    hover(&mut r, &mut s, [15.0, 10.2, 0.0]);
    click(&mut r, &mut s, [15.0, 10.2, 0.0]);
    assert!(!r.busy(), "rectangle done");
    assert_eq!(edges(&s), 4);
    let mut p = Tool::push_pull(APERTURE);
    click(&mut p, &mut s, [12.5, 10.1, 0.0]);
    typed(&mut p, &mut s, "1").unwrap();
    assert_box(&s, [10.0, 10.0, 0.0], [15.0, 10.2, 1.0]);
}

/// A click right beside a wall's edge still draws on the wall, not on a
/// horizontal plane through the snap.
#[test]
fn click_near_a_wall_edge_draws_on_the_wall() {
    let mut s = scene();
    typed_rectangle(&mut s);
    let mut p = Tool::push_pull(APERTURE);
    click(&mut p, &mut s, [12.5, 12.0, 0.0]);
    typed(&mut p, &mut s, "3").unwrap();

    // From the east, aimed at a point just below the wall's top edge.
    let east = |y: f64, z: f64| PickRay {
        origin: Point3::new(60.0, y, z),
        direction: Point3::new(15.0, y, z) - Point3::new(60.0, y, z),
    };
    let mut r = Tool::rectangle(APERTURE);
    r.handle(&mut s, Input::Click(east(12.0, 2.95))).unwrap();
    assert!(r.busy());
    r.handle(&mut s, Input::Hover(east(13.0, 1.0))).unwrap();
    typed(&mut r, &mut s, "1;1").unwrap();
    assert_eq!(
        faces_where(&s, |p| (p.x - 15.0).abs() <= MM),
        2,
        "wall not split"
    );
}

/// A ray exactly on a face's edge still arms Push/Pull.
#[test]
fn push_pull_arms_from_a_click_right_on_an_edge() {
    let mut s = scene();
    typed_rectangle(&mut s);
    let mut p = Tool::push_pull(APERTURE);
    click(&mut p, &mut s, [12.5, 12.0, 0.0]);
    typed(&mut p, &mut s, "3").unwrap();
    click(&mut p, &mut s, [12.5, 14.0, 3.0]);
    assert!(p.busy(), "the edge's face is picked");
    typed(&mut p, &mut s, "1").unwrap();
    assert_eq!(bounds(&s).len(), 1);
}

/// With the cursor exactly on the first corner, a typed size goes the plus way.
#[test]
fn typed_size_with_no_cursor_span_goes_plus() {
    let mut s = scene();
    let mut r = Tool::rectangle(APERTURE);
    click(&mut r, &mut s, [10.0, 10.0, 0.0]);
    hover(&mut r, &mut s, [10.0, 10.0, 0.0]);
    typed(&mut r, &mut s, "2;3").unwrap();
    let mut p = Tool::push_pull(APERTURE);
    click(&mut p, &mut s, [11.0, 11.0, 0.0]);
    typed(&mut p, &mut s, "1").unwrap();
    assert_box(&s, [10.0, 10.0, 0.0], [12.0, 13.0, 1.0]);
}

#[test]
fn meaningless_clicks_change_nothing() {
    let mut s = scene();
    // Push/Pull on empty space arms nothing.
    let mut p = Tool::push_pull(APERTURE);
    click(&mut p, &mut s, [100.0, 100.0, 0.0]);
    assert!(!p.busy());
    assert!(typed(&mut p, &mut s, "3").is_err());
    // A zero-area rectangle keeps the gesture open and draws nothing.
    let mut r = Tool::rectangle(APERTURE);
    click(&mut r, &mut s, [10.0, 10.0, 0.0]);
    click(&mut r, &mut s, [10.0, 10.0, 0.0]);
    assert!(r.busy());
    assert!(typed(&mut r, &mut s, "0;4").is_ok());
    assert!(
        typed(&mut r, &mut s, "4").is_err(),
        "one number is not a size"
    );
    assert!(!s.can_undo());
    assert_eq!(edges(&s), 0);
}

#[test]
fn select_picks_faces_edges_and_clears_on_empty() {
    let mut s = scene();
    typed_rectangle(&mut s);
    let mut p = Tool::push_pull(APERTURE);
    click(&mut p, &mut s, [12.5, 12.0, 0.0]);
    typed(&mut p, &mut s, "3").unwrap();

    let mut t = Tool::select(APERTURE);
    click(&mut t, &mut s, [12.5, 12.0, 3.0]);
    assert!(matches!(t.selected(), Some(Target::Solid { .. })));
    assert_eq!(t.preview().lines.len(), 4, "face outline");
    click(&mut t, &mut s, [12.5, 10.0, 3.0]);
    assert!(matches!(t.selected(), Some(Target::Solid { .. })));
    assert_eq!(t.preview().lines.len(), 1, "edge");
    click(&mut t, &mut s, [100.0, 100.0, 0.0]);
    assert_eq!(t.selected(), None);
    assert!(t.preview().lines.is_empty());
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    /// Moving the mouse never edits, whatever the tool and the ray.
    #[test]
    fn hovering_never_edits(
        tool in 0..4,
        x in -3_000.0f64..3_000.0,
        y in -3_000.0f64..3_000.0,
        z in -5.0f64..200.0,
        dx in -1.0f64..1.0,
        dy in -1.0f64..1.0,
        dz in -1.0f64..1.0,
    ) {
        let mut s = scene();
        typed_rectangle(&mut s);
        let mut t = match tool {
            0 => Tool::select(APERTURE),
            1 => Tool::line(APERTURE),
            2 => Tool::rectangle(APERTURE),
            _ => Tool::push_pull(APERTURE),
        };
        click(&mut t, &mut s, [12.5, 12.0, 0.0]);
        let before = edges(&s);
        s.take_dirty();
        let r = PickRay {
            origin: Point3::new(x, y, z),
            direction: Vec3::new(dx, dy, dz),
        };
        t.handle(&mut s, Input::Hover(r)).unwrap();
        prop_assert_eq!(edges(&s), before);
        prop_assert!(s.take_dirty().is_empty());
    }
}

/// Solids in the project and the top of the highest one.
fn solid_count(s: &Scene) -> usize {
    s.project()
        .tiles
        .values()
        .map(|d| d.visible_object_ids().len())
        .sum()
}

fn box_at_10(s: &mut Scene) {
    typed_rectangle(s);
    let mut p = Tool::push_pull(APERTURE);
    click(&mut p, s, [12.5, 12.0, 0.0]);
    typed(&mut p, s, "3").unwrap();
}

/// East edge, in the tile, of everything.
fn max_x(s: &Scene) -> f64 {
    let mut x = f64::MIN;
    for (key, d) in &s.project().tiles {
        for id in d.visible_object_ids() {
            for v in d.object(id).unwrap().vertices().values() {
                x = x.max(v.position.x + key.center()[0]);
            }
        }
    }
    x
}

#[test]
fn move_by_clicks_and_by_typing() {
    let mut s = scene();
    box_at_10(&mut s);
    let mut m = Tool::move_tool(APERTURE);
    // Pick up the box by its top, put the base point down 4 m on.
    click(&mut m, &mut s, [12.5, 12.0, 3.0]);
    assert!(m.busy());
    assert!(m.preview().lines.len() >= 12, "the box follows");
    m.handle(&mut s, Input::LockAxis(Axis::X)).unwrap();
    hover(&mut m, &mut s, [16.5, 12.0, 3.0]);
    assert!(m.preview().lines.len() >= 13);
    typed(&mut m, &mut s, "4").unwrap();
    assert!(!m.busy());
    assert!((max_x(&s) - 19.0).abs() <= MM, "{}", max_x(&s));
    assert_eq!(solid_count(&s), 1);

    // By clicks, along the same lock, back again.
    click(&mut m, &mut s, [16.5, 12.0, 3.0]);
    m.handle(&mut s, Input::LockAxis(Axis::X)).unwrap();
    click(&mut m, &mut s, [12.5, 12.0, 3.0]);
    assert!(!m.busy());
    assert!((max_x(&s) - 15.0).abs() <= 0.01, "{}", max_x(&s));
}

#[test]
fn copy_then_count_makes_an_array() {
    let mut s = scene();
    box_at_10(&mut s);
    let mut m = Tool::move_tool(APERTURE);
    click(&mut m, &mut s, [12.5, 12.0, 3.0]);
    m.handle(&mut s, Input::ToggleCopy).unwrap();
    assert!(m.copying());
    m.handle(&mut s, Input::LockAxis(Axis::X)).unwrap();
    hover(&mut m, &mut s, [20.0, 12.0, 3.0]);
    typed(&mut m, &mut s, "7").unwrap();
    assert!(!m.copying(), "the next move is a plain move again");
    assert_eq!(solid_count(&s), 2);

    typed(&mut m, &mut s, "x10").unwrap();
    assert_eq!(solid_count(&s), 11);
    assert!((max_x(&s) - 85.0).abs() <= MM, "{}", max_x(&s));
    // Another count replaces the array.
    typed(&mut m, &mut s, "x3").unwrap();
    assert_eq!(solid_count(&s), 4);
    assert!((max_x(&s) - 36.0).abs() <= MM, "{}", max_x(&s));
    // Dividing fills the first distance.
    typed(&mut m, &mut s, "/7").unwrap();
    assert_eq!(solid_count(&s), 8);
    assert!((max_x(&s) - 22.0).abs() <= MM, "{}", max_x(&s));
    s.undo().unwrap();
    assert_eq!(solid_count(&s), 1, "the whole array is one step");
}

#[test]
fn move_ignores_nothing_and_esc_leaves_no_step() {
    let mut s = scene();
    box_at_10(&mut s);
    let saved = s.project().save();
    let mut m = Tool::move_tool(APERTURE);
    // Empty space picks up nothing.
    click(&mut m, &mut s, [200.0, 200.0, 0.0]);
    assert!(!m.busy());
    assert!(typed(&mut m, &mut s, "x5").is_err(), "nothing was copied");
    // A click on the base point itself moves nothing, the gesture stays.
    click(&mut m, &mut s, [12.5, 12.0, 3.0]);
    click(&mut m, &mut s, [12.5, 12.0, 3.0]);
    assert!(m.busy());
    m.handle(&mut s, Input::Cancel).unwrap();
    assert!(!m.busy());
    assert_eq!(s.project().save(), saved);
}

/// The cursor over the moved box itself must not lift it towards the eye.
#[test]
fn a_box_does_not_climb_its_own_faces() {
    let mut s = scene();
    box_at_10(&mut s);
    let mut m = Tool::move_tool(APERTURE);
    click(&mut m, &mut s, [12.5, 10.0, 0.0]);
    assert!(m.busy());
    // Over the top of the box, seen from above: a point on its own face.
    hover(&mut m, &mut s, [12.5, 12.0, 3.0]);
    let (a, b) = *m.preview().lines.last().unwrap();
    assert!((b.z - a.z).abs() <= MM, "base {a:?}, cursor {b:?}");
}

/// A ground click in line with the Z axis starts on the ground, not up the axis.
#[test]
fn drawing_does_not_start_up_the_z_axis() {
    let mut s = scene();
    // Straight towards the axis from afar: at (1, 1) on the ground, the ray
    // passes the axis well within the pick cone.
    let origin = Point3::new(-30.0, -30.0, 30.0);
    let at = Point3::new(1.0, 1.0, 0.0);
    let ray = PickRay {
        origin,
        direction: at - origin,
    };
    let mut r = Tool::rectangle(0.01);
    r.handle(&mut s, Input::Click(ray)).unwrap();
    assert!(r.busy());
    r.handle(&mut s, Input::Typed("2;2".into())).unwrap();
    assert_box_or_sketch_at_ground(&s);
}

fn assert_box_or_sketch_at_ground(s: &Scene) {
    let doc = s.project().tiles.values().next().unwrap();
    let k = doc.sketch(doc.sketch_ids()[0]).unwrap();
    for v in k.vertices().values() {
        assert!(v.position.z.abs() <= MM, "{v:?}");
    }
}

#[test]
fn rotate_by_typing_and_by_clicks_with_a_radial_array() {
    let mut s = scene();
    box_at_10(&mut s);
    let mut r = Tool::rotate(APERTURE);
    // Pivot on the south-west corner, zero direction east.
    click(&mut r, &mut s, [10.0, 10.0, 0.0]);
    assert!(r.busy());
    click(&mut r, &mut s, [14.0, 10.0, 0.0]);
    hover(&mut r, &mut s, [10.0, 14.0, 0.0]);
    assert!(r.preview().lines.len() >= 12);
    typed(&mut r, &mut s, "90").unwrap();
    assert!(!r.busy());
    assert_box(&s, [6.0, 10.0, 0.0], [10.0, 15.0, 3.0]);
    s.undo().unwrap();

    // Near 90 degrees by mouse lands on it.
    click(&mut r, &mut s, [10.0, 10.0, 0.0]);
    click(&mut r, &mut s, [14.0, 10.0, 0.0]);
    let a = 91f64.to_radians();
    click(
        &mut r,
        &mut s,
        [10.0 + 6.0 * a.cos(), 10.0 + 6.0 * a.sin(), 0.0],
    );
    assert!(!r.busy());
    assert_box(&s, [6.0, 10.0, 0.0], [10.0, 15.0, 3.0]);
    s.undo().unwrap();

    // Copy at 90, then three copies: four boxes around the corner.
    click(&mut r, &mut s, [10.0, 10.0, 0.0]);
    r.handle(&mut s, Input::ToggleCopy).unwrap();
    click(&mut r, &mut s, [14.0, 10.0, 0.0]);
    typed(&mut r, &mut s, "90").unwrap();
    assert_eq!(solid_count(&s), 2);
    typed(&mut r, &mut s, "x3").unwrap();
    assert_eq!(solid_count(&s), 4);
    assert_box(&s, [10.0, 5.0, 0.0], [14.0, 10.0, 3.0]);
    s.undo().unwrap();
    assert_eq!(solid_count(&s), 1);
}

#[test]
fn rotation_axis_follows_the_arrow_keys() {
    let mut s = scene();
    box_at_10(&mut s);
    let mut r = Tool::rotate(APERTURE);
    click(&mut r, &mut s, [10.0, 10.0, 0.0]);
    r.handle(&mut s, Input::LockAxis(Axis::X)).unwrap();
    click(&mut r, &mut s, [10.0, 14.0, 0.0]);
    typed(&mut r, &mut s, "90").unwrap();
    assert_box(&s, [10.0, 7.0, 0.0], [15.0, 10.0, 4.0]);
}

#[test]
fn scale_by_typing_and_by_dragging() {
    let mut s = scene();
    box_at_10(&mut s);
    let mut t = Tool::scale(APERTURE);
    assert!(typed(&mut t, &mut s, "2").is_err(), "nothing picked yet");
    click(&mut t, &mut s, [15.0, 14.0, 3.0]);
    assert!(t.busy());
    assert!(typed(&mut t, &mut s, "-1").is_err(), "no mirroring");
    assert!(t.busy(), "a refused value leaves the gesture open");
    typed(&mut t, &mut s, "2").unwrap();
    assert!(!t.busy());
    // Doubled about the center of the box.
    assert_box(&s, [7.5, 8.0, -1.5], [17.5, 16.0, 4.5]);
    s.undo().unwrap();

    // Away from the center grows it; a click ends it.
    click(&mut t, &mut s, [15.0, 14.0, 3.0]);
    hover(&mut t, &mut s, [25.0, 14.0, 1.5]);
    assert!(t.preview().lines.len() >= 12);
    click(&mut t, &mut s, [25.0, 14.0, 1.5]);
    assert!(!t.busy());
    let (lo, hi) = bounds(&s)[0];
    assert!(hi[0] - lo[0] > 5.5, "{lo:?} {hi:?}");
    let ratio = (hi[0] - lo[0]) / 5.0;
    assert!(((hi[2] - lo[2]) / 3.0 - ratio).abs() < 1e-6, "uniform");
}

fn sketch_points(s: &Scene) -> Vec<Point3> {
    let doc = s.project().tiles.values().next().unwrap();
    doc.sketch_ids()
        .iter()
        .filter_map(|i| doc.sketch(*i))
        .flat_map(|k| {
            k.vertices()
                .values()
                .map(|v| v.position)
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn circle_and_polygon_with_typed_sizes() {
    let mut s = scene();
    let mut c = Tool::circle(APERTURE);
    click(&mut c, &mut s, [20.0, 20.0, 0.0]);
    assert!(c.busy());
    hover(&mut c, &mut s, [23.0, 20.0, 0.0]);
    assert_eq!(c.preview().lines.len(), 48, "the preview shows the circle");
    typed(&mut c, &mut s, "2").unwrap();
    assert!(!c.busy());
    let pts = sketch_points(&s);
    assert_eq!(pts.len(), 48);
    for p in &pts {
        let r = (p.x - 20.0).hypot(p.y - 20.0);
        assert!((r - 2.0).abs() <= MM, "{p:?}");
    }
    assert_eq!(edges(&s), 48);
    // One undo step, and it extrudes.
    let mut p = Tool::push_pull(APERTURE);
    click(&mut p, &mut s, [20.0, 20.0, 0.0]);
    typed(&mut p, &mut s, "3").unwrap();
    assert_eq!(bounds(&s).len(), 1);
    s.undo().unwrap();
    s.undo().unwrap();
    assert_eq!(edges(&s), 0);

    let mut g = Tool::polygon(APERTURE);
    assert!(typed(&mut g, &mut s, "2s").is_err(), "fewer than 3 sides");
    typed(&mut g, &mut s, "8s").unwrap();
    click(&mut g, &mut s, [20.0, 20.0, 0.0]);
    click(&mut g, &mut s, [23.0, 20.0, 0.0]);
    assert!(!g.busy());
    assert_eq!(edges(&s), 8);

    let mut c = Tool::circle(APERTURE);
    assert!(
        typed(&mut c, &mut s, "12s").is_err(),
        "a circle has at least 24"
    );
    typed(&mut c, &mut s, "24s").unwrap();
}

#[test]
fn arc_by_clicks_and_typed_height() {
    let mut s = scene();
    let mut a = Tool::arc(APERTURE);
    click(&mut a, &mut s, [20.0, 20.0, 0.0]);
    click(&mut a, &mut s, [26.0, 20.0, 0.0]);
    assert!(a.busy());
    hover(&mut a, &mut s, [23.0, 22.0, 0.0]);
    assert!(a.preview().lines.len() >= 4, "the preview shows the arc");
    typed(&mut a, &mut s, "3").unwrap();
    assert!(!a.busy());
    // A half circle: radius 3 about (23, 20), on the side of the cursor.
    let pts = sketch_points(&s);
    assert!(pts.len() >= 5);
    for p in &pts {
        let r = (p.x - 23.0).hypot(p.y - 20.0);
        assert!((r - 3.0).abs() <= MM, "{p:?}");
        assert!(p.y >= 20.0 - MM, "{p:?}");
        assert!(p.z.abs() <= MM);
    }
    assert!(
        pts.iter().any(|p| (p.y - 23.0).abs() <= MM),
        "reaches the apex: {pts:?}"
    );
    s.undo().unwrap();
    assert_eq!(edges(&s), 0, "one undo step");

    // The other side, by mouse.
    click(&mut a, &mut s, [20.0, 20.0, 0.0]);
    click(&mut a, &mut s, [26.0, 20.0, 0.0]);
    click(&mut a, &mut s, [23.0, 18.0, 0.0]);
    assert!(!a.busy());
    assert!(sketch_points(&s).iter().all(|p| p.y <= 20.0 + MM));
}

#[test]
fn offset_towards_the_cursor_on_regions_and_faces() {
    let mut s = scene();
    typed_rectangle(&mut s);
    let mut f = Tool::offset(APERTURE);
    click(&mut f, &mut s, [12.5, 12.0, 0.0]);
    assert!(f.busy());
    // Inside, by typing: the rectangle at (10, 10) of 5 x 4 gains a ring inside.
    hover(&mut f, &mut s, [12.5, 12.0, 0.0]);
    typed(&mut f, &mut s, "1").unwrap();
    assert!(!f.busy());
    assert_eq!(edges(&s), 8);
    let pts = sketch_points(&s);
    assert!(
        pts.iter()
            .any(|p| (p.x - 11.0).abs() <= MM && (p.y - 11.0).abs() <= MM),
        "{pts:?}"
    );
    s.undo().unwrap();

    // Outside, by mouse.
    click(&mut f, &mut s, [12.5, 12.0, 0.0]);
    click(&mut f, &mut s, [12.5, 16.0, 0.0]);
    assert!(!f.busy());
    assert_eq!(edges(&s), 8);
    let top = sketch_points(&s)
        .iter()
        .map(|p| p.y)
        .fold(f64::MIN, f64::max);
    assert!((top - 16.0).abs() <= MM, "{top}");
    s.undo().unwrap();

    // On a solid's face: the inner loop splits it.
    let mut p = Tool::push_pull(APERTURE);
    click(&mut p, &mut s, [12.5, 12.0, 0.0]);
    typed(&mut p, &mut s, "3").unwrap();
    click(&mut f, &mut s, [12.5, 12.0, 3.0]);
    hover(&mut f, &mut s, [12.5, 12.0, 3.0]);
    typed(&mut f, &mut s, "1").unwrap();
    assert_eq!(faces_where(&s, |p| (p.z - 3.0).abs() <= MM), 2);
}

/// A line starting on the edge between two faces lies on the one the cursor goes
/// to, whichever the first click's ray happened to hit.
#[test]
fn a_line_from_an_edge_follows_the_cursor_to_its_face() {
    let mut s = scene();
    box_at_10(&mut s);
    let mut l = Tool::line(APERTURE);
    // The top's south edge, then across the top.
    click(&mut l, &mut s, [12.5, 10.0, 3.0]);
    assert!(l.busy());
    hover(&mut l, &mut s, [12.5, 14.0, 3.0]);
    click(&mut l, &mut s, [12.5, 14.0, 3.0]);
    assert!(!l.busy(), "it divides the top, which ends the chain");
    assert_eq!(faces_where(&s, |p| (p.z - 3.0).abs() <= MM), 2);
    s.undo().unwrap();

    // The same start, then down the wall to its bottom edge.
    click(&mut l, &mut s, [12.5, 10.0, 3.0]);
    click(&mut l, &mut s, [12.5, 10.0, 0.0]);
    assert!(!l.busy());
    assert_eq!(faces_where(&s, |p| (p.y - 10.0).abs() <= MM), 2);
}

/// Typing a value that changes nothing leaves no step; the gesture stays open.
#[test]
fn identity_values_are_refused() {
    let mut s = scene();
    box_at_10(&mut s);
    let saved = s.project().save();
    let mut r = Tool::rotate(APERTURE);
    click(&mut r, &mut s, [10.0, 10.0, 0.0]);
    assert!(typed(&mut r, &mut s, "0").is_err());
    assert!(r.busy());
    let mut t = Tool::scale(APERTURE);
    click(&mut t, &mut s, [15.0, 14.0, 3.0]);
    assert!(typed(&mut t, &mut s, "1").is_err());
    assert!(t.busy());
    assert_eq!(s.project().save(), saved);
}

/// A lock set after the cursor last moved still rules the typed direction.
#[test]
fn typed_move_keeps_to_a_late_lock() {
    let mut s = scene();
    box_at_10(&mut s);
    let mut m = Tool::move_tool(APERTURE);
    click(&mut m, &mut s, [12.5, 12.0, 3.0]);
    hover(&mut m, &mut s, [16.0, 15.0, 3.0]);
    m.handle(&mut s, Input::LockAxis(Axis::X)).unwrap();
    typed(&mut m, &mut s, "2").unwrap();
    assert_box(&s, [12.0, 10.0, 0.0], [17.0, 14.0, 3.0]);
}

/// Shift adds to what Select holds and takes back what it already holds.
#[test]
fn shift_click_builds_a_selection() {
    let mut s = scene();
    box_at_10(&mut s);
    let mut r = Tool::rectangle(APERTURE);
    click(&mut r, &mut s, [30.0, 10.0, 0.0]);
    hover(&mut r, &mut s, [32.0, 12.0, 0.0]);
    typed(&mut r, &mut s, "5;4").unwrap();
    let mut p = Tool::push_pull(APERTURE);
    click(&mut p, &mut s, [32.5, 12.0, 0.0]);
    typed(&mut p, &mut s, "3").unwrap();

    let mut t = Tool::select(APERTURE);
    assert_eq!(t.picked_nodes(), None);
    click(&mut t, &mut s, [12.5, 12.0, 3.0]);
    assert_eq!(t.picked_nodes().unwrap().1.len(), 1);
    t.handle(&mut s, Input::ClickAdd(ray([32.5, 12.0, 3.0])))
        .unwrap();
    let (tile, both) = t.picked_nodes().unwrap();
    assert_eq!(both.len(), 2);
    assert!(t.preview().lines.len() >= 24, "both are outlined");
    t.handle(&mut s, Input::ClickAdd(ray([32.5, 12.0, 3.0])))
        .unwrap();
    assert_eq!(t.picked_nodes().unwrap().1.len(), 1, "shift takes it back");
    t.handle(&mut s, Input::ClickAdd(ray([32.5, 12.0, 3.0])))
        .unwrap();
    // A plain click starts over.
    click(&mut t, &mut s, [12.5, 12.0, 3.0]);
    assert_eq!(t.picked_nodes().unwrap().1.len(), 1);
    t.handle(&mut s, Input::ClickAdd(ray([32.5, 12.0, 3.0])))
        .unwrap();

    s.group(tile, &both).unwrap();
    assert_eq!(s.project().tiles[&tile].top_level_nodes().len(), 1);
    // Other tools take shift-click as a click.
    let mut m = Tool::move_tool(APERTURE);
    m.handle(&mut s, Input::ClickAdd(ray([12.5, 12.0, 3.0])))
        .unwrap();
    assert!(m.busy());
}
