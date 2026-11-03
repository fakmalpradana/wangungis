//! Kernel tiles and the project container (ADR-004, ADR-006): splitting a
//! large document must be lossless (same objects, same world positions within
//! 1 mm, attributes kept), every tile must stay within the kernel frame bound,
//! and save → open → save must be byte-identical. A container that does not
//! match what this crate writes must be refused, never guessed at.

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};

use kernel::{AttrTarget, AttrValue, Document, EntityRef, NodeId, ObjectId, Plane, Point3, Vec3};
use proptest::prelude::*;
use wangungis_scene::project::{MANIFEST, Project, ProjectError, TileKey, horizontal_reach};
use wangungis_scene::{KERNEL_FRAME_BOUND_M, NS, ProjectOrigin};

const MM: f64 = 1e-3;
const UTM49S: ProjectOrigin = ProjectOrigin {
    epsg: 32749,
    origin: [430_000.123, 9_150_000.456, 12.5],
};

/// Axis-aligned `w` x `d` x 3 m box with its corner at (x, y, 0), tagged with
/// attribute `wangungis.id = id`.
fn add_box(doc: &mut Document, x: f64, y: f64, w: f64, d: f64, id: i64) -> ObjectId {
    let p = |a: f64, b: f64| Point3::new(x + a, y + b, 0.0);
    let ring = [p(0.0, 0.0), p(w, 0.0), p(w, d), p(0.0, d)];
    let sk = doc.add_sketch(Plane::from_polygon(&ring).unwrap());
    let s = doc.sketch_mut(sk).unwrap();
    for k in 0..4 {
        s.add_segment(ring[k], ring[(k + 1) % 4]).unwrap();
    }
    let r = doc.extrudable_regions(sk).unwrap();
    let o = doc.extrude_region(sk, r[0], 3.0).unwrap().0;
    doc.attr_set(
        AttrTarget::Entity(EntityRef::Object(o)),
        NS,
        "id",
        AttrValue::Int(id),
    )
    .unwrap();
    o
}

fn id_of(doc: &Document, o: ObjectId) -> i64 {
    let d = doc
        .attr_get(&AttrTarget::Entity(EntityRef::Object(o)))
        .unwrap()
        .expect("object attributes survive");
    match d[NS]["id"] {
        AttrValue::Int(i) => i,
        ref v => panic!("id {v:?}"),
    }
}

/// XY-and-Z bounds of an object's vertices, shifted by `shift`.
fn bounds(doc: &Document, o: ObjectId, shift: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for v in doc.object(o).unwrap().vertices().values() {
        let p = [v.position.x, v.position.y, v.position.z];
        for k in 0..3 {
            lo[k] = lo[k].min(p[k] + shift[k]);
            hi[k] = hi[k].max(p[k] + shift[k]);
        }
    }
    (lo, hi)
}

fn world_objects(doc: &Document) -> Vec<ObjectId> {
    doc.top_level_nodes()
        .into_iter()
        .flat_map(|n| doc.leaf_objects_under(n))
        .collect()
}

/// Every object of `src` appears exactly once across `p`, in the tile its
/// anchor names, at the same world position within 1 mm, with its id.
fn assert_lossless(src: &Document, p: &Project, split: bool) {
    let want: BTreeMap<i64, ([f64; 3], [f64; 3])> = world_objects(src)
        .into_iter()
        .map(|o| (id_of(src, o), bounds(src, o, [0.0; 3])))
        .collect();
    let mut seen = BTreeMap::new();
    for (key, tile) in &p.tiles {
        assert!(
            horizontal_reach(tile) <= KERNEL_FRAME_BOUND_M,
            "tile {key:?} reaches {} m",
            horizontal_reach(tile)
        );
        for o in world_objects(tile) {
            let id = id_of(tile, o);
            let (lo, hi) = bounds(tile, o, key.center());
            let (wlo, whi) = want[&id];
            for k in 0..3 {
                assert!(
                    (lo[k] - wlo[k]).abs() <= MM && (hi[k] - whi[k]).abs() <= MM,
                    "object {id} moved: {lo:?}..{hi:?} vs {wlo:?}..{whi:?}"
                );
            }
            let home = if split {
                TileKey::of([(wlo[0] + whi[0]) / 2.0, (wlo[1] + whi[1]) / 2.0])
            } else {
                TileKey::ORIGIN
            };
            assert_eq!(*key, home, "object {id} in the wrong tile");
            assert!(seen.insert(id, *key).is_none(), "object {id} twice");
        }
    }
    assert_eq!(seen.len(), want.len(), "objects lost");
}

