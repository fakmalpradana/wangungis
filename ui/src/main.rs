//! WangunGIS desktop shell: open a `.hew` or `.wangun` project, navigate, draw
//! with Line and Rectangle, Push/Pull, undo. A `.hew` reaching past the kernel
//! frame bound is split into kernel tiles on open.
//! All editing logic lives in `tools`; this file turns pixels into rays and
//! keys into tool inputs. The 3D view renders into an offscreen texture shown
//! as an egui image, so the renderer owns its depth buffer.
//! Usage: wangungis [file.hew | file.wangun]

mod camera;
#[cfg(test)]
mod smoke;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use camera::Orbit;
use eframe::{egui, egui_wgpu};
use inference::{Axis, PickRay, SnapKind};
use kernel::{NodeId, Point3, Vec3};
use wangungis_render::{DEPTH_FORMAT, OverlayLines, Renderer, TileMesh};
use wangungis_scene::project::{Project, TileKey, node_visible};
use wangungis_tools::tool::{Input, Preview, Tool};
use wangungis_tools::vcb::Unit;
use wangungis_tools::{OutlineRow, Paint, Scene};

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
/// The Paint Bucket's ready colors.
const PAINTS: [(&str, [u8; 3]); 12] = [
    ("Beton", [190, 190, 185]),
    ("Plester", [240, 238, 230]),
    ("Bata", [170, 85, 60]),
    ("Genteng", [175, 75, 45]),
    ("Kayu", [150, 105, 65]),
    ("Batu", [150, 140, 125]),
    ("Baja", [120, 130, 140]),
    ("Kaca", [150, 200, 220]),
    ("Aspal", [70, 70, 75]),
    ("Rumput", [110, 160, 80]),
    ("Kuning", [235, 200, 80]),
    ("Biru", [70, 110, 170]),
];
/// Unlike the axes, the preview, the snap markers and the pale background.
const GUIDE_COLOR: [f32; 3] = [0.85, 0.0, 0.55];
/// Seconds between autosaves of a project with unsaved changes.
const AUTOSAVE_S: f64 = 60.0;

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

/// What is waiting for the answer to "discard unsaved changes?".
#[derive(Debug, Clone, Copy, PartialEq)]
enum Leaving {
    New,
    Open,
    Close,
}

/// The autosave file. One per user: a second window overwrites the first's.
// ponytail: per-window files when two open projects at once is a real use.
fn default_autosave() -> Option<PathBuf> {
    user_file("autosave.bin")
}

fn ready_paint((name, [r, g, b]): (&str, [u8; 3])) -> Paint {
    Paint {
        name: name.into(),
        color: kernel::Rgba8::rgb(r, g, b),
    }
}

fn user_file(name: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".wangungis").join(name))
}

