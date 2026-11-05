//! Batched wgpu renderer with per-tile RTC (ADR-003).
//!
//! One kernel tile (one Hew `Document`, geometry ≤ 1.5 km from its frame,
//! ADR-004) becomes one render tile: all its Objects merged into one face
//! buffer and one edge buffer, positions as `f32` relative to the tile frame
//! (≤ 0.12 mm resolution at 1.5 km). Each frame, the tile frame's offset from
//! the camera is computed in `f64` and sent as an `f32` uniform, so precision
//! does not depend on how far the project sits from any origin.
//! Port of spike `wangungis-spikes/s6-native-render`.

use bytemuck::{Pod, Zeroable};
use wangungis_pointcloud::{ChangedChunk, CloudPoint, DrawChunk, Slice};
use wangungis_scene::project::node_visible;
use wgpu::util::DeviceExt;

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

const SHADER: &str = r#"
struct U { view_proj: mat4x4<f32>, offset: vec4<f32>, light: vec4<f32> };
@group(0) @binding(0) var<uniform> u: U;
struct O { @builtin(position) p: vec4<f32>, @location(0) n: vec3<f32>, @location(1) color: vec4<f32> };
@vertex fn vs(@location(0) pos: vec3<f32>, @location(1) n: vec3<f32>, @location(2) color: vec4<f32>) -> O {
  var o: O; o.p = u.view_proj * vec4<f32>(pos + u.offset.xyz, 1.0); o.n = n; o.color = color; return o;
}
@fragment fn fs(i: O) -> @location(0) vec4<f32> {
  let l = 0.35 + 0.65 * max(dot(normalize(i.n), u.light.xyz), 0.0);
  // Alpha is how solid the face looks, towards the view's background: geometry
  // outside an open group fades. Never blended, so depth stays right.
  return vec4<f32>(mix(vec3<f32>(0.93, 0.94, 0.95), i.color.rgb * l, i.color.a), 1.0);
}
@vertex fn vs_edge(@location(0) pos: vec3<f32>) -> @builtin(position) vec4<f32> {
  return u.view_proj * vec4<f32>(pos + u.offset.xyz, 1.0);
}
@fragment fn fs_edge() -> @location(0) vec4<f32> { return vec4<f32>(0.1, 0.1, 0.1, 1.0); }
// Overlay lines carry their flat color in the `light` slot of the uniform.
@fragment fn fs_overlay() -> @location(0) vec4<f32> { return vec4<f32>(u.light.xyz, 1.0); }
"#;

/// Point cloud chunks: one quad per point, its corners from the vertex index. The
/// instance holds position as a fraction of the chunk and a color. A point outside
/// the slice goes outside clip space, where nothing rasterizes.
const CLOUD_SHADER: &str = r#"
struct C {
  view_proj: mat4x4<f32>, offset: vec4<f32>, scale: vec4<f32>,
  slice_min: vec4<f32>, slice_max: vec4<f32>, half_px: vec4<f32>,
};
@group(0) @binding(0) var<uniform> c: C;
struct O { @builtin(position) p: vec4<f32>, @location(0) color: vec3<f32> };
@vertex fn vs_cloud(@builtin(vertex_index) i: u32, @location(0) q: vec4<f32>, @location(1) color: vec4<f32>) -> O {
  var o: O;
  o.color = color.rgb;
  let rel = c.offset.xyz + q.xyz * c.scale.xyz;
  if (c.slice_min.w > 0.0 && (any(rel < c.slice_min.xyz) || any(rel > c.slice_max.xyz))) {
    o.p = vec4<f32>(2.0, 2.0, 2.0, 1.0);
    return o;
  }
  var p = c.view_proj * vec4<f32>(rel, 1.0);
  let corner = vec2<f32>(f32(i & 1u) * 2.0 - 1.0, f32((i >> 1u) & 1u) * 2.0 - 1.0);
  p = vec4<f32>(p.xy + corner * c.half_px.xy * p.w, p.zw);
  o.p = p;
  return o;
}
@fragment fn fs_cloud(i: O) -> @location(0) vec4<f32> { return vec4<f32>(i.color, 1.0); }
"#;

