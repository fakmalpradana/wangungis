//! Tools driven by scripted input, the way the UI will drive them: rays and
//! typed text in, geometry out. This is the uji asap without a GPU.

use inference::{Axis, PickRay, SnapKind};
use kernel::{NodeId, Point3, Rgba8, Vec3};
use proptest::prelude::*;
use wangungis_scene::ProjectOrigin;
use wangungis_scene::project::Project;
use wangungis_tools::tool::{Input, Target, Tool};
use wangungis_tools::vcb::Unit;
use wangungis_tools::{EditError, Paint, Scene};

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
    assert!(l.busy());
}

/// All the segments drawn, each as its two ends.
fn segments(s: &Scene) -> Vec<(Point3, Point3)> {
    let mut out = Vec::new();
    for (key, doc) in &s.project().tiles {
        let c = key.center();
        let up = |p: Point3| Point3::new(p.x + c[0], p.y + c[1], p.z + c[2]);
        for id in doc.sketch_ids() {
            let sk = doc.sketch(id).unwrap();
            for e in sk.edges().values() {
                out.push((
                    up(sk.vertices()[e.from].position),
                    up(sk.vertices()[e.to].position),
                ));
            }
        }
    }
    out
}

fn has_segment(s: &Scene, a: [f64; 3], b: [f64; 3]) {
    let near = |p: Point3, q: [f64; 3]| (p - Point3::new(q[0], q[1], q[2])).length() <= MM;
    let all = segments(s);
    assert!(
        all.iter()
            .any(|(p, q)| (near(*p, a) && near(*q, b)) || (near(*p, b) && near(*q, a))),
        "no segment {a:?} to {b:?} in {all:?}"
    );
}

/// An axis off the drawing's plane takes the line off it. Locks one after
/// another stay on one plane where they can, so they close into a region.
#[test]
fn a_locked_axis_leaves_the_plane_and_can_close_a_loop() {
    let mut s = scene();
    let mut l = Tool::line(APERTURE);
    click(&mut l, &mut s, [10.0, 10.0, 0.0]);
    l.handle(&mut s, Input::LockAxis(Axis::Z)).unwrap();
    assert!(l.hint().contains("sumbu Z"), "{}", l.hint());
    hover(&mut l, &mut s, [10.0, 10.0, 2.0]);
    typed(&mut l, &mut s, "4").unwrap();
    has_segment(&s, [10.0, 10.0, 0.0], [10.0, 10.0, 4.0]);
    assert!(!l.hint().contains("Terkunci"), "one segment only");

    l.handle(&mut s, Input::LockAxis(Axis::X)).unwrap();
    hover(&mut l, &mut s, [12.0, 10.0, 4.0]);
    typed(&mut l, &mut s, "3").unwrap();
    has_segment(&s, [10.0, 10.0, 4.0], [13.0, 10.0, 4.0]);

    l.handle(&mut s, Input::LockAxis(Axis::Z)).unwrap();
    hover(&mut l, &mut s, [13.0, 10.0, 2.0]);
    typed(&mut l, &mut s, "4").unwrap();
    has_segment(&s, [13.0, 10.0, 4.0], [13.0, 10.0, 0.0]);

    l.handle(&mut s, Input::LockAxis(Axis::X)).unwrap();
    hover(&mut l, &mut s, [11.0, 10.0, 0.0]);
    typed(&mut l, &mut s, "3").unwrap();
    assert!(!l.busy(), "the loop closed");
    let regions: usize = s
        .project()
        .tiles
        .iter()
        .flat_map(|(k, d)| d.sketch_ids().into_iter().map(move |id| (*k, id)))
        .map(|(k, id)| s.region_count(k, id))
        .sum();
    assert_eq!(regions, 1);
}

/// A lock holds one segment: the next one is free again, on the same plane.
#[test]
fn a_lock_holds_one_segment_only() {
    let mut s = scene();
    let mut l = Tool::line(APERTURE);
    click(&mut l, &mut s, [10.0, 10.0, 0.0]);
    l.handle(&mut s, Input::LockAxis(Axis::X)).unwrap();
    hover(&mut l, &mut s, [20.0, 17.0, 0.0]);
    typed(&mut l, &mut s, "10").unwrap();
    has_segment(&s, [10.0, 10.0, 0.0], [20.0, 10.0, 0.0]);
    click(&mut l, &mut s, [25.0, 17.0, 0.0]);
    has_segment(&s, [20.0, 10.0, 0.0], [25.0, 17.0, 0.0]);
}

/// `30d` turns away from the locked axis, or from X, towards the pointer.
#[test]
fn a_typed_angle_turns_towards_the_pointer() {
    let (c, h) = (
        10.0 * 30f64.to_radians().cos(),
        10.0 * 30f64.to_radians().sin(),
    );
    for (cursor, end) in [
        ([20.0, 15.0], [10.0 + c, 10.0 + h]),
        ([0.0, 15.0], [10.0 - c, 10.0 + h]),
        ([0.0, 5.0], [10.0 - c, 10.0 - h]),
        ([20.0, 5.0], [10.0 + c, 10.0 - h]),
    ] {
        for lock in [false, true] {
            let mut s = scene();
            let mut l = Tool::line(APERTURE);
            click(&mut l, &mut s, [10.0, 10.0, 0.0]);
            if lock {
                l.handle(&mut s, Input::LockAxis(Axis::X)).unwrap();
            }
            hover(&mut l, &mut s, [cursor[0], cursor[1], 0.0]);
            typed(&mut l, &mut s, "30d").unwrap();
            assert!(l.hint().contains("30° dari X"), "{}", l.hint());
            hover(&mut l, &mut s, [cursor[0], cursor[1], 0.0]);
            typed(&mut l, &mut s, "10").unwrap();
            has_segment(&s, [10.0, 10.0, 0.0], [end[0], end[1], 0.0]);
        }
    }

    // Typed again, it starts from the axis again.
    let mut s = scene();
    let mut l = Tool::line(APERTURE);
    click(&mut l, &mut s, [10.0, 10.0, 0.0]);
    hover(&mut l, &mut s, [20.0, 15.0, 0.0]);
    typed(&mut l, &mut s, "10d").unwrap();
    hover(&mut l, &mut s, [20.0, 15.0, 0.0]);
    typed(&mut l, &mut s, "30d").unwrap();
    hover(&mut l, &mut s, [20.0, 15.0, 0.0]);
    typed(&mut l, &mut s, "10").unwrap();
    has_segment(&s, [10.0, 10.0, 0.0], [10.0 + c, 10.0 + h, 0.0]);

    // A negative angle takes the other side.
    let mut s = scene();
    let mut l = Tool::line(APERTURE);
    click(&mut l, &mut s, [10.0, 10.0, 0.0]);
    hover(&mut l, &mut s, [20.0, 15.0, 0.0]);
    typed(&mut l, &mut s, "-30d").unwrap();
    hover(&mut l, &mut s, [20.0, 5.0, 0.0]);
    typed(&mut l, &mut s, "10").unwrap();
    has_segment(&s, [10.0, 10.0, 0.0], [10.0 + c, 10.0 - h, 0.0]);

    // From Z: on the axis plane facing the camera, here y = 10.
    let mut s = scene();
    let mut l = Tool::line(APERTURE);
    click(&mut l, &mut s, [10.0, 10.0, 0.0]);
    l.handle(&mut s, Input::LockAxis(Axis::Z)).unwrap();
    hover(&mut l, &mut s, [15.0, 10.0, 8.0]);
    typed(&mut l, &mut s, "30d").unwrap();
    hover(&mut l, &mut s, [15.0, 10.0, 8.66]);
    typed(&mut l, &mut s, "10").unwrap();
    has_segment(&s, [10.0, 10.0, 0.0], [10.0 + h, 10.0, c]);
    assert!(s.can_undo());
    s.undo().unwrap();
    assert!(segments(&s).is_empty());
}