/// The display unit kept in `path`. Missing or unknown means meters: this is
/// a preference, nothing depends on it surviving.
// ponytail: per user, not per project. Lengths are stored in meters either way,
// so a project moves between users unchanged. Per project needs a format change.
fn read_unit(path: &Path) -> Unit {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    Unit::ALL
        .into_iter()
        .find(|u| u.label() == text.trim())
        .unwrap_or_default()
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
    scene: Scene,
    tool: Tool,
    /// Lengths are shown in this, and bare typed lengths read in it.
    unit: Unit,
    /// What the Paint Bucket puts on; `None` takes paint off.
    paint: Option<Paint>,
    /// `None`: the unit is not remembered, as when the home folder is unknown.
    unit_file: Option<PathBuf>,
    /// What the user is typing for the tool.
    typed: String,
    /// The Outliner's lines and what is open in it, refreshed when something changed.
    outline_rows: Vec<OutlineRow>,
    outline_expanded: BTreeSet<(TileKey, NodeId)>,
    outline_rename: String,
    /// The project's tags, refreshed when something changed.
    tag_rows: Vec<(String, bool)>,
    /// New tag and per-row rename text in the Tags panel.
    tag_name: String,
    tag_renames: BTreeMap<String, String>,
    stats: BTreeMap<TileKey, TileStats>,
    source: Source,
    /// Differs from the file: edited, or split on open.
    unsaved: bool,
    split_on_open: bool,
    overlay_shown: Option<Preview>,
    last_pointer: Option<egui::Pos2>,
    moved_since_click: bool,
    /// A file window waiting for the end of the frame.
    dialog: Option<Dialog>,
    /// What to do once the Save As window this question opened has saved.
    after_save: Option<Leaving>,
    /// The window saying which files go where.
    formats_open: bool,
    ctrl_held: bool,
    /// Another key or button went down while Ctrl was held.
    ctrl_chord: bool,
    error: Option<String>,
    /// What just succeeded, until the next input replaces it.
    notice: Option<String>,
    /// `None`: autosave is off, as when the home folder is unknown.
    autosave: Option<PathBuf>,
    /// When the last autosave ran, and whether anything changed since.
    autosaved_at: f64,
    autosave_stale: bool,
    /// An autosave left by a session that did not end cleanly.
    recovery: bool,
    leaving: Option<Leaving>,
    /// The user answered: the window may close.
    closing: bool,
    #[cfg(test)]
    view_rect: egui::Rect,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, path: Option<String>) -> Self {
        let rs = cc
            .wgpu_render_state
            .clone()
            .expect("eframe started with the wgpu renderer");
        let mut app = Self::with_render_state(rs, path);
        app.autosave = default_autosave();
        app.recovery = app.autosave.as_ref().is_some_and(|p| p.exists());
        app.unit_file = user_file("unit");
        if let Some(p) = &app.unit_file {
            app.unit = read_unit(p);
            app.tool.set_unit(app.unit);
        }
        app
    }

    fn with_render_state(rs: egui_wgpu::RenderState, path: Option<String>) -> Self {
        let renderer = Renderer::new(&rs.device, COLOR_FORMAT);
        let mut app = App {
            rs,
            renderer,
            view: None,
            orbit: Orbit::default(),
            fit_pending: false,
            scene: empty_scene(),
            tool: Tool::select(0.01),
            unit: Unit::default(),
            paint: Some(ready_paint(PAINTS[0])),
            unit_file: None,
            typed: String::new(),
            outline_rows: Vec::new(),
            outline_expanded: BTreeSet::new(),
            outline_rename: String::new(),
            tag_rows: Vec::new(),
            tag_name: String::new(),
            tag_renames: BTreeMap::new(),
            stats: BTreeMap::new(),
            source: Source::default(),
            unsaved: false,
            split_on_open: false,
            overlay_shown: None,
            last_pointer: None,
            moved_since_click: false,
            dialog: None,
            after_save: None,
            formats_open: false,
            ctrl_held: false,
            ctrl_chord: false,
            error: None,
            notice: None,
            autosave: None,
            autosaved_at: 0.0,
            autosave_stale: false,
            recovery: false,
            leaving: None,
            closing: false,
            #[cfg(test)]
            view_rect: egui::Rect::NOTHING,
        };
        if let Some(path) = path {
            app.open_path(Path::new(&path));
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
        self.camera_moved();
        let keys: Vec<TileKey> = self.scene.project().tiles.keys().copied().collect();
        for key in keys {
            self.upload(key);
        }
        self.scene.take_dirty();
        // The ids belong to the project before.
        self.outline_expanded.clear();
        self.refresh_lists();
        self.orbit = Orbit::default();
    }

    /// Runs `what`, after asking when that would lose changes.
    fn leave(&mut self, what: Leaving) {
        if self.unsaved {
            self.leaving = Some(what);
        } else {
            self.do_leave(what);
        }
    }

    fn do_leave(&mut self, what: Leaving) {
        self.leaving = None;
        match what {
            Leaving::New => self.new_project(),
            Leaving::Open => self.dialog = Some(Dialog::Open),
            Leaving::Close => {
                // The user chose to lose the changes, or there were none.
                self.drop_autosave();
                self.closing = true;
            }
        }
    }

    fn drop_autosave(&mut self) {
        self.autosave_stale = false;
        if let Some(p) = &self.autosave {
            // Absent is fine: nothing was autosaved yet.
            let _ = std::fs::remove_file(p);
        }
    }

    /// Writes the autosave when it is due. Not during a gesture: the kernel may
    /// be mid-edit, and the write is a visible pause on a big model.
    fn run_autosave(&mut self, now: f64) {
        let due = self.autosave_stale
            && !self.tool.busy()
            && now - self.autosaved_at >= AUTOSAVE_S
            && !self.recovery;
        let Some(path) = self.autosave.clone().filter(|_| due) else {
            return;
        };
        self.autosaved_at = now;
        let bytes = self.scene.project().save();
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| write_atomic(&path, &bytes));
        match written {
            Ok(()) => self.autosave_stale = false,
            Err(e) => self.error = Some(format!("Autosave gagal: {e}")),
        }
    }

    /// Takes the autosave of the last session as an unsaved new project.
    fn recover(&mut self) {
        self.recovery = false;
        let Some(path) = self.autosave.clone() else {
            return;
        };
        let opened = std::fs::read(&path)
            .map_err(|e| e.to_string())
            .and_then(|b| Project::open(&b).map_err(|e| format!("{e:?}")));
        match opened {
            Ok(project) => {
                self.adopt(Scene::new(project));
                self.source = Source::default();
                self.unsaved = true;
                self.split_on_open = false;
                self.fit_pending = true;
            }
            Err(e) => self.error = Some(format!("Autosave tidak bisa dibuka: {e}")),
        }
    }

    fn discard_recovery(&mut self) {
        self.recovery = false;
        self.drop_autosave();
    }

    fn new_project(&mut self) {
        self.error = None;
        self.notice = None;
        self.adopt(empty_scene());
        self.source = Source::default();
        self.unsaved = false;
        self.split_on_open = false;
    }

    fn open_path(&mut self, path: &Path) {
        self.error = None;
        self.notice = None;
        let opened = std::fs::read(path)
            .map_err(|e| format!("Tidak bisa membaca {}: {e}", path.display()))
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
                    path: Some(path.to_owned()),
                    container,
                };
                self.adopt(Scene::new(project));
                self.fit_pending = true;
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// Simpan: over the file it came from, or asks where for a new project.
    fn save(&mut self) {
        match self.source.path.clone() {
            Some(path) => self.save_to(&path),
            None => self.dialog = Some(Dialog::SaveAs),
        }
    }

    /// Writes the project to `chosen`, in the format its tile count calls for. The
    /// file window has already asked about replacing `chosen`. It has not asked
    /// about a name changed here, so that one never replaces anything.
    fn save_to(&mut self, chosen: &Path) {
        self.notice = None;
        let project = self.scene.project();
        let single = project.single().is_some();
        let target = save_target(&self.source, chosen, single);
        if target != chosen && self.source.path.as_deref() != Some(&target) && target.exists() {
            self.error = Some(format!(
                "{} sudah ada dan bukan file yang sedang dibuka. Pakai Simpan sebagai dengan nama lain.",
                target.display()
            ));
            return;
        }
        match write_atomic(&target, &project.save()) {
            Ok(()) => {
                self.source = Source {
                    path: Some(target),
                    container: !single,
                };
                self.unsaved = false;
                self.split_on_open = false;
                self.error = None;
                self.drop_autosave();
            }
            Err(e) => self.error = Some(format!("Tidak bisa menyimpan {}: {e}", target.display())),
        }
    }

    /// Writes the project to `target` as the format its extension names.
    fn export_to(&mut self, target: &Path) {
        self.notice = None;
        let format = target
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .filter(|e| wangungis_io::FORMATS.contains(&e.as_str()));
        let Some(format) = format else {
            self.error = Some(format!(
                "Nama file harus berakhiran salah satu dari: {}.",
                wangungis_io::FORMATS.join(", ")
            ));
            return;
        };
        let written = wangungis_io::export(self.scene.project(), &format)
            .map_err(|e| match e {
                wangungis_io::ExportError::MultiTile => {
                    "Format ini hanya untuk proyek satu tile. Pakai .obj.".to_string()
                }
                wangungis_io::ExportError::NothingToExport => {
                    "Tidak ada solid untuk diekspor.".to_string()
                }
                other => format!("Ekspor gagal: {other:?}"),
            })
            .and_then(|bytes| write_atomic(target, &bytes).map_err(|e| e.to_string()));
        match written {
            Ok(()) => {
                self.error = None;
                self.notice = Some(format!("Diekspor ke {}", target.display()));
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// Acts on what a file window answered. `None`: it was cancelled.
    fn answer_dialog(&mut self, dialog: Dialog, path: Option<PathBuf>) {
        let after_save = self.after_save.take();
        let Some(path) = path else {
            return;
        };
        match dialog {
            Dialog::Open => self.open_path(&path),
            Dialog::SaveAs => {
                self.save_to(&path);
                if let Some(what) = after_save.filter(|_| !self.unsaved) {
                    self.do_leave(what);
                }
            }
            Dialog::Export => self.export_to(&path),
        }
    }

    /// Shows the OS file window, which holds this frame until it closes.
    // ponytail: blocking. Nothing else can happen meanwhile, as in any modal window.
    #[cfg(not(test))]
    fn run_dialog(&mut self, dialog: Dialog) {
        let single = self.scene.project().single().is_some();
        let opened_at = std::time::Instant::now();
        let picked = dialog.window(self.source.path.as_deref(), single);
        // rfd answers a missing portal and zenity the same as a cancel. Nobody
        // cancels this fast.
        // ponytail: timing guess, as rfd gives no error to read.
        #[cfg(target_os = "linux")]
        if picked.is_none() && opened_at.elapsed().as_millis() < 150 {
            self.error = Some(
                "Jendela berkas tidak bisa dibuka. Pasang xdg-desktop-portal atau zenity.".into(),
            );
        }
        #[cfg(not(target_os = "linux"))]
        let _ = opened_at;
        self.answer_dialog(dialog, picked);
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
                        objects: doc
                            .visible_object_ids()
                            .into_iter()
                            .filter(|id| node_visible(doc, NodeId::Object(*id)))
                            .count(),
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
            self.refresh_lists();
            self.unsaved = true;
            self.autosave_stale = true;
            // Guides are in the overlay.
            self.overlay_shown = None;
        }
        for key in dirty {
            self.upload(key);
        }
    }

    fn feed(&mut self, input: Input) {
        if !matches!(input, Input::Hover(_)) {
            self.notice = None;
        }
        match input {
            Input::Hover(_) => self.moved_since_click = true,
            // What a click picked sits pixels from the pointer, because of
            // snapping. That is no direction until the mouse moves.
            Input::Click(_) | Input::ClickAdd(_) => self.moved_since_click = false,
            // Anything else changes what the parked pointer means to the tool.
            _ if self.moved_since_click => self.last_pointer = None,
            _ => {}
        }
        let guides = self.scene.guide_lines().len();
        match self.tool.handle(&mut self.scene, input) {
            Ok(()) if self.scene.guide_lines().len() > guides => {
                self.notice =
                    Some("Panduan dibuat: garis merah muda, snap menempel padanya.".into());
            }
            Ok(()) => {}
            Err(e) => self.error = Some(e.to_string()),
        }
        self.sync_edits();
    }

    fn switch_tool(&mut self, make: fn(f64) -> Tool) {
        let picked = self.tool.picked_nodes();
        self.tool = make(0.01);
        self.tool.carry_selection(&self.scene, picked);
        self.tool.set_unit(self.unit);
        self.tool.set_paint(self.paint.clone());
        self.typed.clear();
        self.error = None;
        // The parked pointer has something to tell the new tool.
        self.last_pointer = None;
    }

    fn set_paint(&mut self, paint: Option<Paint>) {
        self.tool.set_paint(paint.clone());
        self.paint = paint;
    }

    /// The colors, shown while the Paint Bucket is the tool.
    fn paint_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Cat:");
            for ready in PAINTS {
                let [r, g, b] = ready.1;
                let swatch = egui::Button::new("")
                    .fill(egui::Color32::from_rgb(r, g, b))
                    .min_size(egui::vec2(20.0, 20.0));
                if ui.add(swatch).on_hover_text(ready.0).clicked() {
                    self.set_paint(Some(ready_paint(ready)));
                }
            }
            let held = self.paint.as_ref().map(|p| p.color);
            let mut rgb = held.map_or([255; 3], |c| [c.r, c.g, c.b]);
            if ui.color_edit_button_srgb(&mut rgb).changed() {
                let name = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
                self.set_paint(Some(ready_paint((&name, rgb))));
            }
            if ui.button("Hapus cat").clicked() {
                self.set_paint(None);
            }
            ui.label(self.paint.as_ref().map_or("Tanpa cat", |p| &p.name));
        });
    }

    fn set_unit(&mut self, unit: Unit) {
        self.unit = unit;
        self.tool.set_unit(unit);
        if let Some(p) = &self.unit_file {
            let written = p
                .parent()
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|()| std::fs::write(p, unit.label()));
            if let Err(e) = written {
                self.error = Some(format!("Gagal mengingat satuan: {e}"));
            }
        }
    }

    /// Runs `edit` on what Select holds, then starts the tool over: the ids it
    /// held may be gone.
    fn on_picked(
        &mut self,
        edit: impl FnOnce(
            &mut Scene,
            TileKey,
            &[kernel::NodeId],
        ) -> Result<(), wangungis_tools::EditError>,
    ) {
        self.notice = None;
        let Some((tile, nodes)) = self.tool.picked_nodes() else {
            self.error = Some("Pilih objek dulu dengan Pilih (Spasi).".into());
            return;
        };
        match edit(&mut self.scene, tile, &nodes) {
            Ok(()) => self.error = None,
            Err(e) => self.error = Some(e.to_string()),
        }
        self.tool.reset();
        self.sync_edits();
    }

    /// What the side panel lists. Walks the project: not for every frame.
    fn refresh_lists(&mut self) {
        self.tag_rows = self.scene.tags();
        self.outline_rows = self.scene.outline_rows(&self.outline_expanded);
    }

    fn outline_toggle(&mut self, tile: TileKey, node: NodeId) {
        if !self.outline_expanded.remove(&(tile, node)) {
            self.outline_expanded.insert((tile, node));
        }
        self.outline_rows = self.scene.outline_rows(&self.outline_expanded);
    }

    /// Picks the top-level node a row sits under: what is inside needs opening.
    fn outline_pick(&mut self, tile: TileKey, top: NodeId) {
        self.error = None;
        self.typed.clear();
        self.tool.pick_node(&self.scene, tile, top);
    }

    fn outline_open(&mut self, tile: TileKey, top: NodeId) {
        self.outline_pick(tile, top);
        if !matches!(top, NodeId::Object(_)) {
            self.enter_picked();
        }
    }

    fn set_node_visible(&mut self, tile: TileKey, node: NodeId, visible: bool) {
        self.scene.set_node_hidden(tile, node, !visible);
        self.tool.reset();
        self.sync_edits();
    }

    fn rename_picked(&mut self, name: &str) {
        let name = name.to_owned();
        self.on_picked(move |s, t, n| match n {
            [node] => s.rename_node(t, *node, &name),
            _ => Err(wangungis_tools::EditError::Unsupported(
                "mengubah nama lebih dari satu benda sekaligus",
            )),
        });
    }

    fn outline_panel(&mut self, ui: &mut egui::Ui) {
        enum Action {
            Toggle(TileKey, NodeId),
            Pick(TileKey, NodeId),
            Open(TileKey, NodeId),
            Visible(TileKey, NodeId, bool),
            Rename,
        }
        let mut action = None;
        egui::CollapsingHeader::new("Outliner")
            .default_open(true)
            .show(ui, |ui| {
                let picked = self.tool.picked_nodes();
                let many_tiles = self.scene.project().tiles.len() > 1;
                let height = ui.spacing().interact_size.y;
                // Only the lines in view are laid out: a city has tens of thousands.
                egui::ScrollArea::vertical()
                    .id_salt("outliner")
                    .max_height(280.0)
                    .auto_shrink([false, true])
                    .show_rows(ui, height, self.outline_rows.len(), |ui, lines| {
                        for row in &self.outline_rows[lines] {
                            ui.horizontal(|ui| {
                                ui.add_space(row.depth as f32 * 12.0);
                                if row.has_children {
                                    let mark = if row.expanded { "-" } else { "+" };
                                    if ui.small_button(mark).clicked() {
                                        action = Some(Action::Toggle(row.tile, row.node));
                                    }
                                }
                                let mut visible = !row.hidden;
                                if ui.checkbox(&mut visible, "").changed() {
                                    action = Some(Action::Visible(row.tile, row.node, visible));
                                }
                                let held = picked
                                    .as_ref()
                                    .is_some_and(|(t, n)| *t == row.tile && n.contains(&row.node));
                                let text = if many_tiles {
                                    format!("{} [{},{}]", row.name, row.tile.i, row.tile.j)
                                } else {
                                    row.name.clone()
                                };
                                let line = ui.selectable_label(held, text);
                                if line.double_clicked() {
                                    action = Some(Action::Open(row.tile, row.top));
                                } else if line.clicked() {
                                    action = Some(Action::Pick(row.tile, row.top));
                                }
                            });
                        }
                    });
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.outline_rename).desired_width(120.0),
                    );
                    if ui
                        .add_enabled(picked.is_some(), egui::Button::new("Ubah nama"))
                        .clicked()
                    {
                        action = Some(Action::Rename);
                    }
                });
            });
        match action {
            Some(Action::Toggle(tile, node)) => self.outline_toggle(tile, node),
            Some(Action::Pick(tile, top)) => self.outline_pick(tile, top),
            Some(Action::Open(tile, top)) => self.outline_open(tile, top),
            Some(Action::Visible(tile, node, visible)) => {
                self.set_node_visible(tile, node, visible);
            }
            Some(Action::Rename) => {
                let name = std::mem::take(&mut self.outline_rename);
                self.rename_picked(&name);
            }
            None => {}
        }
    }

    fn tag_picked(&mut self, name: &str) {
        let name = name.to_owned();
        self.on_picked(move |s, t, n| s.tag_nodes(t, n, &name));
    }

    fn untag_picked(&mut self, name: &str) {
        let name = name.to_owned();
        self.on_picked(move |s, t, n| s.untag_nodes(t, n, &name));
    }

    fn set_tag_visible(&mut self, name: &str, visible: bool) {
        match self.scene.set_tag_hidden(name, !visible) {
            Ok(()) => self.error = None,
            Err(e) => self.error = Some(e.to_string()),
        }
        self.tool.reset();
        self.sync_edits();
    }

    fn rename_tag(&mut self, old: &str, new: &str) {
        match self.scene.rename_tag(old, new) {
            Ok(()) => self.error = None,
            Err(e) => self.error = Some(e.to_string()),
        }
        self.tool.reset();
        self.sync_edits();
    }

    fn delete_tag(&mut self, name: &str) {
        match self.scene.delete_tag(name) {
            Ok(()) => self.error = None,
            Err(e) => self.error = Some(e.to_string()),
        }
        self.tool.reset();
        self.sync_edits();
    }

    fn tags_panel(&mut self, ui: &mut egui::Ui) {
        enum Action {
            Visible(String, bool),
            Give(String),
            Take(String),
            Rename(String, String),
            Delete(String),
        }

        let mut action = None;
        egui::CollapsingHeader::new("Tags")
            .default_open(true)
            .show(ui, |ui| {
                let picked = self.tool.picked_nodes().is_some();
                if !picked {
                    ui.label("Pilih objek dulu dengan Pilih (Spasi).");
                }
                for (name, hidden) in self.tag_rows.clone() {
                    ui.horizontal_wrapped(|ui| {
                        let mut visible = !hidden;
                        if ui.checkbox(&mut visible, "").changed() {
                            action = Some(Action::Visible(name.clone(), visible));
                        }
                        ui.label(&name);
                        if ui.add_enabled(picked, egui::Button::new("Beri")).clicked() {
                            action = Some(Action::Give(name.clone()));
                        }
                        if ui.add_enabled(picked, egui::Button::new("Lepas")).clicked() {
                            action = Some(Action::Take(name.clone()));
                        }
                        if ui.button("Hapus").clicked() {
                            action = Some(Action::Delete(name.clone()));
                        }
                    });
                    ui.horizontal(|ui| {
                        let rename = self.tag_renames.entry(name.clone()).or_insert(name.clone());
                        ui.add(egui::TextEdit::singleline(rename).desired_width(120.0));
                        if ui.button("Ubah nama").clicked() {
                            action = Some(Action::Rename(name.clone(), rename.clone()));
                        }
                    });
                    ui.separator();
                }
                ui.label("Tag baru:");
                ui.add(egui::TextEdit::singleline(&mut self.tag_name));
                if ui
                    .add_enabled(picked, egui::Button::new("Buat dan beri"))
                    .clicked()
                {
                    action = Some(Action::Give(std::mem::take(&mut self.tag_name)));
                }
            });

        match action {
            Some(Action::Visible(name, visible)) => self.set_tag_visible(&name, visible),
            Some(Action::Give(name)) => self.tag_picked(&name),
            Some(Action::Take(name)) => self.untag_picked(&name),
            Some(Action::Rename(old, new)) => {
                self.rename_tag(&old, &new);
                if self.error.is_none() {
                    self.tag_renames.remove(&old);
                }
            }
            Some(Action::Delete(name)) => {
                self.delete_tag(&name);
                if self.error.is_none() {
                    self.tag_renames.remove(&name);
                }
            }
            None => {}
        }
    }

    /// Opens the one group or component that is picked.
    fn enter_picked(&mut self) {
        self.on_picked(|s, t, n| match n {
            [node] => s.enter(t, *node),
            _ => Err(wangungis_tools::EditError::Unsupported(
                "mengedit isi lebih dari satu benda sekaligus",
            )),
        });
    }

    fn leave_session(&mut self) {
        self.notice = None;
        match self.scene.leave() {
            Ok(_) => self.error = None,
            Err(e) => self.error = Some(e.to_string()),
        }
        self.tool.reset();
        self.sync_edits();
    }

    /// What is open for editing, for the status bar.
    // ponytail: an open group shows as "Grup", Hew gives no name while it is hidden.
    fn session_label(&self) -> Option<String> {
        let (tile, stack) = self.scene.session()?;
        let names: Vec<_> = stack
            .into_iter()
            .map(|n| self.scene.node_name(tile, n))
            .collect();
        Some(format!("Mengedit: {}", names.join(" > ")))
    }

    fn delete_picked(&mut self) {
        if self.tool.picked_nodes().is_some() {
            self.on_picked(|s, t, n| s.delete_nodes(t, n));
        }
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
        self.last_pointer = None;
        self.sync_edits();
    }

    fn tile_count(&self) -> usize {
        self.scene.project().occupied().count()
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
        // A bare tap of Ctrl flips copying, like the C key; Ctrl+Z or Ctrl+S do not.
        // Tools without copying ignore it. Tracked even while a text box has the
        // keyboard, so a chord there does not look like a tap afterwards.
        let (ctrl, chord) = ctx.input(|i| {
            let pressed = i.events.iter().any(|e| {
                matches!(
                    e,
                    egui::Event::Key { pressed: true, .. }
                        | egui::Event::PointerButton { pressed: true, .. }
                        | egui::Event::Copy
                        | egui::Event::Cut
                        | egui::Event::Paste(_)
                )
            });
            (i.modifiers.ctrl, pressed)
        });
        // Held last frame counts: Ctrl can go up in the frame the key goes down.
        self.ctrl_chord |= (ctrl || self.ctrl_held) && chord;
        let tapped = self.ctrl_held && !ctrl && !self.ctrl_chord;
        if !ctrl {
            self.ctrl_chord = false;
        }
        self.ctrl_held = ctrl;
        // Only a text box keeps the keyboard, and it has no use for this one.
        if ctx.text_edit_focused() {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
                self.save();
            }
            return;
        }
        // Tab must not park focus. On a button, keys stop reaching the tools and
        // Enter presses it.
        ctx.memory_mut(|m| {
            if let Some(id) = m.focused() {
                m.surrender_focus(id);
            }
            m.move_focus(egui::FocusDirection::None);
        });
        if tapped {
            self.feed(Input::ToggleCopy);
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
                    repeat,
                    modifiers,
                    ..
                } => self.handle_key(key, modifiers, repeat),
                _ => {}
            }
        }
    }

    fn handle_key(&mut self, key: egui::Key, m: egui::Modifiers, repeat: bool) {
        use egui::Key as K;
        // Holding a key may repeat undo and backspace, nothing that ends a gesture.
        if repeat && !matches!(key, K::Z | K::Backspace) {
            return;
        }
        if m.command {
            match (key, m.shift) {
                (K::Z, false) => self.undo(),
                (K::Z, true) => self.redo(),
                (K::S, false) => self.save(),
                (K::S, true) => self.dialog = Some(Dialog::SaveAs),
                (K::O, false) => self.leave(Leaving::Open),
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
            K::Backspace if !self.typed.is_empty() => {
                self.typed.pop();
            }
            K::Delete | K::Backspace => self.delete_picked(),
            K::Escape => {
                // Nothing left for Esc to cancel: it leaves the open group.
                let idle = self.typed.is_empty()
                    && !self.tool.busy()
                    && self.tool.picked_nodes().is_none();
                self.typed.clear();
                self.error = None;
                self.feed(Input::Cancel);
                if idle && self.scene.session().is_some() {
                    self.leave_session();
                }
            }
            K::ArrowRight => self.feed(Input::LockAxis(Axis::X)),
            K::ArrowLeft => self.feed(Input::LockAxis(Axis::Y)),
            K::ArrowUp => self.feed(Input::LockAxis(Axis::Z)),
            K::ArrowDown => self.feed(Input::LockNormal),
            // Letters that are units only count as tools with nothing typed.
            K::Space if self.typed.is_empty() => self.switch_tool(Tool::select),
            K::L if self.typed.is_empty() => self.switch_tool(Tool::line),
            K::R if self.typed.is_empty() => self.switch_tool(Tool::rectangle),
            K::P if self.typed.is_empty() => self.switch_tool(Tool::push_pull),
            K::M if self.typed.is_empty() => self.switch_tool(Tool::move_tool),
            K::E if self.typed.is_empty() => self.switch_tool(Tool::eraser),
            K::B if self.typed.is_empty() => self.switch_tool(Tool::paint),
            K::Q if self.typed.is_empty() => self.switch_tool(Tool::rotate),
            K::S if self.typed.is_empty() => self.switch_tool(Tool::scale),
            // C copies in the tools that can; everywhere else it is Circle.
            K::C if self.typed.is_empty() => {
                if self.tool.can_copy() {
                    self.feed(Input::ToggleCopy);
                } else {
                    self.switch_tool(Tool::circle);
                }
            }
            K::G if self.typed.is_empty() => self.switch_tool(Tool::polygon),
            K::A if self.typed.is_empty() => self.switch_tool(Tool::arc),
            K::F if self.typed.is_empty() => self.switch_tool(Tool::offset),
            K::T if self.typed.is_empty() => self.switch_tool(Tool::tape),
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
            color: GUIDE_COLOR,
            lines: self
                .scene
                .guide_lines()
                .iter()
                .map(|(a, b)| (p3(*a), p3(*b)))
                .collect(),
        });
        let inferred = preview
            .snap
            .is_some_and(|h| matches!(h.snap.kind, SnapKind::Parallel | SnapKind::Perpendicular));
        groups.push(OverlayLines {
            color: if inferred { GUIDE_COLOR } else { PREVIEW_COLOR },
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
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
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
        SnapKind::Endpoint | SnapKind::Center | SnapKind::Quadrant => [0.0, 0.7, 0.1],
        SnapKind::Midpoint | SnapKind::Tangent => [0.0, 0.7, 0.8],
        SnapKind::Intersection => [0.9, 0.55, 0.0],
        SnapKind::OnEdge => [0.85, 0.1, 0.1],
        SnapKind::OnFace => [0.1, 0.2, 0.9],
        SnapKind::OnAxis => [0.7, 0.1, 0.7],
        SnapKind::OnGuide | SnapKind::Parallel | SnapKind::Perpendicular => GUIDE_COLOR,
    }
}

/// What the snap is, by the pointer, kept inside the view.
fn paint_tip(ui: &egui::Ui, view: egui::Rect, at: egui::Pos2, text: &str) {
    let painter = ui.painter_at(view);
    let white = egui::Color32::WHITE;
    let text = painter.layout_no_wrap(text.into(), egui::FontId::proportional(12.0), white);
    let margin = 4.0;
    let size = text.size();
    let pos = egui::pos2(
        (at.x + 14.0).min(view.right() - size.x - margin),
        (at.y + 16.0).min(view.bottom() - size.y - margin),
    );
    let back = egui::Rect::from_min_size(pos, size).expand(3.0);
    painter.rect_filled(back, 3.0, egui::Color32::from_black_alpha(190));
    painter.galley(pos, text, white);
}

fn snap_label(kind: SnapKind) -> &'static str {
    match kind {
        SnapKind::Endpoint => "Titik ujung",
        SnapKind::Midpoint => "Titik tengah",
        SnapKind::Center => "Titik pusat",
        SnapKind::Quadrant => "Titik kuadran",
        SnapKind::Intersection => "Perpotongan",
        SnapKind::Tangent => "Singgung",
        SnapKind::OnGuide => "Pada panduan",
        SnapKind::OnEdge => "Pada tepi",
        SnapKind::OnFace => "Pada face",
        SnapKind::OnAxis => "Pada sumbu",
        SnapKind::Parallel => "Sejajar tepi",
        SnapKind::Perpendicular => "Tegak lurus tepi",
    }
}