#[test]
fn small_document_stays_one_plain_hew() {
    let mut doc = Document::new();
    UTM49S.write(&mut doc).unwrap();
    add_box(&mut doc, 10.0, 20.0, 5.0, 4.0, 1);
    add_box(&mut doc, -1_200.0, 300.0, 5.0, 4.0, 2);
    let src = Document::load(&doc.save()).unwrap();
    let p = Project::from_document(doc).unwrap();
    assert!(p.single().is_some());
    assert_lossless(&src, &p, false);
    // Saves as a plain .hew that Hew itself loads, with the origin intact.
    let plain = Document::load(&p.save()).unwrap();
    assert_eq!(ProjectOrigin::read(&plain).unwrap(), Some(UTM49S));
}

#[test]
fn split_keeps_world_sketches_in_the_origin_tile() {
    let mut doc = Document::new();
    add_box(&mut doc, 0.0, 0.0, 5.0, 4.0, 1);
    add_box(&mut doc, 3_000.0, 0.0, 5.0, 4.0, 2);
    let loose = doc.add_sketch(
        Plane::from_polygon(&[
            Point3::new(2_000.0, 0.0, 0.0),
            Point3::new(2_010.0, 0.0, 0.0),
            Point3::new(2_010.0, 10.0, 0.0),
        ])
        .unwrap(),
    );
    doc.sketch_mut(loose)
        .unwrap()
        .add_segment(
            Point3::new(2_000.0, 0.0, 0.0),
            Point3::new(2_010.0, 0.0, 0.0),
        )
        .unwrap();
    let sketches = doc.sketch_ids().len();
    let p = Project::from_document(doc).unwrap();
    assert_eq!(p.tiles[&TileKey::ORIGIN].sketch_ids().len(), sketches);
}

#[test]
fn object_longer_than_a_tile_is_refused() {
    let mut doc = Document::new();
    // One 3.6 km object: its anchor is the origin, but it reaches 1.8 km.
    add_box(&mut doc, -1_800.0, 0.0, 3_600.0, 10.0, 1);
    match Project::from_document(doc) {
        Err(ProjectError::TooFar { tile, reach_m }) => {
            assert_eq!(tile, TileKey::ORIGIN);
            assert!(reach_m > 1_800.0);
        }
        other => panic!("expected TooFar, got {other:?}"),
    }
}

#[test]
fn point_outside_the_project_is_refused() {
    let mut doc = Document::new();
    add_box(&mut doc, 49_990.0, 0.0, 20.0, 4.0, 1);
    assert!(matches!(
        Project::from_document(doc),
        Err(ProjectError::Origin(
            wangungis_scene::OriginError::OutOfBound(_)
        ))
    ));
}