/// An angle chose one way along the line: a typed length goes that way wherever
/// the pointer went since, and without it having moved at all. Behind the start
/// neither the preview nor a click draws anything.
#[test]
fn an_angle_goes_one_way() {
    let (c, h) = (
        10.0 * 30f64.to_radians().cos(),
        10.0 * 30f64.to_radians().sin(),
    );
    let mut s = scene();
    let mut l = Tool::line(APERTURE);
    click(&mut l, &mut s, [10.0, 10.0, 0.0]);
    assert!(typed(&mut l, &mut s, "30d").is_err(), "no side to take yet");
    hover(&mut l, &mut s, [12.0, 15.0, 0.0]);
    typed(&mut l, &mut s, "-30d").unwrap();
    // The pointer is behind this direction, as it often is with a negative angle.
    hover(&mut l, &mut s, [12.0, 15.0, 0.0]);
    assert!(
        l.preview()
            .lines
            .iter()
            .all(|(a, b)| (*a - *b).length() <= MM)
    );
    click(&mut l, &mut s, [12.0, 15.0, 0.0]);
    assert!(segments(&s).is_empty());
    typed(&mut l, &mut s, "10").unwrap();
    has_segment(&s, [10.0, 10.0, 0.0], [10.0 + c, 10.0 - h, 0.0]);

    // Straight after a typed angle, with no hover in between.
    let mut s = scene();
    let mut l = Tool::line(APERTURE);
    click(&mut l, &mut s, [10.0, 10.0, 0.0]);
    hover(&mut l, &mut s, [20.0, 15.0, 0.0]);
    typed(&mut l, &mut s, "30d").unwrap();
    typed(&mut l, &mut s, "10").unwrap();
    has_segment(&s, [10.0, 10.0, 0.0], [10.0 + c, 10.0 + h, 0.0]);
    // The ray of that segment says nothing about the next one.
    assert!(typed(&mut l, &mut s, "30d").is_err());
}

/// An angle does not move the drawing's plane until a segment uses it.
#[test]
fn an_unused_angle_leaves_the_plane_alone() {
    let mut s = scene();
    let mut l = Tool::line(APERTURE);
    click(&mut l, &mut s, [10.0, 10.0, 0.0]);
    l.handle(&mut s, Input::LockAxis(Axis::Z)).unwrap();
    hover(&mut l, &mut s, [15.0, 10.0, 8.0]);
    typed(&mut l, &mut s, "30d").unwrap();
    l.handle(&mut s, Input::LockAxis(Axis::X)).unwrap();
    l.handle(&mut s, Input::LockAxis(Axis::X)).unwrap();
    hover(&mut l, &mut s, [15.0, 14.0, 0.0]);
    click(&mut l, &mut s, [15.0, 14.0, 0.0]);
    has_segment(&s, [10.0, 10.0, 0.0], [15.0, 14.0, 0.0]);
}

/// Uji asap step 19: a square on a solid's wall, a line straight out of it, and
/// Follow Me: a rod, one with the box.
#[test]
fn follow_me_from_a_square_on_a_wall_along_a_line_out_of_it() {
    let mut s = scene();
    box_at_10(&mut s);
    // Narrow, for points on the face rather than the midpoints of its edges.
    let mut r = Tool::rectangle(APERTURE / 10.0);
    click(&mut r, &mut s, [12.0, 10.0, 1.0]);
    hover(&mut r, &mut s, [13.0, 10.0, 2.0]);
    typed(&mut r, &mut s, "1;1").unwrap();

    let mut l = Tool::line(APERTURE / 10.0);
    click(&mut l, &mut s, [12.5, 10.0, 1.5]);
    l.handle(&mut s, Input::LockNormal).unwrap();
    assert!(l.hint().contains("tegak lurus"), "{}", l.hint());
    hover(&mut l, &mut s, [12.5, 8.0, 1.5]);
    typed(&mut l, &mut s, "4").unwrap();
    has_segment(&s, [12.5, 10.0, 1.5], [12.5, 6.0, 1.5]);
    l.handle(&mut s, Input::Cancel).unwrap();

    let saved = s.project().save();
    let mut f = Tool::follow_me(APERTURE);
    click(&mut f, &mut s, [12.2, 10.0, 1.8]);
    assert!(f.busy(), "the square on the wall is a profile");
    assert_eq!(f.preview().lines.len(), 4);
    click(&mut f, &mut s, [12.5, 7.0, 1.5]);
    assert!(!f.busy());
    let y = bounds(&s)
        .iter()
        .map(|(lo, _)| lo[1])
        .fold(f64::INFINITY, f64::min);
    assert!((y - 6.0).abs() <= MM, "{y}");
    assert!(
        s.project()
            .tiles
            .values()
            .all(|d| d.health_report(None).rows.is_empty())
    );
    s.undo().unwrap();
    assert_eq!(s.project().save(), saved, "one undo step");
}

proptest! {
    /// Whatever the lock, angle and pointer, a typed length gives a segment of
    /// that length in that direction, from the point clicked.
    #[test]
    fn locked_lines_keep_length_and_direction(
        // Off the axes: a click on one rightly snaps to it.
        x in 1.0..50.0f64,
        y in 1.0..50.0f64,
        lock in 0usize..4,
        degrees in prop::option::of(-180.0..180.0f64),
        to in prop::array::uniform3(-30.0..30.0f64),
        len in 0.01..40.0f64,
    ) {
        let mut s = scene();
        let mut l = Tool::line(APERTURE / 100.0);
        click(&mut l, &mut s, [x, y, 0.0]);
        let input = match lock {
            0 => Input::LockAxis(Axis::X),
            1 => Input::LockAxis(Axis::Y),
            2 => Input::LockAxis(Axis::Z),
            _ => Input::LockNormal,
        };
        l.handle(&mut s, input).unwrap();
        let cursor = [x + to[0], y + to[1], to[2]];
        hover(&mut l, &mut s, cursor);
        if let Some(d) = degrees {
            typed(&mut l, &mut s, &format!("{d}d")).unwrap();
            hover(&mut l, &mut s, cursor);
        }
        // Looking straight along the lock leaves no direction: refused, nothing drawn.
        if typed(&mut l, &mut s, &format!("{len}")).is_err() {
            prop_assert!(segments(&s).is_empty());
            return Ok(());
        }
        let all = segments(&s);
        prop_assert_eq!(all.len(), 1);
        let (a, b) = all[0];
        let start = Point3::new(x, y, 0.0);
        let (near, far) = if (a - start).length() < (b - start).length() { (a, b) } else { (b, a) };
        prop_assert!((near - start).length() <= MM, "{near:?}");
        let span = far - near;
        prop_assert!((span.length() - len).abs() <= MM, "{span:?} {len}");
        let axis = [Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 0.0, 1.0)]
            [lock.min(2)];
        let cos = (span.dot(axis) / span.length()).abs();
        let want = degrees.map_or(1.0, |d| d.to_radians().cos().abs());
        prop_assert!((cos - want).abs() <= 1e-6, "{cos} {want}");
    }

    /// Follow Me on any square on a wall: the solid stays sound, one undo removes it.
    #[test]
    fn follow_me_on_a_wall_keeps_solids_sound(
        x in 11.0..13.5f64,
        z in 0.5..1.5f64,
        size in 0.3..1.0f64,
        len in 0.5..6.0f64,
    ) {
        let mut s = scene();
        box_at_10(&mut s);
        let mut r = Tool::rectangle(APERTURE / 100.0);
        click(&mut r, &mut s, [x, 10.0, z]);
        hover(&mut r, &mut s, [x + 1.0, 10.0, z + 1.0]);
        typed(&mut r, &mut s, &format!("{size};{size}")).unwrap();
        let mid = [x + size / 2.0, 10.0, z + size / 2.0];
        let mut l = Tool::line(APERTURE / 100.0);
        click(&mut l, &mut s, mid);
        l.handle(&mut s, Input::LockNormal).unwrap();
        hover(&mut l, &mut s, [mid[0], 8.0, mid[2]]);
        typed(&mut l, &mut s, &format!("{len}")).unwrap();
        let saved = s.project().save();
        let mut f = Tool::follow_me(APERTURE / 100.0);
        click(&mut f, &mut s, [mid[0] + size / 4.0, 10.0, mid[2] + size / 4.0]);
        prop_assert!(f.busy());
        click(&mut f, &mut s, [mid[0], 10.0 - len / 2.0, mid[2]]);
        prop_assert!(!f.busy());
        let y = bounds(&s).iter().map(|(lo, _)| lo[1]).fold(f64::INFINITY, f64::min);
        prop_assert!((y - (10.0 - len)).abs() <= MM, "{y}");
        prop_assert!(s.project().tiles.values().all(|d| d.health_report(None).rows.is_empty()));
        s.undo().unwrap();
        prop_assert_eq!(s.project().save(), saved);
    }
}

