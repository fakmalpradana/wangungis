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
use wangungis_tools::vcb::Unit;

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
    path: String,
    scene: Scene,
    tool: Tool,
    /// Lengths are shown in this, and bare typed lengths read in it.
    unit: Unit,
    /// `None`: the unit is not remembered, as when the home folder is unknown.
    unit_file: Option<PathBuf>,
    /// What the user is typing for the tool.
    typed: String,
    stats: BTreeMap<TileKey, TileStats>,
    source: Source,
    /// Differs from the file: edited, or split on open.
    unsaved: bool,
    split_on_open: bool,
    overlay_shown: Option<Preview>,
    last_pointer: Option<egui::Pos2>,
    moved_since_click: bool,
    /// The path box had focus last frame.
    path_focused: bool,
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
    #[cfg(test)]
    path_rect: egui::Rect,
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
            path: path.clone().unwrap_or_default(),
            scene: empty_scene(),
            tool: Tool::select(0.01),
            unit: Unit::default(),
            unit_file: None,
            typed: String::new(),
            stats: BTreeMap::new(),
            source: Source::default(),
            unsaved: false,
            split_on_open: false,
            overlay_shown: None,
            last_pointer: None,
            moved_since_click: false,
            path_focused: false,
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
            #[cfg(test)]
            path_rect: egui::Rect::NOTHING,
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
        self.camera_moved();
        let keys: Vec<TileKey> = self.scene.project().tiles.keys().copied().collect();
        for key in keys {
            self.upload(key);
        }
        self.scene.take_dirty();
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
            Leaving::Open => self.open(),
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
                self.path.clear();
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
        // Simpan writes to this box next: it must not still name the old file.
        self.path.clear();
        self.unsaved = false;
        self.split_on_open = false;
    }

    fn open(&mut self) {
        self.error = None;
        self.notice = None;
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
        self.notice = None;
        let project = self.scene.project();
        let single = project.single().is_some();
        let target = match save_target(&self.source, &self.path, single) {
            Ok(t) => t,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        if self.source.path.as_deref() != Some(&target) && target.exists() {
            self.error = Some(format!(
                "{} sudah ada dan bukan file yang sedang dibuka. Pakai nama lain.",
                target.display()
            ));
            return;
        }
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
                self.drop_autosave();
            }
            Err(e) => self.error = Some(format!("Tidak bisa menyimpan {}: {e}", target.display())),
        }
    }

    /// Writes the project to the path box as the format its extension names.
    /// Never over an existing file.
    fn export(&mut self) {
        self.notice = None;
        let target = PathBuf::from(self.path.trim());
        let format = target
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .filter(|e| wangungis_io::FORMATS.contains(&e.as_str()));
        let Some(format) = format else {
            self.error = Some(format!(
                "Isi kotak atas dengan path ber-ekstensi: {}.",
                wangungis_io::FORMATS.join(", ")
            ));
            return;
        };
        if target.exists() {
            self.error = Some(format!("{} sudah ada. Pakai nama lain.", target.display()));
            return;
        }
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
            .and_then(|bytes| write_atomic(&target, &bytes).map_err(|e| e.to_string()));
        match written {
            Ok(()) => {
                self.error = None;
                self.notice = Some(format!("Diekspor ke {}", target.display()));
            }
            Err(e) => self.error = Some(e),
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
        self.tool = make(0.01);
        self.tool.set_unit(self.unit);
        self.typed.clear();
        self.error = None;
        // The parked pointer has something to tell the new tool.
        self.last_pointer = None;
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
        // Enter presses it. In the path box, tool keys edit the path and Enter
        // starts opening whatever that became.
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
                // egui cleared the box's focus before this frame: Esc is spent.
                egui::Event::Key {
                    key: egui::Key::Escape,
                    ..
                } if self.path_focused => {}
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
            K::Backspace if !self.typed.is_empty() => {
                self.typed.pop();
            }
            K::Delete | K::Backspace => self.delete_picked(),
            K::Escape => {
                self.typed.clear();
                self.error = None;
                self.feed(Input::Cancel);
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

/// Where Simpan writes: the path box, or the opened file when the box is empty.
/// A single-tile project is a plain `.hew`, anything else a `.wangun`. The opened
/// file is overwritten only when it already holds that kind, so a `.hew` that
/// was split or grew tiles is never replaced by a container.
fn save_target(source: &Source, path_box: &str, single: bool) -> Result<PathBuf, String> {
    let boxed = path_box.trim();
    let base: PathBuf = match &source.path {
        _ if !boxed.is_empty() => boxed.into(),
        Some(p) => p.clone(),
        None => return Err("Isi path file di kotak atas dulu, lalu tekan Simpan.".into()),
    };
    let same_kind = source.path.as_ref() == Some(&base) && source.container != single;
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
                ui.label("File:");
                let field = ui.add(egui::TextEdit::singleline(&mut self.path).desired_width(360.0));
                let had_focus = std::mem::replace(&mut self.path_focused, field.has_focus());
                #[cfg(test)]
                {
                    self.path_rect = field.rect;
                }
                // lost_focus stays true a frame longer: an Enter then is not the box's.
                let enter = had_focus
                    && field.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button("Buka").clicked() || enter {
                    self.leave(Leaving::Open);
                }
                if ui.button("Baru").clicked() {
                    self.leave(Leaving::New);
                }
                if ui.button("Simpan").clicked() {
                    self.save();
                }
                if ui.button("Ekspor").clicked() {
                    self.export();
                }
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
                    "{} · {} tile · {} objek · {} segitiga",
                    self.source
                        .path
                        .as_deref()
                        .and_then(Path::file_name)
                        .map_or("Proyek baru".into(), |n| n.to_string_lossy()),
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
            });
    }

    fn modal(&self) -> bool {
        self.recovery || self.leaving.is_some()
    }

    /// The questions that must be answered before going on.
    fn dialogs(&mut self, ctx: &egui::Context) {
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
                        if !self.unsaved {
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
        assert_eq!(
            t(&on_disk("/d/a.hew", false), "/d/a.hew", true),
            PathBuf::from("/d/a.hew")
        );
        // A .hew that was split or grew tiles goes beside it.
        assert_eq!(
            t(&on_disk("/d/a.hew", false), "/d/a.hew", false),
            PathBuf::from("/d/a.wangun")
        );
        // A name typed in the box is a Save As.
        assert_eq!(
            t(&on_disk("/d/a.hew", false), "/d/other.hew", true),
            PathBuf::from("/d/other.hew")
        );
        assert_eq!(
            t(&on_disk("/d/a.wangun", true), "/d/other", false),
            PathBuf::from("/d/other.wangun")
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
        assert!(!typed_accepts("", 'm') && !typed_accepts("", 'c') && !typed_accepts("", ' '));
        assert!(typed_accepts("", 'x') && typed_accepts("", '*') && typed_accepts("", '/'));
        assert!(typed_accepts("5", 'x') && typed_accepts("250", 'c') && typed_accepts("5", ' '));
        assert!(!typed_accepts("5", 'l') && !typed_accepts("", 'p'));
        assert!(typed_accepts("10", 'f') && typed_accepts("10f", 't'));
        assert!(!typed_accepts("", 'f') && !typed_accepts("", 't'));
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
