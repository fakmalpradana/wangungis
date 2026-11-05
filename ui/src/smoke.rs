//! The smoke checklist (`docs/user/uji-asap.md`) and the v0.1 exit task played
//! against the real `App`:
//! raw egui events in, real renderer on an offscreen GPU, no window. This is
//! what stands in for clicking through the app, which CI cannot do. Buttons are
//! called directly: finding them needs layout positions, and they only call
//! the same methods.

use std::sync::Mutex;

use eframe::{egui, egui_wgpu};

use super::*;

const SCREEN: egui::Vec2 = egui::vec2(1280.0, 800.0);
const COMMAND: egui::Modifiers = egui::Modifiers::COMMAND;

/// One GPU test at a time: software adapters race, like in `render`.
static GPU_LOCK: Mutex<()> = Mutex::new(());

fn render_state() -> Option<egui_wgpu::RenderState> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let found = [false, true].into_iter().find_map(|fallback| {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            force_fallback_adapter: fallback,
            ..Default::default()
        }))
        .ok()?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).ok()?;
        Some((adapter, device, queue))
    });
    let Some((adapter, device, queue)) = found else {
        assert!(
            std::env::var_os("WANGUNGIS_REQUIRE_GPU").is_none(),
            "no GPU adapter, and WANGUNGIS_REQUIRE_GPU is set"
        );
        eprintln!("no GPU adapter: smoke test skipped");
        return None;
    };
    let config = egui_wgpu::WgpuConfiguration {
        wgpu_setup: egui_wgpu::WgpuSetupExisting {
            instance: instance.clone(),
            adapter,
            device,
            queue,
        }
        .into(),
        ..Default::default()
    };
    pollster::block_on(egui_wgpu::RenderState::create(
        &config,
        &instance,
        None,
        Default::default(),
    ))
    .ok()
}

struct Driver {
    ctx: egui::Context,
    app: App,
    time: f64,
}

impl Driver {
    /// `None` without a GPU.
    fn start() -> Option<Self> {
        let mut d = Driver {
            ctx: egui::Context::default(),
            app: App::with_render_state(render_state()?),
            time: 0.0,
        };
        // Panels take their size from the frame before: let the layout settle.
        for _ in 0..3 {
            d.frame(vec![]);
        }
        Some(d)
    }

    fn frame(&mut self, events: Vec<egui::Event>) {
        self.time += 0.05;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
            time: Some(self.time),
            predicted_dt: 0.05,
            events,
            ..Default::default()
        };
        let app = &mut self.app;
        // Only the 3D view is under test, nothing paints the egui output.
        let mut output = self.ctx.run_ui(input, |ui| app.shell(ui));
        output.textures_delta.clear();
    }

    /// The pixel showing `p`, from the same public `ray` the tools use.
    fn pixel(&self, p: [f64; 3]) -> egui::Pos2 {
        let r = self.app.view_rect;
        let (w, h) = (r.width(), r.height());
        let (eye, f) = self.app.orbit.ray(w / 2.0, h / 2.0, w, h);
        let (_, right) = self.app.orbit.ray(w, h / 2.0, w, h);
        let (_, up) = self.app.orbit.ray(w / 2.0, 0.0, w, h);
        let dot = |a: [f64; 3], b: [f64; 3]| (0..3).map(|k| a[k] * b[k]).sum::<f64>();
        let sub = |a: [f64; 3], b: [f64; 3]| -> [f64; 3] { std::array::from_fn(|k| a[k] - b[k]) };
        let (right, up) = (sub(right, f), sub(up, f));
        let v = sub(p, eye);
        let z = dot(v, f) / dot(f, f);
        let x = dot(v, right) / dot(right, right) / z;
        let y = dot(v, up) / dot(up, up) / z;
        egui::pos2(
            r.min.x + (w / 2.0) * (1.0 + x as f32),
            r.min.y + (h / 2.0) * (1.0 - y as f32),
        )
    }

    fn hover(&mut self, p: [f64; 3]) {
        let at = self.pixel(p);
        self.frame(vec![egui::Event::PointerMoved(at)]);
    }

    fn click_at(&mut self, at: egui::Pos2) {
        let m = self.ctx.input(|i| i.modifiers);
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: m,
        };
        self.frame(vec![egui::Event::PointerMoved(at)]);
        self.frame(vec![button(true)]);
        self.frame(vec![button(false)]);
    }

    fn hold(&mut self, m: egui::Modifiers) {
        self.frame(vec![egui::Event::ModifiersChanged(m)]);
    }

    fn click(&mut self, p: [f64; 3]) {
        let at = self.pixel(p);
        self.click_at(at);
    }

    fn key_with(&mut self, key: egui::Key, modifiers: egui::Modifiers) {
        let ev = |pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        };
        self.frame(vec![ev(true), ev(false)]);
    }

    fn key(&mut self, key: egui::Key) {
        self.key_with(key, egui::Modifiers::NONE);
    }

    /// A tool letter: the window also delivers it as text.
    fn letter(&mut self, key: egui::Key, text: &str) {
        self.frame(vec![egui::Event::Text(text.into())]);
        self.key(key);
    }

    /// The file window waiting, answered as if the user chose `path`.
    fn answer_dialog(&mut self, path: Option<&std::path::Path>) {
        let dialog = self.app.dialog.take().expect("a file window is waiting");
        self.app.answer_dialog(dialog, path.map(ToOwned::to_owned));
    }

    /// Typed value, then Enter.
    fn value(&mut self, text: &str) {
        self.frame(vec![egui::Event::Text(text.into())]);
        assert_eq!(self.app.typed, text, "typed text reaches the value box");
        self.key(egui::Key::Enter);
        assert_eq!(self.app.error, None, "value {text}");
    }

    fn counts(&self) -> (usize, usize) {
        (
            self.app.stats.values().map(|s| s.objects).sum(),
            self.app.stats.values().map(|s| s.triangles).sum(),
        )
    }

    fn extents(&self) -> [f64; 3] {
        let (lo, hi) = self.app.bounds().expect("something is drawn");
        std::array::from_fn(|k| hi[k] - lo[k])
    }

    /// The 3D view as RGBA, read back from its texture.
    fn pixels(&self) -> (u32, u32, Vec<u8>) {
        let rs = &self.app.rs;
        let v = self.app.view.as_ref().expect("view exists after a frame");
        let [w, h] = v.size;
        let bytes_per_row = (4 * w).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = rs.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("smoke readback"),
            size: u64::from(bytes_per_row * h),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = rs.device.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            v.color.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        rs.queue.submit([enc.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.unwrap());
        rs.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let data = slice.get_mapped_range().unwrap();
        let mut out = Vec::with_capacity((4 * w * h) as usize);
        for row in data.chunks(bytes_per_row as usize) {
            out.extend_from_slice(&row[..(4 * w) as usize]);
        }
        (w, h, out)
    }

    /// Pixels neither background nor an axis line would explain: gray model.
    fn model_pixels(&self) -> usize {
        let (_, _, px) = self.pixels();
        px.chunks(4)
            .filter(|p| {
                let spread = p[0].max(p[1]).max(p[2]) - p[0].min(p[1]).min(p[2]);
                spread < 40 && p[0] < 225
            })
            .count()
    }

    /// With `WANGUNGIS_SMOKE_DUMP=dir`: the view as a PPM, to look at by eye.
    fn dump(&self, name: &str) {
        let Some(dir) = std::env::var_os("WANGUNGIS_SMOKE_DUMP") else {
            return;
        };
        let (w, h, px) = self.pixels();
        let mut out = format!("P6 {w} {h} 255\n").into_bytes();
        for p in px.chunks(4) {
            out.extend_from_slice(&p[..3]);
        }
        std::fs::write(PathBuf::from(dir).join(format!("{name}.ppm")), out).unwrap();
    }
}

fn close(got: [f64; 3], want: [f64; 3]) {
    for k in 0..3 {
        assert!(
            (got[k] - want[k]).abs() < 1e-3,
            "got {got:?}, want {want:?}"
        );
    }
}

/// The model's low corner is where it was clicked, on the ground. In the default
/// view (2, 2, 0) lies right over the Z axis, which must not catch the click.
fn on_the_ground(d: &Driver, at: [f64; 2]) {
    let (lo, _) = d.app.bounds().expect("something is drawn");
    close(lo, [at[0], at[1], 0.0]);
}

