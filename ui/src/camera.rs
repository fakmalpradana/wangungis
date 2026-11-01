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
}

impl Default for Orbit {
    fn default() -> Self {
        Orbit {
            target: [0.0; 3],
            distance: 50.0,
            yaw: -2.2,
            pitch: 0.6,
        }
    }
}

const PITCH_LIMIT: f64 = 1.55;
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

    /// Zoom extents: the bounding sphere of `lo..hi` fills the view.
    pub fn fit(&mut self, lo: [f64; 3], hi: [f64; 3]) {
        self.target = std::array::from_fn(|k| (lo[k] + hi[k]) / 2.0);
        let r = (0..3).map(|k| (hi[k] - lo[k]).powi(2)).sum::<f64>().sqrt() / 2.0;
        self.distance = (r.max(1.0) / (FOVY as f64 / 2.0).sin() * 1.1).clamp(0.1, 1e6);
    }

    pub fn camera(&self, aspect: f32) -> Camera {
        Camera {
            eye: self.eye(),
            target: self.target,
            fovy: FOVY,
            aspect,
            near: (self.distance * 0.01) as f32,
            far: (self.distance * 100.0) as f32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
        (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt()
    }

    #[test]
    fn eye_sits_at_distance_from_target() {
        let o = Orbit {
            target: [430_000.0, 9_150_000.0, 10.0],
            distance: 37.5,
            yaw: 1.0,
            pitch: 0.3,
        };
        assert!((dist(o.eye(), o.target) - 37.5).abs() < 1e-6);
    }

    #[test]
    fn orbit_clamps_pitch_and_keeps_distance() {
        let mut o = Orbit::default();
        o.orbit(10.0, 10_000.0);
        assert_eq!(o.pitch, PITCH_LIMIT);
        o.orbit(0.0, -100_000.0);
        assert_eq!(o.pitch, -PITCH_LIMIT);
        assert!((dist(o.eye(), o.target) - o.distance).abs() < 1e-9);
    }

    #[test]
    fn pan_moves_target_perpendicular_to_view() {
        let mut o = Orbit::default();
        let (t0, b) = (o.target, o.back());
        o.pan(120.0, -45.0, 600.0);
        let d: [f64; 3] = std::array::from_fn(|k| o.target[k] - t0[k]);
        let along_view: f64 = (0..3).map(|k| d[k] * b[k]).sum();
        assert!(dist(o.target, t0) > 1.0, "target did not move");
        assert!(along_view.abs() < 1e-9, "pan changed depth by {along_view}");
    }

    #[test]
    fn zoom_in_reduces_distance_and_is_bounded() {
        let mut o = Orbit::default();
        o.zoom(1.0);
        assert!(o.distance < 50.0);
        o.zoom(10_000.0);
        assert_eq!(o.distance, 0.1);
    }

    #[test]
    fn fit_contains_the_bounding_sphere() {
        let mut o = Orbit::default();
        let (lo, hi) = ([-200.0, 10.0, 0.0], [300.0, 90.0, 45.0]);
        o.fit(lo, hi);
        let r = dist(lo, hi) / 2.0;
        assert_eq!(o.target, [50.0, 50.0, 22.5]);
        assert!(
            o.distance * (FOVY as f64 / 2.0).sin() >= r,
            "sphere not inside the view cone"
        );
        assert!(
            o.camera(1.0).far as f64 > o.distance + r,
            "far plane clips the model"
        );
    }
}
