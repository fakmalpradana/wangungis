//! WangunGIS desktop shell: open a `.hew` or `.wangun` project, navigate, draw
//! with Line and Rectangle, Push/Pull, undo. A `.hew` reaching past the kernel
//! frame bound is split into kernel tiles on open.
//! All editing logic lives in `tools`; this file turns pixels into rays and
//! keys into tool inputs. The 3D view renders into an offscreen texture shown
//! as an egui image, so the renderer owns its depth buffer.
//! Usage: wangungis [file.hew | file.wangun]

mod camera;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use camera::Orbit;
use eframe::{egui, egui_wgpu};
use inference::{Axis, PickRay, SnapKind};
use kernel::{Point3, Vec3};
use wangungis_render::{DEPTH_FORMAT, OverlayLines, Renderer, TileMesh};
use wangungis_scene::project::{Project, TileKey};
use wangungis_tools::Scene;
use wangungis_tools::tool::{Input, Preview, Tool};

/// egui-wgpu 0.37 `register_native_texture` requires Rgba8Unorm; the golden
/// tests in `render` use the same format.
const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// How far from the cursor a snap reaches, in viewport units.
const SNAP_REACH: f32 = 8.0;
/// Half-size of the snap marker, in viewport units.
const MARKER: f64 = 6.0;
/// Half-length (m) of the drawn project axes.
const AXIS_LEN: f64 = 500.0;
const PREVIEW_COLOR: [f32; 3] = [0.95, 0.45, 0.0];

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

/// One tile's numbers for the status bar and zoom extents.
struct TileStats {
    objects: usize,
    triangles: usize,
    bounds: Option<([f32; 3], [f32; 3])>,
}

