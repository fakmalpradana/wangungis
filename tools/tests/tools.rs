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
    let mut z = Vec::new();
    for doc in s.project().tiles.values() {
        for id in doc.sketch_ids() {
            let sk = doc.sketch(id).unwrap();
            if !sk.edges().is_empty() {
                z.extend(
                    sk.edges()
                        .values()
                        .map(|e| sk.vertices()[e.from].position.z),
                );
            }
        }
    }
    assert!(z.iter().any(|v| (v - 3.0).abs() <= MM), "not on top: {z:?}");
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