/// Whether `ch` extends the value being typed. Digits, punctuation and array
/// marks always start a value; unit letters only continue one, since alone they
/// mean tools.
fn typed_accepts(typed: &str, ch: char) -> bool {
    match ch {
        '0'..='9' | '.' | ',' | ';' | '-' | '+' | 'x' | 'X' | '*' | '/' => true,
        'm' | 'M' | 'c' | 'C' | 'f' | 'F' | 't' | 'T' | 's' | 'S' | 'd' | 'D' | '°' | ' ' => {
            !typed.is_empty()
        }
        _ => false,
    }
}

/// Where a save aimed at `chosen` goes. A single-tile project is a plain `.hew`,
/// anything else a `.wangun`. The opened file is overwritten only when it already
/// holds that kind, so a `.hew` that was split or grew tiles is never replaced by
/// a container.
fn save_target(source: &Source, chosen: &Path, single: bool) -> PathBuf {
    let same_kind = source.path.as_deref() == Some(chosen) && source.container != single;
    if same_kind {
        chosen.to_owned()
    } else if single {
        chosen.with_extension("hew")
    } else {
        wangun_path(chosen)
    }
}

/// A file window the user can open.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Dialog {
    Open,
    SaveAs,
    Export,
}

impl Dialog {
    /// Blocks on the OS window. `current`: the open project's file, to start beside
    /// it. `single`: a project of one tile, which every format can hold.
    #[cfg(not(test))]
    fn window(self, current: Option<&Path>, single: bool) -> Option<PathBuf> {
        let mut window = rfd::FileDialog::new();
        if let Some(dir) = current.and_then(Path::parent) {
            window = window.set_directory(dir);
        }
        let stem = current
            .and_then(Path::file_stem)
            .map_or("proyek".into(), |n| n.to_string_lossy());
        match self {
            Dialog::Open => window
                .set_title("Buka proyek")
                .add_filter("Proyek WangunGIS (.hew, .wangun)", &["hew", "wangun"])
                .pick_file(),
            Dialog::SaveAs if single => window
                .set_title("Simpan proyek")
                .set_file_name(format!("{stem}.hew"))
                .add_filter("Proyek satu tile (.hew)", &["hew"])
                .save_file(),
            Dialog::SaveAs => window
                .set_title("Simpan proyek")
                .set_file_name(format!("{stem}.wangun"))
                .add_filter("Proyek banyak tile (.wangun)", &["wangun"])
                .save_file(),
            Dialog::Export if single => window
                .set_title("Ekspor model")
                .set_file_name(format!("{stem}.obj"))
                .add_filter("Wavefront OBJ (.obj)", &["obj"])
                .add_filter("glTF biner (.glb)", &["glb"])
                .add_filter("glTF (.gltf)", &["gltf"])
                .add_filter("STL (.stl)", &["stl"])
                .add_filter("3MF (.3mf)", &["3mf"])
                .save_file(),
            Dialog::Export => window
                .set_title("Ekspor model")
                .set_file_name(format!("{stem}.obj"))
                .add_filter(
                    "Wavefront OBJ (.obj), satu-satunya untuk banyak tile",
                    &["obj"],
                )
                .save_file(),
        }
    }
}