/// A length typed after a lock never follows the cursor from before the lock.
#[test]
fn axis_lock_forgets_the_cursor_from_before() {
    let mut s = scene();
    let mut l = Tool::line(APERTURE);
    click(&mut l, &mut s, [10.0, 10.0, 0.0]);
    hover(&mut l, &mut s, [20.0, 17.0, 0.0]);
    l.handle(&mut s, Input::LockAxis(Axis::X)).unwrap();
    assert!(typed(&mut l, &mut s, "5").is_err(), "no direction yet");
    assert!(!s.can_undo());
    hover(&mut l, &mut s, [20.0, 17.0, 0.0]);
    typed(&mut l, &mut s, "5").unwrap();
    let doc = s.project().tiles.values().next().unwrap();
    let sk = doc.sketch(doc.sketch_ids()[0]).unwrap();
    let e = sk.edges().values().next().unwrap();
    let (a, b) = (sk.vertices()[e.from].position, sk.vertices()[e.to].position);
    assert!((a.y - 10.0).abs() <= MM && (b.y - 10.0).abs() <= MM);
    assert!(((b - a).length() - 5.0).abs() <= MM);
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

/// Rotate takes an angle with its mark too, as Line does.
#[test]
fn rotate_takes_a_marked_angle() {
    let mut s = scene();
    box_at_10(&mut s);
    let mut r = Tool::rotate(APERTURE);
    click(&mut r, &mut s, [10.0, 10.0, 0.0]);
    click(&mut r, &mut s, [15.0, 10.0, 0.0]);
    typed(&mut r, &mut s, "90d").unwrap();
    assert_box(&s, [6.0, 10.0, 0.0], [10.0, 15.0, 3.0]);
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

#[test]
fn tape_measures_and_leaves_guides_that_snap() {
    let mut s = scene();
    box_at_10(&mut s);
    let mut t = Tool::tape(APERTURE);
    // From an edge: a parallel line through where it ends.
    click(&mut t, &mut s, [12.5, 10.0, 0.0]);
    assert!(t.busy());
    hover(&mut t, &mut s, [12.5, 8.0, 0.0]);
    let m = t.measure().unwrap();
    assert!((m - 2.0).abs() < 0.05, "{m}");
    assert!(typed(&mut t, &mut s, "0").is_err(), "zero says so");
    assert!(t.busy());
    typed(&mut t, &mut s, "3").unwrap();
    assert!(!t.busy());
    let lines = s.guide_lines();
    assert_eq!(lines.len(), 1);
    let (a, b) = lines[0];
    assert!(
        (a.y - 7.0).abs() <= MM && (b.y - 7.0).abs() <= MM,
        "{a:?} {b:?}"
    );
    assert!((a.x - b.x).abs() > 100.0, "along the edge");

    // A rectangle corner snaps onto the guide.
    let mut r = Tool::rectangle(APERTURE);
    click(&mut r, &mut s, [30.0, 7.02, 0.0]);
    hover(&mut r, &mut s, [33.0, 4.0, 0.0]);
    typed(&mut r, &mut s, "1;1").unwrap();
    assert!(
        sketch_points(&s).iter().any(|p| (p.y - 7.0).abs() <= MM),
        "{:?}",
        sketch_points(&s)
    );

    // One undo step each, and all gone at once.
    s.undo().unwrap();
    s.undo().unwrap();
    assert!(s.guide_lines().is_empty());
    s.redo().unwrap();
    assert_eq!(s.guide_lines().len(), 1);
    s.delete_guides().unwrap();
    assert!(s.guide_lines().is_empty());
    s.undo().unwrap();
    assert_eq!(s.guide_lines().len(), 1);

    // A cursor off to the side: the distance is still square to the edge.
    s.delete_guides().unwrap();
    click(&mut t, &mut s, [12.5, 10.0, 0.0]);
    hover(&mut t, &mut s, [20.0, 8.0, 0.0]);
    let m = t.measure().unwrap();
    assert!((m - 2.0).abs() < 0.05, "{m}");
    assert_eq!(t.preview().lines.len(), 2, "the span and the guide to come");
    typed(&mut t, &mut s, "3").unwrap();
    let (a, b) = s.guide_lines()[0];
    assert!(
        (a.y - 7.0).abs() <= MM && (b.y - 7.0).abs() <= MM,
        "{a:?} {b:?}"
    );
    s.undo().unwrap();
    s.undo().unwrap();
    assert_eq!(s.guide_lines().len(), 1);

    // From a corner: a point.
    click(&mut t, &mut s, [10.0, 10.0, 3.0]);
    click(&mut t, &mut s, [18.0, 18.0, 3.0]);
    assert!(!t.busy());
    assert_eq!(s.guide_lines().len(), 4, "the line, and a cross of three");
    // A guide saves with the project.
    let bytes = s.project().save();
    assert_eq!(Project::open(&bytes).unwrap().save(), bytes);
}

#[test]
fn follow_me_sweeps_a_region_along_a_line() {
    let mut s = scene();
    // The profile: 1 x 1 on the plane x = 0, facing +x. The path along +x from it.
    let wall =
        kernel::Plane::from_point_normal(Point3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0))
            .unwrap();
    let p = |y: f64, z: f64| Point3::new(0.0, y, z);
    let r = [p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)];
    let segs: Vec<_> = (0..4).map(|k| (r[k], r[(k + 1) % 4])).collect();
    s.draw(None, wall, &segs).unwrap();
    let (a, b, c) = (
        Point3::new(0.0, 0.5, 0.0),
        Point3::new(10.0, 0.5, 0.0),
        Point3::new(10.0, 8.0, 0.0),
    );
    // The path on the ground, away from the wall's own plane.
    let ground =
        kernel::Plane::from_point_normal(Point3::ORIGIN, Vec3::new(0.0, 0.0, 1.0)).unwrap();
    s.draw(None, ground, &[(a, b), (b, c)]).unwrap();

    let mut f = Tool::follow_me(APERTURE);
    let from_x = |at: [f64; 3]| PickRay {
        origin: Point3::new(at[0] + 10.0, at[1], at[2]),
        direction: Vec3::new(-1.0, 0.0, 0.0),
    };
    assert!(
        f.handle(&mut s, Input::Click(ray([10.0, 30.0, 0.0])))
            .is_ok(),
        "a click on nothing does nothing"
    );
    assert!(!f.busy());
    f.handle(&mut s, Input::Click(from_x([0.0, 0.5, 0.6])))
        .unwrap();
    assert!(f.busy(), "the profile is picked");
    assert_eq!(f.preview().lines.len(), 4);
    f.handle(&mut s, Input::Click(ray([5.0, 0.5, 0.0])))
        .unwrap();
    assert!(!f.busy());
    let all = bounds(&s);
    assert_eq!(all.len(), 1, "one solid");
    let (lo, hi) = all[0];
    assert!(
        (hi[1] - 8.0).abs() < 1.0 && hi[0] > 9.5 && lo[2].abs() <= MM,
        "{lo:?} {hi:?}"
    );
    s.undo().unwrap();
    assert!(bounds(&s).is_empty());
}

/// A tile holding only a guide is still part of the saved project.
#[test]
fn a_guide_alone_in_a_tile_is_saved() {
    let mut s = scene();
    s.add_guide_point(Point3::new(3_000.0, 3_000.0, 0.0))
        .unwrap();
    let reopened = Project::open(&s.project().save()).unwrap();
    assert_eq!(
        reopened.occupied().count(),
        2,
        "origin tile and the guide's"
    );
    assert_eq!(Scene::new(reopened).guide_lines().len(), 3);
    assert_eq!(s.guide_lines().len(), 3);
}

/// What Shift takes back stops being lit, and an emptied selection is no selection.
#[test]
fn shift_taking_back_clears_its_outline() {
    let mut s = scene();
    box_at_10(&mut s);
    let mut t = Tool::select(APERTURE);
    click(&mut t, &mut s, [12.5, 12.0, 3.0]);
    assert!(t.busy());
    t.handle(&mut s, Input::ClickAdd(ray([12.5, 12.0, 3.0])))
        .unwrap();
    assert_eq!(t.picked_nodes(), None);
    assert!(t.preview().lines.is_empty());
    assert!(!t.busy(), "nothing is held");
    assert_eq!(t.selected(), None);
}