/// Where the open project lives on disk.
#[derive(Default)]
struct Source {
    /// `None` for a project that was never saved.
    path: Option<PathBuf>,
    /// The file holds a project container, not a plain `.hew`.
    container: bool,
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
    /// Zoom extents on the next frame, once the viewport aspect is known.
    fit_pending: bool,
    path: String,
    scene: Scene,
    tool: Tool,
    /// What the user is typing for the tool.
    typed: String,
    stats: BTreeMap<TileKey, TileStats>,
    source: Source,
    /// Differs from the file: edited, or split on open.
    unsaved: bool,
    split_on_open: bool,
    overlay_shown: Option<Preview>,
    last_pointer: Option<egui::Pos2>,
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
            fit_pending: false,
            path: path.clone().unwrap_or_default(),
            scene: empty_scene(),
            tool: Tool::select(0.01),
            typed: String::new(),
            stats: BTreeMap::new(),
            source: Source::default(),
            unsaved: false,
            split_on_open: false,
            overlay_shown: None,
            last_pointer: None,
            error: None,
        };
        if path.is_some() {
            app.open();
        }
        app
    }

    /// Starts over on `scene`: every tile uploaded, tool and typing cleared.
    fn adopt(&mut self, scene: Scene) {
        self.scene = scene;
        self.renderer.clear_tiles();
        self.stats.clear();
        self.tool.reset();
        self.typed.clear();
        self.overlay_shown = None;
        let keys: Vec<TileKey> = self.scene.project().tiles.keys().copied().collect();
        for key in keys {
            self.upload(key);
        }
        self.scene.take_dirty();
        self.orbit = Orbit::default();
    }

    fn new_project(&mut self) {
        self.error = None;
        self.adopt(empty_scene());
        self.source = Source::default();
        self.unsaved = false;
        self.split_on_open = false;
    }

    fn open(&mut self) {
        self.error = None;
        let opened = std::fs::read(&self.path)
            .map_err(|e| format!("Tidak bisa membaca {}: {e}", self.path))
            .and_then(|bytes| {
                Project::open(&bytes)
                    .map(|p| (p, Project::is_container(&bytes)))
                    .map_err(|e| format!("Tidak bisa membuka proyek: {e:?}"))
            });
        match opened {
            Ok((project, container)) => {
                self.split_on_open = !container && project.single().is_none();
                self.unsaved = self.split_on_open;
                self.source = Source {
                    path: Some(self.path.clone().into()),
                    container,
                };
                self.adopt(Scene::new(project));
                self.fit_pending = true;
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn save(&mut self) {
        let project = self.scene.project();
        let single = project.single().is_some();
        let target = match save_target(&self.source, &self.path, single) {
            Ok(t) => t,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        match write_atomic(&target, &project.save()) {
            Ok(()) => {
                self.path = target.to_string_lossy().into_owned();
                self.source = Source {
                    path: Some(target),
                    container: !single,
                };
                self.unsaved = false;
                self.split_on_open = false;
                self.error = None;
            }
            Err(e) => self.error = Some(format!("Tidak bisa menyimpan {}: {e}", target.display())),
        }
    }

    /// Rebuilds one tile's mesh and numbers from its document.
    fn upload(&mut self, key: TileKey) {
        let Some(doc) = self.scene.project().tiles.get(&key) else {
            return;
        };
        match TileMesh::from_document(doc) {
            Ok(mesh) => {
                self.renderer.add_tile(&self.rs.device, key.center(), &mesh);
                self.stats.insert(
                    key,
                    TileStats {
                        objects: doc.visible_object_ids().len(),
                        triangles: mesh.triangle_count(),
                        bounds: mesh.bounds(),
                    },
                );
            }
            Err(e) => self.error = Some(format!("Gagal membuat mesh tile {key:?}: {e:?}")),
        }
    }

    /// Re-uploads whatever the last edit touched.
    fn sync_edits(&mut self) {
        let dirty = self.scene.take_dirty();
        if !dirty.is_empty() {
            self.unsaved = true;
        }
        for key in dirty {
            self.upload(key);
        }
    }

    fn feed(&mut self, input: Input) {
        match self.tool.handle(&mut self.scene, input) {
            Ok(()) => {}
            Err(e) => self.error = Some(e.to_string()),
        }
        self.sync_edits();
    }

    fn switch_tool(&mut self, make: fn(f64) -> Tool) {
        self.tool = make(0.01);
        self.typed.clear();
        self.error = None;
    }

    fn undo(&mut self) {
        self.history(|s| s.undo());
    }

    fn redo(&mut self) {
        self.history(|s| s.redo());
    }

    fn history(
        &mut self,
        step: fn(&mut Scene) -> Result<Option<TileKey>, wangungis_tools::EditError>,
    ) {
        match step(&mut self.scene) {
            Ok(_) => self.error = None,
            Err(e) => self.error = Some(e.to_string()),
        }
        // Ids held by a gesture may be stale now.
        self.tool.reset();
        self.typed.clear();
        self.sync_edits();
    }

    fn bounds(&self) -> Option<([f64; 3], [f64; 3])> {
        let placed: Vec<PlacedBounds> = self
            .stats
            .iter()
            .map(|(k, s)| (k.center(), s.bounds))
            .collect();
        union_bounds(&placed)
    }

    /// Keys and typed text, unless a text box has them.
    fn handle_keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        for ev in ctx.input(|i| i.events.clone()) {
            match ev {
                egui::Event::Text(text) => {
                    for ch in text.chars() {
                        if typed_accepts(&self.typed, ch) {
                            self.typed.push(ch);
                        }
                    }
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => self.handle_key(key, modifiers),
                _ => {}
            }
        }
    }

    fn handle_key(&mut self, key: egui::Key, m: egui::Modifiers) {
        use egui::Key as K;
        if m.command {
            match (key, m.shift) {
                (K::Z, false) => self.undo(),
                (K::Z, true) => self.redo(),
                (K::S, _) => self.save(),
                _ => {}
            }
            return;
        }
        match key {
            K::Enter if !self.typed.is_empty() => {
                let text = std::mem::take(&mut self.typed);
                self.error = None;
                self.feed(Input::Typed(text));
            }
            K::Backspace => {
                self.typed.pop();
            }
            K::Escape => {
                self.typed.clear();
                self.error = None;
                self.feed(Input::Cancel);
            }
            K::ArrowRight => self.feed(Input::LockAxis(Axis::X)),
            K::ArrowLeft => self.feed(Input::LockAxis(Axis::Y)),
            K::ArrowUp => self.feed(Input::LockAxis(Axis::Z)),
            // Letters that are units only count as tools with nothing typed.
            K::Space if self.typed.is_empty() => self.switch_tool(Tool::select),
            K::L if self.typed.is_empty() => self.switch_tool(Tool::line),
            K::R if self.typed.is_empty() => self.switch_tool(Tool::rectangle),
            K::P if self.typed.is_empty() => self.switch_tool(Tool::push_pull),
            _ => {}
        }
    }

    /// Pushes the tool's preview and the snap marker to the renderer, when
    /// they changed.
    fn refresh_overlay(&mut self, viewport_h: f32) {
        let preview = self.tool.preview();
        if self.overlay_shown.as_ref() == Some(preview) {
            return;
        }
        let p3 = |p: Point3| [p.x, p.y, p.z];
        let mut groups = axes_overlay();
        groups.push(OverlayLines {
            color: PREVIEW_COLOR,
            lines: preview
                .lines
                .iter()
                .map(|(a, b)| (p3(*a), p3(*b)))
                .collect(),
        });
        if let Some(hit) = preview.snap {
            let at = p3(hit.snap.position);
            let eye = self.orbit.eye();
            let d = (0..3).map(|k| (at[k] - eye[k]).powi(2)).sum::<f64>().sqrt();
            let r = MARKER * Orbit::m_per_unit(d, viewport_h);
            groups.push(OverlayLines {
                color: snap_color(hit.snap.kind),
                lines: (0..3)
                    .map(|k| {
                        let (mut a, mut b) = (at, at);
                        a[k] -= r;
                        b[k] += r;
                        (a, b)
                    })
                    .collect(),
            });
        }
        self.renderer.set_overlay(&self.rs.device, &groups);
        self.overlay_shown = Some(preview.clone());
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

fn empty_scene() -> Scene {
    Scene::new(Project::new(None).expect("an empty project without origin is always valid"))
}

/// The project axes, so an empty view still has a reference.
fn axes_overlay() -> Vec<OverlayLines> {
    (0..3)
        .map(|k| {
            let mut color = [0.0f32; 3];
            color[k] = 0.8;
            let (mut a, mut b) = ([0.0; 3], [0.0; 3]);
            a[k] = -AXIS_LEN;
            b[k] = AXIS_LEN;
            OverlayLines {
                color,
                lines: vec![(a, b)],
            }
        })
        .collect()
}

fn snap_color(kind: SnapKind) -> [f32; 3] {
    match kind {
        SnapKind::Endpoint => [0.0, 0.7, 0.1],
        SnapKind::Midpoint => [0.0, 0.7, 0.8],
        SnapKind::OnEdge => [0.85, 0.1, 0.1],
        SnapKind::OnFace => [0.1, 0.2, 0.9],
        _ => [0.7, 0.1, 0.7],
    }
}

fn snap_label(kind: SnapKind) -> &'static str {
    match kind {
        SnapKind::Endpoint => "Titik ujung",
        SnapKind::Midpoint => "Titik tengah",
        SnapKind::Center => "Titik pusat",
        SnapKind::OnEdge => "Pada tepi",
        SnapKind::OnFace => "Pada face",
        SnapKind::OnAxis => "Pada sumbu",
        _ => "Inferensi",
    }
}

/// Whether `ch` extends the value being typed. Digits and punctuation always
/// start a value; unit letters only continue one, since alone they mean tools.
fn typed_accepts(typed: &str, ch: char) -> bool {
    match ch {
        '0'..='9' | '.' | ',' | ';' | '-' | '+' => true,
        'x' | 'X' | 'm' | 'M' | 'c' | 'C' | ' ' => !typed.is_empty(),
        _ => false,
    }
}

/// Where Simpan writes. A single-tile project is a plain `.hew`, anything else
/// a `.wangun`. A file is overwritten only when it already holds that kind, so
/// a `.hew` that was split or grew tiles is never replaced by a container.
fn save_target(source: &Source, path_box: &str, single: bool) -> Result<PathBuf, String> {
    let base: PathBuf = match &source.path {
        Some(p) => p.clone(),
        None if path_box.trim().is_empty() => {
            return Err("Isi path file di kotak atas dulu, lalu tekan Simpan.".into());
        }
        None => path_box.trim().into(),
    };
    let same_kind = source.path.is_some() && source.container != single;
    Ok(if same_kind {
        base
    } else if single {
        base.with_extension("hew")
    } else {
        wangun_path(&base)
    })
}

/// Writes through a temporary file, so a failed write cannot damage `target`.
fn write_atomic(target: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut tmp = target.as_os_str().to_owned();
    tmp.push(".tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, target)
}

/// `kota.hew` (any case) to `kota.wangun`; other names get `.wangun` appended.
fn wangun_path(source: &std::path::Path) -> std::path::PathBuf {
    let is_hew = source
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("hew"));
    let mut name = if is_hew {
        source.with_extension("").into_os_string()
    } else {
        source.as_os_str().to_owned()
    };
    name.push(".wangun");
    name.into()
}

/// Tile center plus that tile's own bounds (`TileMesh::bounds`).
/// Project-local bounds of all tiles: each tile's own bounds shifted by its
/// center. `None` when every tile is empty.
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
        self.handle_keys(ui.ctx());
        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("File:");
                let field = ui.add(egui::TextEdit::singleline(&mut self.path).desired_width(360.0));
                let enter = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button("Buka").clicked() || enter {
                    self.open();
                }
                if ui.button("Baru").clicked() {
                    self.new_project();
                }
                if ui.button("Simpan").clicked() {
                    self.save();
                }
                ui.separator();
                for (label, make) in [
                    ("Pilih (Spasi)", Tool::select as fn(f64) -> Tool),
                    ("Garis (L)", Tool::line),
                    ("Persegi (R)", Tool::rectangle),
                    ("Push/Pull (P)", Tool::push_pull),
                ] {
                    if ui
                        .selectable_label(self.tool.same_kind(&make(0.0)), label)
                        .clicked()
                    {
                        self.switch_tool(make);
                    }
                }
                ui.separator();
                if ui
                    .add_enabled(self.scene.can_undo(), egui::Button::new("Undo"))
                    .clicked()
                {
                    self.undo();
                }
                if ui
                    .add_enabled(self.scene.can_redo(), egui::Button::new("Redo"))
                    .clicked()
                {
                    self.redo();
                }
                if ui.button("Zoom extents").clicked() {
                    self.fit_pending = true;
                }
            });
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Nilai:");
                ui.monospace(if self.typed.is_empty() {
                    "—".to_string()
                } else {
                    format!("{}▏", self.typed)
                });
                ui.separator();
                if let Some(hit) = self.tool.preview().snap {
                    ui.label(snap_label(hit.snap.kind));
                    ui.separator();
                }
                if let Some(e) = &self.error {
                    ui.colored_label(egui::Color32::RED, e);
                    ui.separator();
                }
                ui.label(format!(
                    "{} · {} tile · {} objek · {} segitiga",
                    self.source
                        .path
                        .as_deref()
                        .and_then(Path::file_name)
                        .map_or("Proyek baru".into(), |n| n.to_string_lossy()),
                    self.stats.len(),
                    self.stats.values().map(|s| s.objects).sum::<usize>(),
                    self.stats.values().map(|s| s.triangles).sum::<usize>(),
                ));
                ui.separator();
                match self.scene.project().origin {
                    Some(o) => ui.label(format!(
                        "EPSG:{} · origin ({:.3}, {:.3}, {:.3})",
                        o.epsg, o.origin[0], o.origin[1], o.origin[2]
                    )),
                    None => ui.label("Tanpa origin proyek"),
                };
                if self.split_on_open {
                    ui.separator();
                    ui.colored_label(
                        egui::Color32::from_rgb(200, 120, 0),
                        format!("Dipecah menjadi {} tile kernel 1 km", self.stats.len()),
                    );
                }
                if self.unsaved {
                    ui.separator();
                    ui.colored_label(egui::Color32::from_rgb(200, 120, 0), "Belum disimpan");
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
                    && let Some((lo, hi)) = self.bounds()
                {
                    self.orbit.fit(lo, hi, px[0] as f32 / px[1] as f32);
                }
                let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
                self.navigate(ui, &resp, size);
                self.pointer_input(&resp, rect, size);
                self.refresh_overlay(size.y);
                self.render();
                if let Some(v) = &self.view {
                    egui::Image::new((v.id, size)).paint_at(ui, rect);
                }
            });
    }
}