#[test]
fn smoke_checklist_in_the_real_app() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    use egui::Key as K;

    // Step 1: an empty project, axes in view.
    assert_eq!(d.counts(), (0, 0));
    d.dump("1-empty");

    // Steps 2-3: Rectangle, first corner, typed size.
    d.letter(K::R, "r");
    d.click([2.0, 2.0, 0.0]);
    assert!(d.app.tool.busy(), "the first click starts the rectangle");
    d.hover([3.0, 3.0, 0.0]);
    assert!(!d.app.tool.preview().lines.is_empty(), "preview follows");
    d.value("5;4");
    assert!(!d.app.tool.busy());
    assert_eq!(d.app.scene.project().occupied().count(), 1);
    assert!(d.app.scene.can_undo());
    assert!(d.app.unsaved);
    d.dump("3-rectangle");

    // Steps 4-5: Push/Pull the rectangle to 3 m.
    d.letter(K::P, "p");
    d.hover([3.5, 3.0, 0.0]);
    assert!(
        !d.app.tool.preview().lines.is_empty(),
        "the region lights up"
    );
    d.click([3.5, 3.0, 0.0]);
    assert!(d.app.tool.busy(), "the click picks the region");
    d.value("3");
    assert_eq!(d.counts(), (1, 12));
    close(d.extents(), [5.0, 4.0, 3.0]);
    on_the_ground(&d, [2.0, 2.0]);
    assert!(d.model_pixels() > 1000, "the box shows in the view");
    d.dump("5-box");

    // Step 6: the top face follows the cursor, then a click ends it.
    d.click([3.5, 3.0, 3.0]);
    assert!(d.app.tool.busy(), "the top face is picked");
    d.hover([3.5, 3.0, 4.5]);
    d.click([3.5, 3.0, 4.5]);
    assert!(!d.app.tool.busy());
    assert_eq!(d.app.error, None);
    let tall = d.extents();
    assert!(tall[2] > 3.5, "the box grew: {tall:?}");
    d.dump("6-taller");

    // Step 7: undo to empty, redo back. Rectangle, extrude, pull: three steps.
    for _ in 0..3 {
        assert!(d.app.scene.can_undo());
        d.key_with(K::Z, COMMAND);
    }
    assert!(!d.app.scene.can_undo());
    assert_eq!(d.counts(), (0, 0));
    for _ in 0..3 {
        d.key_with(K::Z, COMMAND | egui::Modifiers::SHIFT);
    }
    assert!(!d.app.scene.can_redo());
    assert_eq!(d.counts(), (1, 12));
    close(d.extents(), tall);

    // Step 8: Line, four corners, closing click ends the chain. The corner
    // snaps on hover. Beside the box, so nothing there interferes.
    d.letter(K::L, "l");
    let corners = [
        [10.0, 1.0, 0.0],
        [13.0, 1.0, 0.0],
        [13.0, 4.0, 0.0],
        [10.0, 4.0, 0.0],
    ];
    for c in corners {
        d.click(c);
    }
    d.hover(corners[0]);
    let snap = d.app.tool.preview().snap.expect("the first corner snaps");
    assert_eq!(snap.snap.kind, SnapKind::Endpoint);
    d.click(corners[0]);
    assert!(!d.app.tool.busy(), "closing the loop ends the chain");
    assert_eq!(d.app.error, None);
    d.dump("8-line");

    // Step 9: save, Baru, open the same file.
    let dir = std::env::temp_dir().join(format!("wangungis-smoke-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("uji.hew");
    // Cmd+S on a project never saved asks where.
    d.key_with(K::S, COMMAND);
    assert_eq!(d.app.dialog, Some(Dialog::SaveAs));
    d.answer_dialog(Some(&file));
    assert_eq!(d.app.error, None);
    assert!(!d.app.unsaved);
    let saved = std::fs::read(&file).unwrap();
    let before = (d.counts(), d.extents());

    d.app.new_project();
    d.frame(vec![]);
    assert_eq!(d.counts(), (0, 0));
    assert_eq!(d.app.source.path, None);
    assert!(!d.app.scene.can_undo());

    d.key_with(K::O, COMMAND);
    assert_eq!(d.app.dialog, Some(Dialog::Open));
    d.answer_dialog(Some(&file));
    d.frame(vec![]);
    assert_eq!(d.app.error, None);
    assert_eq!((d.counts(), d.extents()), before);
    d.dump("9-reopened");

    // Saving again changes nothing on disk.
    d.key_with(K::S, COMMAND);
    assert_eq!(std::fs::read(&file).unwrap(), saved);

    // A name changed here was never asked about: it replaces nothing.
    let other = dir.join("lain.hew");
    std::fs::write(&other, b"milik orang").unwrap();
    d.app.save_to(&dir.join("lain.wangun"));
    assert!(d.app.error.is_some());
    assert_eq!(std::fs::read(&other).unwrap(), b"milik orang");

    // The window asked about this one: Save As replaces it and moves on to it.
    d.key_with(K::S, COMMAND | egui::Modifiers::SHIFT);
    assert_eq!(d.app.dialog, Some(Dialog::SaveAs));
    d.answer_dialog(Some(&other));
    assert_eq!(d.app.error, None);
    assert_eq!(std::fs::read(&other).unwrap(), saved);
    assert_eq!(d.app.source.path.as_deref(), Some(other.as_path()));
    // Cancelling changes nothing.
    d.key_with(K::S, COMMAND | egui::Modifiers::SHIFT);
    d.answer_dialog(None);
    assert_eq!(d.app.source.path.as_deref(), Some(other.as_path()));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn navigation_never_reaches_the_tool() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };

    d.letter(egui::Key::R, "r");
    let start = d.app.orbit.eye();
    let from = d.pixel([0.0, 0.0, 0.0]);
    let to = from + egui::vec2(60.0, 20.0);
    let button = |pos, pressed, modifiers| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers,
    };

    // Alt + left-drag orbits, and draws nothing.
    let m = egui::Modifiers::ALT;
    d.hold(m);
    d.frame(vec![egui::Event::PointerMoved(from)]);
    d.frame(vec![button(from, true, m)]);
    for t in 1..=6 {
        d.frame(vec![egui::Event::PointerMoved(
            from + (to - from) * t as f32 / 6.0,
        )]);
    }
    d.frame(vec![button(to, false, m)]);
    d.hold(egui::Modifiers::NONE);
    assert_ne!(d.app.orbit.eye(), start, "the camera moved");
    assert!(!d.app.tool.busy(), "the drag fed no click to the tool");
    assert!(!d.app.scene.can_undo());

    // An Alt click is navigation too.
    d.hold(m);
    d.click_at(from);
    d.hold(egui::Modifiers::NONE);
    assert!(!d.app.tool.busy());

    // Scroll zooms.
    let far = d.app.orbit.eye();
    let wheel = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, 80.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: egui::Modifiers::NONE,
    };
    d.frame(vec![egui::Event::PointerMoved(from)]);
    d.frame(vec![wheel]);
    for _ in 0..20 {
        d.frame(vec![]);
    }
    assert_ne!(d.app.orbit.eye(), far, "scroll zooms");

    // A digit is a value, a tool letter after it is not a tool.
    d.frame(vec![egui::Event::Text("5".into())]);
    assert_eq!(d.app.typed, "5");
    d.letter(egui::Key::L, "l");
    assert!(
        d.app.tool.same_kind(&Tool::rectangle(0.0)),
        "still Rectangle"
    );
    assert_eq!(d.app.typed, "5");
    d.key(egui::Key::Escape);
    assert!(d.app.typed.is_empty());
}

