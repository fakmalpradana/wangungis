//! What a view gets from a set of clouds: its budgets, its slice, its picks.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use las::point::Format;
use las::{Builder, Color, Point, Writer};
use wangungis_pointcloud::{CloudFile, CloudSet, Progress, SavedClouds, Slice, View, import_las};
use wangungis_render::Camera;

fn temp_dir(name: &str) -> PathBuf {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("wangungis-set-{name}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&path).unwrap();
    path
}

/// A flat square of `side` x `side` points 1 m apart, at z = 10.
fn cloud(name: &str, side: u32) -> (PathBuf, CloudFile) {
    let dir = temp_dir(name);
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
    for y in 0..side {
        for x in 0..side {
            w.write_point(Point {
                x: f64::from(x),
                y: f64::from(y),
                z: 10.0,
                color: Some(Color::new(65535, 0, 0)),
                ..Default::default()
            })
            .unwrap();
        }
    }
    w.close().unwrap();
    let cache = dir.join("a.wpc");
    import_las(&source, &cache, &Progress::default()).unwrap();
    (source, CloudFile::open(&cache).unwrap())
}

fn view(eye: [f64; 3], target: [f64; 3]) -> View {
    let cam = Camera {
        eye,
        target,
        fovy: 0.8,
        aspect: 1.5,
        near: 0.5,
        far: 100_000.0,
    };
    View {
        eye,
        view_proj: cam.view_proj_at_eye(),
        focal_px: 800.0 / (2.0 * 0.4f64.tan()),
        point_px: 2.0,
    }
}

/// Plans until nothing more is on the way.
fn settle(set: &mut CloudSet, v: &View) {
    let start = Instant::now();
    loop {
        set.poll();
        set.plan(v);
        set.take_changed(|_| {});
        if !set.busy() {
            return;
        }
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "reads never ended"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn drawn(set: &CloudSet) -> u64 {
    set.draws().iter().map(|d| u64::from(d.count)).sum()
}

/// Over budget, the whole view gives up its share.
#[test]
fn draw_budget_holds() {
    let (source, file) = cloud("budget", 1000);
    let mut set = CloudSet::default();
    set.draw_budget = 200_000;
    set.add(file, source);
    // Close enough that every chunk would like all its points.
    settle(&mut set, &view([500.0, -50.0, 200.0], [500.0, 500.0, 10.0]));
    let n = drawn(&set);
    let chunks = set.clouds()[0].info.chunks.len() as u64;
    assert!(n <= 200_000, "{n} of {chunks} chunks");
    assert!(n > 10_000, "{n}");
}

/// Memory stays inside its budget as the view wanders.
#[test]
fn load_budget_holds_while_the_view_moves() {
    let (source, file) = cloud("load", 1000);
    let mut set = CloudSet::default();
    set.load_budget = 100_000;
    set.add(file, source);
    for (x, y) in [
        (10.0, 10.0),
        (990.0, 10.0),
        (990.0, 990.0),
        (10.0, 990.0),
        (500.0, 500.0),
    ] {
        settle(&mut set, &view([x, y - 5.0, 15.0], [x, y, 10.0]));
        let total: u64 = set.clouds().iter().map(|c| c.held_points()).sum();
        assert!(total <= 100_000, "{total} points held");
    }
}

/// What the view cannot see is not drawn; what comes into view is.
#[test]
fn only_what_is_in_view_is_drawn() {
    let (source, file) = cloud("frustum", 1000);
    let mut set = CloudSet::default();
    set.add(file, source);
    let close = view([10.0, -5.0, 12.0], [10.0, 5.0, 10.0]);
    settle(&mut set, &close);
    let some = set.draws().len();
    let all = set.clouds()[0].info.chunks.len();
    assert!(some > 0 && some < all, "{some} of {all}");
    // Looking away at the sky.
    settle(&mut set, &view([10.0, -5.0, 12.0], [10.0, -500.0, 800.0]));
    assert_eq!(set.draws().len(), 0);
}

/// Nearer means denser, on one chunk at least.
#[test]
fn closer_chunks_get_more_points() {
    let (source, file) = cloud("near", 1000);
    let mut set = CloudSet::default();
    set.add(file, source);
    settle(
        &mut set,
        &view([500.0, 500.0, 1500.0], [500.0, 500.0, 10.0]),
    );
    let far = drawn(&set);
    settle(&mut set, &view([500.0, 500.0, 60.0], [500.0, 500.0, 10.0]));
    let near_each = drawn(&set) / set.draws().len().max(1) as u64;
    let far_each = far / set.clouds()[0].info.chunks.len() as u64;
    assert!(near_each > far_each, "{near_each} vs {far_each}");
}

/// The box hides chunks wholly outside it, and the same chunks return without a read.
#[test]
fn slice_culls_and_comes_back() {
    let (source, file) = cloud("slice", 1000);
    let mut set = CloudSet::default();
    set.add(file, source);
    let v = view([500.0, -300.0, 900.0], [500.0, 500.0, 10.0]);
    settle(&mut set, &v);
    let all = set.draws().to_vec();

    set.slice = Some(Slice {
        min: [100.0, 100.0, 0.0],
        max: [200.0, 200.0, 20.0],
    });
    set.slice_on = true;
    settle(&mut set, &v);
    assert!(set.draws().len() < all.len() / 4, "{}", set.draws().len());
    assert!(set.draws().iter().all(|d| d.count > 0));

    set.slice_on = false;
    set.poll();
    set.plan(&v);
    assert_eq!(set.draws().len(), all.len(), "all back, in one plan");
}

/// A pick lands on a point on screen, inside the box, in the cone.
#[test]
fn pick_finds_the_point_under_the_ray() {
    let (source, file) = cloud("pick", 200);
    let mut set = CloudSet::default();
    set.add(file, source);
    let v = view([100.0, 100.0, 40.0], [100.0, 100.0, 10.0]);
    settle(&mut set, &v);
    let hit = set
        .pick([100.0, 100.0, 40.0], [0.0, 0.0, -1.0], 0.05)
        .expect("a point under the ray");
    assert!((hit[2] - 10.0).abs() < 0.01, "{hit:?}");
    assert!(
        (hit[0] - 100.0).hypot(hit[1] - 100.0) < 30.0 * 0.05 + 0.01,
        "{hit:?}"
    );

    assert_eq!(
        set.pick([100.0, 100.0, 40.0], [0.0, 0.0, 1.0], 0.05),
        None,
        "behind"
    );
    assert_eq!(
        set.pick([100.0, 100.0, 40.0], [1.0, 0.0, 0.0], 0.001),
        None,
        "beside"
    );

    set.snap = false;
    assert_eq!(set.pick([100.0, 100.0, 40.0], [0.0, 0.0, -1.0], 0.05), None);
    set.snap = true;

    set.slice = Some(Slice {
        min: [0.0, 0.0, 50.0],
        max: [200.0, 200.0, 60.0],
    });
    set.slice_on = true;
    assert_eq!(
        set.pick([100.0, 100.0, 40.0], [0.0, 0.0, -1.0], 0.05),
        None,
        "hidden by the box"
    );
}

/// A frame shifts everything by the same amount, in `f64`.
#[test]
fn frame_moves_picks_and_bounds() {
    let (source, file) = cloud("frame", 100);
    let mut set = CloudSet::default();
    set.frame = [50.0, 50.0, 0.0];
    set.add(file, source);
    let (lo, hi) = set.bounds().unwrap();
    assert_eq!((lo[0], hi[0]), (-50.0, 49.0));
    settle(&mut set, &view([0.0, 0.0, 30.0], [0.0, 0.0, 10.0]));
    let hit = set.pick([0.0, 0.0, 30.0], [0.0, 0.0, -1.0], 0.05).unwrap();
    assert!(hit[0].hypot(hit[1]) < 2.0, "{hit:?}");
}

/// Removing a cloud tells the renderer, and answers still on the way do no harm.
#[test]
fn remove_while_reading() {
    let (source, file) = cloud("remove", 500);
    let mut set = CloudSet::default();
    let id = set.add(file, source);
    let v = view([250.0, 250.0, 100.0], [250.0, 250.0, 10.0]);
    set.plan(&v);
    set.remove(id);
    settle(&mut set, &v);
    assert!(set.is_empty());
    assert_eq!(set.take_removed(), vec![id]);
    assert_eq!(set.bounds(), None);
}

/// Settings from a damaged or hand-edited companion file stay in their ranges.
#[test]
fn saved_settings_are_checked() {
    let mut set = CloudSet::default();
    set.apply_saved(&SavedClouds {
        frame: [f64::NAN, 1.0, 2.0],
        clouds: Vec::new(),
        point_size: 500.0,
        draw_budget: 1,
        snap: false,
        slice: Some(Slice {
            min: [5.0, 0.0, 0.0],
            max: [1.0, 1.0, 1.0],
        }),
        slice_on: true,
    });
    assert_eq!(set.frame, [0.0, 1.0, 2.0]);
    assert_eq!(set.point_size, wangungis_pointcloud::MAX_POINT_SIZE);
    assert_eq!(set.draw_budget, wangungis_pointcloud::MIN_DRAW_BUDGET);
    assert_eq!(set.slice, None);
    assert!(!set.slice_on);
    assert!(!set.snap);
}