/// In millimeters, every length reads in mm. Factors, degrees and counts do not.
#[test]
fn a_bare_number_follows_the_unit_only_where_it_is_a_length() {
    let mut s = scene();
    let mm = |mut t: Tool| {
        t.set_unit(Unit::Mm);
        t
    };

    let mut r = mm(Tool::rectangle(APERTURE));
    click(&mut r, &mut s, [10.0, 10.0, 0.0]);
    hover(&mut r, &mut s, [11.0, 11.0, 0.0]);
    typed(&mut r, &mut s, "5000;4000").unwrap();
    let mut p = mm(Tool::push_pull(APERTURE));
    click(&mut p, &mut s, [12.0, 12.0, 0.0]);
    typed(&mut p, &mut s, "3000").unwrap();
    assert_box(&s, [10.0, 10.0, 0.0], [15.0, 14.0, 3.0]);

    let mut m = mm(Tool::move_tool(APERTURE));
    click(&mut m, &mut s, [12.5, 12.0, 3.0]);
    hover(&mut m, &mut s, [16.0, 12.0, 3.0]);
    m.handle(&mut s, Input::LockAxis(Axis::X)).unwrap();
    typed(&mut m, &mut s, "2000").unwrap();
    assert_box(&s, [12.0, 10.0, 0.0], [17.0, 14.0, 3.0]);
    s.undo().unwrap();

    let mut t = mm(Tool::scale(APERTURE));
    click(&mut t, &mut s, [15.0, 14.0, 3.0]);
    typed(&mut t, &mut s, "2").unwrap();
    assert_box(&s, [7.5, 8.0, -1.5], [17.5, 16.0, 4.5]);
    s.undo().unwrap();

    let mut q = mm(Tool::rotate(APERTURE));
    click(&mut q, &mut s, [10.0, 10.0, 0.0]);
    click(&mut q, &mut s, [15.0, 10.0, 0.0]);
    typed(&mut q, &mut s, "90").unwrap();
    assert_box(&s, [6.0, 10.0, 0.0], [10.0, 15.0, 3.0]);
    s.undo().unwrap();

    // Starting over keeps the unit.
    let mut p = mm(Tool::push_pull(APERTURE));
    p.reset();
    click(&mut p, &mut s, [12.5, 12.0, 3.0]);
    typed(&mut p, &mut s, "1000").unwrap();
    assert_box(&s, [10.0, 10.0, 0.0], [15.0, 14.0, 4.0]);
}

proptest! {
    /// Any text in any unit: no panic, and a length comes out finite.
    #[test]
    fn the_value_box_takes_anything(text in "\\PC{0,12}", unit in 0..Unit::ALL.len()) {
        use wangungis_tools::vcb::{Vcb, parse_in};
        match parse_in(&text, Unit::ALL[unit]) {
            Some(Vcb::One(a)) => prop_assert!(a.is_finite()),
            Some(Vcb::Two(a, b)) => prop_assert!(a.is_finite() && b.is_finite()),
            _ => {}
        }
    }
}

/// A free line from `a` to `b`, then Esc.
fn free_line(s: &mut Scene, a: [f64; 3], b: [f64; 3]) {
    let mut l = Tool::line(APERTURE);
    click(&mut l, s, a);
    click(&mut l, s, b);
    l.handle(s, Input::Cancel).unwrap();
}

fn snap_kind(t: &Tool) -> Option<SnapKind> {
    t.preview().snap.map(|h| h.snap.kind)
}

/// What a pointer resting on the model snaps to, strongest first.
#[test]
fn snaps_keep_their_priority() {
    let mut s = scene();
    box_at_10(&mut s);
    let mut l = Tool::line(APERTURE);
    for (at, want) in [
        ([15.0, 14.0, 3.0], SnapKind::Endpoint),
        // Close enough to the edge and its midpoint: the midpoint wins.
        ([15.0, 11.7, 3.0], SnapKind::Midpoint),
        // Points pull harder than the aperture alone: far from both on the long edge.
        ([13.75, 14.0, 3.0], SnapKind::OnEdge),
        ([12.5, 12.0, 3.0], SnapKind::OnFace),
    ] {
        hover(&mut l, &mut s, at);
        assert_eq!(snap_kind(&l), Some(want), "at {at:?}");
    }
}

/// Past an edge, a line runs parallel or square to it, and a click takes what
/// the hover showed.
#[test]
fn a_line_runs_parallel_or_square_to_an_edge_it_passed() {
    let mut s = scene();
    free_line(&mut s, [20.0, 20.0, 0.0], [24.0, 23.0, 0.0]);

    let mut l = Tool::line(APERTURE);
    click(&mut l, &mut s, [40.0, 30.0, 0.0]);
    hover(&mut l, &mut s, [22.0, 21.5, 0.0]);
    assert_eq!(snap_kind(&l), Some(SnapKind::Midpoint), "over the edge");

    // Off by a degree or so.
    hover(&mut l, &mut s, [44.0, 33.1, 0.0]);
    assert_eq!(snap_kind(&l), Some(SnapKind::Parallel));
    hover(&mut l, &mut s, [43.0, 26.1, 0.0]);
    assert_eq!(snap_kind(&l), Some(SnapKind::Perpendicular));
    // Well off both.
    hover(&mut l, &mut s, [44.0, 36.0, 0.0]);
    assert_eq!(snap_kind(&l), None);
    // Under a lock there is no inference.
    hover(&mut l, &mut s, [44.0, 33.1, 0.0]);
    l.handle(&mut s, Input::LockAxis(Axis::X)).unwrap();
    hover(&mut l, &mut s, [44.0, 33.1, 0.0]);
    assert_ne!(snap_kind(&l), Some(SnapKind::Parallel));
    l.handle(&mut s, Input::LockAxis(Axis::X)).unwrap();

    hover(&mut l, &mut s, [44.0, 33.1, 0.0]);
    click(&mut l, &mut s, [44.0, 33.1, 0.0]);
    let end = [40.0 + 0.8 * 5.06, 30.0 + 0.6 * 5.06, 0.0];
    has_segment(&s, [40.0, 30.0, 0.0], end);

    // The segment just drawn is the next reference, and a typed length follows.
    hover(&mut l, &mut s, [end[0] - 3.0, end[1] + 4.1, 0.0]);
    assert_eq!(snap_kind(&l), Some(SnapKind::Perpendicular));
    typed(&mut l, &mut s, "5").unwrap();
    has_segment(&s, end, [end[0] - 3.0, end[1] + 4.0, 0.0]);
}

/// A real point beats the inference even when it lies exactly on that line.
#[test]
fn a_corner_beats_running_parallel() {
    let mut s = scene();
    free_line(&mut s, [20.0, 20.0, 0.0], [24.0, 23.0, 0.0]);
    let mut l = Tool::line(APERTURE);
    click(&mut l, &mut s, [28.0, 26.0, 0.0]);
    hover(&mut l, &mut s, [22.0, 21.5, 0.0]);
    hover(&mut l, &mut s, [24.0, 23.0, 0.0]);
    assert_eq!(snap_kind(&l), Some(SnapKind::Endpoint));
}

proptest! {
    /// Within the angle the line comes out exactly parallel or square, and only
    /// as long as the pointer's reach along it.
    #[test]
    fn inferred_lines_are_exact(
        degrees in 20.0f64..70.0,
        off in -4.0f64..4.0,
        square in any::<bool>(),
        reach in 3.0f64..30.0,
    ) {
        let mut s = scene();
        let t = degrees.to_radians();
        let u = [t.cos(), t.sin()];
        free_line(&mut s, [20.0, 20.0, 0.0], [20.0 + 4.0 * u[0], 20.0 + 4.0 * u[1], 0.0]);

        let mut l = Tool::line(APERTURE);
        let a = [60.0, 60.0, 0.0];
        click(&mut l, &mut s, a);
        hover(&mut l, &mut s, [20.0 + 2.0 * u[0], 20.0 + 2.0 * u[1], 0.0]);

        let aim = t + if square { -90f64.to_radians() } else { 0.0 } + off.to_radians();
        let at = [a[0] + reach * aim.cos(), a[1] + reach * aim.sin(), 0.0];
        hover(&mut l, &mut s, at);
        // The pointer may fall on the axes, which then rightly win.
        prop_assume!(matches!(
            snap_kind(&l),
            Some(SnapKind::Parallel | SnapKind::Perpendicular)
        ));
        click(&mut l, &mut s, at);

        let start = Point3::new(a[0], a[1], a[2]);
        let (p, q) = segments(&s)
            .into_iter()
            .find(|(p, q)| (*p - start).length() <= MM || (*q - start).length() <= MM)
            .expect("the line was drawn");
        let span = q - p;
        let along = Vec3::new(u[0], u[1], 0.0);
        let want = if square { Vec3::new(u[1], -u[0], 0.0) } else { along };
        prop_assert!(span.cross(want).length() <= MM, "{span:?} against {want:?}");
        prop_assert!((span.length() - reach * off.to_radians().cos()).abs() <= MM);
    }
}

