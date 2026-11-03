//! WangunGIS desktop shell (F0): open a `.hew` or `.wangun` project, orbit/pan/zoom,
//! zoom extents. A `.hew` reaching past the kernel frame bound is split into
//! kernel tiles on open and can be saved as `.wangun`.
//! The 3D view renders into an offscreen texture shown as an egui image, so
//! the renderer owns its depth buffer and knows nothing about egui.
//! Usage: wangungis [file.hew | file.wangun]

mod camera;

use camera::Orbit;
use eframe::{egui, egui_wgpu};
use wangungis_render::{DEPTH_FORMAT, Renderer, TileMesh};
use wangungis_scene::ProjectOrigin;
use wangungis_scene::project::{Project, TileKey};

/// egui-wgpu 0.37 `register_native_texture` requires Rgba8Unorm; the golden
/// tests in `render` use the same format.
const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

fn main() -> eframe::Result {
    let path = std::env::args().nth(1);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("WangunGIS"),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "WangunGIS",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc, path)))),
    )
}

/// Facts about the open model, shown in the status bar.
struct Model {
    name: String,
    objects: usize,
    triangles: usize,
    origin: Option<ProjectOrigin>,
    tiles: usize,
    /// Opened as one `.hew` and split into kernel tiles in memory.
    split: bool,
}

struct Viewport {
    size: [u32; 2],
    color: wgpu::Texture,
    depth: wgpu::TextureView,
    id: egui::TextureId,
}

