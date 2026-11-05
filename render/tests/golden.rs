//! Golden-image precision test (closes the render half of spike S2, ADR-003):
//! the same box must render identically whether its tile sits at the origin,
//! 40 km out, or at full UTM coordinates, as long as the camera moves with it.
//! Negative control: the same scene WITHOUT RTC (geometry baked at UTM in
//! f32) must render visibly wrong, which proves the test can fail.
//!
//! Needs a GPU adapter (CI: Metal, lavapipe, WARP). Without one the tests
//! skip, unless WANGUNGIS_REQUIRE_GPU is set, in which case they fail.

use kernel::{Document, Material, ObjectId, Plane, Point3, Rgba8, Vec3};
use wangungis_pointcloud::{ChangedChunk, CloudPoint, DrawChunk, Slice};
use wangungis_render::{Camera, DEPTH_FORMAT, OverlayLines, Renderer, TileMesh};
use wangungis_scene::project::Project;

const W: u32 = 256;
const H: u32 = 256;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// GPU tests run one at a time: Windows WARP crashed (0xc0000005) once with
/// three devices created in parallel (PR #7 CI, release). Assumed driver race.
static GPU_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn gpu_lock() -> std::sync::MutexGuard<'static, ()> {
    GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    // Hardware first, then the software fallback; a device must come up too.
    let found = [false, true].into_iter().find_map(|fallback| {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            force_fallback_adapter: fallback,
            ..Default::default()
        }))
        .ok()?;
        let info = adapter.get_info();
        let dq = pollster::block_on(adapter.request_device(&Default::default())).ok()?;
        eprintln!("adapter: {} ({:?})", info.name, info.backend);
        Some(dq)
    });
    if found.is_none() {
        assert!(
            std::env::var_os("WANGUNGIS_REQUIRE_GPU").is_none(),
            "no usable GPU device but WANGUNGIS_REQUIRE_GPU is set"
        );
        eprintln!("SKIP: no usable GPU device");
    }
    found
}

/// Axis-aligned 5 x 4.244 x 3 m box with its corner at (x, y, 0). Axis-aligned
/// boxes are accepted by the kernel even at UTM (spike S2), which the
/// negative control needs.
fn box_doc(x: f64, y: f64) -> Document {
    let mut doc = Document::new();
    add_box(&mut doc, x, y);
    doc
}

fn add_box(doc: &mut Document, x: f64, y: f64) -> ObjectId {
    let p = |a: f64, b: f64| Point3::new(x + a, y + b, 0.0);
    let ring = [p(0.0, 0.0), p(5.0, 0.0), p(5.0, 4.244), p(0.0, 4.244)];
    let sk = doc.add_sketch(Plane::from_polygon(&ring).unwrap());
    let s = doc.sketch_mut(sk).unwrap();
    for k in 0..4 {
        s.add_segment(ring[k], ring[(k + 1) % 4]).unwrap();
    }
    let r = doc.extrudable_regions(sk).unwrap();
    doc.extrude_region(sk, r[0], 3.0).unwrap().0
}

fn face_with_normal(doc: &Document, object: ObjectId, normal: Vec3) -> kernel::FaceId {
    doc.object(object)
        .unwrap()
        .faces()
        .iter()
        .find(|(_, face)| {
            let n = face.plane.normal();
            (n.x - normal.x).abs() < 1e-9
                && (n.y - normal.y).abs() < 1e-9
                && (n.z - normal.z).abs() < 1e-9
        })
        .map(|(id, _)| id)
        .unwrap()
}

/// Camera looking at the box whose corner is at `at` (project-local).
fn camera(at: [f64; 3]) -> Camera {
    let o = |d: [f64; 3]| [at[0] + d[0], at[1] + d[1], at[2] + d[2]];
    Camera {
        eye: o([-6.0, -9.0, 7.0]),
        target: o([2.5, 2.122, 1.5]),
        fovy: 0.8,
        aspect: W as f32 / H as f32,
        near: 0.5,
        far: 200.0,
    }
}

/// Renders `tiles` (frame origin, kernel document) and returns RGBA8 pixels
/// plus the draw-call count.
fn render(
    dq: &(wgpu::Device, wgpu::Queue),
    tiles: &[([f64; 3], &Document)],
    cam: &Camera,
) -> (Vec<u8>, usize) {
    let mut r = Renderer::new(&dq.0, FORMAT);
    for (origin, doc) in tiles {
        r.add_tile(&dq.0, *origin, &TileMesh::from_document(doc).unwrap());
    }
    shoot(dq, &r, cam)
}