/// A turned component instance gives its edge's direction as shown, not as defined.
#[test]
fn a_line_runs_parallel_to_an_edge_of_a_turned_instance() {
    let mut s = scene();
    box_at_10(&mut s);
    let (key, doc) = s.project().tiles.iter().next().unwrap();
    let (key, nodes) = (*key, doc.top_level_nodes());
    s.make_component(key, &nodes).unwrap();
    let mut q = Tool::rotate(APERTURE);
    click(&mut q, &mut s, [10.0, 10.0, 0.0]);
    click(&mut q, &mut s, [15.0, 10.0, 0.0]);
    typed(&mut q, &mut s, "30d").unwrap();

    let (sin, cos) = 30f64.to_radians().sin_cos();
    let mut l = Tool::line(APERTURE);
    click(&mut l, &mut s, [40.0, 30.0, 0.0]);
    hover(&mut l, &mut s, [10.0 + 2.5 * cos, 10.0 + 2.5 * sin, 0.0]);
    assert_eq!(snap_kind(&l), Some(SnapKind::Midpoint), "over the edge");
    hover(
        &mut l,
        &mut s,
        [40.0 + 10.0 * cos, 30.0 + 10.0 * sin + 0.1, 0.0],
    );
    assert_eq!(snap_kind(&l), Some(SnapKind::Parallel));
    let dir = l.preview().snap.unwrap().snap.direction.unwrap();
    assert!(
        dir.cross(Vec3::new(cos, sin, 0.0)).length() <= 1e-9,
        "{dir:?}"
    );
}

fn has_box(s: &Scene, lo: [f64; 3], hi: [f64; 3]) {
    let near = |a: [f64; 3], b: [f64; 3]| (0..3).all(|k| (a[k] - b[k]).abs() <= MM);
    let all = bounds(s);
    assert!(
        all.iter().any(|(l, h)| near(*l, lo) && near(*h, hi)),
        "no box {lo:?} to {hi:?} in {all:?}"
    );
}

/// With Ctrl the face leaves as a new solid, and the old one does not change.
#[test]
fn push_pull_keeping_the_solid() {
    let mut s = scene();
    box_at_10(&mut s);
    let saved = s.project().save();
    let mut p = Tool::push_pull(APERTURE / 10.0);
    p.handle(&mut s, Input::ToggleCopy).unwrap();
    assert!(p.keeping());

    click(&mut p, &mut s, [12.5, 12.0, 3.0]);
    typed(&mut p, &mut s, "2").unwrap();
    let boxes = bounds(&s);
    assert_eq!(boxes.len(), 2, "{boxes:?}");
    has_box(&s, [10.0, 10.0, 0.0], [15.0, 14.0, 3.0]);
    has_box(&s, [10.0, 10.0, 3.0], [15.0, 14.0, 5.0]);
    assert!(p.keeping(), "stays on");
    s.undo().unwrap();
    assert_eq!(s.project().save(), saved, "one undo step");

    // Into the solid works too: they may overlap.
    click(&mut p, &mut s, [12.5, 12.0, 3.0]);
    typed(&mut p, &mut s, "-1").unwrap();
    has_box(&s, [10.0, 10.0, 2.0], [15.0, 14.0, 3.0]);
    s.undo().unwrap();

    // Off again: the ordinary push.
    p.handle(&mut s, Input::ToggleCopy).unwrap();
    click(&mut p, &mut s, [12.5, 12.0, 3.0]);
    typed(&mut p, &mut s, "1").unwrap();
    assert_box(&s, [10.0, 10.0, 0.0], [15.0, 14.0, 4.0]);
}

/// A sketch region already becomes a new solid: asking for one more is refused.
#[test]
fn push_pull_keeping_refuses_a_region_and_leaves_it() {
    let mut s = scene();
    typed_rectangle(&mut s);
    let saved = s.project().save();
    let mut p = Tool::push_pull(APERTURE);
    p.handle(&mut s, Input::ToggleCopy).unwrap();
    click(&mut p, &mut s, [12.5, 12.0, 0.0]);
    assert!(typed(&mut p, &mut s, "3").is_err());
    assert!(p.busy(), "the gesture stays open");
    assert_eq!(s.project().save(), saved);
}

proptest! {
    /// Any face, any way, any distance: both solids stay sound, the old one is
    /// untouched, and undo gives the same file back.
    #[test]
    fn keeping_the_solid_keeps_it(
        w in 1.0f64..20.0,
        d in 1.0f64..20.0,
        h in 1.0f64..10.0,
        side in any::<bool>(),
        distance in prop_oneof![-5.0f64..-0.01, 0.01f64..5.0],
    ) {
        let mut s = scene();
        let mut r = Tool::rectangle(APERTURE);
        click(&mut r, &mut s, [10.0, 10.0, 0.0]);
        hover(&mut r, &mut s, [11.0, 11.0, 0.0]);
        typed(&mut r, &mut s, &format!("{w};{d}")).unwrap();
        let mut p = Tool::push_pull(APERTURE / 100.0);
        click(&mut p, &mut s, [10.0 + w / 2.0, 10.0 + d / 2.0, 0.0]);
        typed(&mut p, &mut s, &format!("{h}")).unwrap();
        let before = bounds(&s);
        let saved = s.project().save();

        p.handle(&mut s, Input::ToggleCopy).unwrap();
        // The top, or the wall facing the test's camera.
        let at = if side {
            [10.0 + w / 2.0, 10.0, h / 2.0]
        } else {
            [10.0 + w / 2.0, 10.0 + d / 2.0, h]
        };
        click(&mut p, &mut s, at);
        prop_assert!(p.busy());
        typed(&mut p, &mut s, &format!("{distance}")).unwrap();

        let after = bounds(&s);
        prop_assert_eq!(after.len(), 2);
        prop_assert!(after.contains(&before[0]), "{:?} lost {:?}", after, before[0]);
        prop_assert!(s.project().tiles.values().all(|d| d.health_report(None).rows.is_empty()));
        s.undo().unwrap();
        prop_assert_eq!(s.project().save(), saved);
    }
}

fn paint_of(r: u8, g: u8, b: u8) -> Paint {
    Paint {
        name: format!("{r}-{g}-{b}"),
        color: Rgba8::rgb(r, g, b),
    }
}

/// Every painted face's color, with its normal, in every tile.
fn painted(s: &Scene) -> Vec<([f64; 3], Rgba8)> {
    let mut out = Vec::new();
    for doc in s.project().tiles.values() {
        for o in doc.visible_object_ids() {
            for (f, face) in doc.object(o).unwrap().faces().iter() {
                if let Some(m) = doc.face_material(o, f) {
                    let n = face.plane.normal();
                    out.push(([n.x, n.y, n.z], doc.material(m).unwrap().color));
                }
            }
        }
    }
    out
}

