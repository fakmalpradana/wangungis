//! Golden-image precision test (closes the render half of spike S2, ADR-003):
//! the same box must render identically whether its tile sits at the origin,
//! 40 km out, or at full UTM coordinates, as long as the camera moves with it.
//! Negative control: the same scene WITHOUT RTC (geometry baked at UTM in
//! f32) must render visibly wrong, which proves the test can fail.
//!
//! Needs a GPU adapter (CI: Metal, lavapipe, WARP). Without one the tests
//! skip, unless WANGUNGIS_REQUIRE_GPU is set, in which case they fail.

use kernel::{Document, Plane, Point3};
use wangungis_render::{Camera, DEPTH_FORMAT, Renderer, TileMesh};

const W: u32 = 256;
const H: u32 = 256;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

fn gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = [false, true].into_iter().find_map(|fallback| {
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            force_fallback_adapter: fallback,
            ..Default::default()
        }))
        .ok()
    });
    let Some(adapter) = adapter else {
        assert!(
            std::env::var_os("WANGUNGIS_REQUIRE_GPU").is_none(),
            "no GPU adapter but WANGUNGIS_REQUIRE_GPU is set"
        );
        eprintln!("SKIP: no GPU adapter");
        return None;
    };
    eprintln!("adapter: {:?}", adapter.get_info());
    pollster::block_on(adapter.request_device(&Default::default())).ok()
}

/// Axis-aligned 5 x 4.244 x 3 m box with its corner at (x, y, 0). Axis-aligned
/// boxes are accepted by the kernel even at UTM (spike S2), which the
/// negative control needs.
fn box_doc(x: f64, y: f64) -> Document {
    let mut doc = Document::new();
    let p = |a: f64, b: f64| Point3::new(x + a, y + b, 0.0);
    let ring = [p(0.0, 0.0), p(5.0, 0.0), p(5.0, 4.244), p(0.0, 4.244)];
    let sk = doc.add_sketch(Plane::from_polygon(&ring).unwrap());
    let s = doc.sketch_mut(sk).unwrap();
    for k in 0..4 {
        s.add_segment(ring[k], ring[(k + 1) % 4]).unwrap();
    }
    let r = doc.extrudable_regions(sk).unwrap();
    doc.extrude_region(sk, r[0], 3.0).unwrap();
    doc
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
    let (device, queue) = dq;
    let mut r = Renderer::new(device, FORMAT);
    for (origin, doc) in tiles {
        r.add_tile(device, *origin, &TileMesh::from_document(doc).unwrap());
    }
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
fn far_tiles_render_like_the_origin() {
    let Some(dq) = gpu() else { return };
    let doc = box_doc(0.0, 0.0);
    let (reference, draws) = render(&dq, &[([0.0; 3], &doc)], &camera([0.0; 3]));
    assert_eq!(draws, 2);
    let cov = coverage(&reference);
    assert!(cov > 0.05, "box barely visible: coverage {cov}");
    for origin in [
        [40_000.0, -40_000.0, 0.0],
        [430_000.123, 9_150_000.456, 12.0],
    ] {
        let (img, _) = render(&dq, &[(origin, &doc)], &camera(origin));
        let d = diff(&reference, &img);
        eprintln!("DIFF tile at {origin:?}: {:.4}%", d * 100.0);
        assert!(
            d <= 0.001,
            "tile at {origin:?}: {:.2}% pixels differ",
            d * 100.0
        );
    }
}

#[test]
fn without_rtc_utm_renders_wrong() {
    let Some(dq) = gpu() else { return };
    let (reference, _) = render(&dq, &[([0.0; 3], &box_doc(0.0, 0.0))], &camera([0.0; 3]));
    // Geometry baked at UTM in the tile's f32 buffer, tile frame at 0: no RTC.
    let (x, y) = (430_000.123, 9_150_000.456);
    let (img, _) = render(&dq, &[([0.0; 3], &box_doc(x, y))], &camera([x, y, 0.0]));
    let d = diff(&reference, &img);
    eprintln!("DIFF without RTC: {:.4}%", d * 100.0);
    // Measured 1.12% on Apple M4 (the 4.244 m depth snaps to the 1 m f32 grid
    // at Y = 9.15e6); RTC renders differ by 0.00%. 0.5% separates the two.
    assert!(
        d > 0.005,
        "no-RTC render should be visibly wrong, only {:.2}% differs",
        d * 100.0
    );
}

#[test]
fn two_draws_per_tile() {
    let Some(dq) = gpu() else { return };
    let (a, b) = (box_doc(0.0, 0.0), box_doc(20.0, 0.0));
    let (_, draws) = render(
        &dq,
        &[([0.0; 3], &a), ([1000.0, 0.0, 0.0], &b)],
        &camera([0.0; 3]),
    );
    assert_eq!(draws, 4);
}
