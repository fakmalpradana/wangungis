//! Exports read back: geometry where it should be, in every format.

use kernel::{Plane, Point3, Vec3};
use wangungis_io::{ExportError, export};
use wangungis_scene::project::{Project, TileKey};
use wangungis_tools::Scene;

fn add_box(s: &mut Scene, x: f64, y: f64) {
    let ground =
        Plane::from_point_normal(Point3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0)).unwrap();
    let p = |a: f64, b: f64| Point3::new(x + a, y + b, 0.0);
    let r = [p(0.0, 0.0), p(5.0, 0.0), p(5.0, 4.0), p(0.0, 4.0)];
    let segs: Vec<_> = (0..4).map(|k| (r[k], r[(k + 1) % 4])).collect();
    let drawn = s.draw(None, ground, &segs).unwrap();
    let sk = drawn.sketch.unwrap();
    let region = s.project().tiles[&drawn.tile]
        .extrudable_regions(sk)
        .unwrap()[0];
    s.extrude(drawn.tile, sk, region, 3.0).unwrap();
}

fn obj_bounds(text: &str) -> ([f64; 3], [f64; 3], usize, usize) {
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    let (mut v, mut f) = (0, 0);
    for line in text.lines() {
        let mut w = line.split_whitespace();
        match w.next() {
            Some("v") => {
                v += 1;
                for k in 0..3 {
                    let x: f64 = w.next().unwrap().parse().unwrap();
                    lo[k] = lo[k].min(x);
                    hi[k] = hi[k].max(x);
                }
            }
            Some("f") => {
                f += 1;
                for _ in 0..3 {
                    let i: usize = w.next().unwrap().parse().unwrap();
                    assert!(i >= 1 && i <= v, "index {i} of {v}");
                }
            }
            _ => {}
        }
    }
    (lo, hi, v, f)
}

#[test]
fn obj_places_solids_across_tiles() {
    let mut s = Scene::new(Project::new(None).unwrap());
    add_box(&mut s, 0.0, 0.0);
    add_box(&mut s, 2_000.0, -3_000.0);
    assert!(s.project().occupied().count() > 1);
    let bytes = export(s.project(), "OBJ").unwrap();
    let (lo, hi, _, f) = obj_bounds(std::str::from_utf8(&bytes).unwrap());
    assert_eq!(f, 24, "two boxes of 12 triangles");
    for (got, want) in lo.iter().zip([0.0, -3_000.0, 0.0]) {
        assert!((got - want).abs() < 1e-3, "{lo:?}");
    }
    for (got, want) in hi.iter().zip([2_005.0, 4.0, 3.0]) {
        assert!((got - want).abs() < 1e-3, "{hi:?}");
    }
}

#[test]
fn hew_formats_take_one_tile() {
    let mut s = Scene::new(Project::new(None).unwrap());
    add_box(&mut s, 0.0, 0.0);
    assert!(
        s.project()
            .occupied()
            .map(|(k, _)| *k)
            .eq([TileKey::ORIGIN])
    );
    let stl = export(s.project(), "stl").unwrap();
    // Binary STL: 80 byte header, count, 50 bytes per triangle.
    assert_eq!(u32::from_le_bytes(stl[80..84].try_into().unwrap()), 12);
    assert_eq!(stl.len(), 84 + 12 * 50);
    let glb = export(s.project(), "glb").unwrap();
    assert_eq!(&glb[..4], b"glTF");
    let threemf = export(s.project(), "3mf").unwrap();
    assert_eq!(&threemf[..2], b"PK");

    add_box(&mut s, 2_000.0, 0.0);
    assert!(matches!(
        export(s.project(), "stl"),
        Err(ExportError::MultiTile)
    ));
    assert!(export(s.project(), "obj").is_ok());
}

#[test]
fn nothing_or_nonsense_is_refused() {
    let empty = Project::new(None).unwrap();
    for f in ["obj", "stl", "glb"] {
        assert!(
            matches!(export(&empty, f), Err(ExportError::NothingToExport)),
            "{f}"
        );
    }
    assert!(matches!(
        export(&empty, "dwg"),
        Err(ExportError::UnknownFormat)
    ));
}