/// One click paints one face, in one undo step, and changes no geometry.
#[test]
fn paint_a_face_and_take_it_back() {
    let mut s = scene();
    box_at_10(&mut s);
    let shape = bounds(&s);
    let red = Rgba8::rgb(200, 30, 30);
    let mut b = Tool::paint(APERTURE);
    b.set_paint(Some(paint_of(200, 30, 30)));

    click(&mut b, &mut s, [12.5, 12.0, 3.0]);
    assert_eq!(painted(&s), vec![([0.0, 0.0, 1.0], red)]);
    assert_eq!(bounds(&s), shape);

    // Saves and loads as it is.
    let saved = s.project().save();
    let again = Project::open(&saved).unwrap();
    assert_eq!(again.save(), saved, "byte-identical");
    assert_eq!(painted(&Scene::new(again)), vec![([0.0, 0.0, 1.0], red)]);

    // The same color again is no step: this undo takes off the first click.
    click(&mut b, &mut s, [12.5, 12.0, 3.0]);
    s.undo().unwrap();
    assert!(painted(&s).is_empty());
    s.redo().unwrap();
    assert_eq!(painted(&s).len(), 1);

    // A second face shares the material, and None takes paint off.
    click(&mut b, &mut s, [12.5, 10.0, 1.5]);
    assert_eq!(painted(&s).len(), 2);
    let doc = s.project().tiles.values().next().unwrap();
    assert_eq!(doc.material_ids().len(), 1, "one color, one material");
    b.set_paint(None);
    b.reset();
    click(&mut b, &mut s, [12.5, 12.0, 3.0]);
    assert_eq!(painted(&s).len(), 1);
}

/// What cannot take paint says so, or does nothing, and leaves the file alone.
#[test]
fn paint_refuses_what_is_no_face_of_a_solid() {
    let mut s = scene();
    typed_rectangle(&mut s);
    let mut b = Tool::paint(APERTURE);
    b.set_paint(Some(paint_of(1, 2, 3)));
    let saved = s.project().save();
    assert!(
        b.handle(&mut s, Input::Click(ray([12.5, 12.0, 0.0])))
            .is_err(),
        "a sketch region"
    );
    click(&mut b, &mut s, [100.0, 100.0, 0.0]);
    assert!(typed(&mut b, &mut s, "3").is_err());
    assert_eq!(s.project().save(), saved);

    let mut s = scene();
    box_at_10(&mut s);
    let (key, doc) = s.project().tiles.iter().next().unwrap();
    let (key, nodes) = (*key, doc.top_level_nodes());
    s.make_component(key, &nodes).unwrap();
    let saved = s.project().save();
    assert!(
        b.handle(&mut s, Input::Click(ray([12.5, 12.0, 3.0])))
            .is_err(),
        "inside a component"
    );
    assert_eq!(s.project().save(), saved);
}

proptest! {
    /// Clicks anywhere with any colors: geometry and soundness hold, and as many
    /// undos as steps leave no paint.
    #[test]
    fn paint_never_changes_the_shape(
        clicks in proptest::collection::vec(
            ((5.0f64..20.0, 5.0f64..20.0, 0.0f64..4.0), any::<(u8, u8, u8)>()),
            1..8,
        ),
    ) {
        let mut s = scene();
        box_at_10(&mut s);
        let shape = bounds(&s);
        let mut b = Tool::paint(APERTURE);
        for ((x, y, z), (r, g, bl)) in clicks {
            b.set_paint(Some(paint_of(r, g, bl)));
            click(&mut b, &mut s, [x, y, z]);
            prop_assert_eq!(bounds(&s), shape.clone());
            prop_assert!(s.project().tiles.values().all(|d| d.health_report(None).rows.is_empty()));
        }
        let reload = Project::open(&s.project().save()).unwrap();
        prop_assert_eq!(bounds(&Scene::new(reload)), shape.clone());
        while !painted(&s).is_empty() {
            s.undo().unwrap();
        }
        prop_assert_eq!(bounds(&s), shape);
    }
}

/// Three boxes side by side: the first two grouped, the third free.
fn group_and_a_box(s: &mut Scene) -> (wangungis_scene::project::TileKey, NodeId) {
    for x in [10.0, 30.0, 50.0] {
        let mut r = Tool::rectangle(APERTURE);
        click(&mut r, s, [x, 10.0, 0.0]);
        hover(&mut r, s, [x + 1.0, 11.0, 0.0]);
        typed(&mut r, s, "5;4").unwrap();
        let mut p = Tool::push_pull(APERTURE);
        click(&mut p, s, [x + 2.5, 12.0, 0.0]);
        typed(&mut p, s, "3").unwrap();
    }
    let (key, doc) = s.project().tiles.iter().next().unwrap();
    let key = *key;
    let nodes = doc.top_level_nodes();
    assert_eq!(nodes.len(), 3);
    s.group(key, &nodes[..2]).unwrap();
    let doc = &s.project().tiles[&key];
    let group = doc
        .top_level_nodes()
        .into_iter()
        .find(|n| matches!(n, NodeId::Group(_)))
        .unwrap();
    (key, group)
}

/// Every tool that changes something, aimed at the box at x = 50 and its face.
fn try_every_tool_on_the_free_box(s: &mut Scene) {
    let top = [52.5, 12.0, 3.0];
    let edge = [55.0, 12.0, 3.0];
    let aperture = APERTURE / 10.0;
    let hit = |t: &mut Tool, s: &mut Scene, at: [f64; 3]| {
        let _ = t.handle(s, Input::Click(ray(at)));
    };

    let mut p = Tool::push_pull(aperture);
    hit(&mut p, s, top);
    let _ = typed(&mut p, s, "1");
    let mut k = Tool::push_pull(aperture);
    k.handle(s, Input::ToggleCopy).unwrap();
    hit(&mut k, s, top);
    let _ = typed(&mut k, s, "1");
    let mut b = Tool::paint(aperture);
    b.set_paint(Some(paint_of(9, 9, 9)));
    hit(&mut b, s, top);
    let mut e = Tool::eraser(APERTURE);
    hit(&mut e, s, edge);
    let mut m = Tool::move_tool(APERTURE);
    hit(&mut m, s, edge);
    hit(&mut m, s, [70.0, 12.0, 3.0]);
    let mut q = Tool::rotate(APERTURE);
    hit(&mut q, s, edge);
    hit(&mut q, s, [60.0, 12.0, 3.0]);
    let _ = typed(&mut q, s, "45");
    let mut c = Tool::scale(APERTURE);
    hit(&mut c, s, edge);
    let _ = typed(&mut c, s, "2");
    let mut o = Tool::offset(aperture);
    hit(&mut o, s, top);
    let _ = typed(&mut o, s, "0.5");
    let mut f = Tool::follow_me(aperture);
    hit(&mut f, s, top);
    // A line drawn across the face must not split it.
    let mut l = Tool::line(aperture);
    hit(&mut l, s, [51.0, 11.0, 3.0]);
    hit(&mut l, s, [54.0, 13.0, 3.0]);
    let _ = l.handle(s, Input::Cancel);
}

/// The free box as (vertices, faces, painted faces), whatever else happens.
fn free_box_state(s: &Scene) -> Vec<(usize, usize, usize, [f64; 2])> {
    let mut out = Vec::new();
    for (key, doc) in &s.project().tiles {
        let c = key.center();
        for o in doc.visible_object_ids() {
            let object = doc.object(o).unwrap();
            let x: Vec<f64> = object
                .vertices()
                .values()
                .map(|v| v.position.x + c[0])
                .collect();
            if x.iter().all(|x| *x >= 45.0) {
                let hi = x.iter().copied().fold(f64::MIN, f64::max);
                let top = object
                    .vertices()
                    .values()
                    .map(|v| v.position.z)
                    .fold(f64::MIN, f64::max);
                let paint = object
                    .faces()
                    .keys()
                    .filter(|f| doc.face_material(o, *f).is_some())
                    .count();
                out.push((
                    object.vertices().len(),
                    object.faces().len(),
                    paint,
                    [hi, top],
                ));
            }
        }
    }
    out
}

