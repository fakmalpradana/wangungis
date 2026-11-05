//! Drawing on a point cloud: snapping to its points, pulling up to them, and the
//! box that hides what is outside it.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use inference::{Axis, PickRay, SnapKind};
use kernel::{Point3, Vec3};
use las::point::Format;
use las::{Builder, Color, Point, Writer};
use wangungis_pointcloud::{CloudFile, Progress, Slice, View, import_las};
use wangungis_render::Camera;
use wangungis_scene::project::Project;
use wangungis_tools::Scene;
use wangungis_tools::tool::{Input, Tool};

const APERTURE: f64 = 0.01;

/// Writes a LAS holding `points`, and caches it.
fn cached(name: &str, points: &[[f64; 3]]) -> (PathBuf, CloudFile) {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("wangungis-cloud-{name}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut b = Builder::from((1, 2));
    b.point_format = Format::new(2).unwrap();
    for t in [
        &mut b.transforms.x,
        &mut b.transforms.y,
        &mut b.transforms.z,
    ] {
        t.scale = 0.001;
    }
    let source = dir.join("a.las");
    let mut w = Writer::from_path(&source, b.into_header().unwrap()).unwrap();
    for p in points {
        w.write_point(Point {
            x: p[0],
            y: p[1],
            z: p[2],
            color: Some(Color::new(0, 65535, 0)),
            ..Default::default()
        })
        .unwrap();
    }
    w.close().unwrap();
    let cache = dir.join("a.wpc");
    import_las(&source, &cache, &Progress::default()).unwrap();
    (source, CloudFile::open(&cache).unwrap())
}

/// Reads everything in, seen from above.
fn settle(s: &mut Scene) {
    let eye = [50.0, 50.0, 400.0];
    let cam = Camera {
        eye,
        target: [50.0, 50.0, 0.0],
        fovy: 0.8,
        aspect: 1.0,
        near: 0.5,
        far: 10_000.0,
    };
    let view = View {
        eye,
        view_proj: cam.view_proj_at_eye(),
        // Dense enough that every point of these small clouds is drawn.
        focal_px: 1.0e5,
        scale: 1.0,
    };
    let start = Instant::now();
    loop {
        s.clouds.poll();
        s.clouds.plan(&view);
        s.clouds.take_changed(|_| {});
        if !s.clouds.busy() {
            return;
        }
        assert!(start.elapsed() < Duration::from_secs(30));
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn scene_with(name: &str, points: &[[f64; 3]]) -> Scene {
    let mut s = Scene::new(Project::new(None).unwrap());
    let (source, file) = cached(name, points);
    s.clouds.add(file, source);
    settle(&mut s);
    s
}

/// A ray from straight above.
fn down(x: f64, y: f64) -> PickRay {
    PickRay {
        origin: Point3::new(x, y, 300.0),
        direction: Vec3::new(0.0, 0.0, -1.0),
    }
}

/// Like the tools' tests: a slanted view, aimed at `at`.
fn ray(at: [f64; 3]) -> PickRay {
    let origin = Point3::new(at[0] - 15.0, at[1] - 25.0, at[2] + 40.0);
    PickRay {
        origin,
        direction: Point3::new(at[0], at[1], at[2]) - origin,
    }
}

fn top_of(s: &Scene) -> f64 {
    s.project()
        .tiles
        .values()
        .flat_map(|d| {
            d.visible_object_ids()
                .into_iter()
                .flat_map(|id| {
                    d.object(id)
                        .unwrap()
                        .vertices()
                        .values()
                        .map(|v| v.position.z)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .fold(f64::MIN, f64::max)
}

/// A rectangle started on a roof point lies at the roof's height, and the hit says
/// it came from the cloud.
#[test]
fn a_drawing_starts_on_a_point() {
    let mut s = scene_with(
        "start",
        &[[50.0, 50.0, 12.5], [20.0, 20.0, 0.0], [80.0, 80.0, 0.0]],
    );
    let mut r = Tool::rectangle(APERTURE);
    r.handle(&mut s, Input::Hover(down(50.01, 50.01))).unwrap();
    let hit = r.preview().snap.expect("the point");
    assert!(hit.cloud.is_some());
    assert_eq!(hit.snap.kind, SnapKind::Endpoint);
    assert!((hit.snap.position.z - 12.5).abs() < 0.01);

    r.handle(&mut s, Input::Click(down(50.0, 50.0))).unwrap();
    assert!(r.busy());
    r.handle(&mut s, Input::Hover(down(53.0, 53.0))).unwrap();
    r.handle(&mut s, Input::Typed("5;4".into())).unwrap();
    assert!(!r.busy());
    let mut p = Tool::push_pull(APERTURE);
    p.handle(&mut s, Input::Click(down(52.0, 52.0))).unwrap();
    p.handle(&mut s, Input::Typed("3".into())).unwrap();
    assert!((top_of(&s) - 15.5).abs() < 0.01, "{}", top_of(&s));
}

/// The main job: pull a footprint up until it reaches a roof point.
#[test]
fn push_pull_reaches_a_roof_point() {
    let roof = [60.0, 60.0, 17.25];
    let mut s = scene_with("roof", &[roof, [0.0, 0.0, 0.0], [100.0, 100.0, 0.0]]);
    let mut r = Tool::rectangle(APERTURE / 10.0);
    r.handle(&mut s, Input::Click(down(10.0, 10.0))).unwrap();
    r.handle(&mut s, Input::Hover(down(12.0, 12.0))).unwrap();
    r.handle(&mut s, Input::Typed("5;4".into())).unwrap();

    let mut p = Tool::push_pull(APERTURE);
    p.handle(&mut s, Input::Click(down(12.0, 12.0))).unwrap();
    let at = ray(roof);
    p.handle(&mut s, Input::Hover(at)).unwrap();
    assert!(p.preview().snap.is_some_and(|h| h.cloud.is_some()));
    p.handle(&mut s, Input::Click(at)).unwrap();
    assert!((top_of(&s) - roof[2]).abs() < 0.01, "{}", top_of(&s));
}

/// Next to a model's corner the corner wins, even with a point nearer the ray.
#[test]
fn the_model_outranks_the_cloud() {
    let mut s = scene_with(
        "rank",
        &[[15.2, 10.0, 3.0], [0.0, 0.0, 0.0], [100.0, 100.0, 0.0]],
    );
    let mut r = Tool::rectangle(APERTURE);
    r.handle(&mut s, Input::Click(ray([10.0, 10.0, 0.0])))
        .unwrap();
    r.handle(&mut s, Input::Hover(ray([12.0, 12.0, 0.0])))
        .unwrap();
    r.handle(&mut s, Input::Typed("5;4".into())).unwrap();
    let mut p = Tool::push_pull(APERTURE);
    p.handle(&mut s, Input::Click(ray([12.5, 12.0, 0.0])))
        .unwrap();
    p.handle(&mut s, Input::Typed("3".into())).unwrap();

    let mut l = Tool::line(APERTURE);
    l.handle(&mut s, Input::Hover(ray([15.1, 10.0, 3.0])))
        .unwrap();
    let hit = l.preview().snap.expect("a snap");
    assert!(hit.cloud.is_none());
    assert_eq!(hit.snap.kind, SnapKind::Endpoint);
    assert!((hit.snap.position.x - 15.0).abs() < 1e-6);
}

/// A point behind a solid's face is hidden from the snap as it is from the eye.
#[test]
fn a_solid_hides_the_points_behind_it() {
    let mut s = scene_with(
        "hide",
        &[[12.0, 12.0, 1.0], [0.0, 0.0, 0.0], [100.0, 100.0, 0.0]],
    );
    let mut r = Tool::rectangle(APERTURE / 10.0);
    r.handle(&mut s, Input::Click(down(10.0, 10.0))).unwrap();
    r.handle(&mut s, Input::Hover(down(12.0, 12.0))).unwrap();
    r.handle(&mut s, Input::Typed("5;4".into())).unwrap();
    let mut p = Tool::push_pull(APERTURE / 10.0);
    p.handle(&mut s, Input::Click(down(12.5, 12.0))).unwrap();
    p.handle(&mut s, Input::Typed("3".into())).unwrap();

    let mut l = Tool::line(APERTURE);
    l.handle(&mut s, Input::Hover(down(12.0, 12.0))).unwrap();
    assert!(l.preview().snap.is_none_or(|h| h.cloud.is_none()));
}

/// On a drawing's plane a point lends only its place in the plane.
#[test]
fn a_point_lands_on_the_drawing_plane() {
    let mut s = scene_with(
        "plane",
        &[
            [20.0, 20.0, 5.0],
            [30.0, 25.0, 9.0],
            [0.0, 0.0, 0.0],
            [100.0, 100.0, 0.0],
        ],
    );
    let mut r = Tool::rectangle(APERTURE);
    r.handle(&mut s, Input::Click(down(20.0, 20.0))).unwrap();
    r.handle(&mut s, Input::Hover(down(30.0, 25.0))).unwrap();
    let hit = r.preview().snap.expect("the second point");
    assert!(hit.cloud.is_some());
    let at = hit.snap.position;
    assert!(
        (at.z - 5.0).abs() < 0.01,
        "on the plane of the first: {at:?}"
    );
    assert!(
        (at.x - 30.0).abs() < 0.01 && (at.y - 25.0).abs() < 0.01,
        "{at:?}"
    );
    assert!(
        hit.snap
            .projected_from
            .is_some_and(|p| (p.z - 9.0).abs() < 0.01)
    );
}

/// Under an axis lock a point says only how far along the axis.
#[test]
fn a_point_lands_on_a_locked_axis() {
    let mut s = scene_with(
        "lock",
        &[
            [20.0, 20.0, 5.0],
            [30.0, 25.0, 40.0],
            [0.0, 0.0, 0.0],
            [100.0, 100.0, 0.0],
        ],
    );
    let mut l = Tool::line(APERTURE);
    l.handle(&mut s, Input::Click(down(20.0, 20.0))).unwrap();
    l.handle(&mut s, Input::LockAxis(Axis::Z)).unwrap();
    let at = ray([30.0, 25.0, 40.0]);
    l.handle(&mut s, Input::Hover(at)).unwrap();
    let hit = l.preview().snap.expect("the point");
    assert!(hit.cloud.is_some());
    let p = hit.snap.position;
    assert!(
        (p.x - 20.0).abs() < 0.01 && (p.y - 20.0).abs() < 0.01,
        "{p:?}"
    );
    assert!((p.z - 40.0).abs() < 0.01, "{p:?}");
}

/// Snap off, or the cloud hidden: the points are no target.
#[test]
fn snap_to_points_can_be_turned_off() {
    let mut s = scene_with(
        "off",
        &[[50.0, 50.0, 12.5], [0.0, 0.0, 0.0], [100.0, 100.0, 0.0]],
    );
    let mut l = Tool::line(APERTURE);
    s.clouds.snap = false;
    l.handle(&mut s, Input::Hover(down(50.0, 50.0))).unwrap();
    assert!(l.preview().snap.is_none_or(|h| h.cloud.is_none()));
    s.clouds.snap = true;

    let id = s.clouds.clouds()[0].id;
    s.clouds.set_visible(id, false);
    l.handle(&mut s, Input::Hover(down(50.0, 50.0))).unwrap();
    assert!(l.preview().snap.is_none_or(|h| h.cloud.is_none()));
}

fn corners() -> Vec<[f64; 3]> {
    let mut out = vec![[0.0, 0.0, 0.0], [100.0, 100.0, 30.0]];
    for i in 0..20 {
        out.push([f64::from(i) * 5.0, f64::from(i) * 5.0, f64::from(i)]);
    }
    out
}

/// Two clicks make a box as tall as the cloud.
#[test]
fn two_clicks_make_a_box() {
    let mut s = scene_with("box", &corners());
    let mut t = Tool::cloud_slice(APERTURE / 100.0);
    assert_eq!(s.clouds.active_slice(), None);
    t.handle(&mut s, Input::Click(down(20.0, 20.5))).unwrap();
    assert!(t.busy());
    t.handle(&mut s, Input::Hover(down(60.0, 70.0))).unwrap();
    assert!(!t.preview().lines.is_empty(), "a rubber band");
    t.handle(&mut s, Input::Click(down(60.0, 70.0))).unwrap();
    assert!(!t.busy());

    let b = s.clouds.active_slice().expect("the box");
    assert!(
        (b.min[0] - 20.0).abs() < 1e-6 && (b.max[0] - 60.0).abs() < 1e-6,
        "{b:?}"
    );
    assert!(
        (b.min[1] - 20.5).abs() < 1e-6 && (b.max[1] - 70.0).abs() < 1e-6,
        "{b:?}"
    );
    assert!(
        (b.min[2]).abs() < 0.01 && (b.max[2] - 30.0).abs() < 0.01,
        "{b:?}"
    );

    // Corners given the other way round make the same kind of box.
    s.clouds.slice_on = false;
    let mut t = Tool::cloud_slice(APERTURE / 100.0);
    t.handle(&mut s, Input::Click(down(60.0, 70.0))).unwrap();
    t.handle(&mut s, Input::Click(down(20.0, 20.5))).unwrap();
    let c = s.clouds.active_slice().unwrap();
    assert!(c.min[0] < c.max[0] && c.min[1] < c.max[1]);
}

/// A thin box is refused, and the gesture waits for a real corner.
#[test]
fn a_box_needs_some_size() {
    let mut s = scene_with("thin", &corners());
    let mut t = Tool::cloud_slice(APERTURE / 100.0);
    t.handle(&mut s, Input::Click(down(20.0, 20.0))).unwrap();
    t.handle(&mut s, Input::Click(down(20.0, 60.0))).unwrap();
    assert!(t.busy());
    assert_eq!(s.clouds.active_slice(), None);
}

/// With no cloud the tool says so.
#[test]
fn no_cloud_no_box() {
    let mut s = Scene::new(Project::new(None).unwrap());
    let mut t = Tool::cloud_slice(APERTURE);
    assert!(t.handle(&mut s, Input::Click(down(1.0, 1.0))).is_err());
    t.handle(&mut s, Input::Hover(down(1.0, 1.0))).unwrap();
}

fn with_box(name: &str) -> (Scene, Slice) {
    let mut s = scene_with(name, &corners());
    let slice = Slice {
        min: [20.0, 20.0, 0.0],
        max: [60.0, 60.0, 30.0],
    };
    s.clouds.slice = Some(slice);
    s.clouds.slice_on = true;
    (s, slice)
}

/// From the side: a ray at world height `z`, pointing at the box's east grip.
fn at_height(z: f64) -> PickRay {
    PickRay {
        origin: Point3::new(40.0, -500.0, z),
        direction: Vec3::new(0.0, 1.0, 0.0),
    }
}

/// A grip moves its own side and no other, and stops short of the far side. Esc
/// puts the box back.
#[test]
fn a_grip_moves_its_side() {
    let (mut s, before) = with_box("grip");
    // Snap off: this is about the free pull.
    s.clouds.snap = false;
    let mut t = Tool::cloud_slice(APERTURE);

    // The top's grip, from the side.
    let grab = at_height(30.0);
    t.handle(&mut s, Input::Hover(grab)).unwrap();
    t.handle(&mut s, Input::Click(grab)).unwrap();
    assert!(t.busy(), "holding the top");
    t.handle(&mut s, Input::Hover(at_height(12.0))).unwrap();
    let b = s.clouds.active_slice().unwrap();
    assert!((b.max[2] - 12.0).abs() < 1e-6, "{b:?}");
    assert_eq!(b.min[2], before.min[2]);
    assert_eq!(b.min[0], before.min[0]);
    assert_eq!(b.max[1], before.max[1]);

    // Past the bottom: stops there.
    t.handle(&mut s, Input::Hover(at_height(-50.0))).unwrap();
    let b = s.clouds.active_slice().unwrap();
    assert!(b.max[2] > b.min[2], "{b:?}");

    t.handle(&mut s, Input::Cancel).unwrap();
    assert!(!t.busy());
    assert_eq!(s.clouds.slice, Some(before), "Esc puts it back");

    // A click lets go where it is.
    t.handle(&mut s, Input::Click(grab)).unwrap();
    t.handle(&mut s, Input::Click(at_height(15.0))).unwrap();
    assert!(!t.busy());
    assert!((s.clouds.slice.unwrap().max[2] - 15.0).abs() < 1e-6);
}

/// A grip snaps to a point: bring the top down to just on a roof.
#[test]
fn a_grip_snaps_to_a_point() {
    let (mut s, _) = with_box("gripsnap");
    let mut t = Tool::cloud_slice(APERTURE);
    let grab = at_height(30.0);
    t.handle(&mut s, Input::Click(grab)).unwrap();
    assert!(t.busy());
    // A point of the cloud: (40, 40, 8), inside the box.
    let at = ray([40.0, 40.0, 8.0]);
    t.handle(&mut s, Input::Hover(at)).unwrap();
    assert!(t.preview().snap.is_some_and(|h| h.cloud.is_some()));
    t.handle(&mut s, Input::Click(at)).unwrap();
    assert!((s.clouds.slice.unwrap().max[2] - 8.0).abs() < 0.01);
}

/// The points outside the box are hidden from the snap too, and a click off every
/// grip leaves the box alone.
#[test]
fn outside_the_box_there_is_nothing_to_snap() {
    let (mut s, before) = with_box("outside");
    let mut l = Tool::line(APERTURE);
    l.handle(&mut s, Input::Hover(down(90.0, 90.0))).unwrap();
    assert!(
        l.preview().snap.is_none_or(|h| h.cloud.is_none()),
        "outside"
    );
    l.handle(&mut s, Input::Hover(down(40.0, 40.0))).unwrap();
    assert!(
        l.preview().snap.is_some_and(|h| h.cloud.is_some()),
        "inside"
    );

    let mut t = Tool::cloud_slice(APERTURE);
    t.handle(&mut s, Input::Click(down(90.0, 90.0))).unwrap();
    assert!(!t.busy());
    assert_eq!(s.clouds.slice, Some(before));
}

/// The grips show while the tool is held, and turning the box off takes them away.
#[test]
fn the_box_shows_in_the_preview() {
    let (mut s, _) = with_box("preview");
    let mut t = Tool::cloud_slice(APERTURE);
    t.handle(&mut s, Input::Hover(down(90.0, 90.0))).unwrap();
    assert_eq!(t.preview().lines.len(), 12 + 6 * 3, "edges and grips");
    s.clouds.slice_on = false;
    t.handle(&mut s, Input::Hover(down(90.0, 90.0))).unwrap();
    assert!(t.preview().lines.is_empty());
}

/// A box on the ground, 5 x 4 x 3 m, with its corner at (10, 10), and a cloud far off.
fn box_and_cloud(name: &str) -> (Scene, u32) {
    let mut s = Scene::new(Project::new(None).unwrap());
    let mut r = Tool::rectangle(APERTURE / 10.0);
    r.handle(&mut s, Input::Click(ray([10.0, 10.0, 0.0])))
        .unwrap();
    r.handle(&mut s, Input::Hover(ray([12.0, 12.0, 0.0])))
        .unwrap();
    r.handle(&mut s, Input::Typed("5;4".into())).unwrap();
    let mut p = Tool::push_pull(APERTURE / 10.0);
    p.handle(&mut s, Input::Click(ray([12.5, 12.0, 0.0])))
        .unwrap();
    p.handle(&mut s, Input::Typed("3".into())).unwrap();
    let (source, file) = cached(name, &[[60.0, 60.0, 7.0], [80.0, 80.0, 7.0]]);
    let id = s.clouds.add(file, source);
    settle(&mut s);
    (s, id)
}

/// Picks the cloud's (60, 60, 7) point, then the target. Returns where that point
/// ended up.
fn move_to(s: &mut Scene, target: PickRay) -> ([f64; 3], Option<SnapKind>) {
    let mut t = Tool::cloud_move(APERTURE);
    t.handle(s, Input::Click(ray([60.0, 60.0, 7.0]))).unwrap();
    assert!(t.busy(), "the cloud is taken");
    t.handle(s, Input::Hover(target)).unwrap();
    let kind = t.preview().snap.map(|h| h.snap.kind);
    t.handle(s, Input::Click(target)).unwrap();
    assert!(!t.busy(), "put down");
    assert_eq!(s.clouds.moving, None);
    let o = s.clouds.offset_of(s.clouds.clouds()[0].id).unwrap();
    ([60.0 + o[0], 60.0 + o[1], 7.0 + o[2]], kind)
}

fn close(a: [f64; 3], b: [f64; 3]) {
    assert!((0..3).all(|k| (a[k] - b[k]).abs() < 0.01), "{a:?} vs {b:?}");
}

#[test]
fn a_cloud_snaps_to_a_corner() {
    let (mut s, _) = box_and_cloud("to-corner");
    let (at, kind) = move_to(&mut s, ray([15.0, 10.0, 3.0]));
    assert_eq!(kind, Some(SnapKind::Endpoint));
    close(at, [15.0, 10.0, 3.0]);
}

#[test]
fn a_cloud_snaps_to_a_midpoint() {
    let (mut s, _) = box_and_cloud("to-mid");
    let (at, kind) = move_to(&mut s, ray([12.5, 10.0, 3.0]));
    assert_eq!(kind, Some(SnapKind::Midpoint));
    close(at, [12.5, 10.0, 3.0]);
}

#[test]
fn a_cloud_snaps_to_an_edge() {
    let (mut s, _) = box_and_cloud("to-edge");
    let (at, kind) = move_to(&mut s, ray([11.0, 10.0, 3.0]));
    assert_eq!(kind, Some(SnapKind::OnEdge));
    close(at, [11.0, 10.0, 3.0]);
}

#[test]
fn a_cloud_snaps_to_a_face() {
    let (mut s, _) = box_and_cloud("to-face");
    let (at, kind) = move_to(&mut s, ray([12.5, 12.0, 3.0]));
    assert_eq!(kind, Some(SnapKind::OnFace));
    close(at, [12.5, 12.0, 3.0]);
}

#[test]
fn a_cloud_snaps_to_an_axis() {
    let (mut s, _) = box_and_cloud("to-axis");
    let (at, kind) = move_to(&mut s, ray([40.0, 0.0, 0.0]));
    assert_eq!(kind, Some(SnapKind::OnAxis));
    assert!(at[1].abs() < 0.01 && at[2].abs() < 0.01, "{at:?}");
}

#[test]
fn a_cloud_snaps_to_the_origin() {
    let (mut s, _) = box_and_cloud("to-origin");
    let (at, kind) = move_to(&mut s, ray([0.15, 0.15, 0.0]));
    assert!(kind.is_some());
    close(at, [0.0, 0.0, 0.0]);
    assert_eq!(at.map(|v| (v * 1000.0).round()), [0.0, 0.0, 0.0], "exactly");
}

/// Another cloud's point is a target too, and the moving one's own never is.
#[test]
fn a_cloud_snaps_to_another_cloud_never_itself() {
    let (mut s, a) = box_and_cloud("to-other");
    let (source, file) = cached("other", &[[100.0, 20.0, 9.0], [120.0, 40.0, 9.0]]);
    s.clouds.add(file, source);
    settle(&mut s);

    let mut t = Tool::cloud_move(APERTURE);
    t.handle(&mut s, Input::Click(ray([60.0, 60.0, 7.0])))
        .unwrap();
    // Its own far point, still under the ray at its old place: no snap to itself.
    t.handle(&mut s, Input::Hover(ray([80.0, 80.0, 7.0])))
        .unwrap();
    assert!(t.preview().snap.is_none_or(|h| h.cloud != Some(a)));
    let to = ray([100.0, 20.0, 9.0]);
    t.handle(&mut s, Input::Hover(to)).unwrap();
    assert!(
        t.preview()
            .snap
            .is_some_and(|h| h.cloud.is_some_and(|c| c != a))
    );
    t.handle(&mut s, Input::Click(to)).unwrap();
    let o = s.clouds.offset_of(a).unwrap();
    close([60.0 + o[0], 60.0 + o[1], 7.0 + o[2]], [100.0, 20.0, 9.0]);
}

/// It follows the pointer on the way, and Esc takes it home.
#[test]
fn escape_takes_the_cloud_back() {
    let (mut s, id) = box_and_cloud("esc");
    let before = s.project().save();
    let mut t = Tool::cloud_move(APERTURE);
    t.handle(&mut s, Input::Click(ray([60.0, 60.0, 7.0])))
        .unwrap();
    t.handle(&mut s, Input::Hover(ray([15.0, 10.0, 3.0])))
        .unwrap();
    assert_ne!(s.clouds.offset_of(id), Some([0.0; 3]), "on its way");
    assert!(!t.preview().lines.is_empty());
    t.handle(&mut s, Input::Cancel).unwrap();
    assert_eq!(s.clouds.offset_of(id), Some([0.0; 3]));
    assert_eq!(s.clouds.moving, None);
    assert!(!s.take_clouds_moved());
    assert_eq!(s.project().save(), before, "the model is not involved");
}

/// A typed distance goes along a locked axis, and a click off every point starts
/// nothing unless the panel chose the cloud.
#[test]
fn typed_distance_and_chosen_cloud() {
    let (mut s, id) = box_and_cloud("typed");
    let mut t = Tool::cloud_move(APERTURE / 100.0);
    assert!(
        t.handle(&mut s, Input::Click(down(5.0, 90.0))).is_err(),
        "no cloud there"
    );
    assert!(!t.busy());

    let mut t = Tool::cloud_move(APERTURE);
    t.set_cloud_target(id);
    t.handle(&mut s, Input::Click(down(5.0, 90.0))).unwrap();
    assert!(t.busy(), "any point will do");
    t.handle(&mut s, Input::LockAxis(Axis::X)).unwrap();
    t.handle(&mut s, Input::Hover(down(9.0, 90.0))).unwrap();
    t.handle(&mut s, Input::Typed("2.5".into())).unwrap();
    assert!(!t.busy());
    close(s.clouds.offset_of(id).unwrap(), [2.5, 0.0, 0.0]);
    assert!(s.take_clouds_moved());
}

/// A move is one step among the model's edits, in order, and redoes.
#[test]
fn undo_takes_cloud_moves_in_turn_with_edits() {
    let (mut s, id) = box_and_cloud("undo");
    let box_only = s.project().save();
    move_to(&mut s, ray([15.0, 10.0, 3.0]));
    let moved = s.clouds.offset_of(id).unwrap();
    assert!(s.take_clouds_moved());
    let mut p = Tool::push_pull(APERTURE / 10.0);
    p.handle(&mut s, Input::Click(ray([12.5, 12.0, 3.0])))
        .unwrap();
    p.handle(&mut s, Input::Typed("1".into())).unwrap();
    let pulled = s.project().save();

    s.undo().unwrap();
    assert_eq!(s.project().save(), box_only, "the pull first");
    assert_eq!(s.clouds.offset_of(id), Some(moved));
    s.undo().unwrap();
    assert_eq!(s.clouds.offset_of(id), Some([0.0; 3]), "then the move");
    assert!(s.take_clouds_moved());
    assert_eq!(s.project().save(), box_only);

    s.redo().unwrap();
    assert_eq!(s.clouds.offset_of(id), Some(moved));
    s.redo().unwrap();
    assert_eq!(s.project().save(), pulled);
    assert_eq!(s.clouds.offset_of(id), Some(moved));
}

/// With the cloud gone from the project, its steps do nothing and cost no keypress.
#[test]
fn undo_skips_a_cloud_no_longer_there() {
    let (mut s, id) = box_and_cloud("gone");
    let with_box = s.can_undo();
    assert!(with_box);
    let saved = s.project().save();
    move_to(&mut s, ray([15.0, 10.0, 3.0]));
    s.clouds.remove(id);
    s.undo().unwrap();
    assert_ne!(s.project().save(), saved, "went past it, to the box's pull");
}

/// Too far is refused, with everything as it was.
#[test]
fn a_cloud_cannot_leave_the_project() {
    let (mut s, id) = box_and_cloud("far");
    assert!(s.move_cloud(id, [0.0; 3], [1e9, 0.0, 0.0]).is_err());
    assert!(
        s.move_cloud(id, [0.0; 3], [f64::INFINITY, 0.0, 0.0])
            .is_err()
    );
    assert!(s.move_cloud(99, [0.0; 3], [1.0, 0.0, 0.0]).is_err());
    assert_eq!(s.clouds.offset_of(id), Some([0.0; 3]));
    assert!(!s.take_clouds_moved());
}

proptest::proptest! {
    /// Any moves, undone in turn: exactly where each one started, and redo exactly back.
    #[test]
    fn moves_undo_exactly(
        moves in proptest::collection::vec(
            (-4000.0f64..4000.0, -4000.0f64..4000.0, -100.0f64..100.0),
            1..8,
        ),
    ) {
        let mut s = Scene::new(Project::new(None).unwrap());
        let (source, file) = cached("prop", &[[1.0, 1.0, 1.0], [2.0, 2.0, 2.0]]);
        let id = s.clouds.add(file, source);
        let mut trail = vec![[0.0; 3]];
        for (x, y, z) in moves {
            let from = *trail.last().unwrap();
            s.move_cloud(id, from, [x, y, z]).unwrap();
            trail.push([x, y, z]);
        }
        for want in trail.iter().rev().skip(1) {
            s.undo().unwrap();
            proptest::prop_assert_eq!(s.clouds.offset_of(id), Some(*want));
        }
        for want in trail.iter().skip(1) {
            s.redo().unwrap();
            proptest::prop_assert_eq!(s.clouds.offset_of(id), Some(*want));
        }
    }
}