/// Moved drawing axes, a user-hidden root, a group, and a visibly hidden
/// instance inside that group: each lands in place, hidden state kept.
#[test]
fn split_keeps_axes_hidden_state_groups_and_instances() {
    let mut doc = Document::new();
    add_box(&mut doc, 0.0, 0.0, 5.0, 4.0, 1);
    let hidden = add_box(&mut doc, 2_000.0, 0.0, 5.0, 4.0, 2);
    doc.set_node_user_hidden(NodeId::Object(hidden), true);
    let a = add_box(&mut doc, 3_000.0, 0.0, 5.0, 4.0, 3);
    let b = add_box(&mut doc, 3_010.0, 0.0, 5.0, 4.0, 4);
    let (_, inst, _) = doc.make_component(&[NodeId::Object(b)]).unwrap();
    doc.set_node_user_hidden(NodeId::Instance(inst), true);
    let want_pose = doc.instance_pose(inst).unwrap().to_affine();
    doc.group_nodes(&[NodeId::Object(a), NodeId::Instance(inst)])
        .unwrap();
    let (s, c) = 0.3f64.sin_cos();
    doc.set_axes(
        Point3::new(17.0, -4.0, 0.0),
        Vec3::new(c, s, 0.0),
        Vec3::new(-s, c, 0.0),
    )
    .unwrap();
    let axes = doc.axes();
    let src = Document::load(&doc.save()).unwrap();

    let p = Project::open(&Project::from_document(doc).unwrap().save()).unwrap();
    assert_eq!(
        p.tiles[&TileKey::ORIGIN].axes(),
        axes,
        "source axes restored"
    );
    // Objects 1..3; object 4 is definition geometry now, checked by its pose.
    assert_lossless(&src, &p, true);
    let two = &p.tiles[&TileKey { i: 2, j: 0 }];
    let roots = two.top_level_nodes();
    assert_eq!(roots.len(), 1);
    assert!(two.node_user_hidden(roots[0]), "hidden root became visible");
    let three = &p.tiles[&TileKey { i: 3, j: 0 }];
    let insts: Vec<_> = three
        .top_level_nodes()
        .into_iter()
        .flat_map(|n| three.leaf_instances_under(n))
        .collect();
    assert_eq!(insts.len(), 1);
    assert!(three.node_user_hidden(NodeId::Instance(insts[0])));
    let got = three.instance_pose(insts[0]).unwrap().to_affine();
    for (k, &t) in [3usize, 7, 11].iter().enumerate() {
        let shift = TileKey { i: 3, j: 0 }.center()[k];
        assert!(
            (got[t] + shift - want_pose[t]).abs() <= MM,
            "instance moved: {got:?} vs {want_pose:?}"
        );
    }
}

fn split_city() -> Project {
    let mut doc = Document::new();
    UTM49S.write(&mut doc).unwrap();
    let mut id = 0;
    for gx in -2..=2 {
        for gy in -1..=1 {
            id += 1;
            add_box(
                &mut doc,
                gx as f64 * 1_100.0,
                gy as f64 * 900.0,
                8.0,
                6.0,
                id,
            );
        }
    }
    Project::from_document(doc).unwrap()
}

#[test]
fn container_round_trips_byte_identically() {
    let p = split_city();
    assert!(p.tiles.len() > 1);
    let bytes = p.save();
    let back = Project::open(&bytes).unwrap();
    assert_eq!(back.origin, Some(UTM49S));
    assert_eq!(
        back.tiles.keys().collect::<Vec<_>>(),
        p.tiles.keys().collect::<Vec<_>>()
    );
    assert_eq!(back.save(), bytes, "save -> open -> save changed bytes");
    // Each tile alone is georeferenced at its own center.
    for (key, tile) in &back.tiles {
        let o = ProjectOrigin::read(tile).unwrap().unwrap();
        let c = key.center();
        assert!(
            (0..3).all(|k| (o.origin[k] - (UTM49S.origin[k] + c[k])).abs() <= MM),
            "tile {key:?} origin {:?}",
            o.origin
        );
    }
}

/// Rewrites `bytes` (a container) with `edit` applied to its entries.
fn tamper(bytes: &[u8], edit: impl Fn(&mut BTreeMap<String, Vec<u8>>)) -> Vec<u8> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut entries: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for k in 0..zip.len() {
        let mut f = zip.by_index(k).unwrap();
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).unwrap();
        entries.insert(f.name().unwrap().into_owned(), buf);
    }
    edit(&mut entries);
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, buf) in entries {
        out.start_file(
            name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
        out.write_all(&buf).unwrap();
    }
    out.finish().unwrap().into_inner()
}

fn edit_manifest(bytes: &[u8], edit: impl Fn(&mut serde_json::Value)) -> Vec<u8> {
    tamper(bytes, |e| {
        let mut m: serde_json::Value = serde_json::from_slice(&e[MANIFEST]).unwrap();
        edit(&mut m);
        e.insert(MANIFEST.into(), serde_json::to_vec(&m).unwrap());
    })
}