/// Inside a group its members take edits, and nothing outside does, by any tool.
#[test]
fn only_what_is_open_can_be_edited() {
    let mut s = scene();
    let (key, group) = group_and_a_box(&mut s);
    let free = free_box_state(&s);
    assert_eq!(free.len(), 1);

    // Closed: its members are out of reach too.
    let mut p = Tool::push_pull(APERTURE / 10.0);
    click(&mut p, &mut s, [12.5, 12.0, 3.0]);
    assert!(!p.busy(), "a member of a closed group");
    let mut b = Tool::paint(APERTURE / 10.0);
    b.set_paint(Some(paint_of(9, 9, 9)));
    assert_eq!(
        b.handle(&mut s, Input::Click(ray([12.5, 12.0, 3.0]))),
        Err(EditError::OutOfScope)
    );

    s.enter(key, group).unwrap();
    assert_eq!(s.session(), Some((key, vec![group])));
    try_every_tool_on_the_free_box(&mut s);
    assert_eq!(free_box_state(&s), free, "the box outside did not change");
    assert!(
        s.project()
            .tiles
            .values()
            .all(|d| d.health_report(None).rows.is_empty())
    );

    // A member takes edits.
    let mut p = Tool::push_pull(APERTURE / 10.0);
    click(&mut p, &mut s, [12.5, 12.0, 3.0]);
    assert!(p.busy());
    typed(&mut p, &mut s, "1").unwrap();
    has_box(&s, [10.0, 10.0, 0.0], [15.0, 14.0, 4.0]);

    assert!(s.leave().unwrap());
    assert_eq!(s.session(), None);
    assert!(!s.leave().unwrap(), "nothing left to leave");
    let doc = &s.project().tiles[&key];
    assert_eq!(doc.top_level_nodes().len(), 2, "the group came back");
    assert_eq!(free_box_state(&s), free);
    assert!(doc.health_report(None).rows.is_empty());
}

/// Undo walks back through closing and opening; the session follows.
#[test]
fn undo_goes_through_a_session() {
    let mut s = scene();
    let (key, group) = group_and_a_box(&mut s);
    let saved = s.project().save();
    s.enter(key, group).unwrap();
    let mut p = Tool::push_pull(APERTURE / 10.0);
    click(&mut p, &mut s, [12.5, 12.0, 3.0]);
    typed(&mut p, &mut s, "1").unwrap();
    s.leave().unwrap();

    s.undo().unwrap();
    assert_eq!(s.session().map(|o| o.1), Some(vec![group]), "open again");
    s.undo().unwrap();
    has_box(&s, [10.0, 10.0, 0.0], [15.0, 14.0, 3.0]);
    s.undo().unwrap();
    assert_eq!(s.session(), None);
    assert_eq!(s.project().save(), saved);

    for _ in 0..3 {
        s.redo().unwrap();
    }
    assert_eq!(s.session(), None);
    has_box(&s, [10.0, 10.0, 0.0], [15.0, 14.0, 4.0]);
}

/// A file never holds an open session: saving one writes it as if closed.
#[test]
fn saving_inside_a_session_saves_it_closed() {
    let mut s = scene();
    let (key, group) = group_and_a_box(&mut s);
    s.enter(key, group).unwrap();
    let mut p = Tool::push_pull(APERTURE / 10.0);
    click(&mut p, &mut s, [12.5, 12.0, 3.0]);
    typed(&mut p, &mut s, "1").unwrap();

    let open = s.project().save();
    assert_eq!(s.project().save(), open, "byte-stable");
    assert!(s.session().is_some(), "saving leaves it open");
    let reopened = Project::open(&open).unwrap();
    assert_eq!(reopened.save(), open, "byte-identical again");
    let doc = reopened.tiles.values().next().unwrap();
    assert!(doc.session_stack().is_empty());
    assert_eq!(doc.top_level_nodes().len(), 2, "the group is back");
    let reopened = Scene::new(reopened);
    has_box(&reopened, [10.0, 10.0, 0.0], [15.0, 14.0, 4.0]);
}

/// Inside a component an edit reaches every instance of it.
#[test]
fn editing_a_component_changes_its_instances() {
    let mut s = scene();
    box_at_10(&mut s);
    let (key, doc) = s.project().tiles.iter().next().unwrap();
    let (key, nodes) = (*key, doc.top_level_nodes());
    s.make_component(key, &nodes).unwrap();
    let mut m = Tool::move_tool(APERTURE);
    click(&mut m, &mut s, [15.0, 12.0, 3.0]);
    m.handle(&mut s, Input::ToggleCopy).unwrap();
    hover(&mut m, &mut s, [35.0, 12.0, 3.0]);
    m.handle(&mut s, Input::LockAxis(Axis::X)).unwrap();
    typed(&mut m, &mut s, "20").unwrap();
    let doc = &s.project().tiles[&key];
    let tops = doc.top_level_nodes();
    assert_eq!(tops.len(), 2);

    s.enter(key, tops[0]).unwrap();
    let mut p = Tool::push_pull(APERTURE / 10.0);
    click(&mut p, &mut s, [12.5, 12.0, 3.0]);
    assert!(p.busy(), "the member is editable now");
    typed(&mut p, &mut s, "1").unwrap();
    while s.leave().unwrap() {}

    let doc = &s.project().tiles[&key];
    let heights: Vec<f64> = doc
        .top_level_nodes()
        .into_iter()
        .map(|n| {
            wangungis_scene::project::node_points(doc, n)
                .iter()
                .map(|p| p.z)
                .fold(f64::MIN, f64::max)
        })
        .collect();
    assert_eq!(heights.len(), 2);
    assert!(heights.iter().all(|h| (h - 4.0).abs() < MM), "{heights:?}");
    assert!(doc.health_report(None).rows.is_empty());
}

/// A stretched instance cannot be opened, and what comes back says why.
#[test]
fn a_stretched_instance_stays_closed() {
    let mut s = scene();
    box_at_10(&mut s);
    let (key, doc) = s.project().tiles.iter().next().unwrap();
    let (key, nodes) = (*key, doc.top_level_nodes());
    s.make_component(key, &nodes).unwrap();
    let node = s.project().tiles[&key].top_level_nodes()[0];
    let stretch = kernel::Transform::scale(Vec3::new(2.0, 1.0, 1.0));
    s.transform(
        &wangungis_tools::Movable::Node { tile: key, node },
        &stretch,
    )
    .unwrap();
    assert!(s.enter(key, node).is_err());
    assert_eq!(s.session(), None);
}

/// Groups inside groups open one at a time and close the same way.
#[test]
fn nested_groups_open_in_turn() {
    let mut s = scene();
    let (key, inner) = group_and_a_box(&mut s);
    let doc = &s.project().tiles[&key];
    let tops = doc.top_level_nodes();
    s.group(key, &tops).unwrap();
    let outer = s.project().tiles[&key].top_level_nodes()[0];

    assert_eq!(
        s.enter(key, inner),
        Err(EditError::OutOfScope),
        "outer first"
    );
    s.enter(key, outer).unwrap();
    s.enter(key, inner).unwrap();
    assert_eq!(s.session(), Some((key, vec![outer, inner])));
    let mut p = Tool::push_pull(APERTURE / 10.0);
    click(&mut p, &mut s, [12.5, 12.0, 3.0]);
    typed(&mut p, &mut s, "1").unwrap();
    assert!(s.leave().unwrap());
    assert_eq!(s.session(), Some((key, vec![outer])));
    assert!(s.leave().unwrap());
    assert_eq!(s.session(), None);
    has_box(&s, [10.0, 10.0, 0.0], [15.0, 14.0, 4.0]);
    assert_eq!(s.project().tiles[&key].top_level_nodes(), vec![outer]);
}

/// While one tile holds the session, no other tile can be edited.
#[test]
fn a_session_keeps_edits_in_its_tile() {
    let mut s = scene();
    let (key, group) = group_and_a_box(&mut s);
    s.enter(key, group).unwrap();
    let saved = s.project().save();
    let mut r = Tool::rectangle(APERTURE);
    click(&mut r, &mut s, [3_000.0, 10.0, 0.0]);
    hover(&mut r, &mut s, [3_001.0, 11.0, 0.0]);
    assert_eq!(typed(&mut r, &mut s, "5;4"), Err(EditError::OutOfScope));
    assert_eq!(s.project().save(), saved);
}

/// A drawing outside the session stays outside it.
#[test]
fn a_drawing_outside_the_session_is_refused() {
    let mut s = scene();
    let (key, group) = group_and_a_box(&mut s);
    typed_rectangle_at(&mut s, [70.0, 10.0, 0.0]);
    s.enter(key, group).unwrap();
    let saved = s.project().save();
    let mut p = Tool::push_pull(APERTURE);
    let _ = p.handle(&mut s, Input::Click(ray([72.0, 12.0, 0.0])));
    let _ = typed(&mut p, &mut s, "3");
    let mut m = Tool::move_tool(APERTURE);
    let _ = m.handle(&mut s, Input::Click(ray([70.0, 12.0, 0.0])));
    let _ = m.handle(&mut s, Input::Click(ray([90.0, 12.0, 0.0])));
    let mut e = Tool::eraser(APERTURE);
    let _ = e.handle(&mut s, Input::Click(ray([70.0, 12.0, 0.0])));
    assert_eq!(s.project().save(), saved);
}