/// Draws `r` from `cam` into a fresh target.
fn shoot(dq: &(wgpu::Device, wgpu::Queue), r: &Renderer, cam: &Camera) -> (Vec<u8>, usize) {
    let (device, queue) = dq;
    let tex = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: W,
                height: H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let color = tex(
        FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = tex(DEPTH_FORMAT, wgpu::TextureUsages::RENDER_ATTACHMENT);
    let (cv, dv) = (
        color.create_view(&Default::default()),
        depth.create_view(&Default::default()),
    );
    let row = (W * 4).div_ceil(256) * 256;
    let out = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (row * H) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    r.prepare(queue, cam);
    let mut enc = device.create_command_encoder(&Default::default());
    let draws = {
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &cv,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &dv,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        r.draw(&mut pass)
    };
    enc.copy_texture_to_buffer(
        color.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &out,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(H),
            },
        },
        wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([enc.finish()]);
    out.slice(..).map_async(wgpu::MapMode::Read, |r| r.unwrap());
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let data = out.slice(..).get_mapped_range().expect("mapped");
    let px = (0..H as usize)
        .flat_map(|y| data[y * row as usize..y * row as usize + (W * 4) as usize].to_vec())
        .collect();
    (px, draws)
}

/// Fraction of pixels whose largest channel difference exceeds 16/255.
fn diff(a: &[u8], b: &[u8]) -> f64 {
    let bad = a
        .chunks(4)
        .zip(b.chunks(4))
        .filter(|(p, q)| p.iter().zip(*q).any(|(x, y)| x.abs_diff(*y) > 16))
        .count();
    bad as f64 / (W * H) as f64
}

/// Fraction of non-background pixels: guards against a blank render passing.
fn coverage(px: &[u8]) -> f64 {
    px.chunks(4).filter(|p| p[..3] != [255, 255, 255]).count() as f64 / (W * H) as f64
}

#[test]
fn painted_face_uses_its_material_and_unpainted_face_stays_gray() {
    let _serial = gpu_lock();
    let Some(dq) = gpu() else { return };
    let mut doc = Document::new();
    let object = add_box(&mut doc, 0.0, 0.0);
    let red = doc.add_material(Material::solid("Red", Rgba8::rgb(240, 20, 20)));
    let face = face_with_normal(&doc, object, Vec3::new(-1.0, 0.0, 0.0));
    doc.paint_face(object, face, Some(red)).unwrap();

    let (px, _) = render(&dq, &[([0.0; 3], &doc)], &camera([0.0; 3]));
    let red = px
        .chunks(4)
        .filter(|p| p[0] > p[1].saturating_mul(3) && p[0] > p[2].saturating_mul(3))
        .count();
    let gray = px
        .chunks(4)
        .filter(|p| p[0] > 40 && p[0] < 240 && p[0] == p[1] && p[1] == p[2])
        .count();
    assert!(red > 100, "painted side has only {red} strongly red pixels");
    assert!(gray > 100, "unpainted side has only {gray} gray pixels");
}

#[test]
fn unpainted_faces_render_neutral_gray() {
    let _serial = gpu_lock();
    let Some(dq) = gpu() else { return };
    let (px, _) = render(&dq, &[([0.0; 3], &box_doc(0.0, 0.0))], &camera([0.0; 3]));
    let faces: Vec<_> = px.chunks(4).filter(|p| p[0] < 240 && p[0] > 40).collect();
    assert!(
        faces.len() > 100,
        "box has only {} face pixels",
        faces.len()
    );
    assert!(
        faces.iter().all(|p| p[0] == p[1] && p[1] == p[2]),
        "an unpainted face is not neutral gray"
    );
}

/// With a group open, what is outside it fades, and closing brings it back.
#[test]
fn outside_an_open_group_fades() {
    let _serial = gpu_lock();
    let Some(dq) = gpu() else { return };
    let mut doc = Document::new();
    // The box in view stays outside; the group is out of frame.
    add_box(&mut doc, 0.0, 0.0);
    let far = add_box(&mut doc, 500.0, 0.0);
    let (group, _) = doc.group_nodes(&[kernel::NodeId::Object(far)]).unwrap();
    let dark = |px: &[u8]| px.chunks(4).filter(|p| p[0] < 150).count();
    let (closed, _) = render(&dq, &[([0.0; 3], &doc)], &camera([0.0; 3]));
    doc.open_group_session(group).unwrap();
    let (open, _) = render(&dq, &[([0.0; 3], &doc)], &camera([0.0; 3]));
    assert!(
        dark(&open) < dark(&closed),
        "open {} closed {}",
        dark(&open),
        dark(&closed)
    );
    doc.close_group_session().unwrap();
    let (again, _) = render(&dq, &[([0.0; 3], &doc)], &camera([0.0; 3]));
    assert_eq!(again, closed);
}