/// The v0.1 exit task, start to finish through the real input path: a 5 x 4 x 3 m
/// box with a door, copied ten times in an array, saved and reopened.
#[test]
fn box_with_a_door_in_an_array_of_ten() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    use egui::Key as K;

    d.letter(K::R, "r");
    d.click([2.0, 2.0, 0.0]);
    d.hover([3.0, 3.0, 0.0]);
    d.value("5;4");
    d.letter(K::P, "p");
    d.click([3.5, 3.0, 0.0]);
    d.value("3");
    assert_eq!(d.counts(), (1, 12));
    close(d.extents(), [5.0, 4.0, 3.0]);
    on_the_ground(&d, [2.0, 2.0]);

    // The door, zoomed in so that the corner of the box is out of the snap's reach.
    // The cursor is on the wall just above its bottom edge, which the corner of
    // the rectangle snaps to.
    d.app.fit_pending = true;
    d.frame(vec![]);
    d.letter(K::R, "r");
    d.click([3.0, 2.0, 0.03]);
    assert!(d.app.tool.busy());
    d.hover([3.6, 2.0, 1.0]);
    d.value("1;2");
    assert_eq!(d.app.error, None);
    assert_eq!(d.app.scene.project().occupied().count(), 1);
    let object = |d: &Driver| {
        let doc = &d.app.scene.project().tiles[&TileKey::ORIGIN];
        doc.object(doc.visible_object_ids()[0])
            .unwrap()
            .faces()
            .len()
    };
    assert_eq!(object(&d), 7, "the wall was split");
    d.dump("exit-1-door-drawn");

    d.letter(K::P, "p");
    // Where the corner snapped depends on the zoom: ask the model where it went.
    let door = {
        let doc = &d.app.scene.project().tiles[&TileKey::ORIGIN];
        let o = doc.object(doc.visible_object_ids()[0]).unwrap();
        let ring: Vec<Point3> = o
            .faces()
            .values()
            .map(|f| o.loop_positions(f.outer_loop).collect::<Vec<_>>())
            .find(|r| {
                let span = |k: fn(&Point3) -> f64| {
                    let (lo, hi) = r
                        .iter()
                        .map(k)
                        .fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(v), b.max(v)));
                    hi - lo
                };
                span(|p| p.y) < 1e-6 && (span(|p| p.x) - 1.0).abs() < 1e-3
            })
            .expect("the door face");
        let n = ring.len() as f64;
        [
            ring.iter().map(|p| p.x).sum::<f64>() / n,
            ring[0].y,
            ring.iter().map(|p| p.z).sum::<f64>() / n,
        ]
    };
    d.click(door);
    assert!(d.app.tool.busy(), "the door face is picked");
    d.value("-4");
    assert_eq!(
        object(&d),
        10,
        "an inverted U: the door opens right through"
    );
    d.dump("exit-2-door-open");
    let solid = d.app.stats.values().map(|s| s.triangles).sum::<usize>();
    assert!(solid > 12, "the box gained an opening: {solid} triangles");

    // Move, copy, along X, 7 m on, then ten copies.
    d.letter(K::M, "m");
    d.click([3.5, 3.0, 3.0]);
    assert!(d.app.tool.busy());
    d.letter(K::C, "c");
    assert!(d.app.tool.copying());
    d.key(K::ArrowRight);
    d.hover([12.0, 3.0, 3.0]);
    d.value("7");
    assert_eq!(d.counts().0, 2);
    d.value("x10");
    assert_eq!(d.counts().0, 11, "the source and ten copies");
    close(d.extents(), [75.0, 4.0, 3.0]);
    d.app.fit_pending = true;
    d.frame(vec![]);
    d.frame(vec![]);
    d.dump("exit-3-array");

    // One undo removes the array, one redo brings it back.
    d.key_with(K::Z, COMMAND);
    assert_eq!(d.counts().0, 1);
    d.key_with(K::Z, COMMAND | egui::Modifiers::SHIFT);
    assert_eq!(d.counts().0, 11);

    // Save, reopen, save again: byte-identical.
    let dir = std::env::temp_dir().join(format!("wangungis-exit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("kota.hew");
    d.app.save_to(&file);
    assert_eq!(d.app.error, None);
    let saved = std::fs::read(&file).unwrap();
    let before = d.counts();
    d.app.new_project();
    d.frame(vec![]);
    d.app.open_path(&file);
    d.frame(vec![]);
    assert_eq!(d.counts(), before);
    close(d.extents(), [75.0, 4.0, 3.0]);
    d.key_with(K::S, COMMAND);
    assert_eq!(std::fs::read(&file).unwrap(), saved, "byte-identical");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A bare tap of Ctrl flips copying like the C key; a chord with it does not.
#[test]
fn a_ctrl_tap_toggles_copying_in_move() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    d.letter(egui::Key::M, "m");
    d.hold(egui::Modifiers::CTRL);
    d.frame(vec![]);
    assert!(!d.app.tool.copying(), "nothing until it is released");
    d.hold(egui::Modifiers::NONE);
    d.frame(vec![]);
    assert!(d.app.tool.copying(), "a tap copies");
    d.hold(egui::Modifiers::CTRL);
    d.hold(egui::Modifiers::NONE);
    d.frame(vec![]);
    assert!(!d.app.tool.copying(), "a second tap goes back");

    // Ctrl+S on the way does not count as a tap.
    d.hold(egui::Modifiers::CTRL);
    d.frame(vec![egui::Event::Key {
        key: egui::Key::S,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::CTRL,
    }]);
    d.hold(egui::Modifiers::NONE);
    d.frame(vec![]);
    assert!(!d.app.tool.copying());

    // Ctrl+C reaches egui as Copy, not as a key. And Ctrl may go up in the same
    // frame as the key goes down.
    d.hold(egui::Modifiers::CTRL);
    d.frame(vec![egui::Event::Copy]);
    d.hold(egui::Modifiers::NONE);
    assert!(!d.app.tool.copying());
    d.hold(egui::Modifiers::CTRL);
    d.frame(vec![
        egui::Event::ModifiersChanged(egui::Modifiers::NONE),
        egui::Event::Key {
            key: egui::Key::S,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::CTRL,
        },
    ]);
    d.frame(vec![]);
    assert!(!d.app.tool.copying());
}

/// A box drawn through the real input path.
fn draw_a_box(d: &mut Driver) {
    use egui::Key as K;
    d.letter(K::R, "r");
    d.click([2.0, 2.0, 0.0]);
    d.hover([3.0, 3.0, 0.0]);
    d.value("5;4");
    d.letter(K::P, "p");
    d.click([3.5, 3.0, 0.0]);
    d.value("3");
    assert_eq!(d.counts(), (1, 12));
}

/// The unit reads back after a restart, rules what is typed, and shows on the tape.
#[test]
fn millimeters_from_the_status_bar_to_the_model() {
    use egui::Key as K;
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    let dir = temp_dir("unit");
    d.app.unit_file = Some(dir.join("data").join("unit"));

    d.app.set_unit(Unit::Mm);
    assert_eq!(d.app.error, None);
    assert_eq!(read_unit(&dir.join("data").join("unit")), Unit::Mm);

    d.letter(K::R, "r");
    d.click([2.0, 2.0, 0.0]);
    d.hover([3.0, 3.0, 0.0]);
    d.value("5000;4000");
    d.letter(K::P, "p");
    d.click([3.5, 3.0, 0.0]);
    d.value("3000");
    close(d.extents(), [5.0, 4.0, 3.0]);

    // F and T are units here, not the Offset and Tape tools.
    d.letter(K::P, "p");
    d.click([3.5, 3.0, 3.0]);
    d.frame(vec![egui::Event::Text("10".into())]);
    d.letter(K::F, "f");
    d.letter(K::T, "t");
    assert_eq!(d.app.typed, "10ft");
    d.key(K::Enter);
    assert_eq!(d.app.error, None);
    close(d.extents(), [5.0, 4.0, 6.048]);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The pointer on a corner names it, and the line drawn past an edge runs along it.
#[test]
fn a_line_drawn_parallel_through_the_app() {
    use egui::Key as K;
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };

    d.letter(K::L, "l");
    d.click([20.0, 20.0, 0.0]);
    d.click([24.0, 23.0, 0.0]);
    d.key(K::Escape);
    d.app.fit_pending = true;
    d.frame(vec![]);
    d.frame(vec![]);

    d.click([26.0, 21.0, 0.0]);
    d.hover([24.0, 23.0, 0.0]);
    let kind = d.app.tool.preview().snap.map(|h| snap_label(h.snap.kind));
    assert_eq!(kind, Some("Titik ujung"));
    d.hover([22.0, 21.5, 0.0]);
    d.hover([28.0, 22.52, 0.0]);
    let kind = d.app.tool.preview().snap.map(|h| snap_label(h.snap.kind));
    assert_eq!(kind, Some("Sejajar tepi"));
    d.click([28.0, 22.52, 0.0]);
    assert_eq!(d.app.error, None);
    d.key(K::Escape);
}

/// A tap of Ctrl in Push/Pull: the top goes up as a second solid.
#[test]
fn push_pull_with_ctrl_through_the_app() {
    use egui::Key as K;
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    draw_a_box(&mut d);
    d.app.fit_pending = true;
    d.frame(vec![]);
    d.frame(vec![]);

    d.letter(K::P, "p");
    assert!(!d.app.tool.keeping());
    d.hold(egui::Modifiers::CTRL);
    d.hold(egui::Modifiers::NONE);
    assert!(d.app.tool.keeping(), "tapping Ctrl turned it on");

    d.click([4.5, 4.0, 3.0]);
    d.value("2");
    assert_eq!(d.counts().0, 2, "a second solid");
    close(d.extents(), [5.0, 4.0, 5.0]);
    assert!(d.app.tool.keeping(), "it stays on for the next face");

    d.key_with(K::Z, egui::Modifiers::COMMAND);
    assert_eq!(d.counts().0, 1, "one undo takes it back");
    close(d.extents(), [5.0, 4.0, 3.0]);
}

/// Pulling a top up to a corner of another solid, through the app: the marker
/// names the corner and the height comes out exact.
#[test]
fn push_pull_snaps_through_the_app() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    two_boxes(&mut d);

    d.click([4.5, 4.0, 3.0]);
    d.hover([12.0, 2.0, 6.0]);
    let kind = d.app.tool.preview().snap.map(|h| snap_label(h.snap.kind));
    assert_eq!(kind, Some(snap_label(inference::SnapKind::Endpoint)));
    d.click([12.0, 2.0, 6.0]);
    assert_eq!(d.app.error, None);
    assert_eq!(d.counts().0, 2);
    let top = d
        .app
        .scene
        .project()
        .tiles
        .values()
        .flat_map(|doc| {
            doc.visible_object_ids()
                .into_iter()
                .filter_map(|id| doc.object(id).map(|o| o.vertices().clone()))
                .collect::<Vec<_>>()
        })
        .flat_map(|v| {
            v.values()
                .map(|p| (p.position.x, p.position.z))
                .collect::<Vec<_>>()
        })
        .filter(|(x, _)| *x < 8.0)
        .map(|(_, z)| z)
        .fold(f64::MIN, f64::max);
    assert!((top - 6.0).abs() < 1e-3, "{top}");
}

/// Two boxes next to each other, 3 m and 6 m tall, in view.
fn two_boxes(d: &mut Driver) {
    use egui::Key as K;
    draw_a_box(d);
    d.letter(K::R, "r");
    d.click([12.0, 2.0, 0.0]);
    d.hover([13.0, 3.0, 0.0]);
    d.value("5;4");
    d.letter(K::P, "p");
    d.click([13.5, 3.0, 0.0]);
    d.value("6");
    d.app.fit_pending = true;
    d.frame(vec![]);
    d.frame(vec![]);
}

/// Pick two solids, press P, pull one top: both go up in one undo. Then the same
/// with Shift+click in the tool.
#[test]
fn push_pull_two_solids_through_the_app() {
    use egui::Key as K;
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    two_boxes(&mut d);

    d.letter(K::Space, " ");
    d.click([4.5, 4.0, 3.0]);
    d.hold(egui::Modifiers::SHIFT);
    d.click([14.5, 4.0, 6.0]);
    d.hold(egui::Modifiers::NONE);
    assert_eq!(d.app.tool.picked_nodes().map(|p| p.1.len()), Some(2));
    d.letter(K::P, "p");
    d.click([4.5, 4.0, 3.0]);
    d.value("1");
    assert_eq!(d.extents()[2], 7.0, "the taller one moved too");
    d.key_with(K::Z, COMMAND);
    assert_eq!(d.extents()[2], 6.0, "one undo");

    // An undo resets the tool, and the selection with it.
    d.letter(K::P, "p");
    d.hold(egui::Modifiers::SHIFT);
    d.click([4.5, 4.0, 3.0]);
    d.click([14.5, 4.0, 6.0]);
    d.hold(egui::Modifiers::NONE);
    d.value("2");
    assert_eq!(d.extents()[2], 8.0);
    assert_eq!(d.app.error, None);
}

