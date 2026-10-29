//! WangunGIS spike S3 (roadmap F0): polygon-face building -> Manifold boolean
//! (window opening) -> triangles -> coplanar regrouping back into planar faces.
//! Question: after the round trip, are regrouped faces planar within the
//! kernel tolerance, and is the volume exact?
//! Run: cargo test --release -- --nocapture

use manifold_csg::{CrossSection, Manifold};

/// An L-shaped footprint (CCW), `a` x `b` overall with a `c` x `d` notch removed.
pub fn l_footprint(a: f64, b: f64, c: f64, d: f64) -> Vec<[f64; 2]> {
    vec![[0.0, 0.0], [a, 0.0], [a, b - d], [a - c, b - d], [a - c, b], [0.0, b]]
}

/// Plane of a triangle: unit normal and offset (n . p).
fn plane(p: [f64; 3], q: [f64; 3], r: [f64; 3]) -> Option<([f64; 3], f64)> {
    let u = [q[0] - p[0], q[1] - p[1], q[2] - p[2]];
    let v = [r[0] - p[0], r[1] - p[1], r[2] - p[2]];
    let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if len < 1e-12 {
        return None; // sliver; reported separately
    }
    let n = [n[0] / len, n[1] / len, n[2] / len];
    Some((n, n[0] * p[0] + n[1] * p[1] + n[2] * p[2]))
}

pub struct RoundTrip {
    pub volume: f64,
    pub genus: i32,
    pub faces: usize,
    pub slivers: usize,
    /// Max distance (m) of any vertex from the plane of the face it was regrouped into.
    pub planarity: f64,
}

/// Regroup output triangles into planar faces (clustered by normal within
/// `angle_tol` and offset within `dist_tol`) and measure planarity residual.
pub fn regroup(m: &Manifold, angle_tol: f64, dist_tol: f64) -> RoundTrip {
    let (verts, n_props, tris) = m.to_mesh_f64();
    let at = |i: u64| {
        let k = i as usize * n_props;
        [verts[k], verts[k + 1], verts[k + 2]]
    };
    let mut faces: Vec<([f64; 3], f64, Vec<[f64; 3]>)> = Vec::new();
    let mut slivers = 0;
    for t in tris.chunks(3) {
        let (p, q, r) = (at(t[0]), at(t[1]), at(t[2]));
        let Some((n, d)) = plane(p, q, r) else {
            slivers += 1;
            continue;
        };
        let hit = faces.iter_mut().find(|(fnrm, fd, _)| {
            let dot = fnrm[0] * n[0] + fnrm[1] * n[1] + fnrm[2] * n[2];
            dot > 1.0 - angle_tol && (fd - d).abs() < dist_tol
        });
        match hit {
            Some((_, _, pts)) => pts.extend([p, q, r]),
            None => faces.push((n, d, vec![p, q, r])),
        }
    }
    let planarity = faces
        .iter()
        .flat_map(|(n, d, pts)| pts.iter().map(move |p| (n[0] * p[0] + n[1] * p[1] + n[2] * p[2] - d).abs()))
        .fold(0.0, f64::max);
    RoundTrip { volume: m.volume(), genus: m.genus(), faces: faces.len(), slivers, planarity }
}

/// Building = extruded L footprint; window = box cut through the south wall.
/// Everything is placed at (ox, oy) and rotated `rot_deg` about z.
#[allow(clippy::too_many_arguments)]
pub fn building_with_window(
    (a, b, c, d, h): (f64, f64, f64, f64, f64),
    (wx, wz, ww, wh): (f64, f64, f64, f64),
    rot_deg: f64,
    (ox, oy): (f64, f64),
) -> Manifold {
    let body = CrossSection::from_simple_polygon(&l_footprint(a, b, c, d)).extrude(h);
    // Window: through the south wall (y = 0), wall assumed thicker than nothing:
    // the box spans y in [-0.5, 0.5] so it cuts a 0.5 m deep pocket into the solid.
    let window = Manifold::cube(ww, 1.0, wh, false).translate(wx, -0.5, wz);
    (&body - &window).rotate(0.0, 0.0, rot_deg).translate(ox, oy, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const ANGLE_TOL: f64 = 1e-9;
    const DIST_TOL: f64 = 1e-6;

    fn expected_volume(a: f64, b: f64, c: f64, d: f64, h: f64, ww: f64, wh: f64) -> f64 {
        (a * b - c * d) * h - ww * 0.5 * wh
    }

    fn check(dims: (f64, f64, f64, f64, f64), win: (f64, f64, f64, f64), rot: f64, origin: (f64, f64)) -> RoundTrip {
        let m = building_with_window(dims, win, rot, origin);
        assert!(m.status().is_ok() && !m.is_empty(), "manifold error/empty");
        regroup(&m, ANGLE_TOL, DIST_TOL)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]

        /// Local coordinates (origin within ±1 km, any rotation): exact volume,
        /// genus 0, and regrouped faces planar to 1e-9 m.
        #[test]
        fn local_round_trip_is_planar_and_exact(
            a in 8.0..30.0f64, b in 8.0..30.0f64, cf in 0.2..0.7f64, df in 0.2..0.7f64, h in 3.0..20.0f64,
            wxf in 0.05..0.5f64, wzf in 0.1..0.5f64, ww in 0.6..2.0f64, wh in 0.6..1.5f64,
            rot in 0.0..360.0f64, ox in -1000.0..1000.0f64, oy in -1000.0..1000.0f64,
        ) {
            let (c, d) = (a * cf, b * df);
            let wx = wxf * (a - ww);
            let wz = wzf * (h - wh);
            let r = check((a, b, c, d, h), (wx, wz, ww, wh), rot, (ox, oy));
            let v = expected_volume(a, b, c, d, h, ww, wh);
            prop_assert!(((r.volume - v) / v).abs() < 1e-9, "volume {} vs {}", r.volume, v);
            prop_assert_eq!(r.genus, 0);
            prop_assert_eq!(r.slivers, 0);
            prop_assert!(r.planarity <= 1e-9, "planarity {:e}", r.planarity);
        }
    }

    /// Same building at absolute UTM 49S coordinates: report, do not assert,
    /// so the spike records the magnitude of the residual.
    #[test]
    fn utm_absolute_report() {
        let mut worst = 0.0f64;
        let mut worst_vol = 0.0f64;
        for i in 0..200 {
            let rot = i as f64 * 1.7;
            let dims = (20.0, 15.0, 8.0, 6.0, 9.0);
            let win = (3.0, 1.0, 1.2, 1.4);
            let r = check(dims, win, rot, (430_000.123, 9_150_000.456));
            let v = expected_volume(20.0, 15.0, 8.0, 6.0, 9.0, 1.2, 1.4);
            worst = worst.max(r.planarity);
            worst_vol = worst_vol.max(((r.volume - v) / v).abs());
        }
        println!("UTM absolute, 200 rotations: max planarity residual {worst:.3e} m, max rel volume error {worst_vol:.3e}");
    }
}
