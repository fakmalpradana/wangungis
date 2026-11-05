use std::fs;
use std::time::Duration;

use std::sync::atomic::Ordering;

use las::point::Format;
use las::{Builder, Color, Point, Version, Writer};
use proptest::prelude::*;
use wangungis_pointcloud::{CloudFile, ImportError, Progress, cache_path, import_las};

fn temp_dir(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "wangungis-pointcloud-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn cache_path_changes_when_source_changes() {
    let dir = temp_dir("cache-path");
    let source = dir.join("survey.las");
    let cache = dir.join("cache");
    fs::write(&source, b"first").unwrap();
    let first = cache_path(&source, &cache).unwrap();
    std::thread::sleep(Duration::from_millis(5));
    fs::write(&source, b"second version").unwrap();
    let second = cache_path(&source, &cache).unwrap();
    assert_ne!(first, second);
    assert_eq!(first.extension().unwrap(), "wpc");
    assert!(
        first
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("survey-")
    );
    fs::remove_dir_all(dir).unwrap();
}

fn write_las(path: &std::path::Path, version: Version, format: u8, colors: bool) -> Vec<[f64; 3]> {
    let mut builder = Builder::from(version);
    builder.point_format = Format::new(format).unwrap();
    builder.transforms.x.scale = 0.001;
    builder.transforms.y.scale = 0.001;
    builder.transforms.z.scale = 0.001;
    builder.transforms.x.offset = 710_000.0;
    builder.transforms.y.offset = 9_316_000.0;
    let header = builder.into_header().unwrap();
    let mut writer = Writer::from_path(path, header).unwrap();
    let points = vec![
        [710_000.0, 9_316_000.0, 4.0],
        [710_500.125, 9_316_400.25, 12.5],
        [710_999.999, 9_316_999.999, 20.0],
    ];
    for (i, xyz) in points.iter().enumerate() {
        let mut point = Point {
            x: xyz[0],
            y: xyz[1],
            z: xyz[2],
            ..Point::default()
        };
        if writer.header().point_format().has_gps_time {
            point.gps_time = Some(i as f64);
        }
        if writer.header().point_format().has_color {
            point.color = Some(if colors {
                Color::new((i as u16 + 1) * 257, 514, 771)
            } else {
                Color::new(0, 0, 0)
            });
        }
        writer.write_point(point).unwrap();
    }
    writer.close().unwrap();
    points
}

fn write_points(path: &std::path::Path, format: u8, points: &[(f64, f64, f64, Option<Color>)]) {
    let mut builder = Builder::from((1, 4));
    builder.point_format = Format::new(format).unwrap();
    builder.transforms.x.scale = 0.001;
    builder.transforms.y.scale = 0.001;
    builder.transforms.z.scale = 0.001;
    let mut writer = Writer::from_path(path, builder.into_header().unwrap()).unwrap();
    for &(x, y, z, color) in points {
        let mut point = Point {
            x,
            y,
            z,
            ..Point::default()
        };
        if writer.header().point_format().has_gps_time {
            point.gps_time = Some(0.0);
        }
        if writer.header().point_format().has_color {
            point.color = color;
        }
        writer.write_point(point).unwrap();
    }
    writer.close().unwrap();
}

