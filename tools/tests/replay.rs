//! Random edit sessions: whatever the sequence, every solid stays valid, replaying
//! the same inputs gives the same file byte for byte, saving a reopened file does
//! too, and undoing everything gives the empty project back. `PROPTEST_CASES=10000` is the v0.1 exit check.

use std::num::NonZeroU32;

use inference::{ApertureMode, PickRay, SnapQuery, SnapWeights};
use kernel::{NodeId, Plane, Point3, Rgba8, Transform, Vec3};
use proptest::prelude::*;
use wangungis_scene::ProjectOrigin;
use wangungis_scene::project::{Project, TileKey};
use wangungis_tools::{Hit, Movable, Paint, Scene};

const UTM49S: ProjectOrigin = ProjectOrigin {
    epsg: 32749,
    origin: [430_000.123, 9_150_000.456, 12.5],
};

/// Coordinates are small grids so that operations meet each other's geometry.
#[derive(Debug, Clone)]
enum Op {
    Rect {
        x: i8,
        y: i8,
        w: u8,
        d: u8,
    },
    OnTop {
        x: i8,
        y: i8,
        w: u8,
        d: u8,
    },
    Extrude {
        x: i8,
        y: i8,
        h: i8,
    },
    Pull {
        x: i8,
        y: i8,
        h: i8,
    },
    Move {
        x: i8,
        y: i8,
        dx: i8,
        dy: i8,
        dz: i8,
    },
    Copy {
        x: i8,
        y: i8,
        dx: i8,
        n: u8,
    },
    Turn {
        x: i8,
        y: i8,
        degrees: i16,
    },
    Scale {
        x: i8,
        y: i8,
        tenths: u8,
    },
    Offset {
        x: i8,
        y: i8,
        tenths: i8,
    },
    Erase {
        x: i8,
        y: i8,
    },
    /// Push/Pull with Ctrl: the face leaves as a new solid.
    Keep {
        x: i8,
        y: i8,
        h: i8,
    },
    Paint {
        x: i8,
        y: i8,
        color: u8,
    },
    Group {
        x: i8,
        y: i8,
    },
    Component {
        x: i8,
        y: i8,
    },
    Enter {
        x: i8,
        y: i8,
    },
    /// Opens whatever group or component there is, so sessions really happen.
    EnterAny,
    /// Groups the first solid there is, so there are groups to open.
    GroupAny,
    Leave,
    Tag {
        x: i8,
        y: i8,
    },
    /// Shows or hides the tag `Tag` gives.
    HideTag(bool),
    Rename {
        x: i8,
        y: i8,
    },
    Undo,
    Redo,
}

fn op() -> impl Strategy<Value = Op> {
    let at = -2i8..14;
    prop_oneof![
        3 => (at.clone(), at.clone(), 1u8..12, 1u8..12).prop_map(|(x, y, w, d)| Op::Rect { x, y, w, d }),
        2 => (at.clone(), at.clone(), 1u8..6, 1u8..6).prop_map(|(x, y, w, d)| Op::OnTop { x, y, w, d }),
        3 => (at.clone(), at.clone(), -5i8..9).prop_map(|(x, y, h)| Op::Extrude { x, y, h }),
        3 => (at.clone(), at.clone(), -5i8..6).prop_map(|(x, y, h)| Op::Pull { x, y, h }),
        2 => (at.clone(), at.clone(), -20i8..20, -20i8..20, -3i8..3)
            .prop_map(|(x, y, dx, dy, dz)| Op::Move { x, y, dx, dy, dz }),
        2 => (at.clone(), at.clone(), 5i8..20, 1u8..5).prop_map(|(x, y, dx, n)| Op::Copy { x, y, dx, n }),
        1 => (at.clone(), at.clone(), -180i16..180).prop_map(|(x, y, degrees)| Op::Turn { x, y, degrees }),
        1 => (at.clone(), at.clone(), 3u8..30).prop_map(|(x, y, tenths)| Op::Scale { x, y, tenths }),
        1 => (at.clone(), at.clone(), -10i8..10).prop_map(|(x, y, tenths)| Op::Offset { x, y, tenths }),
        1 => (at.clone(), at.clone()).prop_map(|(x, y)| Op::Erase { x, y }),
        2 => (at.clone(), at.clone(), -5i8..6).prop_map(|(x, y, h)| Op::Keep { x, y, h }),
        2 => (at.clone(), at.clone(), 0u8..4).prop_map(|(x, y, color)| Op::Paint { x, y, color }),
        1 => (at.clone(), at.clone()).prop_map(|(x, y)| Op::Group { x, y }),
        1 => (at.clone(), at.clone()).prop_map(|(x, y)| Op::Component { x, y }),
        2 => (at.clone(), at.clone()).prop_map(|(x, y)| Op::Enter { x, y }),
        3 => Just(Op::EnterAny),
        2 => Just(Op::GroupAny),
        2 => Just(Op::Leave),
        1 => (at.clone(), at.clone()).prop_map(|(x, y)| Op::Tag { x, y }),
        1 => any::<bool>().prop_map(Op::HideTag),
        1 => (at.clone(), at).prop_map(|(x, y)| Op::Rename { x, y }),
        2 => Just(Op::Undo),
        1 => Just(Op::Redo),
    ]
}

