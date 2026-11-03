//! Orbit camera controls (#31 orbit/pan/zoom, #32 zoom extents), pure `f64`.

use wangungis_render::Camera;

pub const FOVY: f32 = 0.8;

/// Camera orbiting `target` at `distance`, z up. Angles in radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orbit {
    pub target: [f64; 3],
    pub distance: f64,
    pub yaw: f64,
    pub pitch: f64,
    /// Bounding sphere (center, radius) of the model, set by `fit`; keeps the
    /// far plane behind the whole model when zoomed in close.
    pub scene: Option<([f64; 3], f64)>,
}

impl Default for Orbit {
    fn default() -> Self {
        Orbit {
            target: [0.0; 3],
            distance: 50.0,
            yaw: -2.2,
            pitch: 0.6,
            scene: None,
        }
    }
}

/// Below asin(0.999) ≈ 1.526, where `Camera::view_proj_at_eye` switches its up
/// hint: past it the view would roll and `pan` would move along wrong axes.
const PITCH_LIMIT: f64 = 1.5;
const RAD_PER_PX: f64 = 0.008;

impl Orbit {
    /// Unit vector from the target towards the eye.
    fn back(&self) -> [f64; 3] {
        let (sp, cp) = self.pitch.sin_cos();
        let (sy, cy) = self.yaw.sin_cos();
        [cp * cy, cp * sy, sp]
    }

    pub fn eye(&self) -> [f64; 3] {
        let b = self.back();
        std::array::from_fn(|k| self.target[k] + self.distance * b[k])
    }

    /// Drag by (dx, dy) pixels rotates around the target.
    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx as f64 * RAD_PER_PX;
        self.pitch = (self.pitch + dy as f64 * RAD_PER_PX).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    /// Drag by (dx, dy) pixels moves the target in the view plane so the point
    /// under the cursor follows it at the target's depth.
    pub fn pan(&mut self, dx: f32, dy: f32, viewport_height_px: f32) {
        let m_per_px =
            2.0 * self.distance * (FOVY as f64 / 2.0).tan() / viewport_height_px.max(1.0) as f64;
        let (sy, cy) = self.yaw.sin_cos();
        let right = [-sy, cy, 0.0];
        let b = self.back();
        // up = back × right, perpendicular to both.
        let up = [
            b[1] * right[2] - b[2] * right[1],
            b[2] * right[0] - b[0] * right[2],
            b[0] * right[1] - b[1] * right[0],
        ];
        for k in 0..3 {
            self.target[k] += (-right[k] * dx as f64 + up[k] * dy as f64) * m_per_px;
        }
    }

    /// Positive `steps` (scroll up) moves closer.
    pub fn zoom(&mut self, steps: f32) {
        self.distance = (self.distance * 0.9f64.powf(steps as f64)).clamp(0.1, 1e6);
    }

    /// Zoom extents: the bounding sphere of `lo..hi` fits the narrower of the
    /// vertical and horizontal fields of view.
    pub fn fit(&mut self, lo: [f64; 3], hi: [f64; 3], aspect: f32) {
        self.target = std::array::from_fn(|k| (lo[k] + hi[k]) / 2.0);
        let r = (0..3).map(|k| (hi[k] - lo[k]).powi(2)).sum::<f64>().sqrt() / 2.0;
        let half = ((FOVY as f64 / 2.0).tan() * (aspect as f64).min(1.0)).atan();
        self.distance = (r.max(1.0) / half.sin() * 1.1).clamp(0.1, 1e6);
        self.scene = Some((self.target, r));
    }

