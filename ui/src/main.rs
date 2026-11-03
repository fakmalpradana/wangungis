//! WangunGIS desktop shell (F0): open a `.hew`, orbit/pan/zoom, zoom extents.
//! The 3D view renders into an offscreen texture shown as an egui image, so
//! the renderer owns its depth buffer and knows nothing about egui.
//! Usage: wangungis [file.hew]

mod camera;

use camera::Orbit;
use eframe::{egui, egui_wgpu};
use wangungis_render::{DEPTH_FORMAT, Renderer, TileMesh};
use wangungis_scene::{KERNEL_FRAME_BOUND_M, ProjectOrigin};

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
    /// `Err` when the `wangungis` attributes exist but are malformed.
    origin: Result<Option<ProjectOrigin>, String>,
    /// Some geometry lies farther than KERNEL_FRAME_BOUND_M from the document origin.
    outside_frame: bool,
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
            Ok((doc, mesh)) => {
                self.renderer.clear_tiles();
                // F0: one document = one tile whose frame is the project origin.
                self.renderer.add_tile(&self.rs.device, [0.0; 3], &mesh);
                self.bounds = mesh
                    .bounds()
                    .map(|(lo, hi)| (lo.map(f64::from), hi.map(f64::from)));
                let outside_frame = f64::from(mesh.horizontal_reach()) > KERNEL_FRAME_BOUND_M;
                self.orbit = Orbit::default();
                self.fit_pending = true;
                self.model = Some(Model {
                    name: std::path::Path::new(&self.path)
                        .file_name()
                        .map_or(self.path.clone(), |n| n.to_string_lossy().into_owned()),
                    objects: doc
                        .live_object_ids()
                        .into_iter()
                        .filter(|o| doc.is_world_object(*o))
                        .count(),
                    triangles: mesh.triangle_count(),
                    origin: ProjectOrigin::read(&doc).map_err(|e| format!("{e:?}")),
                    outside_frame,
                });
            }
            Err(e) => self.error = Some(e),
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

fn load(path: &str) -> Result<(kernel::Document, TileMesh), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Tidak bisa membaca {path}: {e}"))?;
    let doc =
        kernel::Document::load(&bytes).map_err(|e| format!("Bukan file .hew yang valid: {e:?}"))?;
    let mesh = TileMesh::from_document(&doc).map_err(|e| format!("Gagal membuat mesh: {e:?}"))?;
    Ok((doc, mesh))
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("File .hew:");
                let field = ui.add(egui::TextEdit::singleline(&mut self.path).desired_width(480.0));
                let enter = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button("Buka").clicked() || enter {
                    self.open();
                }
                if ui.button("Zoom extents").clicked() {
                    self.fit_pending = true;
                }
            });
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(e) = &self.error {
                    ui.colored_label(egui::Color32::RED, e);
                } else if let Some(m) = &self.model {
                    ui.label(format!(
                        "{} · {} objek · {} segitiga",
                        m.name, m.objects, m.triangles
                    ));
                    ui.separator();
                    match &m.origin {
                        Ok(Some(o)) => ui.label(format!(
                            "EPSG:{} · origin ({:.3}, {:.3}, {:.3})",
                            o.epsg, o.origin[0], o.origin[1], o.origin[2]
                        )),
                        Ok(None) => ui.label("Tanpa origin proyek (koordinat lokal)"),
                        Err(e) => ui.colored_label(
                            egui::Color32::RED,
                            format!("Atribut origin proyek rusak: {e}"),
                        ),
                    };
                    if m.outside_frame {
                        ui.separator();
                        ui.colored_label(
                            egui::Color32::from_rgb(200, 120, 0),
                            "Geometri > 1,5 km dari origin dokumen: edit kernel bisa ditolak",
                        );
                    }
                } else {
                    ui.label("Belum ada file. Ketik path .hew lalu tekan Buka.");
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
