//! WangunGIS spike for ADR-000 criterion #5 (extension points): an
//! out-of-tree crate on top of Hew's `kernel` that stores a project CRS +
//! f64 origin and per-building attributes in Hew's attribute dictionaries,
//! round-trips them through `.hew`, and exports the Objects as CityJSON 2.0
//! LOD1 with normal-derived semantic surfaces. No change to Hew itself.
//! Usage: cargo run -- <out.city.json> <out.hew>

use kernel::attr::AttrValue;
use kernel::{AttrTarget, Document, EntityRef, KernelOp, ObjectId, Plane, Point3, Vec3, tol};
use std::collections::BTreeMap;

const NS: &str = "wangungis";

fn p(x: f64, y: f64, z: f64) -> Point3 {
    Point3::new(x, y, z)
}

/// Box on the ground with footprint corner (x0, y0), rotated `rot` rad, in LOCAL coordinates.
fn make_box(doc: &mut Document, x0: f64, y0: f64, rot: f64, w: f64, d: f64, h: f64) -> ObjectId {
    let (s, c) = rot.sin_cos();
    let at = |u: f64, v: f64| p(x0 + u * c - v * s, y0 + u * s + v * c, 0.0);
    let plane = Plane::from_polygon(&[p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0)]).unwrap();
    let sk = doc.add_sketch(plane);
    let corners = [at(0.0, 0.0), at(w, 0.0), at(w, d), at(0.0, d)];
    for k in 0..4 {
        doc.sketch_mut(sk).unwrap().add_segment(corners[k], corners[(k + 1) % 4]).unwrap();
    }
    let regions = doc.extrudable_regions(sk).unwrap();
    doc.extrude_region(sk, regions[0], h).unwrap().0
}