#[test]
fn far_tiles_render_like_the_origin() {
    let _serial = gpu_lock();
    let Some(dq) = gpu() else { return };
    let doc = box_doc(0.0, 0.0);
    let (reference, draws) = render(&dq, &[([0.0; 3], &doc)], &camera([0.0; 3]));
    assert_eq!(draws, 2);
    let cov = coverage(&reference);
    assert!(cov > 0.05, "box barely visible: coverage {cov}");
    // The box near the tile frame and, for the worst case inside a kernel
    // tile, ~1.6 km from it (f32 positions there resolve 0.12 mm).
    let edge_of_tile = (1200.0, -1100.0);
    let far_doc = box_doc(edge_of_tile.0, edge_of_tile.1);
    for origin in [
        [40_000.0, -40_000.0, 0.0],
        [430_000.123, 9_150_000.456, 12.0],
    ] {
        for (in_tile, d) in [((0.0, 0.0), &doc), (edge_of_tile, &far_doc)] {
            let at = [origin[0] + in_tile.0, origin[1] + in_tile.1, origin[2]];
            let (img, _) = render(&dq, &[(origin, d)], &camera(at));
            let diff = diff(&reference, &img);
            eprintln!(
                "DIFF tile at {origin:?}, box at {in_tile:?} in tile: {:.4}%",
                diff * 100.0
            );
            assert!(
                diff <= 0.001,
                "tile {origin:?}, box {in_tile:?}: {:.2}% pixels differ",
                diff * 100.0
            );
        }
    }
}

#[test]
fn without_rtc_utm_renders_wrong() {
    let _serial = gpu_lock();
    let Some(dq) = gpu() else { return };
    let (reference, _) = render(&dq, &[([0.0; 3], &box_doc(0.0, 0.0))], &camera([0.0; 3]));
    // Geometry baked at UTM in the tile's f32 buffer, tile frame at 0: no RTC.
    let (x, y) = (430_000.123, 9_150_000.456);
    let (img, _) = render(&dq, &[([0.0; 3], &box_doc(x, y))], &camera([x, y, 0.0]));
    // Not blank: the box is drawn, just wrongly.
    assert!(coverage(&img) > 0.05, "no-RTC render is blank");
    let d = diff(&reference, &img);
    eprintln!("DIFF without RTC: {:.4}%", d * 100.0);
    // Measured 1.1–1.2% on Apple M4 (the 4.244 m depth snaps to the 1 m f32 grid
    // at Y = 9.15e6); RTC renders differ by 0.00%. 0.5% separates the two.
    assert!(
        d > 0.005,
        "no-RTC render should be visibly wrong, only {:.2}% differs",
        d * 100.0
    );
}

/// Two tiles, both in view: 2 draws each, and the second tile's own uniform
/// puts its box beside the first (more pixels covered than one tile alone).
#[test]
fn two_tiles_two_draws_each() {
    let _serial = gpu_lock();
    let Some(dq) = gpu() else { return };
    let doc = box_doc(0.0, 0.0);
    let (one, _) = render(&dq, &[([0.0; 3], &doc)], &camera([0.0; 3]));
    let (two, draws) = render(
        &dq,
        &[([0.0; 3], &doc), ([7.0, 0.0, 0.0], &doc)],
        &camera([0.0; 3]),
    );
    assert_eq!(draws, 4);
    let (c1, c2) = (coverage(&one), coverage(&two));
    assert!(
        c2 > c1 * 1.3,
        "second tile not drawn where expected: {c1:.3} -> {c2:.3}"
    );
}