fn horizontal(z: f64) -> Plane {
    Plane::from_point_normal(Point3::new(0.0, 0.0, z), Vec3::new(0.0, 0.0, 1.0)).unwrap()
}

fn ring(x: f64, y: f64, z: f64, w: f64, d: f64) -> Vec<(Point3, Point3)> {
    let p = |a: f64, b: f64| Point3::new(x + a, y + b, z);
    let r = [p(0.0, 0.0), p(w, 0.0), p(w, d), p(0.0, d)];
    (0..4).map(|k| (r[k], r[(k + 1) % 4])).collect()
}

/// What lies straight down at (x, y), off the integer grid the drawing is on.
fn inside(s: &mut Scene, x: f64, y: f64) -> Option<Hit> {
    below(s, x + 0.37, y + 0.41)
}

/// What lies straight down at (x, y).
fn below(s: &mut Scene, x: f64, y: f64) -> Option<Hit> {
    let ray = PickRay {
        origin: Point3::new(x, y, 200.0),
        direction: Vec3::new(0.0, 0.0, -1.0),
    };
    s.snap(&SnapQuery {
        ray,
        anchor: None,
        lock: None,
        aperture: 1e-3,
        aperture_mode: ApertureMode::Cone,
        constraint_plane: None,
        weights: SnapWeights::standard().for_picking(),
        soft_axis_aperture_scale: None,
        off_plane_points: false,
        see_through_points: false,
    })
}

/// The node Move would take at (x, y).
fn node_at(s: &mut Scene, x: f64, y: f64) -> Option<(TileKey, NodeId)> {
    match inside(s, x, y).and_then(|h| s.movable_at(&h))? {
        Movable::Node { tile, node } => Some((tile, node)),
        Movable::Island { .. } => None,
    }
}