struct App {
    rs: egui_wgpu::RenderState,
    renderer: Renderer,
    view: Option<Viewport>,
    orbit: Orbit,
    bounds: Option<([f64; 3], [f64; 3])>,
    /// Zoom extents on the next frame, once the viewport aspect is known.
    fit_pending: bool,
    path: String,
    model: Option<Model>,
    project: Option<Project>,
    error: Option<String>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, path: Option<String>) -> Self {
        let rs = cc
            .wgpu_render_state
            .clone()
            .expect("eframe started with the wgpu renderer");
        let renderer = Renderer::new(&rs.device, COLOR_FORMAT);
        let mut app = App {
            rs,
            renderer,
            view: None,
            orbit: Orbit::default(),
            bounds: None,
            fit_pending: false,
            path: path.clone().unwrap_or_default(),
            model: None,
            project: None,
            error: None,
        };
        if path.is_some() {
            app.open();
        }
        app
    }

    fn open(&mut self) {
        self.error = None;
        match load(&self.path) {
            Ok((project, meshes)) => {
                self.renderer.clear_tiles();
                let mut placed = Vec::new();
                for (key, mesh) in &meshes {
                    self.renderer.add_tile(&self.rs.device, key.center(), mesh);
                    placed.push((key.center(), mesh.bounds()));
                }
                self.bounds = union_bounds(&placed);
                self.orbit = Orbit::default();
                self.fit_pending = true;
                let is_container = project.single().is_none();
                self.model = Some(Model {
                    name: std::path::Path::new(&self.path)
                        .file_name()
                        .map_or(self.path.clone(), |n| n.to_string_lossy().into_owned()),
                    objects: project
                        .tiles
                        .values()
                        .map(|d| {
                            d.live_object_ids()
                                .into_iter()
                                .filter(|o| d.is_world_object(*o))
                                .count()
                        })
                        .sum(),
                    triangles: meshes.iter().map(|(_, m)| m.triangle_count()).sum(),
                    origin: project.origin,
                    tiles: project.tiles.len(),
                    split: is_container && !self.path.ends_with(".wangun"),
                });
                self.project = Some(project);
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// Writes the split project next to the opened `.hew`, never over it.
    fn save_project(&mut self) {
        let Some(p) = &self.project else { return };
        let target = std::path::Path::new(&self.path).with_extension("wangun");
        self.error = std::fs::write(&target, p.save())
            .map(|()| self.path = target.to_string_lossy().into_owned())
            .err()
            .map(|e| format!("Tidak bisa menyimpan {}: {e}", target.display()));
        if let Some(m) = &mut self.model
            && self.error.is_none()
        {
            m.split = false;
            m.name = target
                .file_name()
                .map_or(String::new(), |n| n.to_string_lossy().into_owned());
        }
    }

    /// (Re)creates the offscreen target when the panel size changes.
    fn ensure_view(&mut self, size: [u32; 2]) {
        if self.view.as_ref().is_some_and(|v| v.size == size) {
            return;
        }
        let device = &self.rs.device;
        let tex = |format, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("viewport"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
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
            COLOR_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let depth = tex(DEPTH_FORMAT, wgpu::TextureUsages::RENDER_ATTACHMENT)
            .create_view(&Default::default());
        let cv = color.create_view(&Default::default());
        let mut egui_renderer = self.rs.renderer.write();
        let id = match &self.view {
            Some(v) => {
                egui_renderer.update_egui_texture_from_wgpu_texture(
                    device,
                    &cv,
                    wgpu::FilterMode::Linear,
                    v.id,
                );
                v.id
            }
            None => egui_renderer.register_native_texture(device, &cv, wgpu::FilterMode::Linear),
        };
        self.view = Some(Viewport {
            size,
            color,
            depth,
            id,
        });
    }

    fn render(&self) {
        let Some(v) = &self.view else { return };
        let cam = self.orbit.camera(v.size[0] as f32 / v.size[1] as f32);
        self.renderer.prepare(&self.rs.queue, &cam);
        let cv = v.color.create_view(&Default::default());
        let mut enc = self.rs.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &cv,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.93,
                            g: 0.94,
                            b: 0.95,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &v.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            self.renderer.draw(&mut pass);
        }
        self.rs.queue.submit([enc.finish()]);
    }
}

type TileMeshes = Vec<(TileKey, TileMesh)>;

fn load(path: &str) -> Result<(Project, TileMeshes), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Tidak bisa membaca {path}: {e}"))?;
    let project = Project::open(&bytes).map_err(|e| format!("Tidak bisa membuka proyek: {e:?}"))?;
    let meshes = project
        .tiles
        .iter()
        .map(|(key, doc)| {
            TileMesh::from_document(doc)
                .map(|m| (*key, m))
                .map_err(|e| format!("Gagal membuat mesh tile {key:?}: {e:?}"))
        })
        .collect::<Result<_, _>>()?;
    Ok((project, meshes))
}

/// Project-local bounds of all tiles: each tile's own bounds shifted by its
/// center. `None` when every tile is empty.
/// Tile center plus that tile's own bounds (`TileMesh::bounds`).
type PlacedBounds = ([f64; 3], Option<([f32; 3], [f32; 3])>);

fn union_bounds(tiles: &[PlacedBounds]) -> Option<([f64; 3], [f64; 3])> {
    tiles
        .iter()
        .filter_map(|(c, b)| {
            b.map(|(lo, hi)| {
                (
                    std::array::from_fn(|k| c[k] + f64::from(lo[k])),
                    std::array::from_fn(|k| c[k] + f64::from(hi[k])),
                )
            })
        })
        .reduce(|(alo, ahi): ([f64; 3], [f64; 3]), (blo, bhi)| {
            (
                std::array::from_fn(|k| alo[k].min(blo[k])),
                std::array::from_fn(|k| ahi[k].max(bhi[k])),
            )
        })
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("File .hew / .wangun:");
                let field = ui.add(egui::TextEdit::singleline(&mut self.path).desired_width(480.0));
                let enter = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button("Buka").clicked() || enter {
                    self.open();
                }
                if ui.button("Zoom extents").clicked() {
                    self.fit_pending = true;
                }
                if self.model.as_ref().is_some_and(|m| m.split)
                    && ui.button("Simpan proyek (.wangun)").clicked()
                {
                    self.save_project();
                }
            });
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(e) = &self.error {
                    ui.colored_label(egui::Color32::RED, e);
                } else if let Some(m) = &self.model {
                    ui.label(format!(
                        "{} · {} tile · {} objek · {} segitiga",
                        m.name, m.tiles, m.objects, m.triangles
                    ));
                    ui.separator();
                    match &m.origin {
                        Some(o) => ui.label(format!(
                            "EPSG:{} · origin ({:.3}, {:.3}, {:.3})",
                            o.epsg, o.origin[0], o.origin[1], o.origin[2]
                        )),
                        None => ui.label("Tanpa origin proyek (koordinat lokal)"),
                    };
                    if m.split {
                        ui.separator();
                        ui.colored_label(
                            egui::Color32::from_rgb(200, 120, 0),
                            format!(
                                "Dipecah menjadi {} tile kernel 1 km (belum disimpan)",
                                m.tiles
                            ),
                        );
                    }
                } else {
                    ui.label("Belum ada file. Ketik path .hew atau .wangun lalu tekan Buka.");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let dt = ui.input(|i| i.stable_dt);
                    ui.label(format!("{:.1} ms/frame", dt * 1000.0));
                });
            });
        });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let size = ui.available_size();
                let ppp = ui.ctx().pixels_per_point();
                let max = self.rs.device.limits().max_texture_dimension_2d as f32;
                let px = [size.x, size.y].map(|v| (v * ppp).round().clamp(1.0, max) as u32);
                self.ensure_view(px);
                if std::mem::take(&mut self.fit_pending)
                    && let Some((lo, hi)) = self.bounds
                {
                    self.orbit.fit(lo, hi, px[0] as f32 / px[1] as f32);
                }
                let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::drag());
                let d = resp.drag_delta();
                let pan = resp.dragged_by(egui::PointerButton::Middle)
                    || resp.dragged_by(egui::PointerButton::Secondary)
                    || (resp.dragged_by(egui::PointerButton::Primary)
                        && ui.input(|i| i.modifiers.shift));
                if pan {
                    self.orbit.pan(d.x, d.y, size.y);
                } else if resp.dragged_by(egui::PointerButton::Primary) {
                    self.orbit.orbit(d.x, d.y);
                }
                if resp.hovered() {
                    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                    if scroll != 0.0 {
                        self.orbit.zoom(scroll / 50.0);
                    }
                }
                self.render();
                if let Some(v) = &self.view {
                    egui::Image::new((v.id, size)).paint_at(ui, rect);
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_union_shifts_each_tile_by_its_center() {
        let b = Some(([-1.0f32, -2.0, 0.0], [1.0f32, 2.0, 3.0]));
        let got = union_bounds(&[
            ([0.0; 3], b),
            ([2_000.0, -1_000.0, 0.0], b),
            ([9_000.0, 9_000.0, 0.0], None),
        ]);
        assert_eq!(got, Some(([-1.0, -1_002.0, 0.0], [2_001.0, 2.0, 3.0])));
        assert_eq!(union_bounds(&[([5.0; 3], None)]), None);
    }
}