/// End to end with `scene`: a document split into kernel tiles renders like
/// the unsplit source, looking at the tile corner (500, 500) where four
/// neighbouring boxes land in four different tiles.
#[test]
fn split_project_renders_like_the_source() {
    let _serial = gpu_lock();
    let Some(dq) = gpu() else { return };
    let mut doc = Document::new();
    // Box anchors (x + 2.5, y + 2.122): one per tile (0,0), (1,0), (0,1), (1,1).
    for (x, y) in [
        (490.0, 490.0),
        (504.0, 490.0),
        (490.0, 504.0),
        (504.0, 504.0),
    ] {
        add_box(&mut doc, x, y);
    }
    // Pushes the reach past the kernel frame bound, so the document splits.
    add_box(&mut doc, 3_000.0, 0.0);
    let cam = Camera {
        eye: [480.0, 470.0, 30.0],
        target: [500.0, 500.0, 1.5],
        fovy: 0.8,
        aspect: W as f32 / H as f32,
        near: 0.5,
        far: 200.0,
    };
    let (source, _) = render(&dq, &[([0.0; 3], &doc)], &cam);
    assert!(coverage(&source) > 0.05, "boxes barely visible");
    let project = Project::from_document(doc).unwrap();
    // Exactly one box per tile: a box copied but not removed would show here.
    let per_tile: Vec<_> = project
        .tiles
        .iter()
        .map(|(k, d)| ((k.i, k.j), d.top_level_nodes().len()))
        .collect();
    assert_eq!(
        per_tile,
        [
            ((0, 0), 1),
            ((0, 1), 1),
            ((1, 0), 1),
            ((1, 1), 1),
            ((3, 0), 1)
        ]
    );
    let tiles: Vec<_> = project.tiles.iter().map(|(k, d)| (k.center(), d)).collect();
    let (img, draws) = render(&dq, &tiles, &cam);
    assert_eq!(draws, 10);
    let d = diff(&source, &img);
    eprintln!("DIFF split vs source: {:.4}%", d * 100.0);
    assert!(
        d <= 0.001,
        "split project renders {:.2}% differently",
        d * 100.0
    );
    // The other tiles really contribute: tile (0, 0) alone looks different.
    let (alone, _) = render(&dq, &tiles[..1], &cam);
    assert!(
        diff(&source, &alone) > 0.005,
        "tiles beyond (0, 0) add nothing"
    );
}

/// A tile uploaded again at the same origin replaces the old one.
#[test]
fn add_tile_replaces_the_tile_at_that_origin() {
    let _serial = gpu_lock();
    let Some(dq) = gpu() else { return };
    let cam = camera([0.0; 3]);
    let (a, b) = (box_doc(0.0, 0.0), box_doc(3.0, 0.0));
    let (want, _) = render(&dq, &[([0.0; 3], &b)], &cam);
    let mut r = Renderer::new(&dq.0, FORMAT);
    r.add_tile(&dq.0, [0.0; 3], &TileMesh::from_document(&a).unwrap());
    r.add_tile(&dq.0, [0.0; 3], &TileMesh::from_document(&b).unwrap());
    let (got, draws) = shoot(&dq, &r, &cam);
    assert_eq!(draws, 2, "the old tile is gone");
    assert!(diff(&want, &got) <= 0.001);
}

/// Overlay lines appear in their color, on top of the model, at UTM too.
#[test]
fn overlay_draws_on_top_in_its_color() {
    let _serial = gpu_lock();
    let Some(dq) = gpu() else { return };
    let red = [1.0, 0.0, 0.0];
    let reds = |px: &[u8]| {
        px.chunks(4)
            .filter(|p| p[0] > 200 && p[1] < 60 && p[2] < 60)
            .count()
    };
    for at in [[0.0; 3], [430_000.123, 9_150_000.456, 12.0]] {
        let cam = camera(at);
        let doc = box_doc(0.0, 0.0);
        let mut r = Renderer::new(&dq.0, FORMAT);
        r.add_tile(&dq.0, at, &TileMesh::from_document(&doc).unwrap());
        let (plain, draws) = shoot(&dq, &r, &cam);
        assert_eq!((reds(&plain), draws), (0, 2));
        // Through the middle of the box: the depth test would hide it.
        let p = |x: f64, y: f64, z: f64| [at[0] + x, at[1] + y, at[2] + z];
        r.set_overlay(
            &dq.0,
            &[OverlayLines {
                color: red,
                lines: vec![(p(0.5, 2.0, 1.5), p(4.5, 2.0, 1.5))],
            }],
        );
        let (with, draws) = shoot(&dq, &r, &cam);
        assert_eq!(draws, 3);
        assert!(
            reds(&with) > 20,
            "line barely visible: {} red px",
            reds(&with)
        );
        r.set_overlay(&dq.0, &[]);
        let (off, draws) = shoot(&dq, &r, &cam);
        assert_eq!((reds(&off), draws), (0, 2));
    }
}