/// A refused op is fine: refusals must leave the project as it was, which the
/// caller checks. Only panics and broken solids fail the test.
fn apply(s: &mut Scene, op: &Op) {
    let f = |v: i8| f64::from(v);
    match *op {
        Op::Rect { x, y, w, d } => {
            let _ = s.draw(
                None,
                horizontal(0.0),
                &ring(f(x), f(y), 0.0, f(w as i8), f(d as i8)),
            );
        }
        Op::OnTop { x, y, w, d } => {
            if let Some(h) = below(s, f(x), f(y)) {
                let z = h.snap.position.z;
                let _ = s.draw(
                    Some(h.tile),
                    horizontal(z),
                    &ring(f(x), f(y), z, f(w as i8), f(d as i8)),
                );
            }
        }
        Op::Extrude { x, y, h } => {
            if let Some(hit) = inside(s, f(x), f(y))
                && let Some((sketch, region)) = hit.snap.sketch_region_source
                && h != 0
            {
                let _ = s.extrude(hit.tile, sketch, region, f(h));
            }
        }
        Op::Pull { x, y, h } => {
            if let Some(hit) = inside(s, f(x), f(y))
                && let Some(source) = hit.snap.source
                && h != 0
            {
                let _ = s.push_pull(hit.tile, source, f(h));
            }
        }
        Op::Move { x, y, dx, dy, dz } => {
            if let Some(m) = inside(s, f(x), f(y)).and_then(|h| s.movable_at(&h)) {
                let d = Vec3::new(f(dx), f(dy), f(dz));
                let _ = s.transform(&m, &Transform::translation(d));
            }
        }
        Op::Copy { x, y, dx, n } => {
            if let Some(m) = inside(s, f(x), f(y)).and_then(|h| s.movable_at(&h)) {
                let d = Transform::translation(Vec3::new(f(dx), 0.0, 0.0));
                let _ = s.copy(&m, &d, NonZeroU32::new(u32::from(n)).unwrap());
            }
        }
        Op::Turn { x, y, degrees } => {
            if let Some(m) = inside(s, f(x), f(y)).and_then(|h| s.movable_at(&h)) {
                let spin =
                    Transform::rotation(Vec3::new(0.0, 0.0, 1.0), f64::from(degrees).to_radians());
                let _ = s.transform(&m, &spin.unwrap());
            }
        }
        Op::Scale { x, y, tenths } => {
            if let Some(m) = inside(s, f(x), f(y)).and_then(|h| s.movable_at(&h)) {
                let _ = s.transform(&m, &Transform::uniform_scale(f64::from(tenths) / 10.0));
            }
        }
        Op::Offset { x, y, tenths } => {
            if let Some(hit) = inside(s, f(x), f(y))
                && tenths != 0
            {
                let z = hit.snap.position.z;
                let _ = s.offset(
                    hit.tile,
                    horizontal(z),
                    hit.snap.position,
                    f64::from(tenths) / 10.0,
                );
            }
        }
        Op::Erase { x, y } => {
            if let Some(e) = below(s, f(x), f(y)).and_then(|h| s.erasable_at(&h)) {
                let _ = s.erase(&e);
            }
        }
        Op::Keep { x, y, h } => {
            if let Some(hit) = inside(s, f(x), f(y))
                && let Some(source) = hit.snap.source
                && h != 0
            {
                let _ = s.extrude_face_new(hit.tile, source, f(h));
            }
        }
        Op::Paint { x, y, color } => {
            if let Some(hit) = inside(s, f(x), f(y))
                && let Some(source) = hit.snap.source
            {
                let paint = (color > 0).then(|| Paint {
                    name: format!("Warna {color}"),
                    color: Rgba8::rgb(color * 60, 40, 200 - color * 40),
                });
                let _ = s.paint(hit.tile, source, paint.as_ref());
            }
        }
        Op::Group { x, y } => {
            if let Some((tile, node)) = node_at(s, f(x), f(y)) {
                let _ = s.group(tile, &[node]);
            }
        }
        Op::Component { x, y } => {
            if let Some((tile, node)) = node_at(s, f(x), f(y)) {
                let _ = s.make_component(tile, &[node]);
            }
        }
        Op::Enter { x, y } => {
            if let Some((tile, node)) = node_at(s, f(x), f(y)) {
                let _ = s.enter(tile, node);
            }
        }
        Op::EnterAny => {
            let found = s
                .outline_rows(&Default::default())
                .into_iter()
                .find(|r| !matches!(r.node, NodeId::Object(_)) && s.node_in_scope(r.tile, r.node));
            if let Some(r) = found {
                let _ = s.enter(r.tile, r.node);
            }
        }
        Op::GroupAny => {
            let found = s
                .outline_rows(&Default::default())
                .into_iter()
                .find(|r| matches!(r.node, NodeId::Object(_)) && s.node_in_scope(r.tile, r.node));
            if let Some(r) = found {
                let _ = s.group(r.tile, &[r.node]);
            }
        }
        Op::Leave => {
            let _ = s.leave();
        }
        Op::Tag { x, y } => {
            if let Some((tile, node)) = node_at(s, f(x), f(y)) {
                let _ = s.tag_nodes(tile, &[node], "Lantai");
            }
        }
        Op::HideTag(hidden) => {
            let _ = s.set_tag_hidden("Lantai", hidden);
        }
        Op::Rename { x, y } => {
            if let Some((tile, node)) = node_at(s, f(x), f(y)) {
                let _ = s.rename_node(tile, node, "Rumah");
            }
        }
        Op::Undo => {
            let _ = s.undo();
        }
        Op::Redo => {
            let _ = s.redo();
        }
    }
}

fn assert_valid(s: &Scene, op: &Op) -> Result<(), TestCaseError> {
    for doc in s.project().tiles.values() {
        for id in doc.visible_object_ids() {
            let checked = doc.object(id).unwrap().validate();
            prop_assert!(checked.is_ok(), "{op:?}: {checked:?}");
        }
    }
    Ok(())
}