#[test]
fn round_trip_supported_formats_and_versions() {
    for &(major, minor, format) in &[(1, 2, 0), (1, 2, 1), (1, 2, 2), (1, 2, 3), (1, 4, 7)] {
        let dir = temp_dir(&format!("round-{major}-{minor}-{format}"));
        let source = dir.join("input.las");
        let cache = dir.join("output.wpc");
        let expected = write_las(&source, Version::new(major, minor), format, true);
        import_las(&source, &cache, &Progress::default()).unwrap();
        let mut cloud = CloudFile::open(&cache).unwrap();
        let chunks = cloud.info.chunks.clone();
        let mut actual = Vec::new();
        for (i, chunk) in chunks.iter().enumerate() {
            actual.extend(
                cloud
                    .read(i, 0, chunk.count)
                    .unwrap()
                    .iter()
                    .map(|p| chunk.world(p)),
            );
        }
        assert_eq!(actual.len(), expected.len());
        let tolerance: [f64; 3] = std::array::from_fn(|k| {
            chunks
                .iter()
                .map(|chunk| chunk.size[k] / 65535.0 / 2.0)
                .fold(0.0_f64, f64::max)
                + 0.0005
                + 1e-9
        });
        for want in expected {
            assert!(
                actual
                    .iter()
                    .any(|got| (0..3).all(|k| (got[k] - want[k]).abs() <= tolerance[k]))
            );
        }
        fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn import_is_deterministic_and_cancel_cleans_up() {
    let dir = temp_dir("deterministic");
    let source = dir.join("input.las");
    write_las(&source, Version::new(1, 2), 3, true);
    let first = dir.join("first.wpc");
    let second = dir.join("second.wpc");
    import_las(&source, &first, &Progress::default()).unwrap();
    import_las(&source, &second, &Progress::default()).unwrap();
    assert_eq!(fs::read(first).unwrap(), fs::read(second).unwrap());
    let cancelled = dir.join("cancelled.wpc");
    let progress = Progress::default();
    progress.cancel.store(true, Ordering::Relaxed);
    assert!(matches!(
        import_las(&source, &cancelled, &progress),
        Err(ImportError::Cancelled)
    ));
    assert!(!cancelled.exists());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn colors_are_scaled_or_derived_from_elevation() {
    let cases = [
        (
            "sixteen",
            2,
            vec![(0.0, 0.0, 0.0, Some(Color::new(256, 512, 65_535)))],
            [1, 2, 255, 255],
        ),
        (
            "eight",
            2,
            vec![(0.0, 0.0, 0.0, Some(Color::new(12, 34, 255)))],
            [12, 34, 255, 255],
        ),
        (
            "ramp",
            0,
            vec![(0.0, 0.0, 0.0, None), (1.0, 0.0, 10.0, None)],
            [0, 0, 255, 255],
        ),
    ];
    for (name, format, points, expected) in cases {
        let dir = temp_dir(name);
        let source = dir.join("input.las");
        let cache = dir.join("output.wpc");
        write_points(&source, format, &points);
        import_las(&source, &cache, &Progress::default()).unwrap();
        let mut cloud = CloudFile::open(&cache).unwrap();
        let count = cloud.info.chunks[0].count;
        let colors: Vec<_> = cloud
            .read(0, 0, count)
            .unwrap()
            .into_iter()
            .map(|p| p.color)
            .collect();
        assert!(colors.contains(&expected));
        if name == "ramp" {
            assert!(colors.contains(&[255, 0, 0, 255]));
        }
        fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn shuffled_prefix_covers_the_chunk() {
    let dir = temp_dir("prefix");
    let source = dir.join("input.las");
    let cache = dir.join("output.wpc");
    let points: Vec<_> = (0..100)
        .flat_map(|x| (0..100).map(move |y| (x as f64, y as f64, 0.0, None)))
        .collect();
    write_points(&source, 0, &points);
    import_las(&source, &cache, &Progress::default()).unwrap();
    let mut cloud = CloudFile::open(&cache).unwrap();
    let chunk = cloud.info.chunks[0];
    let sample = cloud.read(0, 0, chunk.count / 10).unwrap();
    let occupied: std::collections::HashSet<_> = sample
        .iter()
        .map(|point| {
            let world = chunk.world(point);
            (
                (world[0] / 10.0).floor() as u8,
                (world[1] / 10.0).floor() as u8,
            )
        })
        .collect();
    assert!(
        occupied.len() >= 90,
        "occupied {} of 100 coarse cells",
        occupied.len()
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn missing_output_parent_is_created() {
    let dir = temp_dir("parent");
    let source = dir.join("input.las");
    write_las(&source, Version::new(1, 2), 0, false);
    let cache = dir.join("new/deep/output.wpc");
    import_las(&source, &cache, &Progress::default()).unwrap();
    assert!(CloudFile::open(&cache).is_ok());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn damaged_cache_and_unknown_chunk_are_errors() {
    let dir = temp_dir("damage");
    let bad = dir.join("bad.wpc");
    fs::write(&bad, b"WPCLOUD1").unwrap();
    assert!(CloudFile::open(&bad).is_err());
    let source = dir.join("input.las");
    let cache = dir.join("good.wpc");
    write_las(&source, Version::new(1, 2), 0, false);
    import_las(&source, &cache, &Progress::default()).unwrap();
    let mut cloud = CloudFile::open(&cache).unwrap();
    assert!(cloud.read(usize::MAX, 0, 1).is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn bad_las_inputs_fail_without_output_or_temp() {
    let dir = temp_dir("bad-las");
    for (name, bytes) in [
        ("empty.las", Vec::new()),
        ("random.las", vec![17; 300]),
        ("header.las", b"LASF".to_vec()),
    ] {
        let source = dir.join(name);
        let cache = dir.join(format!("{name}.wpc"));
        fs::write(&source, bytes).unwrap();
        assert!(import_las(&source, &cache, &Progress::default()).is_err());
        assert!(!cache.exists());
        assert!(
            !cache
                .with_file_name(format!("{name}.wpc.{}.tmp", std::process::id()))
                .exists()
        );
    }

    let valid = dir.join("valid.las");
    write_las(&valid, Version::new(1, 2), 0, false);
    let bytes = fs::read(&valid).unwrap();
    for (name, cut) in [("cut-header.las", 100), ("cut-points.las", bytes.len() - 5)] {
        let source = dir.join(name);
        let cache = dir.join(format!("{name}.wpc"));
        fs::write(&source, &bytes[..cut]).unwrap();
        assert!(import_las(&source, &cache, &Progress::default()).is_err());
        assert!(!cache.exists());
    }
    let mut lying = bytes;
    lying[107..111].copy_from_slice(&10_u32.to_le_bytes());
    let source = dir.join("lying-count.las");
    let cache = dir.join("lying-count.wpc");
    fs::write(&source, lying).unwrap();
    assert!(matches!(
        import_las(&source, &cache, &Progress::default()),
        Err(ImportError::Truncated { .. })
    ));
    assert!(!cache.exists());

    let empty = dir.join("no-points.las");
    write_points(&empty, 0, &[]);
    let cache = dir.join("no-points.wpc");
    assert!(matches!(
        import_las(&empty, &cache, &Progress::default()),
        Err(ImportError::NoPoints)
    ));
    assert!(!cache.exists());
    fs::remove_dir_all(dir).unwrap();
}

/// A big flat file, for what takes time.
fn big_las(path: &std::path::Path, points: u32) {
    let mut builder = Builder::from(Version::new(1, 2));
    builder.point_format = Format::new(2).unwrap();
    let mut writer = Writer::from_path(path, builder.into_header().unwrap()).unwrap();
    for i in 0..points {
        writer
            .write_point(Point {
                x: f64::from(i % 1000),
                y: f64::from(i / 1000),
                z: 1.0,
                color: Some(Color::new(1000, 2000, 3000)),
                ..Default::default()
            })
            .unwrap();
    }
    writer.close().unwrap();
}

/// Cancelling in each pass stops the import there and leaves nothing behind.
#[test]
fn cancel_in_any_pass_cleans_up() {
    let dir = temp_dir("cancel-during");
    let source = dir.join("a.las");
    big_las(&source, 400_000);
    for phase in [
        wangungis_pointcloud::PHASE_SCAN,
        wangungis_pointcloud::PHASE_WRITE,
        wangungis_pointcloud::PHASE_SHUFFLE,
    ] {
        let out = dir.join(format!("out-{phase}"));
        fs::create_dir_all(&out).unwrap();
        let cache = out.join("a.wpc");
        let progress = Progress::default();
        let stopped = std::thread::scope(|scope| {
            scope.spawn(|| {
                while progress.phase.load(Ordering::Relaxed) < phase {
                    std::hint::spin_loop();
                }
                progress.cancel.store(true, Ordering::Relaxed);
            });
            let result = import_las(&source, &cache, &progress);
            // Finished before the cancel landed: let the watcher go.
            progress.phase.store(phase, Ordering::Relaxed);
            result
        });
        match stopped {
            Err(ImportError::Cancelled) => {
                assert_eq!(fs::read_dir(&out).unwrap().count(), 0, "phase {phase}");
            }
            // Too quick to catch on this machine, which must then have finished.
            Ok(()) => assert!(CloudFile::open(&cache).is_ok()),
            Err(e) => panic!("phase {phase}: {e}"),
        }
    }
    fs::remove_dir_all(dir).unwrap();
}

/// A cache folder that cannot exist is an error with its reason, not a panic.
#[test]
fn unwritable_output_is_an_error() {
    let dir = temp_dir("unwritable");
    let source = dir.join("a.las");
    write_las(&source, Version::new(1, 2), 3, true);
    let blocker = dir.join("berkas");
    fs::write(&blocker, b"bukan folder").unwrap();
    let err = import_las(&source, &blocker.join("a.wpc"), &Progress::default()).unwrap_err();
    assert!(matches!(err, ImportError::Io(_)), "{err:?}");
    assert!(err.to_string().contains(": "), "says why: {err}");
    fs::remove_dir_all(dir).unwrap();
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    #[test]
    fn damaged_las_never_panics(index in 0usize..400, value in any::<u8>(), cut in 0usize..400) {
        let dir = temp_dir("fuzz-las");
        let source = dir.join("input.las");
        let cache = dir.join("output.wpc");
        write_las(&source, Version::new(1, 2), 3, true);
        let original = fs::read(&source).unwrap();
        let mut damaged = original.clone();
        let at = index % damaged.len();
        damaged[at] = value;
        if cut < original.len() { damaged.truncate(cut); }
        fs::write(&source, damaged).unwrap();
        let result = std::panic::catch_unwind(|| import_las(&source, &cache, &Progress::default()));
        prop_assert!(result.is_ok());
        if matches!(result.unwrap(), Ok(())) { prop_assert!(CloudFile::open(&cache).is_ok()); }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn damaged_cache_never_panics(index in 0usize..400, value in any::<u8>(), truncate in any::<bool>()) {
        let dir = temp_dir("fuzz-cache");
        let source = dir.join("input.las");
        let cache = dir.join("output.wpc");
        write_las(&source, Version::new(1, 2), 3, true);
        import_las(&source, &cache, &Progress::default()).unwrap();
        let mut bytes = fs::read(&cache).unwrap();
        let at = index % bytes.len();
        bytes[at] = value;
        if truncate { bytes.truncate(at); }
        fs::write(&cache, bytes).unwrap();
        let result = std::panic::catch_unwind(|| {
            if let Ok(mut cloud) = CloudFile::open(&cache) { let _ = cloud.read(0, 0, u64::MAX); }
        });
        prop_assert!(result.is_ok());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn random_and_degenerate_points_stay_in_one_chunk_each(
        coords in prop::collection::vec((-20i16..20, -20i16..20, -20i16..20), 1..40),
        shape in 0u8..3,
        narrow_header in any::<bool>(),
    ) {
        let dir = temp_dir("property-points");
        let source = dir.join("input.las");
        let cache = dir.join("output.wpc");
        let points: Vec<_> = coords.into_iter().map(|(x, y, z)| match shape {
            0 => (4.0, 4.0, 4.0, None),
            1 => (f64::from(x), 0.0, 0.0, None),
            _ => (f64::from(x), f64::from(y), f64::from(z), None),
        }).collect();
        write_points(&source, 0, &points);
        if narrow_header {
            let mut bytes = fs::read(&source).unwrap();
            for offset in [179usize, 187, 195, 203, 211, 219] {
                bytes[offset..offset + 8].copy_from_slice(&0.0_f64.to_le_bytes());
            }
            fs::write(&source, bytes).unwrap();
        }
        import_las(&source, &cache, &Progress::default()).unwrap();
        let mut cloud = CloudFile::open(&cache).unwrap();
        prop_assert_eq!(cloud.info.points as usize, points.len());
        let chunks = cloud.info.chunks.clone();
        let mut read = 0usize;
        for (i, chunk) in chunks.iter().enumerate() {
            let cached = cloud.read(i, 0, u64::MAX).unwrap();
            read += cached.len();
            for point in cached {
                let world = chunk.world(&point);
                prop_assert!((0..3).all(|k| world[k] >= chunk.min[k] && world[k] <= chunk.min[k] + chunk.size[k]));
            }
        }
        prop_assert_eq!(read, points.len());
        let _ = fs::remove_dir_all(dir);
    }
}

#[test]
#[ignore]
fn real_file() {
    let source = std::path::PathBuf::from(std::env::var("WANGUNGIS_LAS_SAMPLE").unwrap());
    let dir = temp_dir("real");
    let cache = dir.join("sample.wpc");
    let progress = Progress::default();
    let started = std::time::Instant::now();
    let mut scan_end = None;
    let mut write_end = None;
    std::thread::scope(|scope| {
        let handle = scope.spawn(|| import_las(&source, &cache, &progress));
        while !handle.is_finished() {
            match progress.phase.load(Ordering::Relaxed) {
                wangungis_pointcloud::PHASE_WRITE if scan_end.is_none() => {
                    scan_end = Some(started.elapsed())
                }
                wangungis_pointcloud::PHASE_SHUFFLE if write_end.is_none() => {
                    scan_end.get_or_insert(started.elapsed());
                    write_end = Some(started.elapsed());
                }
                _ => {}
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        handle.join().unwrap().unwrap();
    });
    let elapsed = started.elapsed().as_secs_f64();
    let scan = scan_end.unwrap().as_secs_f64();
    let write = write_end.unwrap().as_secs_f64() - scan;
    let shuffle = elapsed - scan - write;
    let mut cloud = CloudFile::open(&cache).unwrap();
    let chunks = cloud.info.chunks.clone();
    let largest = chunks.iter().map(|c| c.count).max().unwrap_or(0);
    println!(
        "scan_seconds={scan:.3} write_seconds={write:.3} shuffle_seconds={shuffle:.3} elapsed_seconds={elapsed:.3} cache_bytes={} chunks={} largest_chunk={} points_per_second={:.0}",
        fs::metadata(&cache).unwrap().len(),
        chunks.len(),
        largest,
        cloud.info.points as f64 / elapsed
    );
    for (i, chunk) in chunks.iter().enumerate() {
        for point in cloud.read(i, 0, 100).unwrap() {
            let world = chunk.world(&point);
            assert!(
                (0..3)
                    .all(|k| world[k] >= chunk.min[k] && world[k] <= chunk.min[k] + chunk.size[k])
            );
        }
    }
    fs::remove_dir_all(dir).unwrap();
}