/// A long line, one end behind the eye: what reaches the screen is still drawn.
#[test]
fn overlay_line_past_the_eye_is_clipped_not_lost() {
    let _serial = gpu_lock();
    let Some(dq) = gpu() else { return };
    let at = [0.0; 3];
    let cam = camera(at);
    let mut r = Renderer::new(&dq.0, FORMAT);
    let dir = [8.5, 11.122, -5.5];
    let along = |t: f64| std::array::from_fn(|k| cam.target[k] + [2.0, -1.5, 0.0][k] + dir[k] * t);
    r.set_overlay(
        &dq.0,
        &[OverlayLines {
            color: [1.0, 0.0, 0.0],
            lines: vec![(along(-5.0), along(5.0))],
        }],
    );
    let (px, _) = shoot(&dq, &r, &cam);
    let reds = px
        .chunks(4)
        .filter(|p| p[0] > 200 && p[1] < 60 && p[2] < 60)
        .count();
    assert!(reds > 20, "line lost: {reds} red px");
}

/// Lines of every reach, under a camera like the app's. Reports all cases at once.
#[test]
fn overlay_lines_of_any_reach_draw() {
    let _serial = gpu_lock();
    let Some(dq) = gpu() else { return };
    let base = camera([0.0; 3]);
    let wide = Camera {
        near: 0.15,
        far: 1500.0,
        ..camera([0.0; 3])
    };
    let t = base.target;
    let on_x = |r: f64| ([t[0] - r, t[1], t[2]], [t[0] + r, t[1], t[2]]);
    let mut lost = Vec::new();
    for (name, cam, line) in [
        ("short", &base, on_x(1.0)),
        ("off both sides", &base, on_x(40.0)),
        ("far past the near plane", &base, on_x(150.0)),
        ("app camera short", &wide, on_x(1.0)),
        ("app camera 40", &wide, on_x(40.0)),
        ("app camera 200", &wide, on_x(200.0)),
        ("app camera 500", &wide, on_x(500.0)),
    ] {
        let mut r = Renderer::new(&dq.0, FORMAT);
        r.set_overlay(
            &dq.0,
            &[OverlayLines {
                color: [1.0, 0.0, 0.0],
                lines: vec![line],
            }],
        );
        let (px, _) = shoot(&dq, &r, cam);
        let reds = px
            .chunks(4)
            .filter(|p| p[0] > 200 && p[1] < 60 && p[2] < 60)
            .count();
        if reds < 20 {
            lost.push((name, reds));
        }
    }
    assert!(lost.is_empty(), "lost: {lost:?}");
}

fn user_hidden_box() -> Document {
    let mut doc = box_doc(0.0, 0.0);
    let node = doc.top_level_nodes()[0];
    doc.set_node_user_hidden(node, true);
    doc
}