    /// The ray through a viewport point: `(x, y)` in `w` x `h` units, origin
    /// top-left. Same basis as `Camera::view_proj_at_eye`. Returns the eye and
    /// the direction through the point.
    pub fn ray(&self, x: f32, y: f32, w: f32, h: f32) -> ([f64; 3], [f64; 3]) {
        let (w, h) = (w.max(1.0) as f64, h.max(1.0) as f64);
        let b = self.back();
        let f = b.map(|v| -v);
        // f x z, unit: the pitch limit keeps f away from z.
        let len = f[0].hypot(f[1]);
        let right = [f[1] / len, -f[0] / len, 0.0];
        // right x f.
        let up = [
            right[1] * f[2] - right[2] * f[1],
            right[2] * f[0] - right[0] * f[2],
            right[0] * f[1] - right[1] * f[0],
        ];
        let t = (FOVY as f64 / 2.0).tan();
        let sx = (2.0 * x as f64 / w - 1.0) * t * w / h;
        let sy = (1.0 - 2.0 * y as f64 / h) * t;
        (
            self.eye(),
            std::array::from_fn(|k| f[k] + right[k] * sx + up[k] * sy),
        )
    }

    /// Meters per viewport unit at distance `d` from the eye, for a viewport
    /// `h` units tall.
    pub fn m_per_unit(d: f64, h: f32) -> f64 {
        2.0 * d * (FOVY as f64 / 2.0).tan() / h.max(1.0) as f64
    }