/// How solid geometry outside an open group looks, in a vertex's alpha.
// ponytail: its edges stay dark, the edge pipeline has no color.
const FADED: u8 = 90;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 3],
    nrm: [f32; 3],
    color: [u8; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniform {
    view_proj: [[f32; 4]; 4],
    offset: [f32; 4],
    light: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CloudUniform {
    view_proj: [[f32; 4]; 4],
    offset: [f32; 4],
    scale: [f32; 4],
    slice_min: [f32; 4],
    slice_max: [f32; 4],
    half_px: [f32; 4],
}

/// CPU-side merged geometry of one tile, in the tile's own frame.
#[derive(Default)]
pub struct TileMesh {
    verts: Vec<Vertex>,
    indices: Vec<u32>,
    edges: Vec<[f32; 3]>,
}

/// A region as a sheet object: its outer ring, with each hole imprinted so that
/// triangulation respects it.
fn region_sheet(sk: &kernel::Sketch, region: kernel::SketchRegionId) -> Option<kernel::Object> {
    let profile = sk.profile(region).ok()?;
    let outer = profile.outer();
    let ring = (0..outer.len()).collect();
    let mut sheet = kernel::Object::from_polygons(outer, &[ring]).ok()?;
    let face = sheet.faces().keys().next()?;
    for hole in profile.holes() {
        sheet.split_face_inner(face, hole).ok()?;
    }
    Some(sheet)
}

impl TileMesh {
    /// Merges `doc`'s visible geometry (the tile's kernel document): world
    /// Objects, component instances through their poses, and sketch lines and
    /// region fills.
    pub fn from_document(doc: &kernel::Document) -> Result<Self, tessellate::TessellateError> {
        let palette = doc.materials();
        let mut m = TileMesh::default();
        // With a group or component open, the rest of the tile fades.
        // ponytail: other tiles do not fade, only the session's own.
        let open = doc.session_direct_members();
        for node in doc.top_level_nodes() {
            if !node_visible(doc, node) {
                continue;
            }
            let faded = open.as_ref().is_some_and(|o| !o.contains(&node));
            for oid in doc.leaf_objects_under(node) {
                if !node_visible(doc, kernel::NodeId::Object(oid)) {
                    continue;
                }
                if let Some(obj) = doc.object(oid) {
                    m.push(&tessellate::tessellate(obj, palette)?, None, faded);
                }
            }
            for inst in doc.leaf_instances_under(node) {
                let (Some(def), Some(pose)) = (doc.instance_def(inst), doc.instance_pose(inst))
                else {
                    continue;
                };
                if !node_visible(doc, kernel::NodeId::Instance(inst)) {
                    continue;
                }
                let shown = doc.expanded_def_placements_shown(def);
                for ((member, local), shown) in
                    doc.expanded_def_placements(def).into_iter().zip(shown)
                {
                    if shown && let Some(obj) = doc.object(member) {
                        let t = tessellate::tessellate(obj, palette)?;
                        m.push(&t, Some(local.then(&pose)), faded);
                    }
                }
            }
        }
        for id in doc.visible_sketch_ids() {
            let Some(sk) = doc.sketch(id) else { continue };
            for e in sk.edges().values() {
                for v in [e.from, e.to] {
                    let p = sk.vertices()[v].position;
                    m.edges.push([p.x as f32, p.y as f32, p.z as f32]);
                }
            }
            for (region, r) in sk.regions().iter() {
                if !r.faceless {
                    m.push_region(sk, region);
                }
            }
        }
        Ok(m)
    }

    /// Fills a sketch region, visible from both sides. Hew triangulates: the
    /// region becomes a throwaway sheet object, whose triangles falling in a
    /// hole are dropped. A region Hew cannot sheet keeps just its edges.
    fn push_region(&mut self, sk: &kernel::Sketch, region: kernel::SketchRegionId) {
        let palette = kernel::MaterialPalette::default();
        let Some(mesh) = region_sheet(sk, region)
            .and_then(|sheet| tessellate::tessellate(&sheet, &palette).ok())
        else {
            return;
        };
        let plane = sk.plane();
        let n = plane.normal();
        let inside = |tri: &[u32]| {
            let c = (0..3).map(|k| {
                let at = |i: u32| f64::from(mesh.positions[i as usize * 3 + k]);
                (at(tri[0]) + at(tri[1]) + at(tri[2])) / 3.0
            });
            let c: Vec<f64> = c.collect();
            let p = kernel::Point3::new(c[0], c[1], c[2]);
            let p = p - n * plane.signed_distance(p);
            sk.region_contains_point(region, p).unwrap_or(false)
        };
        let base = self.verts.len() as u32;
        let front = [n.x as f32, n.y as f32, n.z as f32];
        let color = tessellate::DEFAULT_MATERIAL_RGBA;
        let color = [color.r, color.g, color.b, color.a];
        for p in mesh.positions.chunks(3) {
            let pos = [p[0], p[1], p[2]];
            self.verts.push(Vertex {
                pos,
                nrm: front,
                color,
            });
            self.verts.push(Vertex {
                pos,
                nrm: front.map(|v| -v),
                color,
            });
        }
        for tri in mesh.indices.chunks(3).filter(|t| inside(t)) {
            let at = |k: usize, back: u32| base + tri[k] * 2 + back;
            self.indices.extend([at(0, 0), at(1, 0), at(2, 0)]);
            self.indices.extend([at(2, 1), at(1, 1), at(0, 1)]);
        }
    }

    /// Appends a tessellated Object, baked through `pose` when it has one.
    fn push(&mut self, t: &tessellate::RenderMesh, pose: Option<kernel::Transform>, faded: bool) {
        let at = |p: &[f32]| match &pose {
            None => [p[0], p[1], p[2]],
            Some(x) => {
                let q = x.apply_point(kernel::Point3::new(p[0].into(), p[1].into(), p[2].into()));
                [q.x as f32, q.y as f32, q.z as f32]
            }
        };
        let base = self.verts.len() as u32;
        for ((p, n), color) in t
            .positions
            .chunks(3)
            .zip(t.normals.chunks(3))
            .zip(t.colors.chunks(3))
        {
            let nrm = match &pose {
                None => [n[0], n[1], n[2]],
                Some(x) => {
                    let q =
                        x.apply_vector(kernel::Vec3::new(n[0].into(), n[1].into(), n[2].into()));
                    [q.x as f32, q.y as f32, q.z as f32]
                }
            };
            let color: [u8; 3] = std::array::from_fn(|i| (color[i] * 255.0).round() as u8);
            self.verts.push(Vertex {
                pos: at(p),
                nrm,
                color: [
                    color[0],
                    color[1],
                    color[2],
                    if faded { FADED } else { 255 },
                ],
            });
        }
        self.indices.extend(t.indices.iter().map(|i| i + base));
        self.edges.extend(t.edge_positions.chunks(3).map(at));
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Axis-aligned bounds in the tile frame, sketch lines included. `None`
    /// when empty.
    pub fn bounds(&self) -> Option<([f32; 3], [f32; 3])> {
        let mut it = self
            .verts
            .iter()
            .map(|v| v.pos)
            .chain(self.edges.iter().copied());
        let first = it.next()?;
        Some(it.fold((first, first), |(lo, hi), p| {
            (
                std::array::from_fn(|k| lo[k].min(p[k])),
                std::array::from_fn(|k| hi[k].max(p[k])),
            )
        }))
    }
}

/// Perspective camera in project-local `f64` coordinates (z up).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub eye: [f64; 3],
    pub target: [f64; 3],
    /// Vertical field of view (rad).
    pub fovy: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    /// View rotation + projection with the eye at the origin (column-major):
    /// translation is left out and applied per tile as an `f64` offset.
    pub fn view_proj_at_eye(&self) -> [[f32; 4]; 4] {
        let f = norm_or(sub(self.target, self.eye), [0.0, 1.0, 0.0]);
        // Z is up; looking straight up or down, take world Y as the up hint.
        let up_hint = if f[2].abs() > 0.999 {
            [0.0, 1.0, 0.0]
        } else {
            [0.0, 0.0, 1.0]
        };
        let s = norm_or(cross(f, up_hint), [1.0, 0.0, 0.0]);
        let up = cross(s, f);
        let c = |v: [f64; 3]| v.map(|x| x as f32);
        let (s, up, f) = (c(s), c(up), c(f));
        let view = [
            [s[0], up[0], -f[0], 0.0],
            [s[1], up[1], -f[1], 0.0],
            [s[2], up[2], -f[2], 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let t = 1.0 / (self.fovy / 2.0).tan();
        let (n, fa) = (self.near, self.far);
        let proj = [
            [t / self.aspect, 0.0, 0.0, 0.0],
            [0.0, t, 0.0, 0.0],
            [0.0, 0.0, fa / (n - fa), -1.0],
            [0.0, 0.0, n * fa / (n - fa), 0.0],
        ];
        std::array::from_fn(|col| {
            std::array::from_fn(|row| (0..4).map(|k| proj[k][row] * view[col][k]).sum())
        })
    }
}

/// The part of the segment `a`..`b`, given from the eye, that `view_proj` shows:
/// where along it that part starts and ends, from 0 to 1.
fn visible_span(view_proj: &[[f32; 4]; 4], a: [f64; 3], b: [f64; 3]) -> Option<(f64, f64)> {
    // How far inside each of the six planes of the view a point is.
    let inside = |p: [f64; 3]| {
        let c: [f64; 4] = std::array::from_fn(|r| {
            (0..3)
                .map(|col| f64::from(view_proj[col][r]) * p[col])
                .sum::<f64>()
                + f64::from(view_proj[3][r])
        });
        [
            c[3] + c[0],
            c[3] - c[0],
            c[3] + c[1],
            c[3] - c[1],
            c[2],
            c[3] - c[2],
        ]
    };
    let (ia, ib) = (inside(a), inside(b));
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (da, db) in ia.into_iter().zip(ib) {
        match (da < 0.0, db < 0.0) {
            (true, true) => return None,
            (true, false) => t0 = t0.max(da / (da - db)),
            (false, true) => t1 = t1.min(da / (da - db)),
            (false, false) => {}
        }
    }
    (t0 < t1).then_some((t0, t1))
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
/// Unit vector along `a`, or `fallback` when `a` has (almost) no length.
fn norm_or(a: [f64; 3], fallback: [f64; 3]) -> [f64; 3] {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if l < 1e-12 {
        fallback
    } else {
        a.map(|x| x / l)
    }
}

struct Tile {
    origin: [f64; 3],
    vb: wgpu::Buffer,
    ib: wgpu::Buffer,
    eb: wgpu::Buffer,
    n_idx: u32,
    n_edge: u32,
    ub: wgpu::Buffer,
    bg: wgpu::BindGroup,
}

/// Tool feedback drawn over the model: one flat color, project-local.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OverlayLines {
    pub color: [f32; 3],
    pub lines: Vec<([f64; 3], [f64; 3])>,
}

struct Overlay {
    origin: [f64; 3],
    color: [f32; 3],
    lines: Vec<([f64; 3], [f64; 3])>,
    vb: wgpu::Buffer,
    n: u32,
    ub: wgpu::Buffer,
    bg: wgpu::BindGroup,
}

/// One chunk of points on the GPU.
struct CloudChunk {
    min: [f64; 3],
    size: [f64; 3],
    vb: wgpu::Buffer,
    n: u32,
    ub: wgpu::Buffer,
    bg: wgpu::BindGroup,
}

pub struct Renderer {
    cloud: wgpu::RenderPipeline,
    cloud_chunks: std::collections::HashMap<(u32, usize), CloudChunk>,
    /// What the next [`Renderer::draw`] shows, in draw order.
    cloud_draws: Vec<DrawChunk>,
    tri: wgpu::RenderPipeline,
    edge: wgpu::RenderPipeline,
    over: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    tiles: Vec<Tile>,
    overlays: Vec<Overlay>,
}

impl Renderer {
    /// Pipelines for a color target of `format` with a [`DEPTH_FORMAT`] depth buffer.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tile"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bgl)],
            ..Default::default()
        });
        // Faces are pushed slightly back so coplanar edges win the depth test.
        let depth = |write: bool, bias| {
            Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(write),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias,
            })
        };
        let face_bias = wgpu::DepthBiasState {
            constant: 2,
            slope_scale: 1.0,
            clamp: 0.0,
        };
        let target = [Some(wgpu::ColorTargetState {
            format,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let tri_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4];
        let edge_attrs = wgpu::vertex_attr_array![0 => Float32x3];
        let pipeline = |vs: &str,
                        fs: &str,
                        stride: u64,
                        attrs: &[wgpu::VertexAttribute],
                        primitive,
                        depth_stencil| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(vs),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some(vs),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: stride,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: attrs,
                    })],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &target,
                }),
                primitive,
                depth_stencil,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let tri = pipeline(
            "vs",
            "fs",
            28,
            &tri_attrs,
            wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth(true, face_bias),
        );
        let edge = pipeline(
            "vs_edge",
            "fs_edge",
            12,
            &edge_attrs,
            wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth(false, Default::default()),
        );
        // Always on top: a rubber band must stay visible inside a solid.
        let over = pipeline(
            "vs_edge",
            "fs_overlay",
            12,
            &edge_attrs,
            wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
        );
        let cloud_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cloud"),
            source: wgpu::ShaderSource::Wgsl(CLOUD_SHADER.into()),
        });
        let cloud_attrs = wgpu::vertex_attr_array![0 => Unorm16x4, 1 => Unorm8x4];
        let cloud = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cloud"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &cloud_module,
                entry_point: Some("vs_cloud"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<CloudPoint>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &cloud_attrs,
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &cloud_module,
                entry_point: Some("fs_cloud"),
                compilation_options: Default::default(),
                targets: &target,
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: depth(true, Default::default()),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        Renderer {
            cloud,
            cloud_chunks: Default::default(),
            cloud_draws: Vec::new(),
            tri,
            edge,
            over,
            bgl,
            tiles: Vec::new(),
            overlays: Vec::new(),
        }
    }

    /// One uniform buffer and its bind group.
    fn uniform(&self, device: &wgpu::Device) -> (wgpu::Buffer, wgpu::BindGroup) {
        let ub = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniform"),
            size: std::mem::size_of::<Uniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ub.as_entire_binding(),
            }],
        });
        (ub, bg)
    }

    /// Replaces what the overlay draws. Each group is one draw call, positions
    /// relative to its own first point, so precision holds at UTM.
    pub fn set_overlay(&mut self, device: &wgpu::Device, groups: &[OverlayLines]) {
        self.overlays = groups
            .iter()
            .filter(|g| !g.lines.is_empty())
            .map(|g| {
                let n = 2 * g.lines.len() as u32;
                let (ub, bg) = self.uniform(device);
                Overlay {
                    origin: g.lines[0].0,
                    color: g.color,
                    lines: g.lines.clone(),
                    // Filled by `prepare`, as what shows depends on the camera.
                    vb: device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("overlay"),
                        size: u64::from(n) * 12,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    }),
                    n,
                    ub,
                    bg,
                }
            })
            .collect();
    }

    /// Uploads the tile whose frame sits at `origin` (project-local `f64`),
    /// replacing the one already there. Origins are computed, never typed, so
    /// the same tile has the same bits.
    pub fn add_tile(&mut self, device: &wgpu::Device, origin: [f64; 3], mesh: &TileMesh) {
        self.tiles
            .retain(|t| t.origin.map(f64::to_bits) != origin.map(f64::to_bits));
        let buf = |contents: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents,
                usage,
            })
        };
        let (ub, bg) = self.uniform(device);
        self.tiles.push(Tile {
            origin,
            vb: buf(
                bytemuck::cast_slice(&mesh.verts),
                wgpu::BufferUsages::VERTEX,
            ),
            ib: buf(
                bytemuck::cast_slice(&mesh.indices),
                wgpu::BufferUsages::INDEX,
            ),
            eb: buf(
                bytemuck::cast_slice(&mesh.edges),
                wgpu::BufferUsages::VERTEX,
            ),
            n_idx: mesh.indices.len() as u32,
            n_edge: mesh.edges.len() as u32,
            ub,
            bg,
        });
    }

    pub fn clear_tiles(&mut self) {
        self.tiles.clear();
    }

    /// Replaces one chunk of a cloud's points. None left: drops it.
    pub fn set_cloud_chunk(&mut self, device: &wgpu::Device, chunk: ChangedChunk<'_>) {
        let key = (chunk.id, chunk.chunk);
        if chunk.points.is_empty() {
            self.cloud_chunks.remove(&key);
            return;
        }
        let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cloud"),
            contents: bytemuck::cast_slice(chunk.points),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let (ub, bg) = match self.cloud_chunks.remove(&key) {
            Some(old) => (old.ub, old.bg),
            None => {
                let ub = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("cloud"),
                    size: std::mem::size_of::<CloudUniform>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &self.bgl,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: ub.as_entire_binding(),
                    }],
                });
                (ub, bg)
            }
        };
        self.cloud_chunks.insert(
            key,
            CloudChunk {
                min: chunk.min,
                size: chunk.size,
                vb,
                n: chunk.points.len() as u32,
                ub,
                bg,
            },
        );
    }

    /// Drops one cloud from the GPU.
    pub fn remove_cloud(&mut self, id: u32) {
        self.cloud_chunks.retain(|k, _| k.0 != id);
    }

    pub fn clear_clouds(&mut self) {
        self.cloud_chunks.clear();
        self.cloud_draws.clear();
    }

    /// Sets what the next [`Renderer::draw`] shows of the clouds. `viewport`: the
    /// target's size in pixels, `point_px`: a point's edge in those pixels.
    pub fn prepare_clouds(
        &mut self,
        queue: &wgpu::Queue,
        camera: &Camera,
        draws: &[DrawChunk],
        viewport: [u32; 2],
        point_px: f32,
        slice: Option<Slice>,
    ) {
        let view_proj = camera.view_proj_at_eye();
        // A quad's half size in clip units: a pixel is 2 / width of them, so half of
        // `point_px` pixels is `point_px / width`.
        let half_px = [
            point_px / viewport[0].max(1) as f32,
            point_px / viewport[1].max(1) as f32,
            0.0,
            0.0,
        ];
        let at = |p: [f64; 3], on: f32| {
            let r = sub(p, camera.eye).map(|x| x as f32);
            [r[0], r[1], r[2], on]
        };
        let (slice_min, slice_max) =
            slice.map_or(([0.0; 4], [0.0; 4]), |s| (at(s.min, 1.0), at(s.max, 1.0)));
        self.cloud_draws.clear();
        for d in draws {
            let Some(c) = self.cloud_chunks.get(&(d.id, d.chunk)) else {
                continue;
            };
            // The f64 subtraction is the RTC step, as for tiles.
            let o = sub(c.min, camera.eye).map(|x| x as f32);
            let s = c.size.map(|x| x as f32);
            let u = CloudUniform {
                view_proj,
                offset: [o[0], o[1], o[2], 0.0],
                scale: [s[0], s[1], s[2], 0.0],
                slice_min,
                slice_max,
                half_px,
            };
            queue.write_buffer(&c.ub, 0, bytemuck::bytes_of(&u));
            self.cloud_draws.push(*d);
        }
    }

    /// Writes each tile's uniform for `camera`. Call before [`Renderer::draw`].
    pub fn prepare(&self, queue: &wgpu::Queue, camera: &Camera) {
        let view_proj = camera.view_proj_at_eye();
        for t in &self.tiles {
            // The f64 subtraction is the RTC step: only the small result is narrowed.
            let o = sub(t.origin, camera.eye).map(|x| x as f32);
            let u = Uniform {
                view_proj,
                offset: [o[0], o[1], o[2], 0.0],
                // Unit vector; lights the two walls facing -x and -y differently.
                light: [-0.4, -0.6, 0.6928, 0.0],
            };
            queue.write_buffer(&t.ub, 0, bytemuck::bytes_of(&u));
        }
        for o in &self.overlays {
            let d = sub(o.origin, camera.eye).map(|x| x as f32);
            let c = o.color;
            let u = Uniform {
                view_proj,
                offset: [d[0], d[1], d[2], 0.0],
                light: [c[0], c[1], c[2], 0.0],
            };
            queue.write_buffer(&o.ub, 0, bytemuck::bytes_of(&u));
            // Cut to the view here rather than leave it to the driver: lavapipe
            // loses a line of 200 m or more whole in a wide view, seemingly as its
            // clipped end lands too far off screen.
            // ponytail: every line, every frame; overlays hold few. Skip this
            // when the camera has not moved if a huge preview shows up in a profile.
            let verts: Vec<[f32; 3]> = o
                .lines
                .iter()
                .flat_map(|(a, b)| {
                    let (ea, eb) = (sub(*a, camera.eye), sub(*b, camera.eye));
                    // Out of view: no length, on a point out of view.
                    let (t0, t1) = visible_span(&view_proj, ea, eb).unwrap_or((0.0, 0.0));
                    let at = |t: f64| {
                        let start = sub(*a, o.origin);
                        let along = sub(*b, *a);
                        std::array::from_fn(|k| (start[k] + along[k] * t) as f32)
                    };
                    [at(t0), at(t1)]
                })
                .collect();
            queue.write_buffer(&o.vb, 0, bytemuck::cast_slice(&verts));
        }
    }

    /// Records the draws: up to 2 per tile (faces, edges), then one per overlay
    /// group; empty buffers are skipped. Returns the number of draw calls issued.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) -> usize {
        let mut calls = 0;
        for t in &self.tiles {
            pass.set_bind_group(0, &t.bg, &[]);
            if t.n_idx > 0 {
                pass.set_pipeline(&self.tri);
                pass.set_vertex_buffer(0, t.vb.slice(..));
                pass.set_index_buffer(t.ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..t.n_idx, 0, 0..1);
                calls += 1;
            }
            if t.n_edge > 0 {
                pass.set_pipeline(&self.edge);
                pass.set_vertex_buffer(0, t.eb.slice(..));
                pass.draw(0..t.n_edge, 0..1);
                calls += 1;
            }
        }
        if !self.cloud_draws.is_empty() {
            pass.set_pipeline(&self.cloud);
        }
        for d in &self.cloud_draws {
            let Some(c) = self.cloud_chunks.get(&(d.id, d.chunk)) else {
                continue;
            };
            pass.set_bind_group(0, &c.bg, &[]);
            pass.set_vertex_buffer(0, c.vb.slice(..));
            pass.draw(0..4, 0..d.count.min(c.n));
            calls += 1;
        }
        for o in &self.overlays {
            pass.set_pipeline(&self.over);
            pass.set_bind_group(0, &o.bg, &[]);
            pass.set_vertex_buffer(0, o.vb.slice(..));
            pass.draw(0..o.n, 0..1);
            calls += 1;
        }
        calls
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(m: &[[f32; 4]; 4], p: [f32; 3]) -> [f32; 4] {
        std::array::from_fn(|r| (0..3).map(|c| m[c][r] * p[c]).sum::<f32>() + m[3][r])
    }

    #[test]
    fn visible_span_cuts_a_line_to_the_view() {
        let cam = Camera {
            eye: [0.0, 0.0, 0.0],
            target: [0.0, 10.0, 0.0],
            fovy: 0.8,
            aspect: 2.0,
            near: 0.5,
            far: 100.0,
        };
        let m = cam.view_proj_at_eye();
        let span = |a, b| visible_span(&m, a, b);
        // Wholly inside: untouched.
        assert_eq!(span([-1.0, 10.0, 0.0], [1.0, 10.0, 0.0]), Some((0.0, 1.0)));
        // Wholly off to one side, past the far plane, or behind the eye: nothing.
        assert_eq!(span([500.0, 10.0, 0.0], [600.0, 10.0, 0.0]), None);
        assert_eq!(span([0.0, 200.0, 0.0], [0.0, 300.0, 0.0]), None);
        assert_eq!(span([-1.0, -10.0, 0.0], [1.0, -10.0, 0.0]), None);
        // From behind the eye to the far side: starts at the near plane, ends at the far.
        let (t0, t1) = span([0.0, -100.0, 0.0], [0.0, 300.0, 0.0]).unwrap();
        assert!((-100.0 + 400.0 * t0 - 0.5).abs() < 1e-3, "{t0}");
        assert!((-100.0 + 400.0 * t1 - 100.0).abs() < 1e-3, "{t1}");
        // Across the view, wider than it: symmetric, and inside what the fov allows.
        let half = 10.0 * (0.4f64).tan() * 2.0;
        let (t0, t1) = span([-1000.0, 10.0, 0.0], [1000.0, 10.0, 0.0]).unwrap();
        assert!((-1000.0 + 2000.0 * t0 + half).abs() < 1e-3, "{t0}");
        assert!((-1000.0 + 2000.0 * t1 - half).abs() < 1e-3, "{t1}");
        // Both ends out, passing the view by on the outside of a corner: nothing.
        assert_eq!(span([100.0, 10.0, 0.0], [10.0, 0.0, 0.0]), None);
    }

    #[test]
    fn straight_down_and_degenerate_views_are_finite() {
        let mut cam = Camera {
            eye: [5.0, 5.0, 100.0],
            target: [5.0, 5.0, 0.0],
            fovy: 0.8,
            aspect: 1.0,
            near: 0.5,
            far: 500.0,
        };
        assert!(
            cam.view_proj_at_eye()
                .iter()
                .flatten()
                .all(|v| v.is_finite()),
            "top-down view"
        );
        cam.target = cam.eye;
        assert!(
            cam.view_proj_at_eye()
                .iter()
                .flatten()
                .all(|v| v.is_finite()),
            "eye == target"
        );
        cam.target = [5.0, 5.0, 0.0];
        let c = clip(&cam.view_proj_at_eye(), [0.0, 0.0, -100.0]);
        assert!(
            c[3] > 0.0 && (c[0] / c[3]).abs() < 1e-5 && (c[1] / c[3]).abs() < 1e-5,
            "top-down target not centered: {c:?}"
        );
    }

    #[test]
    fn target_projects_to_screen_center_in_front() {
        let cam = Camera {
            eye: [430_010.0, 9_150_020.0, 30.0],
            target: [430_000.0, 9_150_000.0, 5.0],
            fovy: 0.8,
            aspect: 1.5,
            near: 0.5,
            far: 500.0,
        };
        let m = cam.view_proj_at_eye();
        // The target relative to the eye, computed in f64 as the renderer does.
        let rel = sub(cam.target, cam.eye).map(|x| x as f32);
        let c = clip(&m, rel);
        assert!(c[3] > 0.0, "target behind the camera");
        assert!(
            (c[0] / c[3]).abs() < 1e-5 && (c[1] / c[3]).abs() < 1e-5,
            "not centered: {c:?}"
        );
        let depth = c[2] / c[3];
        assert!((0.0..1.0).contains(&depth), "outside depth range: {depth}");
    }
}
