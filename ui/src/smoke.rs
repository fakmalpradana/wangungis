//! The smoke checklist (`docs/user/uji-asap.md`) played against the real `App`:
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
            app: App::with_render_state(render_state()?, None),
            time: 0.0,
        };
        d.frame(vec![]);
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
    d.click([1.0, 1.0, 0.0]);
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
    d.app.path = file.to_string_lossy().into_owned();
    d.key_with(K::S, COMMAND);
    assert_eq!(d.app.error, None);
    assert!(!d.app.unsaved);
    let saved = std::fs::read(&file).unwrap();
    let before = (d.counts(), d.extents());

    d.app.new_project();
    d.frame(vec![]);
    assert_eq!(d.counts(), (0, 0));
    assert!(d.app.path.is_empty());
    assert!(!d.app.scene.can_undo());

    d.app.path = file.to_string_lossy().into_owned();
    d.app.open();
    d.frame(vec![]);
    assert_eq!(d.app.error, None);
    assert_eq!((d.counts(), d.extents()), before);
    d.dump("9-reopened");

    // Saving again changes nothing on disk.
    d.key_with(K::S, COMMAND);
    assert_eq!(std::fs::read(&file).unwrap(), saved);

    // A different existing file is never replaced.
    let other = dir.join("lain.hew");
    std::fs::write(&other, b"milik orang").unwrap();
    d.app.path = other.to_string_lossy().into_owned();
    d.key_with(K::S, COMMAND);
    assert!(d.app.error.is_some());
    assert_eq!(std::fs::read(&other).unwrap(), b"milik orang");

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
fn the_keyboard_and_the_path_box() {
    let _gpu = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut d) = Driver::start() else { return };
    use egui::Key as K;

    // Tab from the view does not walk into the box and take the tool keys.
    d.key(K::Tab);
    d.frame(vec![]);
    d.letter(K::L, "l");
    assert!(d.app.tool.same_kind(&Tool::line(0.0)));
    assert!(d.app.path.is_empty());

    // Esc that leaves the box does not also cancel the gesture.
    d.letter(K::R, "r");
    d.click([1.0, 1.0, 0.0]);
    assert!(d.app.tool.busy());
    let box_at = d.app.path_rect.center();
    d.click_at(box_at);
    d.frame(vec![egui::Event::Text("abc".into())]);
    assert_eq!(d.app.path, "abc", "the box has the keyboard");
    d.key(K::Escape);
    assert!(d.app.tool.busy(), "the rectangle survives");
    d.key(K::Escape);
    assert!(!d.app.tool.busy(), "the next Esc is for the tool");

    // Cmd+S works from the box.
    let dir = std::env::temp_dir().join(format!("wangungis-box-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("dari-kotak.hew");
    d.click_at(box_at);
    d.frame(vec![egui::Event::Key {
        key: K::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: COMMAND,
    }]);
    d.frame(vec![egui::Event::Text(file.to_string_lossy().into())]);
    assert_eq!(d.app.path, file.to_string_lossy());
    d.key_with(K::S, COMMAND);
    assert_eq!(d.app.error, None);
    assert!(file.exists(), "saved while the box had focus");
    std::fs::remove_dir_all(&dir).unwrap();
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