#[test]
fn tampered_containers_are_refused() {
    let good = split_city().save();
    let cases: Vec<(&str, Vec<u8>)> = vec![
        (
            "origin moved",
            edit_manifest(&good, |m| m["origin"][0] = 430_001.0.into()),
        ),
        (
            "origin without epsg",
            edit_manifest(&good, |m| m["epsg"] = serde_json::Value::Null),
        ),
        (
            "wrong version",
            edit_manifest(&good, |m| m["version"] = 2.into()),
        ),
        (
            "tile size",
            edit_manifest(&good, |m| m["tile_size_m"] = 500.into()),
        ),
        (
            "no tiles",
            edit_manifest(&good, |m| m["tiles"] = serde_json::json!([])),
        ),
        (
            "keys swapped, files kept",
            edit_manifest(&good, |m| {
                let t = m["tiles"].as_array_mut().unwrap();
                let (a, b) = (t[0]["file"].clone(), t[1]["file"].clone());
                t[0]["file"] = b;
                t[1]["file"] = a;
            }),
        ),
        (
            // Manifest and entry name agree, but the tile document's own key
            // and origin still say it is another tile.
            "tile relabeled",
            tamper(&good, |e| {
                let mut m: serde_json::Value = serde_json::from_slice(&e[MANIFEST]).unwrap();
                let t = &mut m["tiles"][0];
                let old = t["file"].as_str().unwrap().to_string();
                let i = t["i"].as_i64().unwrap() + 20;
                let new = format!("tiles/{i}_{}.hew", t["j"]);
                t["i"] = i.into();
                t["file"] = new.clone().into();
                let doc = e.remove(&old).unwrap();
                e.insert(new, doc);
                e.insert(MANIFEST.into(), serde_json::to_vec(&m).unwrap());
            }),
        ),
        (
            "tile listed twice",
            edit_manifest(&good, |m| {
                let t = m["tiles"].as_array_mut().unwrap();
                let first = t[0].clone();
                t.push(first);
            }),
        ),
        (
            "tile missing",
            tamper(&good, |e| {
                let name = e.keys().find(|n| n.starts_with("tiles/")).unwrap().clone();
                e.remove(&name);
            }),
        ),
        (
            "extra entry",
            tamper(&good, |e| {
                e.insert("tiles/99_99.hew".into(), b"x".to_vec());
            }),
        ),
        (
            "single-tile container",
            edit_manifest(&good, |m| {
                m["tiles"] = serde_json::json!([{"i": 0, "j": 0, "file": "tiles/0_0.hew"}]);
            }),
        ),
        (
            "manifest not json",
            tamper(&good, |e| {
                e.insert(MANIFEST.into(), b"{".to_vec());
            }),
        ),
    ];
    for (what, bytes) in cases {
        assert!(Project::open(&bytes).is_err(), "{what}: accepted");
    }
    assert!(Project::open(&good).is_ok(), "untampered container refused");
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 24, ..ProptestConfig::default() })]

    /// Random boxes over ±5 km: splitting is lossless and every object lands
    /// in its anchor's tile; the result survives a container round trip.
    #[test]
    fn split_is_lossless(
        boxes in prop::collection::vec(
            (-5_000.0f64..5_000.0, -5_000.0f64..5_000.0, 1.0f64..40.0, 1.0f64..40.0),
            1..12,
        )
    ) {
        let mut doc = Document::new();
        for (k, (x, y, w, d)) in boxes.iter().enumerate() {
            add_box(&mut doc, *x, *y, *w, *d, k as i64);
        }
        let split = horizontal_reach(&doc) > KERNEL_FRAME_BOUND_M;
        let src = Document::load(&doc.save()).unwrap();
        let p = Project::from_document(doc).unwrap();
        assert_lossless(&src, &p, split);
        let back = Project::open(&p.save()).unwrap();
        assert_lossless(&src, &back, split);
    }
}
