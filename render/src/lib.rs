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
use wgpu::util::DeviceExt;

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

const SHADER: &str = r#"
struct U { view_proj: mat4x4<f32>, offset: vec4<f32>, light: vec4<f32> };
@group(0) @binding(0) var<uniform> u: U;
struct O { @builtin(position) p: vec4<f32>, @location(0) n: vec3<f32> };
@vertex fn vs(@location(0) pos: vec3<f32>, @location(1) n: vec3<f32>) -> O {
  var o: O; o.p = u.view_proj * vec4<f32>(pos + u.offset.xyz, 1.0); o.n = n; return o;
}
@fragment fn fs(i: O) -> @location(0) vec4<f32> {
  let l = 0.35 + 0.65 * max(dot(normalize(i.n), u.light.xyz), 0.0);
  return vec4<f32>(vec3<f32>(0.85, 0.82, 0.78) * l, 1.0);
}
@vertex fn vs_edge(@location(0) pos: vec3<f32>) -> @builtin(position) vec4<f32> {
  return u.view_proj * vec4<f32>(pos + u.offset.xyz, 1.0);
}
@fragment fn fs_edge() -> @location(0) vec4<f32> { return vec4<f32>(0.1, 0.1, 0.1, 1.0); }
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 3],
    nrm: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniform {
    view_proj: [[f32; 4]; 4],
    offset: [f32; 4],
    light: [f32; 4],
}

/// CPU-side merged geometry of one tile, in the tile's own frame.
#[derive(Default)]
pub struct TileMesh {
    verts: Vec<Vertex>,
    indices: Vec<u32>,
    edges: Vec<[f32; 3]>,
}

impl TileMesh {
    /// Merges every visible world Object of `doc` (the tile's kernel document).
    pub fn from_document(doc: &kernel::Document) -> Result<Self, tessellate::TessellateError> {
        let palette = kernel::MaterialPalette::default();
        let mut m = TileMesh::default();
        for oid in doc.live_object_ids() {
            if !doc.is_world_object(oid) {
                continue;
            }
            let Some(obj) = doc.object(oid) else { continue };
            let t = tessellate::tessellate(obj, &palette)?;
            let base = m.verts.len() as u32;
            for (p, n) in t.positions.chunks(3).zip(t.normals.chunks(3)) {
                m.verts.push(Vertex {
                    pos: [p[0], p[1], p[2]],
                    nrm: [n[0], n[1], n[2]],
                });
            }
            m.indices.extend(t.indices.iter().map(|i| i + base));
            m.edges
                .extend(t.edge_positions.chunks(3).map(|p| [p[0], p[1], p[2]]));
        }
        Ok(m)
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Axis-aligned bounds in the tile frame, `None` when empty.
    pub fn bounds(&self) -> Option<([f32; 3], [f32; 3])> {
        let mut it = self.verts.iter().map(|v| v.pos);
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
        let f = norm(sub(self.target, self.eye));
        let s = norm(cross(f, [0.0, 0.0, 1.0]));
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
fn norm(a: [f64; 3]) -> [f64; 3] {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    a.map(|x| x / l)
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

pub struct Renderer {
    tri: wgpu::RenderPipeline,
    edge: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    tiles: Vec<Tile>,
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
        let depth = Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: Default::default(),
            bias: Default::default(),
        });
        let target = [Some(wgpu::ColorTargetState {
            format,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let tri_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
        let edge_attrs = wgpu::vertex_attr_array![0 => Float32x3];
        let pipeline =
            |vs: &str, fs: &str, stride: u64, attrs: &[wgpu::VertexAttribute], primitive| {
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
                    depth_stencil: depth.clone(),
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                })
            };
        let tri = pipeline(
            "vs",
            "fs",
            24,
            &tri_attrs,
            wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
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
        );
        Renderer {
            tri,
            edge,
            bgl,
            tiles: Vec::new(),
        }
    }

    /// Uploads one tile whose frame sits at `origin` (project-local `f64`).
    pub fn add_tile(&mut self, device: &wgpu::Device, origin: [f64; 3], mesh: &TileMesh) {
        let buf = |contents: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents,
                usage,
            })
        };
        let ub = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tile uniform"),
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

    /// Writes each tile's uniform for `camera`. Call before [`Renderer::draw`].
    pub fn prepare(&self, queue: &wgpu::Queue, camera: &Camera) {
        let view_proj = camera.view_proj_at_eye();
        for t in &self.tiles {
            // The f64 subtraction is the RTC step: only the small result is narrowed.
            let o = sub(t.origin, camera.eye).map(|x| x as f32);
            let u = Uniform {
                view_proj,
                offset: [o[0], o[1], o[2], 0.0],
                light: [0.4, 0.3, 0.866, 0.0],
            };
            queue.write_buffer(&t.ub, 0, bytemuck::bytes_of(&u));
        }
    }

    /// Records the draws: 2 per tile (faces, edges). Returns the draw-call count.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) -> usize {
        for t in &self.tiles {
            pass.set_bind_group(0, &t.bg, &[]);
            pass.set_pipeline(&self.tri);
            pass.set_vertex_buffer(0, t.vb.slice(..));
            pass.set_index_buffer(t.ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..t.n_idx, 0, 0..1);
            pass.set_pipeline(&self.edge);
            pass.set_vertex_buffer(0, t.eb.slice(..));
            pass.draw(0..t.n_edge, 0..1);
        }
        2 * self.tiles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(m: &[[f32; 4]; 4], p: [f32; 3]) -> [f32; 4] {
        std::array::from_fn(|r| (0..3).map(|c| m[c][r] * p[c]).sum::<f32>() + m[3][r])
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