/// CPU only: what a tile's mesh holds.
#[test]
fn mesh_holds_instances_and_sketches_but_not_hidden_nodes() {
    let solid = TileMesh::from_document(&box_doc(0.0, 0.0)).unwrap();
    assert_eq!(solid.triangle_count(), 12);
    assert_eq!(
        TileMesh::from_document(&user_hidden_box())
            .unwrap()
            .triangle_count(),
        0
    );

    // An instance renders the same box.
    let mut doc = box_doc(40.0, 0.0);
    let node = doc.top_level_nodes()[0];
    doc.make_component(&[node]).unwrap();
    let inst = TileMesh::from_document(&doc).unwrap();
    assert_eq!(inst.triangle_count(), 12);
    let (lo, _) = inst.bounds().unwrap();
    assert!((lo[0] - 40.0).abs() < 1e-3, "instance at {lo:?}");
    // A hidden instance renders nothing.
    let node = doc.top_level_nodes()[0];
    doc.set_node_user_hidden(node, true);
    assert_eq!(TileMesh::from_document(&doc).unwrap().triangle_count(), 0);
    doc.set_node_user_hidden(node, false);
    let kernel::NodeId::Instance(i) = node else {
        panic!("the box became an instance")
    };
    let group = doc.group_nodes(&[kernel::NodeId::Instance(i)]).unwrap().0;
    assert_eq!(doc.top_level_nodes(), [kernel::NodeId::Group(group)]);
    doc.set_node_user_hidden(kernel::NodeId::Instance(i), true);
    assert_eq!(
        TileMesh::from_document(&doc).unwrap().triangle_count(),
        0,
        "hidden inside a group"
    );

    // Hidden through a tag: the object itself, and a whole group around it.
    let mut doc = box_doc(0.0, 0.0);
    let node = doc.top_level_nodes()[0];
    let tag = vec!["Lantai".to_string()];
    doc.add_node_tag(node, tag.clone()).unwrap();
    doc.set_tag_hidden(tag.clone(), true);
    assert_eq!(TileMesh::from_document(&doc).unwrap().triangle_count(), 0);
    doc.set_tag_hidden(tag.clone(), false);
    assert_eq!(TileMesh::from_document(&doc).unwrap().triangle_count(), 12);
    doc.remove_node_tag(node, &tag).unwrap();
    let group = doc.group_nodes(&[node]).unwrap().0;
    doc.add_node_tag(kernel::NodeId::Group(group), tag.clone())
        .unwrap();
    doc.set_tag_hidden(tag, true);
    assert_eq!(TileMesh::from_document(&doc).unwrap().triangle_count(), 0);

    // A drawing with no solid has bounds, so zoom extents can find it.
    let mut doc = Document::new();
    let ring = [
        Point3::new(7.0, 0.0, 0.0),
        Point3::new(9.0, 0.0, 0.0),
        Point3::new(9.0, 2.0, 0.0),
    ];
    let sk = doc.add_sketch(Plane::from_polygon(&ring).unwrap());
    doc.sketch_mut(sk)
        .unwrap()
        .add_segment(ring[0], ring[1])
        .unwrap();
    let drawn = TileMesh::from_document(&doc).unwrap();
    assert_eq!(drawn.triangle_count(), 0);
    let (lo, hi) = drawn.bounds().expect("sketch bounds");
    assert_eq!((lo[0], hi[0]), (7.0, 9.0));
}

/// A closed sketch region is filled from either side, and a hole stays open.
#[test]
fn sketch_regions_fill_from_both_sides_around_holes() {
    let _serial = gpu_lock();
    let Some(dq) = gpu() else { return };
    let plane = kernel::Plane::from_point_normal(
        Point3::new(0.0, 0.0, 0.0),
        kernel::Vec3::new(0.0, 0.0, 1.0),
    )
    .unwrap();
    let ring = |x: f64, y: f64, w: f64, d: f64| {
        let p = |a: f64, b: f64| Point3::new(x + a, y + b, 0.0);
        let r = [p(0.0, 0.0), p(w, 0.0), p(w, d), p(0.0, d)];
        (0..4)
            .map(move |k| (r[k], r[(k + 1) % 4]))
            .collect::<Vec<_>>()
    };
    let mut doc = Document::new();
    let id = doc.add_sketch(plane);
    let sk = doc.sketch_mut(id).unwrap();
    for (a, b) in ring(-5.0, -5.0, 10.0, 10.0) {
        sk.add_segment(a, b).unwrap();
    }
    let mesh = TileMesh::from_document(&doc).unwrap();
    assert_eq!(mesh.triangle_count(), 4, "two triangles per side");

    let eye = |z: f64| Camera {
        eye: [0.0, -1.0, z],
        target: [0.0; 3],
        fovy: 0.8,
        aspect: 1.0,
        near: 0.1,
        far: 200.0,
    };
    let mut r = Renderer::new(&dq.0, FORMAT);
    r.add_tile(&dq.0, [0.0; 3], &mesh);
    let (above, _) = shoot(&dq, &r, &eye(20.0));
    let (below, _) = shoot(&dq, &r, &eye(-20.0));
    let solid = |px: &[u8]| coverage(px);
    assert!(solid(&above) > 0.1, "above: {}", solid(&above));
    assert!(solid(&below) > 0.1, "below: {}", solid(&below));

    // A hole: the inner loop becomes its own region, unless it is faceless.
    let sk = doc.sketch_mut(id).unwrap();
    for (a, b) in ring(-2.0, -2.0, 4.0, 4.0) {
        sk.add_segment(a, b).unwrap();
    }
    let inner = sk
        .regions()
        .iter()
        .find(|(_, r)| r.holes.is_empty())
        .map(|(id, _)| id)
        .unwrap();
    sk.set_region_faceless(inner, true).unwrap();
    let mesh = TileMesh::from_document(&doc).unwrap();
    let mut r = Renderer::new(&dq.0, FORMAT);
    r.add_tile(&dq.0, [0.0; 3], &mesh);
    let (holed, _) = shoot(&dq, &r, &eye(20.0));
    let (a, b) = (solid(&above), solid(&holed));
    // The hole takes 16 of the 100 square meters, give or take perspective.
    assert!(b < a * 0.9 && b > a * 0.7, "filled {a}, with hole {b}");
}

