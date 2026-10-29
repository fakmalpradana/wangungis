//! WangunGIS spike for option A2: load a `.hew` natively with Hew's `kernel`,
//! tessellate every Object with Hew's `tessellate`, merge everything into ONE
//! vertex/index buffer (+ one edge line buffer), and render offscreen with
//! wgpu at 1920x1080 while orbiting. Measures what batching buys over Hew's
//! web viewport (2 draw calls per Object).
//! Usage: cargo run --release -- <model.hew> [frames]

use bytemuck::{Pod, Zeroable};
use std::time::Instant;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 3],
    nrm: [f32; 3],
}

const SHADER: &str = r#"
struct U { mvp: mat4x4<f32>, light: vec4<f32> };
@group(0) @binding(0) var<uniform> u: U;
struct O { @builtin(position) p: vec4<f32>, @location(0) n: vec3<f32> };
@vertex fn vs(@location(0) pos: vec3<f32>, @location(1) n: vec3<f32>) -> O {
  var o: O; o.p = u.mvp * vec4<f32>(pos, 1.0); o.n = n; return o;
}
@fragment fn fs(i: O) -> @location(0) vec4<f32> {
  let l = 0.35 + 0.65 * max(dot(normalize(i.n), u.light.xyz), 0.0);
  return vec4<f32>(vec3<f32>(0.85, 0.82, 0.78) * l, 1.0);
}
@vertex fn vs_edge(@location(0) pos: vec3<f32>) -> @builtin(position) vec4<f32> {
  return u.mvp * vec4<f32>(pos, 1.0);
}
@fragment fn fs_edge() -> @location(0) vec4<f32> { return vec4<f32>(0.1, 0.1, 0.1, 1.0); }
"#;