impl App {
    /// Middle-drag, or Alt + left-drag on a trackpad: orbit; with Shift: pan.
    /// Scroll: zoom. The plain left button belongs to the tool.
    fn navigate(&mut self, ui: &egui::Ui, resp: &egui::Response, size: egui::Vec2) {
        let m = ui.input(|i| i.modifiers);
        let held = resp.dragged_by(egui::PointerButton::Middle)
            || (m.alt && resp.dragged_by(egui::PointerButton::Primary));
        if held {
            let d = resp.drag_delta();
            if m.shift {
                self.orbit.pan(d.x, d.y, size.y);
            } else {
                self.orbit.orbit(d.x, d.y);
            }
            self.overlay_shown = None;
        }
        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.orbit.zoom(scroll / 50.0);
                self.overlay_shown = None;
            }
        }
    }

    fn pointer_input(&mut self, resp: &egui::Response, rect: egui::Rect, size: egui::Vec2) {
        // The cone tracks the view: SNAP_REACH units at any window height.
        let t = (camera::FOVY as f64 / 2.0).tan();
        self.tool
            .set_aperture((2.0 * t * SNAP_REACH as f64 / size.y.max(1.0) as f64).atan());
        let ray_at = |orbit: &Orbit, p: egui::Pos2| {
            let (eye, dir) = orbit.ray(p.x - rect.min.x, p.y - rect.min.y, size.x, size.y);
            PickRay {
                origin: Point3::new(eye[0], eye[1], eye[2]),
                direction: Vec3::new(dir[0], dir[1], dir[2]),
            }
        };
        if let Some(p) = resp.hover_pos()
            && self.last_pointer != Some(p)
        {
            self.last_pointer = Some(p);
            self.feed(Input::Hover(ray_at(&self.orbit, p)));
        }
        let navigating = resp.ctx.input(|i| i.modifiers.alt);
        if resp.clicked_by(egui::PointerButton::Primary)
            && !navigating
            && let Some(p) = resp.interact_pointer_pos()
        {
            self.error = None;
            self.feed(Input::Click(ray_at(&self.orbit, p)));
        }
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

    #[test]
    fn wangun_path_replaces_only_a_hew_extension() {
        let w = |p: &str| wangun_path(std::path::Path::new(p));
        assert_eq!(w("/d/kota.hew"), std::path::PathBuf::from("/d/kota.wangun"));
        assert_eq!(w("/d/KOTA.HEW"), std::path::PathBuf::from("/d/KOTA.wangun"));
        assert_eq!(
            w("/d/kota.v1"),
            std::path::PathBuf::from("/d/kota.v1.wangun")
        );
        assert_eq!(w("/d/kota"), std::path::PathBuf::from("/d/kota.wangun"));
        assert_eq!(
            w("/d/x.wangun"),
            std::path::PathBuf::from("/d/x.wangun.wangun")
        );
    }

    #[test]
    fn save_never_turns_a_hew_into_a_container() {
        let on_disk = |p: &str, container| Source {
            path: Some(p.into()),
            container,
        };
        let t = |s: &Source, boxed: &str, single| save_target(s, boxed, single).unwrap();
        // Same kind: overwrite what was opened.
        assert_eq!(
            t(&on_disk("/d/a.hew", false), "", true),
            PathBuf::from("/d/a.hew")
        );
        assert_eq!(
            t(&on_disk("/d/a.wangun", true), "", false),
            PathBuf::from("/d/a.wangun")
        );
        // A .hew that was split or grew tiles goes beside it.
        assert_eq!(
            t(&on_disk("/d/a.hew", false), "/d/other", false),
            PathBuf::from("/d/a.wangun")
        );
        // Never saved: the path box decides, the kind decides the extension.
        let new = Source::default();
        assert_eq!(t(&new, " /d/b.hew ", true), PathBuf::from("/d/b.hew"));
        assert_eq!(t(&new, "/d/b", true), PathBuf::from("/d/b.hew"));
        assert_eq!(t(&new, "/d/b.hew", false), PathBuf::from("/d/b.wangun"));
        assert!(save_target(&new, "  ", true).is_err());
    }

    #[test]
    fn write_atomic_replaces_and_leaves_no_temp() {
        let dir = std::env::temp_dir().join(format!("wangungis-save-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("a.hew");
        write_atomic(&target, b"one").unwrap();
        write_atomic(&target, b"two").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"two");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unit_letters_only_continue_a_value() {
        assert!(typed_accepts("", '5') && typed_accepts("", '-') && typed_accepts("", ','));
        assert!(!typed_accepts("", 'm') && !typed_accepts("", 'x') && !typed_accepts("", ' '));
        assert!(typed_accepts("5", 'x') && typed_accepts("250", 'c') && typed_accepts("5", ' '));
        assert!(!typed_accepts("5", 'l') && !typed_accepts("", 'p'));
    }
}