/// The Scene's own methods hold the line too, whatever a caller hands them.
#[test]
fn every_scene_method_refuses_what_is_out_of_scope() {
    use wangungis_tools::{Erasable, Movable};
    let mut s = scene();
    let (key, group) = group_and_a_box(&mut s);
    let out = s.project().tiles[&key]
        .top_level_nodes()
        .into_iter()
        .find(|n| matches!(n, NodeId::Object(_)))
        .unwrap();
    let mut guide = Tool::tape(APERTURE);
    click(&mut guide, &mut s, [100.0, 100.0, 0.0]);
    hover(&mut guide, &mut s, [100.0, 105.0, 0.0]);
    typed(&mut guide, &mut s, "1").unwrap();
    s.enter(key, group).unwrap();
    let saved = s.project().save();
    let m = Movable::Node {
        tile: key,
        node: out,
    };
    let shift = kernel::Transform::translation(Vec3::new(5.0, 0.0, 0.0));
    let count = std::num::NonZeroU32::new(2).unwrap();
    let no = Err(EditError::OutOfScope);
    assert_eq!(s.group(key, &[out]), no);
    assert_eq!(s.make_component(key, &[out]), no);
    assert_eq!(s.dissolve(key, &[out]), no);
    assert_eq!(s.delete_nodes(key, &[out]), no);
    assert_eq!(s.transform(&m, &shift), no);
    assert_eq!(s.copy(&m, &shift, count), no);
    assert_eq!(
        s.erase(&Erasable::Node {
            tile: key,
            node: out
        }),
        no
    );
    assert_eq!(s.add_guide_point(Point3::new(3_000.0, 0.0, 0.0)), no);
    assert_eq!(
        s.add_guide_line(Point3::new(3_000.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
        no
    );
    assert_eq!(s.delete_guides(), no);
    assert_eq!(s.project().save(), saved);
}

/// A drawing owned by a solid goes with that solid: out of reach while its group
/// is closed, editable inside.
#[test]
fn a_solids_own_drawing_follows_the_solid() {
    let mut doc = kernel::Document::new();
    let p = |x: f64, y: f64| Point3::new(x, y, 0.0);
    let ring = [p(0.0, 0.0), p(5.0, 0.0), p(5.0, 4.0), p(0.0, 4.0)];
    let plane = kernel::Plane::from_polygon(&ring).unwrap();
    let base = doc.add_sketch(plane);
    let sk = doc.sketch_mut(base).unwrap();
    for k in 0..4 {
        sk.add_segment(ring[k], ring[(k + 1) % 4]).unwrap();
    }
    let region = doc.extrudable_regions(base).unwrap()[0];
    let (solid, _) = doc.extrude_region(base, region, 3.0).unwrap();
    // On the top face: an owned drawing must lie on a face of its solid.
    let top =
        kernel::Plane::from_point_normal(Point3::new(0.0, 0.0, 3.0), Vec3::new(0.0, 0.0, 1.0));
    let owned = doc.add_sketch(top.unwrap());
    doc.set_sketch_object_owner(owned, Some(solid)).unwrap();
    let (group, _) = doc.group_nodes(&[NodeId::Object(solid)]).unwrap();

    let mut s = Scene::new(Project::from_document(doc).unwrap());
    let key = *s.project().tiles.keys().next().unwrap();
    assert!(!s.sketch_editable(key, owned), "the group is closed");
    s.enter(key, NodeId::Group(group)).unwrap();
    assert!(s.sketch_editable(key, owned), "the group is open");
}

/// A line drawn inside goes with the group, and stays with it afterwards.
#[test]
fn a_drawing_made_inside_stays_inside() {
    let mut s = scene();
    let (key, group) = group_and_a_box(&mut s);
    s.enter(key, group).unwrap();

    let mut l = Tool::line(APERTURE);
    click(&mut l, &mut s, [90.0, 10.0, 0.0]);
    click(&mut l, &mut s, [95.0, 13.0, 0.0]);
    l.handle(&mut s, Input::Cancel).unwrap();

    let mut m = Tool::move_tool(APERTURE);
    click(&mut m, &mut s, [92.5, 11.5, 0.0]);
    assert!(m.busy(), "its own drawing moves");
    m.handle(&mut s, Input::Cancel).unwrap();

    s.leave().unwrap();
    let inside = |s: &Scene| {
        let doc = &s.project().tiles[&key];
        let NodeId::Group(g) = group else {
            unreachable!()
        };
        doc.group_member_sketches(g)
            .iter()
            .map(|id| doc.sketch(*id).unwrap().edges().len())
            .sum::<usize>()
    };
    assert_eq!(inside(&s), 1, "the line joined the group");

    // Closed again, it is out of reach. Hew would pull a new line on that plane
    // into the group's drawing, so that is refused.
    click(&mut m, &mut s, [92.5, 11.5, 0.0]);
    assert!(!m.busy(), "out of reach");
    m.handle(&mut s, Input::Cancel).unwrap();
    let mut l = Tool::line(APERTURE);
    click(&mut l, &mut s, [90.0, 30.0, 0.0]);
    let refused = l.handle(&mut s, Input::Click(ray([95.0, 33.0, 0.0])));
    assert!(
        matches!(refused, Err(EditError::Unsupported(_))),
        "{refused:?}"
    );
    assert_eq!(inside(&s), 1);
}

/// Hew would merge a line drawn inside into a drawing outside on the same plane,
/// so it is refused, and that drawing stays as it was.
#[test]
fn a_plane_with_drawing_outside_is_not_drawn_on_inside() {
    let mut s = scene();
    let (key, group) = group_and_a_box(&mut s);
    typed_rectangle_at(&mut s, [70.0, 10.0, 0.0]);
    s.enter(key, group).unwrap();
    let saved = s.project().save();

    let mut l = Tool::line(APERTURE);
    click(&mut l, &mut s, [90.0, 10.0, 0.0]);
    let refused = l.handle(&mut s, Input::Click(ray([95.0, 13.0, 0.0])));
    assert!(
        matches!(refused, Err(EditError::Unsupported(_))),
        "{refused:?}"
    );
    assert_eq!(s.project().save(), saved);

    let mut m = Tool::move_tool(APERTURE);
    click(&mut m, &mut s, [70.0, 12.0, 0.0]);
    assert!(!m.busy(), "the drawing outside does not move");
}

fn typed_rectangle_at(s: &mut Scene, at: [f64; 3]) {
    let mut r = Tool::rectangle(APERTURE);
    click(&mut r, s, at);
    hover(&mut r, s, [at[0] + 1.0, at[1] + 1.0, at[2]]);
    typed(&mut r, s, "5;4").unwrap();
}

proptest! {
    /// Any order of opening, editing, closing and undoing: solids stay sound, the
    /// box outside never changes, and a save reads back without a session.
    #[test]
    fn sessions_keep_everything_sound(steps in proptest::collection::vec(0u8..5, 1..14)) {
        let mut s = scene();
        let (key, group) = group_and_a_box(&mut s);
        let mut free = free_box_state(&s);
        for step in steps {
            match step {
                0 => { let _ = s.enter(key, group); }
                1 => { let _ = s.leave(); }
                2 => {
                    let mut p = Tool::push_pull(APERTURE / 10.0);
                    let _ = p.handle(&mut s, Input::Click(ray([12.5, 12.0, 3.0])));
                    if p.busy() {
                        let _ = typed(&mut p, &mut s, "0.5");
                    }
                }
                3 => { let _ = s.undo(); }
                _ => try_every_tool_on_the_free_box(&mut s),
            }
            prop_assert!(s.project().tiles.values().all(|d| d.health_report(None).rows.is_empty()));
            // With nothing open the tools may reach it; with a session, none may.
            if s.session().is_some() {
                prop_assert_eq!(free_box_state(&s), free.clone());
            } else {
                free = free_box_state(&s);
            }
            let file = s.project().save();
            let again = Project::open(&file).unwrap();
            prop_assert!(again.tiles.values().all(|d| d.session_stack().is_empty()));
            prop_assert_eq!(again.save(), file);
        }
    }
}