/// A flat grid of points on z = 0: `n` x `n` across 10 x 10 m, its corner at `at`.
fn add_cloud_grid(r: &mut Renderer, device: &wgpu::Device, at: [f64; 3], n: u16) {
    let step = 65535 / (n - 1);
    let points: Vec<CloudPoint> = (0..n)
        .flat_map(|y| {
            (0..n).map(move |x| CloudPoint {
                pos: [x * step, y * step, 0, 0],
                color: [200, 40, 40, 255],
            })
        })
        .collect();
    r.set_cloud_chunk(
        device,
        ChangedChunk {
            id: 7,
            chunk: 0,
            min: at,
            size: [10.0, 10.0, 0.0],
            points: &points,
        },
    );
}

fn cloud_pixels(
    dq: &(wgpu::Device, wgpu::Queue),
    at: [f64; 3],
    count: u32,
    point_px: f32,
    slice: Option<Slice>,
) -> (Vec<u8>, usize) {
    let mut r = Renderer::new(&dq.0, FORMAT);
    add_cloud_grid(&mut r, &dq.0, at, 64);
    let cam = camera(at);
    let draws = [DrawChunk {
        id: 7,
        chunk: 0,
        count,
        point_size: point_px,
        offset: [0.0; 3],
    }];
    r.prepare_clouds(&dq.1, &cam, &draws, [W, H], 1.0, slice);
    shoot(dq, &r, &cam)
}

/// Pixels of the points' color.
fn red(px: &[u8]) -> usize {
    px.chunks(4)
        .filter(|p| p[0] > 150 && p[1] < 90 && p[2] < 90)
        .count()
}

/// Points hold their place at UTM as well as at home, by the same RTC as tiles.
#[test]
fn points_render_the_same_at_utm() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(dq) = gpu() else { return };
    let (home, calls) = cloud_pixels(&dq, [0.0; 3], 4096, 2.0, None);
    assert_eq!(calls, 1);
    assert!(red(&home) > 500, "{}", red(&home));
    let (far, _) = cloud_pixels(&dq, [710_386.5, 9_316_096.9, 0.0], 4096, 2.0, None);
    let d = diff(&home, &far);
    assert!(d <= 0.001, "{:.3}% differ", d * 100.0);
}

/// A bigger point covers more of the view, a smaller count less.
#[test]
fn point_size_and_count_change_what_shows() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(dq) = gpu() else { return };
    let (small, _) = cloud_pixels(&dq, [0.0; 3], 4096, 1.0, None);
    let (big, _) = cloud_pixels(&dq, [0.0; 3], 4096, 6.0, None);
    assert!(red(&big) > 2 * red(&small), "{} {}", red(&big), red(&small));
    let (few, _) = cloud_pixels(&dq, [0.0; 3], 100, 6.0, None);
    assert!(red(&few) * 2 < red(&big), "{} {}", red(&few), red(&big));
}

/// Outside the box nothing shows. Taking the box away gives the same picture back.
#[test]
fn a_slice_hides_what_is_outside_it() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(dq) = gpu() else { return };
    let at = [710_386.5, 9_316_096.9, 0.0];
    let (all, _) = cloud_pixels(&dq, at, 4096, 3.0, None);
    let half = Slice {
        min: [at[0], at[1], -1.0],
        max: [at[0] + 5.0, at[1] + 10.0, 1.0],
    };
    let (cut, _) = cloud_pixels(&dq, at, 4096, 3.0, Some(half));
    assert!(red(&cut) > 100, "{}", red(&cut));
    assert!(red(&cut) < red(&all) * 3 / 4, "{} {}", red(&cut), red(&all));
    let (back, _) = cloud_pixels(&dq, at, 4096, 3.0, None);
    assert_eq!(back, all, "identical once it is off");

    let away = Slice {
        min: [at[0], at[1], 5.0],
        max: [at[0] + 10.0, at[1] + 10.0, 6.0],
    };
    let (none, _) = cloud_pixels(&dq, at, 4096, 3.0, Some(away));
    assert_eq!(red(&none), 0, "the whole grid is below the box");
}