    pub fn camera(&self, aspect: f32) -> Camera {
        let eye = self.eye();
        // ponytail: plain Depth32Float; far/near can reach 1e5 when zoomed in on
        // a big tile. Switch to reversed-Z if distant faces z-fight.
        let far = self.scene.map_or(0.0, |(c, r)| {
            (0..3).map(|k| (eye[k] - c[k]).powi(2)).sum::<f64>().sqrt() + r
        });
        Camera {
            eye,
            target: self.target,
            fovy: FOVY,
            aspect,
            near: (self.distance * 0.01) as f32,
            far: (self.distance * 100.0).max(far) as f32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
        (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt()
    }

    /// World point to normalized device coordinates (x right, y up, z depth).
    fn ndc(cam: &Camera, p: [f64; 3]) -> [f64; 3] {
        let m = cam.view_proj_at_eye();
        let v = [p[0] - cam.eye[0], p[1] - cam.eye[1], p[2] - cam.eye[2], 1.0];
        let c: [f64; 4] = std::array::from_fn(|r| (0..4).map(|k| m[k][r] as f64 * v[k]).sum());
        [c[0] / c[3], c[1] / c[3], c[2] / c[3]]
    }

    /// A point on the ray projects back to the pixel it came from, at any view
    /// and far from the origin.
    #[test]
    fn ray_goes_through_its_pixel() {
        let (w, h) = (800.0f32, 600.0f32);
        for (yaw, pitch) in [(-2.2, 0.6), (0.7, PITCH_LIMIT), (2.0, -1.0), (0.0, 0.0)] {
            let o = Orbit {
                target: [430_000.0, 9_150_000.0, 5.0],
                distance: 80.0,
                yaw,
                pitch,
                scene: None,
            };
            for (x, y) in [
                (400.0f32, 300.0f32),
                (0.0, 0.0),
                (790.0, 20.0),
                (123.0, 555.0),
            ] {
                let (eye, dir) = o.ray(x, y, w, h);
                assert_eq!(eye, o.eye());
                let p: [f64; 3] = std::array::from_fn(|k| eye[k] + dir[k] * 50.0);
                let c = ndc(&o.camera(w / h), p);
                let (sx, sy) = ((c[0] + 1.0) / 2.0 * w as f64, (1.0 - c[1]) / 2.0 * h as f64);
                assert!(
                    (sx - x as f64).abs() < 1e-3 && (sy - y as f64).abs() < 1e-3,
                    "yaw {yaw} pitch {pitch}: ({x}, {y}) came back as ({sx}, {sy})"
                );
            }
        }
    }

    #[test]
    fn eye_sits_at_distance_from_target() {
        let o = Orbit {
            target: [430_000.0, 9_150_000.0, 10.0],
            distance: 37.5,
            yaw: 1.0,
            pitch: 0.3,
            scene: None,
        };
        assert!((dist(o.eye(), o.target) - 37.5).abs() < 1e-6);
        // Positive pitch looks down on the target; the target is centered.
        assert!(o.eye()[2] > o.target[2]);
        let c = ndc(&o.camera(1.5), o.target);
        assert!(c[0].abs() < 1e-6 && c[1].abs() < 1e-6, "{c:?}");
    }

    #[test]
    fn orbit_turns_yaw_and_clamps_pitch() {
        let mut o = Orbit::default();
        let yaw0 = o.yaw;
        o.orbit(10.0, 10_000.0);
        assert!(o.yaw != yaw0);
        assert_eq!(o.pitch, PITCH_LIMIT);
        o.orbit(0.0, -100_000.0);
        assert_eq!(o.pitch, -PITCH_LIMIT);
        assert!((dist(o.eye(), o.target) - o.distance).abs() < 1e-9);
    }

    /// The point under the cursor follows the drag on screen, including at the
    /// pitch limit (top-down view).
    #[test]
    fn pan_follows_the_cursor() {
        let (w, h) = (800.0f32, 600.0f32);
        for (yaw, pitch) in [
            (-2.2, 0.6),
            (0.7, PITCH_LIMIT),
            (2.0, -PITCH_LIMIT),
            (0.0, 0.0),
        ] {
            let mut o = Orbit {
                target: [430_000.0, 9_150_000.0, 5.0],
                distance: 80.0,
                yaw,
                pitch,
                scene: None,
            };
            let p = o.target;
            o.pan(40.0, -25.0, h);
            let c = ndc(&o.camera(w / h), p);
            // Screen pixels: +x right, +y down; NDC y is up.
            let (sx, sy) = (c[0] * w as f64 / 2.0, -c[1] * h as f64 / 2.0);
            assert!(
                (sx - 40.0).abs() < 1e-3 && (sy + 25.0).abs() < 1e-3,
                "yaw {yaw} pitch {pitch}: moved ({sx}, {sy})"
            );
        }
    }

    #[test]
    fn zoom_is_bounded_both_ways() {
        let mut o = Orbit::default();
        o.zoom(1.0);
        assert!(o.distance < 50.0);
        o.zoom(-1.0);
        assert!((o.distance - 50.0).abs() < 1e-9);
        o.zoom(10_000.0);
        assert_eq!(o.distance, 0.1);
        o.zoom(-10_000.0);
        assert_eq!(o.distance, 1e6);
    }

    /// All 8 box corners land inside the clip volume, portrait and landscape,
    /// and stay inside the far plane after zooming in on a corner.
    #[test]
    fn fit_shows_every_corner() {
        let (lo, hi) = ([-200.0, 10.0, 0.0], [300.0, 90.0, 45.0]);
        let corners = (0..8).map(|i| {
            std::array::from_fn::<f64, 3, _>(|k| if i >> k & 1 == 0 { lo[k] } else { hi[k] })
        });
        for aspect in [0.4f32, 1.0, 2.5] {
            let mut o = Orbit::default();
            o.fit(lo, hi, aspect);
            assert_eq!(o.target, [50.0, 50.0, 22.5]);
            let cam = o.camera(aspect);
            for p in corners.clone() {
                let c = ndc(&cam, p);
                assert!(
                    c[0].abs() <= 1.0 && c[1].abs() <= 1.0 && (0.0..=1.0).contains(&c[2]),
                    "aspect {aspect}: corner {p:?} at {c:?}"
                );
            }
            // Zoomed in close on one end, the far end is still before the far plane.
            o.target = lo;
            o.distance = 0.5;
            let cam = o.camera(aspect);
            for p in corners.clone() {
                assert!((dist(cam.eye, p) as f32) < cam.far, "far plane clips {p:?}");
            }
        }
    }
}