fn text(s: &str) -> AttrValue {
    AttrValue::Text(s.into())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (out_json, out_hew) = (&args[1], &args[2]);
    let origin = [430_000.123_f64, 9_150_000.456, 0.0];

    // --- model in local coordinates ---
    let mut doc = Document::new();
    let a = make_box(&mut doc, 0.0, 0.0, 0.0, 10.0, 8.0, 6.0);
    let b = make_box(&mut doc, 20.0, 5.0, 30f64.to_radians(), 12.0, 9.0, 9.0);

    // --- extension data in Hew attribute dictionaries (no kernel change) ---
    doc.attr_set(AttrTarget::Document, NS, "crs", text("EPSG:32749")).unwrap();
    doc.attr_set(AttrTarget::Document, NS, "origin",
        AttrValue::List(origin.iter().map(|v| AttrValue::Float(*v)).collect())).unwrap();
    for (obj, func) in [(a, "residential"), (b, "office")] {
        doc.attr_set(AttrTarget::Entity(EntityRef::Object(obj)), NS, "function", text(func)).unwrap();
    }

    // Edits AFTER the attributes are set: push/pull, undo, redo. The object
    // attribute must survive all of them (and the reload below).
    let top = doc.object(b).unwrap().faces().iter()
        .find(|(_, f)| f.plane.normal().approx_eq(Vec3::new(0.0, 0.0, 1.0), tol::NORMAL_DIRECTION))
        .map(|(id, _)| id).unwrap();
    doc.apply_object_op(b, KernelOp::PushPull { face: top, distance: 1.5 }).unwrap();
    doc.undo().unwrap();
    doc.redo().unwrap();
    let func_of = |doc: &Document, o: ObjectId| doc.attr_get(&AttrTarget::Entity(EntityRef::Object(o))).unwrap()
        .and_then(|d| d.get(NS)).and_then(|n| n.get("function")).cloned();
    assert_eq!(func_of(&doc, b), Some(text("office")), "object attr survives push/pull + undo + redo");

    // --- round trip through .hew ---
    let bytes = doc.save();
    std::fs::write(out_hew, &bytes).unwrap();
    let doc = Document::load(&bytes).expect("reload .hew");
    let dict = doc.attr_get(&AttrTarget::Document).unwrap().expect("document attrs survive");
    let ns = &dict[NS];
    let crs = match &ns["crs"] { AttrValue::Text(t) => t.clone(), v => panic!("crs {v:?}") };
    let org: Vec<f64> = match &ns["origin"] {
        AttrValue::List(l) => l.iter().map(|v| match v { AttrValue::Float(f) => *f, _ => panic!() }).collect(),
        v => panic!("origin {v:?}"),
    };
    assert_eq!(org, origin, "origin must round-trip bit-exactly");
    let funcs: Vec<_> = doc.live_object_ids().into_iter().filter_map(|o| {
        doc.attr_get(&AttrTarget::Entity(EntityRef::Object(o))).ok().flatten()
            .and_then(|d| d.get(NS)).and_then(|n| n.get("function")).cloned()
    }).collect();
    assert_eq!(funcs.len(), 2, "both object attrs survive reload");
    let top_z = doc.live_object_ids().iter().filter_map(|o| doc.object(*o))
        .flat_map(|o| o.vertices().values().map(|v| v.position.z)).fold(0.0, f64::max);
    assert!((top_z - 10.5).abs() < 1e-9, "redone push/pull survives reload: top {top_z}");

    // --- export CityJSON 2.0: integer vertices in mm relative to the origin ---
    let scale = 0.001;
    let mut verts: Vec<[i64; 3]> = Vec::new();
    let mut index: BTreeMap<[i64; 3], usize> = BTreeMap::new();
    let mut city_objects = serde_json::Map::new();
    for oid in doc.live_object_ids() {
        if !doc.is_world_object(oid) { continue; }
        let obj = doc.object(oid).unwrap();
        let mut shell = Vec::new();
        let mut sem = Vec::new();
        for (_, f) in obj.faces().iter() {
            let mut rings = Vec::new();
            for l in std::iter::once(f.outer_loop).chain(f.inner_loops.iter().copied()) {
                let ring: Vec<usize> = obj.loop_positions(l).map(|q| {
                    let k = [(q.x / scale).round() as i64, (q.y / scale).round() as i64, (q.z / scale).round() as i64];
                    *index.entry(k).or_insert_with(|| { verts.push(k); verts.len() - 1 })
                }).collect();
                rings.push(ring);
            }
            shell.push(rings);
            let nz = f.plane.normal().z;
            sem.push(if nz > 0.9 { 0 } else if nz < -0.9 { 1 } else { 2 });
        }
        let func = doc.attr_get(&AttrTarget::Entity(EntityRef::Object(oid))).unwrap()
            .and_then(|d| d.get(NS)).and_then(|n| n.get("function"))
            .map(|v| match v { AttrValue::Text(t) => t.clone(), _ => String::new() }).unwrap_or_default();
        city_objects.insert(format!("{oid:?}"), serde_json::json!({
            "type": "Building",
            "attributes": { "function": func },
            "geometry": [{
                "type": "Solid", "lod": "1",
                "boundaries": [shell],
                "semantics": {
                    "surfaces": [{"type": "RoofSurface"}, {"type": "GroundSurface"}, {"type": "WallSurface"}],
                    "values": [sem]
                }
            }]
        }));
    }
    let epsg = crs.trim_start_matches("EPSG:");
    let cj = serde_json::json!({
        "type": "CityJSON", "version": "2.0",
        "metadata": { "referenceSystem": format!("https://www.opengis.net/def/crs/EPSG/0/{epsg}") },
        "transform": { "scale": [scale, scale, scale], "translate": org },
        "CityObjects": city_objects,
        "vertices": verts,
    });
    std::fs::write(out_json, serde_json::to_string_pretty(&cj).unwrap()).unwrap();
    println!("ok: {} buildings, {} vertices, crs {crs}, origin {org:?}, .hew {} bytes", city_objects_len(&cj), cj["vertices"].as_array().unwrap().len(), bytes.len());
}

fn city_objects_len(cj: &serde_json::Value) -> usize {
    cj["CityObjects"].as_object().unwrap().len()
}