/// B paints the face under the pointer: the view shows it, undo takes it off.
#[test]
fn paint_a_face_through_the_app() {
    use egui::Key as K;
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    draw_a_box(&mut d);
    d.app.fit_pending = true;
    d.frame(vec![]);
    d.frame(vec![]);
    let blue = |d: &Driver| {
        let (_, _, px) = d.pixels();
        px.chunks(4)
            .filter(|p| i32::from(p[2]) > i32::from(p[0]) + 40)
            .count()
    };
    // The Z axis is blue too.
    let before = blue(&d);
    let plain = wangungis_io::export(d.app.scene.project(), "glb").unwrap();

    d.letter(K::B, "b");
    d.app.set_paint(Some(ready_paint(("Biru", [70, 110, 170]))));
    d.click([4.5, 4.0, 3.0]);
    assert_eq!(d.app.error, None);
    d.frame(vec![]);
    d.dump("paint");
    assert!(blue(&d) > before + 500, "the top is blue: {}", blue(&d));

    // The export carries it.
    let painted = wangungis_io::export(d.app.scene.project(), "glb").unwrap();
    let has = |b: &[u8], what: &[u8]| b.windows(what.len()).any(|w| w == what);
    assert!(!has(&plain, b"Biru"), "not painted yet");
    assert!(has(&painted, b"baseColorFactor") && has(&painted, b"Biru"));

    d.key_with(K::Z, egui::Modifiers::COMMAND);
    d.frame(vec![]);
    // No more than before: the row of colors shrinks the view, and the axis in it.
    assert!(blue(&d) <= before, "undo took it off: {}", blue(&d));
}

/// Two clicks open a group, a member takes an edit, Esc closes it.
#[test]
fn edit_a_group_in_place_through_the_app() {
    use egui::Key as K;
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    draw_a_box(&mut d);
    d.app.fit_pending = true;
    d.frame(vec![]);
    d.frame(vec![]);

    d.key(K::Space);
    d.click([4.5, 4.0, 3.0]);
    d.app.on_picked(|s, t, n| s.group(t, n));
    assert_eq!(d.app.error, None);

    // A member of a closed group is out of reach.
    d.letter(K::P, "p");
    d.click([4.5, 4.0, 3.0]);
    assert!(!d.app.tool.busy());

    // A quick click after the one above is a double click to egui, but it only picks.
    d.key(K::Space);
    d.click([4.5, 4.0, 3.0]);
    assert_eq!(d.app.session_label(), None);
    d.key(K::Escape);

    // Far from the click before: a third quick click is no double click.
    d.time += 1.0;
    let at = d.pixel([4.5, 4.0, 3.0]);
    d.click_at(at);
    d.click_at(at);
    assert_eq!(d.app.error, None);
    let label = d.app.session_label();
    assert_eq!(
        label.as_deref(),
        Some("Mengedit: Grup"),
        "two clicks opened it"
    );
    d.frame(vec![]);
    d.dump("inside-group");

    d.letter(K::P, "p");
    d.click([4.5, 4.0, 3.0]);
    assert!(d.app.tool.busy());
    d.value("1");
    close(d.extents(), [5.0, 4.0, 4.0]);

    // The first Esc ends nothing here: Push/Pull is idle, so it closes.
    d.key(K::Escape);
    assert_eq!(d.app.session_label(), None);
    close(d.extents(), [5.0, 4.0, 4.0]);
    assert_eq!(d.app.error, None);
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("wangungis-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn autosave_survives_a_crash_and_goes_after_a_save() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    let dir = temp_dir("autosave");
    let auto = dir.join("data").join("autosave.bin");
    d.app.autosave = Some(auto.clone());

    draw_a_box(&mut d);
    d.frame(vec![]);
    assert!(!auto.exists(), "not yet due");
    // Not in the middle of a gesture.
    d.letter(egui::Key::R, "r");
    d.click([20.0, 20.0, 0.0]);
    assert!(d.app.tool.busy());
    d.time += AUTOSAVE_S;
    d.frame(vec![]);
    assert!(!auto.exists(), "the tool is busy");
    d.key(egui::Key::Escape);
    d.frame(vec![]);
    assert!(auto.exists(), "due, and the tool is idle");
    let opened = Project::open(&std::fs::read(&auto).unwrap()).unwrap();
    assert_eq!(opened.occupied().count(), 1);

    // No change since: no rewrite.
    std::fs::remove_file(&auto).unwrap();
    d.time += AUTOSAVE_S;
    d.frame(vec![]);
    assert!(!auto.exists());

    // An edit makes it stale again, then the "crash": a new session sees it.
    d.key_with(egui::Key::Z, COMMAND);
    d.time += AUTOSAVE_S;
    d.frame(vec![]);
    assert!(auto.exists());
    d.key_with(egui::Key::Z, COMMAND | egui::Modifiers::SHIFT);
    d.time += AUTOSAVE_S;
    d.frame(vec![]);

    let mut next = Driver::start().expect("a GPU, as before");
    next.app.autosave = Some(auto.clone());
    next.app.recovery = true;
    // While the question stands, the tools get nothing.
    next.letter(egui::Key::R, "r");
    assert!(next.app.tool.same_kind(&Tool::select(0.0)));
    next.app.recover();
    next.frame(vec![]);
    assert_eq!(next.counts(), (1, 12), "the box is back");
    assert!(next.app.unsaved && next.app.source.path.is_none());
    assert!(!next.app.modal());

    // Saving makes the autosave pointless.
    next.app.save_to(&dir.join("uji.hew"));
    assert_eq!(next.app.error, None);
    assert!(!auto.exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn leaving_with_changes_asks_first() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };

    // Nothing to lose: no question.
    d.app.leave(Leaving::New);
    assert_eq!(d.app.leaving, None);

    draw_a_box(&mut d);
    d.app.leave(Leaving::New);
    assert_eq!(d.app.leaving, Some(Leaving::New));
    d.frame(vec![]);
    assert_eq!(d.counts(), (1, 12), "nothing happens before the answer");
    d.app.leaving = None;

    // Closing the window: refused until answered.
    let close = |d: &mut Driver| {
        d.time += 0.05;
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
            time: Some(d.time),
            ..Default::default()
        };
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .events
            .push(egui::ViewportEvent::Close);
        let app = &mut d.app;
        let mut output = d.ctx.run_ui(input, |ui| app.shell(ui));
        output.textures_delta.clear();
        output.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .clone()
    };
    let cmds = close(&mut d);
    assert!(
        cmds.contains(&egui::ViewportCommand::CancelClose),
        "{cmds:?}"
    );
    assert_eq!(d.app.leaving, Some(Leaving::Close));
    assert!(!d.app.closing);

    // Discarding lets it go.
    d.app.unsaved = false;
    d.app.do_leave(Leaving::Close);
    assert!(d.app.closing);
    let cmds = close(&mut d);
    assert!(cmds.contains(&egui::ViewportCommand::Close), "{cmds:?}");
    assert!(!cmds.contains(&egui::ViewportCommand::CancelClose));
}

#[test]
fn a_parked_pointer_follows_the_tool() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    use egui::Key as K;

    d.letter(K::L, "l");
    d.click([0.0, 0.0, 0.0]);
    d.hover([3.0, 4.0, 0.0]);

    // Locking with the mouse still: the preview and a typed length both obey.
    d.key(K::ArrowRight);
    let (_, end) = d.app.tool.preview().lines[0];
    assert!(end.y.abs() < 1e-3, "the preview left the X axis: {end:?}");
    d.value("5");

    // The chain goes on with the mouse still.
    let (from, _) = d.app.tool.preview().lines[0];
    close([from.x, from.y, from.z], [5.0, 0.0, 0.0]);
    d.key(K::ArrowLeft);
    d.value("4");
    let (from, _) = d.app.tool.preview().lines[0];
    close([from.x, from.y, from.z], [5.0, 4.0, 0.0]);
}

/// A click that snapped sits a few pixels from what it picked. With the mouse
/// still, those pixels are no direction: a typed distance has nothing to follow.
#[test]
fn a_snapped_click_is_not_a_direction() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    use egui::Key as K;

    draw_a_box(&mut d);
    let size = d.extents();
    d.letter(K::M, "m");
    let beside_corner = d.pixel([2.0, 2.0, 3.0]) + egui::vec2(-3.0, 0.0);
    d.click_at(beside_corner);
    assert!(d.app.tool.busy(), "the corner picks the box");
    d.frame(vec![]);

    let refused = |d: &mut Driver| {
        d.frame(vec![egui::Event::Text("5".into())]);
        d.key(K::Enter);
        assert!(d.app.error.is_some(), "a distance with no direction");
        assert!(d.app.tool.busy());
        close(d.extents(), size);
        on_the_ground(d, [2.0, 2.0]);
    };
    refused(&mut d);
    d.key(K::ArrowRight);
    refused(&mut d);

    // Moving the mouse gives it one.
    d.hover([12.0, 2.0, 3.0]);
    d.value("5");
    assert!(!d.app.tool.busy());
    on_the_ground(&d, [7.0, 2.0]);
}

#[test]
fn a_click_drops_the_value_being_typed() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };

    d.letter(egui::Key::R, "r");
    d.click([1.0, 1.0, 0.0]);
    d.frame(vec![egui::Event::Text("5".into())]);
    d.click([4.0, 4.0, 0.0]);
    assert!(!d.app.tool.busy(), "the click finished the rectangle");
    assert!(d.app.typed.is_empty());
    d.letter(egui::Key::L, "l");
    assert!(d.app.tool.same_kind(&Tool::line(0.0)));
}

#[test]
fn holding_still_while_navigating_goes_idle() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };

    let m = egui::Modifiers::ALT;
    let from = d.pixel([0.0, 0.0, 0.0]);
    d.hold(m);
    d.frame(vec![egui::Event::PointerMoved(from)]);
    d.frame(vec![egui::Event::PointerButton {
        pos: from,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: m,
    }]);
    d.frame(vec![egui::Event::PointerMoved(
        from + egui::vec2(40.0, 0.0),
    )]);
    // Let what the move itself asked for finish, then nothing happens.
    for _ in 0..5 {
        d.frame(vec![]);
    }
    assert!(
        !d.ctx.has_requested_repaint(),
        "no input, yet the frame asked for another"
    );
}

#[test]
fn the_keyboard_stays_with_the_tools() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    use egui::Key as K;

    // Tab from the view does not park focus and take the tool keys.
    d.key(K::Tab);
    d.frame(vec![]);
    d.letter(K::L, "l");
    assert!(d.app.tool.same_kind(&Tool::line(0.0)));
}