/// What a project holds, whatever order its tables are in: exact coordinates of
/// every solid vertex and sketch edge, and each solid's face count.
fn content(p: &Project) -> Vec<String> {
    let at = |q: Point3| {
        format!(
            "{:x},{:x},{:x}",
            q.x.to_bits(),
            q.y.to_bits(),
            q.z.to_bits()
        )
    };
    let mut out = Vec::new();
    for (key, doc) in &p.tiles {
        for id in doc.visible_object_ids() {
            let o = doc.object(id).unwrap();
            let mut vs: Vec<String> = o.vertices().values().map(|v| at(v.position)).collect();
            vs.sort();
            out.push(format!(
                "{key:?} solid {} {}",
                o.faces().len(),
                vs.join(" ")
            ));
        }
        for id in doc.visible_sketch_ids() {
            let k = doc.sketch(id).unwrap();
            for e in k.edges().values() {
                let mut ends = [
                    at(k.vertices()[e.from].position),
                    at(k.vertices()[e.to].position),
                ];
                ends.sort();
                out.push(format!("{key:?} edge {} {}", ends[0], ends[1]));
            }
        }
    }
    out.sort();
    out
}

fn session(ops: &[Op]) -> Result<(Scene, Vec<u8>), TestCaseError> {
    let mut s = Scene::new(Project::new(Some(UTM49S)).unwrap());
    // Something to work on, so that most operations meet geometry.
    let seed = [
        Op::Rect {
            x: 0,
            y: 0,
            w: 5,
            d: 4,
        },
        Op::Extrude { x: 1, y: 1, h: 3 },
        Op::Rect {
            x: 7,
            y: 0,
            w: 5,
            d: 8,
        },
    ];
    for op in seed.iter().chain(ops) {
        let could_undo = s.can_undo();
        apply(&mut s, op);
        assert_valid(&s, op)?;
        if matches!(op, Op::Redo) {
            prop_assert!(could_undo || s.can_undo() || !s.can_redo());
        }
    }
    // A file never holds an open group, so what it holds compares only when closed.
    while s.leave().unwrap() {
        assert_valid(&s, &Op::Leave)?;
    }
    let bytes = s.project().save();
    Ok((s, bytes))
}

proptest! {
    #[test]
    fn random_sessions_stay_valid_replay_identically_and_undo_to_nothing(
        ops in prop::collection::vec(op(), 1..24),
    ) {
        let (mut s, first) = session(&ops)?;
        if std::env::var_os("REPLAY_STATS").is_some() {
            let objects: usize = s.project().tiles.values().map(|d| d.visible_object_ids().len()).sum();
            let faces: usize = s.project().tiles.values().map(|d| d.visible_object_ids().iter().map(|i| d.object(*i).unwrap().faces().len()).sum::<usize>()).sum();
            eprintln!("STAT {objects} {faces} {}", s.project().tiles.values().map(|d| d.sketch_ids().len()).sum::<usize>());
        }
        let (_, second) = session(&ops)?;
        prop_assert_eq!(&first, &second, "the same inputs make the same file");

        // Save, load, save is byte-identical whatever the session made.
        let reopened = Project::open(&first).unwrap();
        prop_assert_eq!(&reopened.save(), &first);

        // Redo after undo restores the geometry. Not the bytes: Hew rebuilds a
        // sketch's region table on undo, in another slot order. Undo all the way
        // is the empty project.
        if s.can_undo() {
            s.undo().unwrap();
            assert_valid(&s, &Op::Undo)?;
            s.redo().unwrap();
            prop_assert_eq!(content(s.project()), content(&reopened));
        }
        while s.can_undo() {
            s.undo().unwrap();
            assert_valid(&s, &Op::Undo)?;
        }
        let empty = Project::new(Some(UTM49S)).unwrap();
        prop_assert_eq!(content(s.project()), content(&empty));
        prop_assert!(s.session().is_none());
        // Hew's palette and tag registry keep their entries through undo. Nothing
        // else may remain, so the file is the empty one when neither was touched.
        let registers = ops.iter().any(|o| matches!(o, Op::Paint { .. } | Op::Tag { .. } | Op::HideTag(_)));
        if !registers {
            prop_assert_eq!(s.project().save(), empty.save());
        }
    }
}