/// What the file windows can read and write, as the Format window shows it.
const FILE_FORMATS: [(&str, &str, &str); 7] = [
    (
        "Buka, Simpan",
        ".hew",
        "Proyek satu tile: geometri, warna, tag, nama, grup, komponen.",
    ),
    (
        "Buka, Simpan",
        ".wangun",
        "Proyek banyak tile, isi sama. Dibuat otomatis saat proyek lebih dari satu tile.",
    ),
    (
        "Ekspor",
        ".obj",
        "Semua tile. Tanpa warna. Koordinat lokal proyek, origin tertulis di komentar.",
    ),
    ("Ekspor", ".glb", "Satu tile, dengan warna. Biner."),
    ("Ekspor", ".gltf", "Satu tile, dengan warna. Teks."),
    ("Ekspor", ".stl", "Satu tile, tanpa warna."),
    ("Ekspor", ".3mf", "Satu tile, dengan warna."),
];

/// Writes through a temporary file, so a failed write cannot damage `target`.
fn write_atomic(target: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut tmp = target.as_os_str().to_owned();
    tmp.push(format!(".{}.tmp", std::process::id()));
    let tmp = PathBuf::from(tmp);
    // create_new: never clobber a file that happens to have this name.
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .and_then(|mut f| {
            std::io::Write::write_all(&mut f, bytes)?;
            f.sync_all()
        });
    match written.and_then(|()| std::fs::rename(&tmp, target)) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Removes only our own partial file; a pre-existing one failed
            // create_new above and keeps its name.
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                let _ = std::fs::remove_file(&tmp);
            }
            Err(e)
        }
    }
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
        self.shell(ui);
    }
}