/// With changes unsaved in a project never saved, Buka asks, Simpan opens the file
/// window, and only a real answer goes on to the next window.
#[test]
fn leaving_through_a_save_window() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    use egui::Key as K;
    let dir = std::env::temp_dir().join(format!("wangungis-pergi-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    draw_a_box(&mut d);
    d.key_with(K::O, COMMAND);
    assert_eq!(d.app.leaving, Some(Leaving::Open));
    assert_eq!(d.app.dialog, None, "not before the question");

    // What the Simpan button in that question does.
    d.app.save();
    assert_eq!(d.app.dialog, Some(Dialog::SaveAs));
    d.app.after_save = d.app.leaving.take();
    d.answer_dialog(None);
    assert_eq!(d.app.dialog, None);
    assert!(d.app.unsaved, "cancelled: nothing saved");
    assert_eq!(d.app.after_save, None, "and not going anywhere");

    d.key_with(K::O, COMMAND);
    d.app.save();
    d.app.after_save = d.app.leaving.take();
    d.answer_dialog(Some(&dir.join("pergi.hew")));
    assert!(!d.app.unsaved);
    assert_eq!(d.app.dialog, Some(Dialog::Open), "on to what was asked for");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Ekspor writes what the name asks for, and says why when it cannot.
#[test]
fn export_by_the_name_chosen() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    let dir = std::env::temp_dir().join(format!("wangungis-ekspor-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    d.app.export_to(&dir.join("kosong.obj"));
    assert!(d.app.error.is_some(), "nothing to export");
    assert!(!dir.join("kosong.obj").exists());

    draw_a_box(&mut d);
    for name in ["m.obj", "m.glb", "m.gltf", "m.stl", "m.3mf"] {
        d.app.dialog = Some(Dialog::Export);
        d.answer_dialog(Some(&dir.join(name)));
        assert_eq!(d.app.error, None, "{name}");
        assert!(
            std::fs::metadata(dir.join(name)).unwrap().len() > 0,
            "{name}"
        );
    }
    let saved_obj = std::fs::read(dir.join("m.obj")).unwrap();
    // The file window asked: replacing is fine.
    d.app.export_to(&dir.join("m.obj"));
    assert_eq!(d.app.error, None);
    assert_eq!(std::fs::read(dir.join("m.obj")).unwrap(), saved_obj);

    d.app.export_to(&dir.join("m.dxf"));
    assert!(d.app.error.is_some(), "not a format");
    assert!(!dir.join("m.dxf").exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The formats window lists every format the app really writes.
#[test]
fn the_formats_window_tells_the_truth() {
    for format in wangungis_io::FORMATS {
        assert!(
            FILE_FORMATS
                .iter()
                .any(|(_, f, _)| *f == format!(".{format}")),
            "{format}"
        );
    }
}

/// A LAS of a green roof at 12 m over a 40 x 40 m square, 1 m apart, far from zero.
fn write_roof_las(path: &std::path::Path, wkt_epsg: Option<u32>) {
    use las::{Builder, Color, Point, Writer, point::Format};
    let mut b = Builder::from((1, 4));
    b.point_format = Format::new(7).unwrap();
    for t in [
        &mut b.transforms.x,
        &mut b.transforms.y,
        &mut b.transforms.z,
    ] {
        t.scale = 0.001;
    }
    b.transforms.x.offset = 710_000.0;
    b.transforms.y.offset = 9_316_000.0;
    let mut header = b.into_header().unwrap();
    if let Some(epsg) = wkt_epsg {
        let wkt = format!("PROJCS[\"uji\",AUTHORITY[\"EPSG\",\"{epsg}\"]]");
        header.set_wkt_crs(wkt.into_bytes()).unwrap();
    }
    let mut w = Writer::from_path(path, header).unwrap();
    for y in 0..40 {
        for x in 0..40 {
            w.write_point(Point {
                x: 710_000.0 + f64::from(x),
                y: 9_316_000.0 + f64::from(y),
                z: 12.0,
                gps_time: Some(0.0),
                color: Some(Color::new(0, 60000, 0)),
                ..Default::default()
            })
            .unwrap();
        }
    }
    w.close().unwrap();
}

/// Frames until the import and the reads behind it have ended.
fn settle_clouds(d: &mut Driver) {
    let start = std::time::Instant::now();
    loop {
        d.frame(vec![]);
        d.frame(vec![]);
        if d.app.cloud_import.is_none()
            && d.app.cloud_queue.is_empty()
            && !d.app.scene.clouds.busy()
        {
            d.frame(vec![]);
            return;
        }
        assert!(start.elapsed().as_secs() < 60, "the import never ended");
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

/// The pointer out of the view and no marker left, as a marker covers pixels.
fn pointer_away(d: &mut Driver) {
    d.frame(vec![egui::Event::PointerGone]);
    d.key(egui::Key::Escape);
    d.frame(vec![]);
}

fn green_pixels(d: &Driver) -> usize {
    let (_, _, px) = d.pixels();
    // The roof's own color: the view's Y axis is green too.
    px.chunks(4).filter(|p| p[..3] == [0, 234, 0]).count()
}

/// The whole job through the app: import, see it, draw on it, slice it, save it,
/// open it again.
#[test]
fn a_las_from_import_to_reopen() {
    use egui::Key as K;
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    let dir = std::env::temp_dir().join(format!("wangungis-las-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    d.app.cache_dir = Some(dir.join("cache"));
    let source = dir.join("atap.las");
    write_roof_las(&source, Some(32748));

    // Through the menu's route. A brand-new project takes its origin from the cloud.
    d.app.dialog = Some(Dialog::ImportCloud);
    d.answer_dialog(Some(&source));
    settle_clouds(&mut d);
    assert_eq!(d.app.error, None);
    let origin = d.app.scene.project().origin.expect("taken from the cloud");
    assert_eq!(origin.epsg, 32748);
    assert_eq!(origin.origin, [710_020.0, 9_316_020.0, 0.0]);
    assert!(d.app.unsaved);
    assert!(d.app.notice.as_deref().is_some_and(|n| n.contains("1600")));
    assert_eq!(d.app.scene.clouds.clouds().len(), 1);
    let all = green_pixels(&d);
    assert!(all > 500, "the roof shows: {all}");
    d.dump("awan-1");

    // A footprint on the ground, under the roof: the roof would take the clicks.
    d.app.scene.clouds.snap = false;
    d.letter(K::R, "r");
    d.click([-5.0, -5.0, 0.0]);
    d.hover([-4.0, -4.0, 0.0]);
    d.value("5;4");
    d.app.scene.clouds.snap = true;
    // Pulled up to a point of the roof.
    d.letter(K::P, "p");
    d.click([-2.5, -3.0, 0.0]);
    d.hover([5.0, 5.0, 12.0]);
    let snap = d.app.tool.preview().snap;
    assert!(snap.is_some_and(|h| h.cloud.is_some()), "{snap:?}");
    assert_eq!(snap.map(|h| hit_label(&h)), Some("Titik awan"));
    d.click([5.0, 5.0, 12.0]);
    assert_eq!(d.app.error, None);
    let (_, hi) = d
        .app
        .scene
        .project()
        .tiles
        .values()
        .next()
        .map_or(([0.0; 3], [0.0; 3]), |doc| {
            let m = wangungis_render::TileMesh::from_document(doc).unwrap();
            let (a, b) = m.bounds().unwrap();
            (a.map(f64::from), b.map(f64::from))
        });
    assert!(
        (hi[2] - 12.0).abs() < 0.01,
        "the wall reaches the roof: {hi:?}"
    );

    // The point size shows at once.
    let small = green_pixels(&d);
    let cloud = d.app.scene.clouds.clouds()[0].id;
    d.app.scene.clouds.set_point_size(cloud, 6.0);
    d.frame(vec![]);
    assert!(green_pixels(&d) > small, "bigger points");
    d.app.scene.clouds.set_point_size(cloud, 2.0);
    d.frame(vec![]);

    // Two clicks make a box. What is outside hides, and comes back.
    d.letter(K::Space, " ");
    settle_clouds(&mut d);
    pointer_away(&mut d);
    let full = green_pixels(&d);
    d.letter(K::K, "k");
    d.app.scene.clouds.snap = false;
    d.click([-20.0, -20.0, 12.0]);
    assert!(d.app.tool.busy());
    d.click([0.0, 0.0, 12.0]);
    assert!(!d.app.tool.busy());
    assert_eq!(d.app.error, None);
    d.app.scene.clouds.snap = true;
    // Away from the tool, whose lines cover pixels.
    d.letter(K::Space, " ");
    pointer_away(&mut d);
    let cut = green_pixels(&d);
    d.dump("awan-2-potong");
    assert!(cut < full * 2 / 3, "{cut} of {full}");
    assert!(cut > 0);

    d.app.scene.clouds.slice_on = false;
    d.frame(vec![]);
    assert_eq!(green_pixels(&d), full, "all back");
    d.app.scene.clouds.slice_on = true;
    d.frame(vec![]);

    // Saved with its companion, then opened: the same cloud, the same box.
    let project = dir.join("kota.hew");
    d.app.save_to(&project);
    assert_eq!(d.app.error, None);
    assert!(wangungis_pointcloud::saved_clouds_path(&project).exists());
    let slice = d.app.scene.clouds.slice;
    d.app.new_project();
    d.frame(vec![]);
    assert!(d.app.scene.clouds.is_empty());
    d.app.open_path(&project);
    settle_clouds(&mut d);
    assert_eq!(d.app.error, None);
    assert_eq!(d.app.scene.clouds.clouds().len(), 1);
    assert_eq!(d.app.scene.clouds.slice, slice);
    assert!(d.app.scene.clouds.slice_on);
    assert!(!d.app.unsaved, "opening changes nothing");
    assert!(green_pixels(&d) > 0);

    // Lepas, save: no companion left behind.
    let id = d.app.scene.clouds.clouds()[0].id;
    d.app.remove_cloud(id);
    d.frame(vec![]);
    assert_eq!(green_pixels(&d), 0);
    assert_eq!(d.app.scene.clouds.slice, None, "the box went with it");
    d.app.save_to(&project);
    assert!(!wangungis_pointcloud::saved_clouds_path(&project).exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// What cannot be imported says so and leaves the app running: garbage, a missing
/// file, a CRS that is not the project's, a cancel. A project with work in it keeps
/// its coordinates.
#[test]
fn a_las_that_cannot_come_in() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    let dir = std::env::temp_dir().join(format!("wangungis-lasburuk-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    d.app.cache_dir = Some(dir.join("cache"));

    let junk = dir.join("sampah.las");
    std::fs::write(&junk, vec![7u8; 5000]).unwrap();
    d.app.import_cloud(junk.clone());
    settle_clouds(&mut d);
    assert!(d.app.error.is_some(), "garbage");
    assert!(d.app.scene.clouds.is_empty());

    d.app.error = None;
    d.app.import_cloud(dir.join("tidak-ada.las"));
    settle_clouds(&mut d);
    assert!(d.app.error.is_some(), "missing");

    // Work first: the model keeps its coordinates and the cloud comes to it.
    d.app.error = None;
    d.app.clouds_missing.clear();
    draw_a_box(&mut d);
    let saved = d.app.scene.project().save();
    let utm = dir.join("utm48.las");
    write_roof_las(&utm, Some(32748));
    d.app.import_cloud(utm);
    settle_clouds(&mut d);
    assert_eq!(d.app.error, None);
    assert_eq!(d.app.scene.project().origin, None);
    assert_eq!(
        d.app.scene.project().save(),
        saved,
        "the model is untouched"
    );
    assert_eq!(d.app.scene.clouds.frame, [710_020.0, 9_316_020.0, 0.0]);
    assert!(d.app.scene.can_undo());
    assert!(
        d.app
            .notice
            .as_deref()
            .is_some_and(|n| n.contains("origin"))
    );

    // Another zone next to it: refused.
    let other = dir.join("utm49.las");
    write_roof_las(&other, Some(32749));
    d.app.import_cloud(other);
    settle_clouds(&mut d);
    assert!(d.app.error.as_deref().is_some_and(|e| e.contains("32749")));
    assert_eq!(d.app.scene.clouds.clouds().len(), 1);

    // Moving on from the project stops what was under way.
    d.app.error = None;
    let late = dir.join("akhir.las");
    write_roof_las(&late, None);
    d.app.import_cloud(late);
    d.app.new_project();
    settle_clouds(&mut d);
    assert!(d.app.scene.clouds.is_empty());
    assert_eq!(d.app.error, None);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Saved means all of it: a companion file that cannot be written keeps the project
/// marked as changed, so that closing still asks.
#[test]
fn a_companion_that_cannot_be_written_is_not_saved() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    let dir = std::env::temp_dir().join(format!("wangungis-pendamping-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    d.app.cache_dir = Some(dir.join("cache"));
    let source = dir.join("atap.las");
    write_roof_las(&source, Some(32748));
    d.app.import_cloud(source);
    settle_clouds(&mut d);
    let project = dir.join("kota.hew");
    // A folder in the companion's place.
    std::fs::create_dir(wangungis_pointcloud::saved_clouds_path(&project)).unwrap();
    d.app.save_to(&project);
    assert!(
        d.app
            .error
            .as_deref()
            .is_some_and(|e| e.contains("point cloud"))
    );
    assert!(d.app.unsaved);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A crash with a cloud in the project: the autosave brings the cloud back too.
#[test]
fn recovery_brings_the_clouds_back() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    let dir = std::env::temp_dir().join(format!("wangungis-pulih-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    d.app.cache_dir = Some(dir.join("cache"));
    d.app.autosave = Some(dir.join("autosave.bin"));
    let source = dir.join("atap.las");
    write_roof_las(&source, Some(32748));
    d.app.import_cloud(source);
    settle_clouds(&mut d);
    let frame = d.app.scene.clouds.frame;
    let id = d.app.scene.clouds.clouds()[0].id;
    d.app.scene.clouds.set_point_size(id, 5.0);
    d.app.autosave_stale = true;
    d.app.autosaved_at = -1e9;
    d.frame(vec![]);
    assert!(dir.join("autosave.bin").exists());

    let Some(mut next) = Driver::start() else {
        return;
    };
    next.app.cache_dir = Some(dir.join("cache"));
    next.app.autosave = Some(dir.join("autosave.bin"));
    next.app.recover();
    settle_clouds(&mut next);
    assert_eq!(next.app.error, None);
    assert_eq!(next.app.scene.clouds.clouds().len(), 1);
    assert_eq!(next.app.scene.clouds.frame, frame);
    assert_eq!(next.app.scene.clouds.clouds()[0].point_size, 5.0);
    assert!(next.app.unsaved);

    next.app.drop_autosave();
    assert!(!wangungis_pointcloud::saved_clouds_path(&dir.join("autosave.bin")).exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The next cloud after the last one is removed starts clean: no old box, no old frame.
#[test]
fn a_new_cloud_after_the_last_one_starts_clean() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    let dir = std::env::temp_dir().join(format!("wangungis-baru-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    d.app.cache_dir = Some(dir.join("cache"));
    draw_a_box(&mut d);
    let first = dir.join("a.las");
    write_roof_las(&first, None);
    d.app.import_cloud(first);
    settle_clouds(&mut d);
    assert_eq!(d.app.scene.clouds.frame, [710_020.0, 9_316_020.0, 0.0]);
    d.app.scene.clouds.slice = Some(wangungis_pointcloud::Slice {
        min: [0.0; 3],
        max: [1.0; 3],
    });
    d.app.scene.clouds.slice_on = true;
    let id = d.app.scene.clouds.clouds()[0].id;
    d.app.remove_cloud(id);
    assert_eq!(d.app.scene.clouds.frame, [0.0; 3]);
    assert_eq!(d.app.scene.clouds.active_slice(), None);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Using the box tool without changing the box changes nothing to save.
#[test]
fn the_box_tool_alone_changes_nothing() {
    use egui::Key as K;
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    d.letter(K::K, "k");
    d.key(K::Escape);
    assert!(!d.app.unsaved);
}

/// Moving a cloud through the app: J, a point of it, a corner of the model. It is
/// saved and undone like any edit, keeps its size and place on reopening, and
/// **Kembalikan posisi** brings it home.
#[test]
fn a_cloud_moved_and_sized_through_the_app() {
    use egui::Key as K;
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    let dir = std::env::temp_dir().join(format!("wangungis-geser-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    d.app.cache_dir = Some(dir.join("cache"));
    let source = dir.join("atap.las");
    write_roof_las(&source, Some(32748));
    d.app.import_cloud(source);
    settle_clouds(&mut d);
    // A box beside the roof, out of its way.
    d.app.scene.clouds.snap = false;
    d.letter(K::R, "r");
    d.click([30.0, 30.0, 0.0]);
    d.hover([31.0, 31.0, 0.0]);
    d.value("5;4");
    d.letter(K::P, "p");
    d.click([32.5, 32.0, 0.0]);
    d.value("3");
    d.app.scene.clouds.snap = true;
    d.frame(vec![]);
    let project = dir.join("kota.hew");
    d.app.save_to(&project);
    assert!(!d.app.unsaved);
    let id = d.app.scene.clouds.clouds()[0].id;

    d.letter(K::J, "j");
    assert!(d.app.tool.same_kind(&Tool::cloud_move(0.0)));
    d.click([0.0, 0.0, 12.0]);
    assert!(d.app.tool.busy(), "{:?}", d.app.error);
    assert_eq!(d.app.scene.clouds.moving, Some(id));
    d.hover([35.0, 30.0, 3.0]);
    assert!(!d.app.unsaved, "only on its way");
    d.click([35.0, 30.0, 3.0]);
    assert!(!d.app.tool.busy());
    assert_eq!(d.app.error, None);
    assert!(d.app.unsaved);
    let moved = d.app.scene.clouds.offset_of(id).unwrap();
    assert_ne!(moved, [0.0; 3]);
    assert!((moved[2] - (3.0 - 12.0)).abs() < 0.01, "{moved:?}");
    d.frame(vec![]);
    assert!(green_pixels(&d) > 0, "still in view");

    d.key_with(K::Z, COMMAND);
    assert_eq!(d.app.scene.clouds.offset_of(id), Some([0.0; 3]));
    d.key_with(K::Z, COMMAND | egui::Modifiers::SHIFT);
    assert_eq!(d.app.scene.clouds.offset_of(id), Some(moved));

    // Switching tools in the middle of a move, or undoing, puts it back first.
    d.letter(K::J, "j");
    d.click([5.0, 5.0, 12.0]);
    d.hover([20.0, 20.0, 3.0]);
    assert!(d.app.tool.busy());
    d.letter(K::Space, " ");
    assert_eq!(d.app.scene.clouds.offset_of(id), Some(moved));
    assert_eq!(d.app.scene.clouds.moving, None);

    d.app.scene.clouds.set_point_size(id, 4.0);
    d.app.save_to(&project);
    d.app.new_project();
    d.frame(vec![]);
    d.app.open_path(&project);
    settle_clouds(&mut d);
    assert_eq!(d.app.error, None);
    assert!(!d.app.unsaved);
    let c = &d.app.scene.clouds.clouds()[0];
    assert_eq!((c.point_size, c.offset), (4.0, moved));

    let id = c.id;
    d.app.reset_cloud_offset(id);
    assert_eq!(d.app.scene.clouds.offset_of(id), Some([0.0; 3]));
    assert!(d.app.unsaved);
    d.key_with(K::Z, COMMAND);
    assert_eq!(d.app.scene.clouds.offset_of(id), Some(moved), "one undo");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Saving in the middle of a move saves where the cloud was, not where the pointer
/// had it, and the gesture ends there.
#[test]
fn saving_mid_move_keeps_the_cloud_where_it_was() {
    use egui::Key as K;
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    let dir = std::env::temp_dir().join(format!("wangungis-simpangeser-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    d.app.cache_dir = Some(dir.join("cache"));
    let source = dir.join("atap.las");
    write_roof_las(&source, Some(32748));
    d.app.import_cloud(source);
    settle_clouds(&mut d);
    let id = d.app.scene.clouds.clouds()[0].id;
    let project = dir.join("kota.hew");

    d.letter(K::J, "j");
    d.click([0.0, 0.0, 12.0]);
    d.hover([30.0, 30.0, 0.0]);
    assert_ne!(
        d.app.scene.clouds.offset_of(id),
        Some([0.0; 3]),
        "on its way"
    );
    d.app.save_to(&project);
    assert!(!d.app.tool.busy());
    assert_eq!(d.app.scene.clouds.offset_of(id), Some([0.0; 3]));
    assert_eq!(d.app.scene.clouds.moving, None);
    let file = wangungis_pointcloud::saved_clouds_path(&project);
    let saved: SavedClouds = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(saved.clouds[0].offset, [0.0; 3]);

    // A companion file edited to somewhere no move could reach: ignored, with a word.
    let mut far = saved;
    far.clouds[0].offset = [1e308, 0.0, 0.0];
    std::fs::write(&file, serde_json::to_vec(&far).unwrap()).unwrap();
    d.app.open_path(&project);
    settle_clouds(&mut d);
    assert_eq!(d.app.scene.clouds.clouds()[0].offset, [0.0; 3]);
    assert!(
        d.app
            .notice
            .as_deref()
            .is_some_and(|n| n.contains("batas proyek"))
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Two clouds, two sizes, through the frame: one growing leaves the other alone.
#[test]
fn each_cloud_sized_on_its_own_in_the_app() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    let dir = std::env::temp_dir().join(format!("wangungis-ukuran-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    d.app.cache_dir = Some(dir.join("cache"));
    for name in ["a.las", "b.las"] {
        let source = dir.join(name);
        write_roof_las(&source, Some(32748));
        d.app.import_cloud(source);
    }
    settle_clouds(&mut d);
    let (a, b) = {
        let c = d.app.scene.clouds.clouds();
        (c[0].id, c[1].id)
    };
    d.app.scene.clouds.set_point_size(a, 6.0);
    d.frame(vec![]);
    let sizes: std::collections::BTreeMap<u32, f32> = d
        .app
        .scene
        .clouds
        .draws()
        .iter()
        .map(|x| (x.id, x.point_size))
        .collect();
    assert_eq!(sizes[&a], 6.0);
    assert_eq!(sizes[&b], 2.0);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A project whose companion file names a LAS that went away still opens, and
/// saving does not forget it.
#[test]
fn a_project_whose_las_went_away() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    let dir = std::env::temp_dir().join(format!("wangungis-lashilang-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    d.app.cache_dir = Some(dir.join("cache"));
    let source = dir.join("atap.las");
    write_roof_las(&source, Some(32748));
    d.app.import_cloud(source.clone());
    settle_clouds(&mut d);
    let project = dir.join("kota.hew");
    d.app.save_to(&project);
    assert_eq!(d.app.error, None);

    std::fs::remove_file(&source).unwrap();
    d.app.new_project();
    d.app.open_path(&project);
    settle_clouds(&mut d);
    assert!(d.app.error.is_some(), "says so");
    assert!(d.app.scene.clouds.is_empty());
    assert_eq!(d.app.clouds_missing.len(), 1);

    d.app.error = None;
    d.app.save_to(&project);
    let companion = std::fs::read_to_string(wangungis_pointcloud::saved_clouds_path(&project));
    assert!(companion.unwrap().contains("atap.las"), "not forgotten");

    // A damaged companion file: the project opens anyway.
    std::fs::write(wangungis_pointcloud::saved_clouds_path(&project), "{ rusak").unwrap();
    d.app.open_path(&project);
    settle_clouds(&mut d);
    assert!(d.app.error.is_some());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The real thing, when a sample is at hand: `WANGUNGIS_LAS_SAMPLE=<file>`. Times the
/// import, the first picture, frames while orbiting and picks, and prints memory.
#[test]
#[ignore]
fn real_las_numbers() {
    use std::time::{Duration, Instant};
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    let Some(sample) = std::env::var_os("WANGUNGIS_LAS_SAMPLE") else {
        return;
    };
    let dir = std::env::temp_dir().join(format!("wangungis-lasnyata-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    d.app.cache_dir = Some(dir.clone());
    let rss = || {
        let out = std::process::Command::new("ps")
            .args(["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse::<u64>()
            .unwrap_or(0)
            / 1024
    };
    println!("rss before: {} MB", rss());

    let t = Instant::now();
    d.app.import_cloud(sample.clone().into());
    while d.app.cloud_import.is_some() {
        d.frame(vec![]);
        std::thread::sleep(Duration::from_millis(5));
    }
    println!(
        "import: {:.2} s, rss {} MB",
        t.elapsed().as_secs_f64(),
        rss()
    );
    assert_eq!(d.app.error, None);
    d.frame(vec![]);
    d.frame(vec![]);

    let t = Instant::now();
    settle_clouds(&mut d);
    println!(
        "first full view settled: {:.2} s",
        t.elapsed().as_secs_f64()
    );
    let drawn = |d: &Driver| {
        d.app
            .scene
            .clouds
            .draws()
            .iter()
            .map(|x| u64::from(x.count))
            .sum::<u64>()
    };
    let held = |d: &Driver| {
        d.app
            .scene
            .clouds
            .clouds()
            .iter()
            .map(|c| c.held_points())
            .sum::<u64>()
    };
    println!(
        "full view: {} drawn, {} held, rss {} MB",
        drawn(&d),
        held(&d),
        rss()
    );
    d.dump("real-1-penuh");

    // Reopening from the cache.
    let source: std::path::PathBuf = sample.into();
    let saved = d.app.scene.clouds.clouds().len();
    let t = Instant::now();
    d.app.new_project();
    d.app.import_cloud(source);
    settle_clouds(&mut d);
    println!("from the cache: {:.2} s", t.elapsed().as_secs_f64());
    assert_eq!(d.app.scene.clouds.clouds().len(), saved);

    // Orbit: frame times with the camera moving, read back so that the GPU's work counts.
    let mut times = Vec::new();
    for i in 0..60 {
        d.app.orbit.yaw += 0.03;
        d.app.camera_moved();
        let t = Instant::now();
        d.frame(vec![]);
        d.pixels();
        times.push(t.elapsed().as_secs_f64() * 1000.0);
        let _ = i;
    }
    times.sort_by(f64::total_cmp);
    println!(
        "orbit full view: median {:.1} ms, worst {:.1} ms, {} drawn",
        times[30],
        times[59],
        drawn(&d)
    );

    // Zoom in on one spot: more points come in, within the budget.
    d.app.orbit.distance /= 12.0;
    d.app.camera_moved();
    settle_clouds(&mut d);
    println!(
        "zoomed in: {} drawn, {} held, rss {} MB",
        drawn(&d),
        held(&d),
        rss()
    );
    d.dump("real-2-dekat");

    let cloud_snap = |d: &mut Driver, count: u32| {
        let mut worst = 0.0f64;
        let mut hits = 0;
        let mut total = 0.0;
        for i in 0..count {
            let f = f64::from(i) / f64::from(count) * 6.0;
            let ray = inference::PickRay {
                origin: Point3::new(
                    d.app.orbit.eye()[0],
                    d.app.orbit.eye()[1],
                    d.app.orbit.eye()[2],
                ),
                direction: {
                    let (_, dir) = d.app.orbit.ray(
                        400.0 + 50.0 * f.sin() as f32,
                        300.0 + 50.0 * f.cos() as f32,
                        800.0,
                        600.0,
                    );
                    Vec3::new(dir[0], dir[1], dir[2])
                },
            };
            let t = Instant::now();
            let hit = d.app.scene.clouds.pick(
                [ray.origin.x, ray.origin.y, ray.origin.z],
                [ray.direction.x, ray.direction.y, ray.direction.z],
                0.008,
            );
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            worst = worst.max(ms);
            total += ms;
            hits += usize::from(hit.is_some());
        }
        (total / f64::from(count), worst, hits)
    };
    let (mean, worst, hits) = cloud_snap(&mut d, 60);
    println!("pick: mean {mean:.2} ms, worst {worst:.2} ms, {hits} of 60 hit");

    // One building's worth, by the box.
    let (lo, hi) = d.app.scene.clouds.bounds().unwrap();
    d.app.scene.clouds.slice = Some(wangungis_pointcloud::Slice {
        min: [-50.0, -50.0, lo[2]],
        max: [50.0, 50.0, hi[2]],
    });
    d.app.scene.clouds.slice_on = true;
    let t = Instant::now();
    d.frame(vec![]);
    println!(
        "slice on, one frame: {:.1} ms",
        t.elapsed().as_secs_f64() * 1000.0
    );
    settle_clouds(&mut d);
    println!("sliced: {} drawn, {} held", drawn(&d), held(&d));
    d.dump("real-3-potong");
    let t = Instant::now();
    d.app.scene.clouds.slice_on = false;
    d.frame(vec![]);
    println!(
        "slice off, one frame: {:.1} ms",
        t.elapsed().as_secs_f64() * 1000.0
    );
    println!("rss end: {} MB", rss());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Uji asap step 18: a guide shows, in its own color, and says so.
#[test]
fn a_guide_is_seen_and_announced() {
    use egui::Key as K;
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    let guide_pixels = |d: &Driver| {
        let want = GUIDE_COLOR.map(|c| (c * 255.0) as i32);
        let (_, _, px) = d.pixels();
        px.chunks(4)
            .filter(|p| (0..3).all(|k| (i32::from(p[k]) - want[k]).abs() < 50))
            .count()
    };
    draw_a_box(&mut d);
    assert_eq!(guide_pixels(&d), 0);
    d.letter(K::T, "t");
    assert!(d.app.tool.hint().contains("tepi"), "{}", d.app.tool.hint());
    d.click([4.5, 2.0, 0.0]);
    d.hover([9.0, 1.0, 0.0]);
    let m = d.app.tool.measure().unwrap();
    assert!((m - 1.0).abs() < 0.05, "measured square to the edge: {m}");
    d.value("3");
    d.frame(vec![]);
    d.dump("18-guide");
    assert_eq!(d.app.scene.guide_lines().len(), 1);
    assert!(
        d.app
            .notice
            .as_deref()
            .is_some_and(|n| n.contains("Panduan")),
        "{:?}",
        d.app.notice
    );
    assert!(guide_pixels(&d) > 200, "{}", guide_pixels(&d));
}

/// Uji asap step 19: a line straight out of a wall, by the arrow keys and by an
/// angle, and Follow Me along it.
#[test]
fn a_rod_out_of_a_wall() {
    use egui::Key as K;
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    draw_a_box(&mut d);
    // Close enough that a click on the wall stays clear of its edges.
    d.app.fit_pending = true;
    d.frame(vec![]);
    d.frame(vec![]);
    d.letter(K::R, "r");
    d.click([4.0, 2.0, 1.0]);
    d.hover([4.5, 2.0, 1.5]);
    d.value("1;1");
    d.letter(K::L, "l");
    d.click([4.5, 2.0, 1.5]);
    d.key(K::ArrowDown);
    assert!(d.app.tool.hint().contains("tegak lurus"));
    d.hover([4.5, 1.0, 1.5]);
    d.value("4");
    d.key(K::Escape);
    d.dump("19-line");
    d.app.switch_tool(Tool::follow_me);
    d.click([4.8, 2.0, 1.8]);
    assert_eq!(d.app.tool.preview().lines.len(), 4, "the square is picked");
    d.click([4.5, -1.0, 1.5]);
    assert_eq!(d.app.error, None);
    d.dump("19-rod");
    let (lo, _) = d.app.bounds().unwrap();
    close(lo, [2.0, -2.0, 0.0]);

    // Up the Z axis, then 30° off X.
    d.app.undo();
    d.letter(K::L, "l");
    d.click([9.0, 9.0, 0.0]);
    d.key(K::ArrowUp);
    d.hover([9.0, 9.0, 1.0]);
    d.value("2");
    d.key(K::Escape);
    d.click([10.0, 9.0, 0.0]);
    d.hover([12.0, 10.0, 0.0]);
    d.value("30d");
    assert!(d.app.tool.hint().contains("30°"), "{}", d.app.tool.hint());
    d.hover([12.0, 10.0, 0.0]);
    d.value("2");
    assert_eq!(d.app.error, None);
}

/// Uji asap step 20 without a big file: a long rectangle, and one past the border
/// of its tile.
#[test]
fn a_second_tile_from_an_empty_project() {
    use egui::Key as K;
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    d.letter(K::R, "r");
    d.click([1.0, 1.0, 0.0]);
    d.hover([2.0, 2.0, 0.0]);
    d.value("800;10");
    d.app.fit_pending = true;
    d.frame(vec![]);
    d.frame(vec![]);
    assert_eq!(d.app.tile_count(), 1);
    d.click([790.0, 40.0, 0.0]);
    d.hover([795.0, 45.0, 0.0]);
    d.value("10;10");
    d.frame(vec![]);
    d.dump("20-two-tiles");
    assert_eq!(d.app.tile_count(), 2);
    // A line from the second snaps to a corner of the first, in the other tile.
    d.letter(K::L, "l");
    d.click([790.0, 40.0, 0.0]);
    d.hover([801.0, 11.0, 0.0]);
    let hit = d.app.tool.preview().snap.expect("the corner next door");
    assert_eq!(hit.snap.kind, SnapKind::Endpoint);
}

#[test]
fn the_split_label_counts_what_the_status_bar_counts() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };

    // An edit undone in a new tile leaves an empty document behind.
    d.app.new_project();
    let far = TileKey { i: 3, j: 0 };
    let ground =
        kernel::Plane::from_point_normal(Point3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0))
            .unwrap();
    let c = far.center();
    let (a, b) = (
        Point3::new(c[0], c[1], 0.0),
        Point3::new(c[0] + 5.0, c[1], 0.0),
    );
    d.app.scene.draw(Some(far), ground, &[(a, b)]).unwrap();
    d.frame(vec![]);
    d.app.undo();
    d.frame(vec![]);
    assert_eq!(d.app.stats.len(), 2, "the renderer still knows the tile");
    assert_eq!(d.app.tile_count(), 1);
}

#[test]
fn select_delete_and_group_through_the_app() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    use egui::Key as K;
    draw_a_box(&mut d);
    // A second box, by copying.
    d.letter(K::M, "m");
    d.click([4.5, 4.0, 3.0]);
    d.letter(K::C, "c");
    d.key(K::ArrowRight);
    d.hover([10.0, 4.0, 3.0]);
    d.value("7");
    assert_eq!(d.counts().0, 2);

    // Space, click, Shift + click: both picked.
    d.key(K::Space);
    d.click([4.5, 4.0, 3.0]);
    d.hold(egui::Modifiers::SHIFT);
    d.click([11.5, 4.0, 3.0]);
    d.hold(egui::Modifiers::NONE);
    assert_eq!(d.app.tool.picked_nodes().unwrap().1.len(), 2);

    d.app.on_picked(|s, t, n| s.group(t, n));
    assert_eq!(d.app.error, None);
    assert_eq!(d.counts().0, 2, "two solids in one group");

    // The group goes with one Delete, and comes back.
    d.click([4.5, 4.0, 3.0]);
    d.key(K::Delete);
    assert_eq!(d.counts(), (0, 0));
    d.key_with(K::Z, COMMAND);
    assert_eq!(d.counts().0, 2);

    // With a value typed, Backspace edits it instead of deleting.
    d.click([4.5, 4.0, 3.0]);
    d.frame(vec![egui::Event::Text("5".into())]);
    d.key(K::Backspace);
    assert!(d.app.typed.is_empty());
    assert_eq!(d.counts().0, 2);
}

#[test]
fn tag_visibility_through_the_app() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    use egui::Key as K;
    draw_a_box(&mut d);
    d.app.fit_pending = true;
    d.frame(vec![]);
    d.frame(vec![]);
    let shown = d.model_pixels();
    assert!(shown > 1000);

    d.key(K::Space);
    d.click([4.5, 4.0, 3.0]);
    d.app.tag_picked("Bangunan");
    assert_eq!(d.app.error, None);
    assert_eq!(d.counts(), (1, 12));
    assert_eq!(d.app.tag_rows, vec![("Bangunan".to_string(), false)]);

    d.app.set_tag_visible("Bangunan", false);
    d.frame(vec![]);
    assert_eq!(d.counts(), (0, 0));
    assert_eq!(d.app.tag_rows, vec![("Bangunan".to_string(), true)]);
    assert!(d.model_pixels() < shown / 10);

    d.app.set_tag_visible("Bangunan", true);
    d.frame(vec![]);
    assert_eq!(d.counts(), (1, 12));
    assert!(d.model_pixels() > 1000);
    d.app.undo();
    let (&tile, doc) = d.app.scene.project().tiles.iter().next().unwrap();
    let node = doc.top_level_nodes()[0];
    assert!(
        d.app.scene.project().tiles[&tile]
            .node_tags(node)
            .is_empty()
    );
    assert_eq!(d.counts(), (1, 12));
}

/// The Outliner lists, picks, opens, hides and names, on the same app state.
#[test]
fn outliner_through_the_app() {
    use egui::Key as K;
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    draw_a_box(&mut d);
    d.app.fit_pending = true;
    d.frame(vec![]);
    d.frame(vec![]);
    assert_eq!(d.app.outline_rows.len(), 1);
    assert_eq!(d.app.outline_rows[0].kind, "Objek");

    d.key(K::Space);
    d.click([4.5, 4.0, 3.0]);
    d.app.on_picked(|s, t, n| s.group(t, n));
    assert_eq!(d.app.outline_rows.len(), 1, "a collapsed group");
    let group = d.app.outline_rows[0].clone();
    assert_eq!((group.kind, group.has_children), ("Grup", true));

    d.app.outline_toggle(group.tile, group.node);
    assert_eq!(d.app.outline_rows.len(), 2);
    let member = d.app.outline_rows[1].clone();
    assert_eq!((member.depth, member.top), (1, group.node));

    // A line inside picks what it sits in, and a double click opens that.
    d.app.outline_pick(member.tile, member.top);
    assert_eq!(
        d.app.tool.picked_nodes(),
        Some((group.tile, vec![group.node]))
    );
    d.app.rename_picked("Rumah");
    assert_eq!(d.app.error, None);
    assert_eq!(d.app.outline_rows[0].name, "Rumah");

    d.app.outline_open(group.tile, group.node);
    assert_eq!(d.app.error, None);
    // Hew reads no name from a group while it is open, so the label is generic.
    assert_eq!(d.app.session_label().as_deref(), Some("Mengedit: Grup"));
    d.app.leave_session();
    assert_eq!(d.app.session_label(), None);

    // Hide and show, in the view too.
    d.frame(vec![]);
    let shown = d.model_pixels();
    assert!(shown > 1000);
    d.app.set_node_visible(group.tile, group.node, false);
    d.frame(vec![]);
    assert!(d.model_pixels() < shown / 10);
    assert!(d.app.outline_rows[0].hidden);
    d.app.set_node_visible(group.tile, group.node, true);
    d.frame(vec![]);
    assert!(d.model_pixels() > 1000);

    // Undo the close, the open, then the name.
    for _ in 0..3 {
        d.app.undo();
    }
    assert_eq!(d.app.outline_rows[0].name, "Grup");
}