fn look_at_persp(eye: [f32; 3], target: [f32; 3], aspect: f32, fovy: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let norm = |a: [f32; 3]| { let l = dot(a, a).sqrt(); [a[0] / l, a[1] / l, a[2] / l] };
    let f = norm(sub(target, eye));
    let s = norm(cross(f, [0.0, 0.0, 1.0]));
    let up = cross(s, f);
    let view = [
        [s[0], up[0], -f[0], 0.0],
        [s[1], up[1], -f[1], 0.0],
        [s[2], up[2], -f[2], 0.0],
        [-dot(s, eye), -dot(up, eye), dot(f, eye), 1.0],
    ];
    let t = 1.0 / (fovy / 2.0).tan();
    let proj = [
        [t / aspect, 0.0, 0.0, 0.0],
        [0.0, t, 0.0, 0.0],
        [0.0, 0.0, far / (near - far), -1.0],
        [0.0, 0.0, near * far / (near - far), 0.0],
    ];
    let mut m = [[0.0f32; 4]; 4];
    for (c, col) in view.iter().enumerate() {
        for r in 0..4 {
            m[c][r] = (0..4).map(|k| proj[k][r] * col[k]).sum();
        }
    }
    m
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let frames: usize = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(300);

    // --- load + tessellate natively ---
    let t0 = Instant::now();
    let bytes = std::fs::read(&args[1]).unwrap();
    let doc = kernel::Document::load(&bytes).expect("load .hew");
    let t_load = t0.elapsed();
    let palette = kernel::MaterialPalette::default();
    let (mut verts, mut idx, mut edges) = (Vec::<Vertex>::new(), Vec::<u32>::new(), Vec::<[f32; 3]>::new());
    let mut objects = 0usize;
    let mut faces = 0usize;
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for oid in doc.live_object_ids() {
        if !doc.is_world_object(oid) { continue; }
        let obj = doc.object(oid).unwrap();
        faces += obj.faces().len();
        objects += 1;
        let m = tessellate::tessellate(obj, &palette).expect("tessellate");
        let base = verts.len() as u32;
        for (p, n) in m.positions.chunks(3).zip(m.normals.chunks(3)) {
            for k in 0..3 { lo[k] = lo[k].min(p[k]); hi[k] = hi[k].max(p[k]); }
            verts.push(Vertex { pos: [p[0], p[1], p[2]], nrm: [n[0], n[1], n[2]] });
        }
        idx.extend(m.indices.iter().map(|i| i + base));
        edges.extend(m.edge_positions.chunks(3).map(|p| [p[0], p[1], p[2]]));
    }
    let t_tess = t0.elapsed() - t_load;
    println!("objects {objects}, faces {faces}, triangles {}, edge segments {}, load {:.2?}, tessellate+merge {:.2?}",
        idx.len() / 3, edges.len() / 2, t_load, t_tess);
    drop(doc);

    // --- wgpu offscreen ---
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance, ..Default::default()
    })).expect("adapter");
    println!("adapter: {:?}", adapter.get_info().name);
    let mut limits = wgpu::Limits::default();
    limits.max_buffer_size = adapter.limits().max_buffer_size;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_limits: limits, ..Default::default()
    })).expect("device");

    let (w, h) = (1920u32, 1080u32);
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: None, size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
    });
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: None, size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
    });
    let (cv, dv) = (color.create_view(&Default::default()), depth.create_view(&Default::default()));
    let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::cast_slice(&verts), usage: wgpu::BufferUsages::VERTEX });
    let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::cast_slice(&idx), usage: wgpu::BufferUsages::INDEX });
    let eb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::cast_slice(&edges), usage: wgpu::BufferUsages::VERTEX });
    let ub = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: 80, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: None, entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0, visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None }] });
    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &bgl, entries: &[wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() }] });
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&bgl)], ..Default::default() });
    let depth_state = Some(wgpu::DepthStencilState { format: wgpu::TextureFormat::Depth32Float, depth_write_enabled: Some(true),
        depth_compare: Some(wgpu::CompareFunction::LessEqual), stencil: Default::default(), bias: Default::default() });
    let target = [Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: None, write_mask: wgpu::ColorWrites::ALL })];
    let tri_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
    let edge_attrs = wgpu::vertex_attr_array![0 => Float32x3];
    let tri = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout),
        vertex: wgpu::VertexState { module: &module, entry_point: Some("vs"), compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout { array_stride: 24, step_mode: wgpu::VertexStepMode::Vertex, attributes: &tri_attrs })] },
        fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fs"), compilation_options: Default::default(), targets: &target }),
        primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
        depth_stencil: depth_state.clone(), multisample: Default::default(), multiview_mask: None, cache: None,
    });
    let edge = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout),
        vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_edge"), compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout { array_stride: 12, step_mode: wgpu::VertexStepMode::Vertex, attributes: &edge_attrs })] },
        fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fs_edge"), compilation_options: Default::default(), targets: &target }),
        primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::LineList, ..Default::default() },
        depth_stencil: depth_state, multisample: Default::default(), multiview_mask: None, cache: None,
    });

    let c = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, (lo[2] + hi[2]) / 2.0];
    let r = ((hi[0] - lo[0]).powi(2) + (hi[1] - lo[1]).powi(2)).sqrt() * 0.75;
    let mut times = Vec::with_capacity(frames);
    for i in 0..frames {
        let t = Instant::now();
        let a = i as f32 * 0.02;
        let eye = [c[0] + r * a.cos(), c[1] + r * a.sin(), c[2] + r * 0.6];
        let mvp = look_at_persp(eye, c, w as f32 / h as f32, 0.8, r * 0.01, r * 4.0);
        let mut u = [0f32; 20];
        for col in 0..4 { for row in 0..4 { u[col * 4 + row] = mvp[col][row]; } }
        u[16..19].copy_from_slice(&[0.4, 0.3, 0.866]);
        queue.write_buffer(&ub, 0, bytemuck::cast_slice(&u));
        let mut enc = device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &cv, resolve_target: None, depth_slice: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::WHITE), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view: &dv,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }), stencil_ops: None }),
                ..Default::default()
            });
            pass.set_bind_group(0, &bg, &[]);
            pass.set_pipeline(&tri);
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..idx.len() as u32, 0, 0..1);
            pass.set_pipeline(&edge);
            pass.set_vertex_buffer(0, eb.slice(..));
            pass.draw(0..edges.len() as u32, 0..1);
        }
        queue.submit([enc.finish()]);
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        times.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    let median = times[times.len() / 2];
    let p95 = times[times.len() * 95 / 100];
    println!("frames {frames}, draw calls/frame 2, frame median {median:.2} ms ({:.0} FPS), p95 {p95:.2} ms", 1000.0 / median);
}