/// A solid in front hides points behind it, and points in front of it stay.
#[test]
fn points_and_solids_share_the_depth_buffer() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(dq) = gpu() else { return };
    let at = [0.0; 3];
    let mut r = Renderer::new(&dq.0, FORMAT);
    add_cloud_grid(&mut r, &dq.0, at, 64);
    let cam = camera(at);
    let draws = [DrawChunk {
        id: 7,
        chunk: 0,
        count: 4096,
        point_size: 3.0,
        offset: [0.0; 3],
    }];
    r.prepare_clouds(&dq.1, &cam, &draws, [W, H], 1.0, None);
    let (open, _) = shoot(&dq, &r, &cam);
    let doc = box_doc(1.0, 1.0);
    r.add_tile(&dq.0, [0.0; 3], &TileMesh::from_document(&doc).unwrap());
    let (hidden, calls) = shoot(&dq, &r, &cam);
    assert_eq!(calls, 3, "faces, edges, one chunk");
    assert!(red(&hidden) < red(&open), "{} {}", red(&hidden), red(&open));
    assert!(red(&hidden) > 100, "the rest stays");

    r.clear_clouds();
    let (none, calls) = shoot(&dq, &r, &cam);
    assert_eq!(calls, 2);
    assert_eq!(red(&none), 0);
}

/// Replacing a chunk's points and dropping a cloud leave nothing behind.
#[test]
fn cloud_chunks_replace_and_go() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(dq) = gpu() else { return };
    let at = [0.0; 3];
    let mut r = Renderer::new(&dq.0, FORMAT);
    add_cloud_grid(&mut r, &dq.0, at, 64);
    add_cloud_grid(&mut r, &dq.0, at, 8);
    let cam = camera(at);
    // Asking beyond what it holds draws what it holds.
    let draws = [DrawChunk {
        id: 7,
        chunk: 0,
        count: 4096,
        point_size: 3.0,
        offset: [0.0; 3],
    }];
    r.prepare_clouds(&dq.1, &cam, &draws, [W, H], 1.0, None);
    let (few, calls) = shoot(&dq, &r, &cam);
    assert_eq!(calls, 1);
    assert!(red(&few) > 0);
    r.remove_cloud(7);
    let (none, calls) = shoot(&dq, &r, &cam);
    assert_eq!(calls, 0);
    assert_eq!(red(&none), 0);
}

/// Two chunks at two sizes: each covers what its own size says, and the screen's
/// scale multiplies both.
#[test]
fn each_chunk_draws_at_its_own_size() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(dq) = gpu() else { return };
    let at = [0.0; 3];
    let mut r = Renderer::new(&dq.0, FORMAT);
    add_cloud_grid(&mut r, &dq.0, at, 64);
    let cam = camera(at);
    let shot = |r: &mut Renderer, size: f32, scale: f32| {
        let draws = [DrawChunk {
            id: 7,
            chunk: 0,
            count: 4096,
            point_size: size,
            offset: [0.0; 3],
        }];
        r.prepare_clouds(&dq.1, &cam, &draws, [W, H], scale, None);
        red(&shoot(&dq, r, &cam).0)
    };
    let small = shot(&mut r, 1.0, 1.0);
    let big = shot(&mut r, 5.0, 1.0);
    assert!(big > 2 * small, "{big} {small}");
    assert_eq!(shot(&mut r, 2.5, 2.0), big, "same pixels either way");
}

/// A chunk uploaded at home and offset to UTM looks as it does uploaded there, and
/// the offset takes no upload.
#[test]
fn an_offset_is_the_same_as_being_there() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(dq) = gpu() else { return };
    let far = [710_386.5, 9_316_096.9, 3.0];
    let (there, _) = cloud_pixels(&dq, far, 4096, 2.0, None);
    let mut r = Renderer::new(&dq.0, FORMAT);
    add_cloud_grid(&mut r, &dq.0, [0.0; 3], 64);
    let cam = camera(far);
    let draws = [DrawChunk {
        id: 7,
        chunk: 0,
        count: 4096,
        point_size: 2.0,
        offset: far,
    }];
    r.prepare_clouds(&dq.1, &cam, &draws, [W, H], 1.0, None);
    let (moved, _) = shoot(&dq, &r, &cam);
    assert!(red(&moved) > 500);
    let d = diff(&there, &moved);
    assert!(d <= 0.001, "{:.3}% differ", d * 100.0);
}