impl App {
    /// One frame of the whole window. The smoke test drives this directly.
    fn shell(&mut self, ui: &mut egui::Ui) {
        let (now, close_asked) = ui.ctx().input(|i| (i.time, i.viewport().close_requested()));
        if close_asked && !self.closing {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.leave(Leaving::Close);
        }
        if self.closing {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        self.run_autosave(now);
        if self.autosave_stale {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f64(AUTOSAVE_S));
        }
        self.dialogs(ui.ctx());
        if self.modal() {
            // Keys and clicks answer the dialog, not the tool.
            ui.ctx().input_mut(|i| i.events.clear());
        }
        self.handle_keys(ui.ctx());
        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                self.file_menu(ui);
                ui.label(format!(
                    "{}{}",
                    self.source
                        .path
                        .as_deref()
                        .and_then(Path::file_name)
                        .map_or("Proyek baru".into(), |n| n.to_string_lossy()),
                    if self.unsaved { " *" } else { "" }
                ));
                ui.separator();
                for (label, make) in [
                    ("Pilih (Spasi)", Tool::select as fn(f64) -> Tool),
                    ("Garis (L)", Tool::line),
                    ("Persegi (R)", Tool::rectangle),
                    ("Lingkaran (C)", Tool::circle),
                    ("Poligon (G)", Tool::polygon),
                    ("Busur (A)", Tool::arc),
                    ("Push/Pull (P)", Tool::push_pull),
                    ("Offset (F)", Tool::offset),
                    ("Follow Me", Tool::follow_me),
                    ("Ukur (T)", Tool::tape),
                    ("Pindah (M)", Tool::move_tool),
                    ("Putar (Q)", Tool::rotate),
                    ("Skala (S)", Tool::scale),
                    ("Cat (B)", Tool::paint),
                    ("Hapus (E)", Tool::eraser),
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
                if ui.button("Hapus panduan").clicked() {
                    match self.scene.delete_guides() {
                        Ok(()) => self.error = None,
                        Err(e) => self.error = Some(e.to_string()),
                    }
                    self.sync_edits();
                }
                if ui.button("Grup").clicked() {
                    self.on_picked(|s, t, n| s.group(t, n));
                }
                if ui.button("Komponen").clicked() {
                    self.on_picked(|s, t, n| s.make_component(t, n));
                }
                if ui.button("Pecah").clicked() {
                    self.on_picked(|s, t, n| s.dissolve(t, n));
                }
                if ui.button("Edit isi").clicked() {
                    self.enter_picked();
                }
                if self.scene.session().is_some() && ui.button("Keluar").clicked() {
                    self.leave_session();
                }
                ui.separator();
                if ui.button("Zoom extents").clicked() {
                    self.fit_pending = true;
                }
                ui.separator();
                for (label, yaw, pitch) in camera::VIEWS {
                    if ui.button(label).clicked() {
                        self.orbit.yaw = yaw;
                        self.orbit.pitch = pitch;
                        self.camera_moved();
                    }
                }
            });
        });
        if self.tool.same_kind(&Tool::paint(0.0)) {
            egui::Panel::top("paints").show(ui, |ui| self.paint_row(ui));
        }
        egui::Panel::bottom("hint").show(ui, |ui| {
            ui.label(self.tool.hint());
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Nilai:");
                ui.monospace(if self.typed.is_empty() {
                    "-"
                } else {
                    &self.typed
                });
                let mut unit = self.unit;
                egui::ComboBox::from_id_salt("unit")
                    .selected_text(unit.label())
                    .width(48.0)
                    .show_ui(ui, |ui| {
                        for u in Unit::ALL {
                            ui.selectable_value(&mut unit, u, u.label());
                        }
                    });
                if unit != self.unit {
                    self.set_unit(unit);
                }
                ui.separator();
                if let Some(m) = self.tool.measure() {
                    ui.label(format!("Jarak: {}", self.unit.format(m)));
                    ui.separator();
                }
                if self.tool.copying() {
                    ui.colored_label(egui::Color32::from_rgb(200, 120, 0), "Menyalin");
                    ui.separator();
                }
                if let Some(open) = self.session_label() {
                    ui.colored_label(egui::Color32::from_rgb(0, 110, 170), open);
                    ui.separator();
                }
                if self.tool.keeping() {
                    ui.colored_label(egui::Color32::from_rgb(200, 120, 0), "Solid baru");
                    ui.separator();
                }
                if let Some(hit) = self.tool.preview().snap {
                    ui.label(snap_label(hit.snap.kind));
                    ui.separator();
                }
                if let Some(e) = &self.error {
                    ui.colored_label(egui::Color32::RED, e);
                    ui.separator();
                }
                if let Some(n) = &self.notice {
                    ui.label(n);
                    ui.separator();
                }
                ui.label(format!(
                    "{} tile · {} objek · {} segitiga",
                    self.tile_count(),
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
                        format!("Dipecah menjadi {} tile kernel 1 km", self.tile_count()),
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
        egui::Panel::right("tags")
            .default_size(250.0)
            .resizable(true)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.outline_panel(ui);
                    self.tags_panel(ui);
                });
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let size = ui.available_size();
                let ppp = ui.ctx().pixels_per_point();
                let max = self.rs.device.limits().max_texture_dimension_2d as f32;
                let px = [size.x, size.y].map(|v| (v * ppp).round().clamp(1.0, max) as u32);
                if self.view.as_ref().is_some_and(|v| v.size != px) {
                    self.camera_moved();
                }
                self.ensure_view(px);
                if std::mem::take(&mut self.fit_pending)
                    && let Some((lo, hi)) = self.bounds()
                {
                    self.orbit.fit(lo, hi, px[0] as f32 / px[1] as f32);
                    self.camera_moved();
                }
                let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
                #[cfg(test)]
                {
                    self.view_rect = rect;
                }
                if !self.modal() {
                    self.navigate(ui, &resp, size);
                    // The panels above were drawn before this input: show it now.
                    if self.pointer_input(&resp, rect, size) {
                        ui.ctx().request_repaint();
                    }
                }
                self.refresh_overlay(size.y);
                self.render();
                if let Some(v) = &self.view {
                    egui::Image::new((v.id, size)).paint_at(ui, rect);
                }
                if let (Some(hit), Some(at)) = (self.tool.preview().snap, resp.hover_pos()) {
                    paint_tip(ui, rect, at, snap_label(hit.snap.kind));
                }
            });
        #[cfg(not(test))]
        if let Some(dialog) = self.dialog.take() {
            self.run_dialog(dialog);
        }
    }

    /// The Berkas menu. Every entry says what it reads or writes.
    fn file_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Berkas", |ui| {
            let item = |ui: &mut egui::Ui, label: &str, hint: &str| {
                let clicked = ui.button(label).on_hover_text(hint).clicked();
                if clicked {
                    ui.close();
                }
                clicked
            };
            if item(ui, "Baru", "Mulai proyek kosong.") {
                self.leave(Leaving::New);
            }
            if item(
                ui,
                "Buka…  (Cmd/Ctrl+O)",
                "Membuka proyek .hew atau .wangun.",
            ) {
                self.leave(Leaving::Open);
            }
            if item(
                ui,
                "Simpan  (Cmd/Ctrl+S)",
                "Menyimpan ke file yang sedang dibuka. Proyek baru akan ditanya lokasinya.",
            ) {
                self.save();
            }
            if item(
                ui,
                "Simpan sebagai…  (Cmd/Ctrl+Shift+S)",
                "Satu tile menjadi .hew, banyak tile menjadi .wangun.",
            ) {
                self.dialog = Some(Dialog::SaveAs);
            }
            if item(
                ui,
                "Ekspor…",
                "Menulis model ke .obj, .glb, .gltf, .stl, atau .3mf. Proyek tidak berubah.",
            ) {
                self.dialog = Some(Dialog::Export);
            }
            ui.separator();
            if item(ui, "Format yang didukung…", "Penjelasan tiap format.") {
                self.formats_open = true;
            }
        });
    }

    /// The window explaining the formats.
    fn formats_window(&mut self, ctx: &egui::Context) {
        let mut open = self.formats_open;
        egui::Window::new("Format yang didukung")
            .open(&mut open)
            .collapsible(false)
            .show(ctx, |ui| {
                egui::Grid::new("formats").striped(true).show(ui, |ui| {
                    ui.strong("Aksi");
                    ui.strong("Format");
                    ui.strong("Isi dan batas");
                    ui.end_row();
                    for (action, format, what) in FILE_FORMATS {
                        ui.label(action);
                        ui.monospace(format);
                        ui.label(what);
                        ui.end_row();
                    }
                });
            });
        self.formats_open = open;
    }

    fn modal(&self) -> bool {
        self.recovery || self.leaving.is_some()
    }

    /// The questions that must be answered before going on.
    fn dialogs(&mut self, ctx: &egui::Context) {
        self.formats_window(ctx);
        let window = |title: &str| {
            egui::Window::new(title.to_owned())
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        };
        if self.recovery {
            window("Pulihkan pekerjaan?").show(ctx, |ui| {
                ui.label("Ada autosave dari sesi yang tidak berakhir normal.");
                ui.horizontal(|ui| {
                    if ui.button("Pulihkan").clicked() {
                        self.recover();
                    }
                    if ui.button("Buang").clicked() {
                        self.discard_recovery();
                    }
                });
            });
        }
        if let Some(what) = self.leaving {
            window("Perubahan belum disimpan").show(ctx, |ui| {
                ui.label("Perubahan terakhir belum disimpan.");
                ui.horizontal(|ui| {
                    if ui.button("Simpan").clicked() {
                        self.save();
                        if self.dialog.is_some() {
                            // Never saved: goes on once the file window has.
                            self.after_save = Some(what);
                            self.leaving = None;
                        } else if !self.unsaved {
                            self.do_leave(what);
                        } else {
                            // The reason is in the status bar.
                            self.leaving = None;
                        }
                    }
                    if ui.button("Buang").clicked() {
                        self.unsaved = false;
                        self.do_leave(what);
                    }
                    if ui.button("Batal").clicked() {
                        self.leaving = None;
                    }
                });
            });
        }
    }

    /// Middle-drag, or Alt + left-drag on a trackpad: orbit; with Shift: pan.
    /// Scroll: zoom. The plain left button belongs to the tool.
    fn navigate(&mut self, ui: &egui::Ui, resp: &egui::Response, size: egui::Vec2) {
        let m = ui.input(|i| i.modifiers);
        let held = resp.dragged_by(egui::PointerButton::Middle)
            || (m.alt && resp.dragged_by(egui::PointerButton::Primary));
        let d = resp.drag_delta();
        // A held button that does not move changes nothing, and must not spin.
        if held && d != egui::Vec2::ZERO {
            if m.shift {
                self.orbit.pan(d.x, d.y, size.y);
            } else {
                self.orbit.orbit(d.x, d.y);
            }
            self.camera_moved();
        }
        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.orbit.zoom(scroll / 50.0);
                self.camera_moved();
            }
        }
    }

    /// The same pixel now means another ray, and the marker another size.
    fn camera_moved(&mut self) {
        self.overlay_shown = None;
        self.last_pointer = None;
    }

    /// Feeds the tool. Returns whether it got anything.
    fn pointer_input(&mut self, resp: &egui::Response, rect: egui::Rect, size: egui::Vec2) -> bool {
        let mut fed = false;
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
            fed = true;
        }
        let navigating = resp.ctx.input(|i| i.modifiers.alt);
        let picked_before = self.tool.picked_nodes();
        if resp.clicked_by(egui::PointerButton::Primary)
            && !navigating
            && let Some(p) = resp.interact_pointer_pos()
        {
            self.error = None;
            // A click answers instead of the value; left over, it would block
            // the tool letters and feed the next Enter.
            self.typed.clear();
            let ray = ray_at(&self.orbit, p);
            let shift = resp.ctx.input(|i| i.modifiers.shift);
            self.feed(if shift {
                Input::ClickAdd(ray)
            } else {
                Input::Click(ray)
            });
            fed = true;
        }
        // Both clicks picked the same thing, so it opens. A quick click on it after
        // some other click is only a pick.
        let picked = self.tool.picked_nodes();
        if resp.double_clicked_by(egui::PointerButton::Primary)
            && !navigating
            && !resp.ctx.input(|i| i.modifiers.shift)
            && self.tool.same_kind(&Tool::select(0.0))
            && picked == picked_before
            && picked.is_some_and(|(_, n)| {
                use kernel::NodeId as N;
                matches!(n[..], [N::Group(_) | N::Instance(_)])
            })
        {
            self.enter_picked();
            fed = true;
        }
        fed
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
        let t = |s: &Source, chosen: &str, single| save_target(s, Path::new(chosen), single);
        // Same kind: overwrite what was opened.
        assert_eq!(
            t(&on_disk("/d/a.hew", false), "/d/a.hew", true),
            PathBuf::from("/d/a.hew")
        );
        assert_eq!(
            t(&on_disk("/d/a.wangun", true), "/d/a.wangun", false),
            PathBuf::from("/d/a.wangun")
        );
        // A .hew that was split or grew tiles goes beside it.
        assert_eq!(
            t(&on_disk("/d/a.hew", false), "/d/a.hew", false),
            PathBuf::from("/d/a.wangun")
        );
        // Another name is a Save As.
        assert_eq!(
            t(&on_disk("/d/a.hew", false), "/d/other.hew", true),
            PathBuf::from("/d/other.hew")
        );
        assert_eq!(
            t(&on_disk("/d/a.wangun", true), "/d/other", false),
            PathBuf::from("/d/other.wangun")
        );
        // Never saved: the kind decides the extension.
        let new = Source::default();
        assert_eq!(t(&new, "/d/b.hew", true), PathBuf::from("/d/b.hew"));
        assert_eq!(t(&new, "/d/b", true), PathBuf::from("/d/b.hew"));
        assert_eq!(t(&new, "/d/b.hew", false), PathBuf::from("/d/b.wangun"));
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
        assert!(!typed_accepts("", 'm') && !typed_accepts("", 'c') && !typed_accepts("", ' '));
        assert!(typed_accepts("", 'x') && typed_accepts("", '*') && typed_accepts("", '/'));
        assert!(typed_accepts("5", 'x') && typed_accepts("250", 'c') && typed_accepts("5", ' '));
        assert!(!typed_accepts("5", 'l') && !typed_accepts("", 'p'));
        assert!(typed_accepts("10", 'f') && typed_accepts("10f", 't'));
        assert!(!typed_accepts("", 'f') && !typed_accepts("", 't'));
    }

    #[test]
    fn every_snap_kind_is_named_apart_and_has_a_color() {
        let mut names: Vec<_> = SnapKind::ALL.iter().map(|k| snap_label(*k)).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), SnapKind::COUNT);
        for k in SnapKind::ALL {
            assert_ne!(snap_color(k), [0.0; 3], "{k:?}");
        }
    }

    #[test]
    fn the_unit_is_remembered_and_junk_means_meters() {
        let dir = std::env::temp_dir().join(format!("wangungis-unit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("unit");
        assert_eq!(read_unit(&file), Unit::M, "no file yet");
        for (text, want) in [("mm", Unit::Mm), ("ft\n", Unit::Ft), ("parsec", Unit::M)] {
            std::fs::write(&file, text).unwrap();
            assert_eq!(read_unit(&file), want, "{text:?}");
        }
        std::fs::write(&file, [0xff, 0xfe]).unwrap();
        assert_eq!(read_unit(&file), Unit::M, "not text");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
