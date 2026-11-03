//! Project origin and CRS for a WangunGIS document (ADR-004, ADR-006).
//!
//! Hew's kernel works in absolute `f64` with a 1e-9 m merge tolerance, which
//! fails at raw UTM coordinates (spike S2). So positions are kept relative to a
//! per-project `f64` origin, and the origin plus the EPSG
//! code are stored in the document's attribute dictionary under namespace
//! `"wangungis"`. The kernel stores that data without interpreting it.

use kernel::{AttrTarget, AttrValue, Document, Point3};

pub mod project;

/// Attribute namespace for all WangunGIS data in a Hew document.
pub const NS: &str = "wangungis";

/// Largest allowed |coordinate| (m), on any axis, of a point relative to the
/// project origin: the extent of one project (a city). Kernel geometry does
/// NOT live at these distances; see [`KERNEL_FRAME_BOUND_M`].
pub const PROJECT_BOUND_M: f64 = 50_000.0;

/// Largest allowed horizontal distance (m) of geometry from the origin of the
/// Hew `Document` it lives in. Hew's booleans start refusing rotated geometry
/// between 2.5 and 2.8 km out (`tests/kernel_bound.rs`), so the project is split
/// into kernel tiles whose frames sit at their tile centers (ADR-004), and this
/// bound keeps a ~40% margin. Changing it needs human approval (AGENTS.md §14, K6).
pub const KERNEL_FRAME_BOUND_M: f64 = 1_500.0;

const KEY_CRS: &str = "crs";
const KEY_ORIGIN: &str = "origin";

/// Where kernel-local (0, 0, 0) sits in the project CRS.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectOrigin {
    /// EPSG code of the project CRS, e.g. 32749 for UTM 49S.
    pub epsg: u32,
    /// World coordinates (m) of the kernel-local origin.
    pub origin: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub enum OriginError {
    /// The stored attribute exists but is not what this crate writes.
    Malformed(&'static str),
    /// A local coordinate is outside ±[`PROJECT_BOUND_M`].
    OutOfBound([f64; 3]),
    /// The kernel refused the attribute read or write.
    Attr(String),
}

impl ProjectOrigin {
    /// Reads the origin from `doc`. `Ok(None)` when the document has none yet.
    pub fn read(doc: &Document) -> Result<Option<Self>, OriginError> {
        let dict = doc
            .attr_get(&AttrTarget::Document)
            .map_err(|e| OriginError::Attr(format!("{e:?}")))?;
        let Some(ns) = dict.as_ref().and_then(|d| d.get(NS)) else {
            return Ok(None);
        };
        let (crs, origin) = match (ns.get(KEY_CRS), ns.get(KEY_ORIGIN)) {
            (None, None) => return Ok(None),
            (Some(c), Some(o)) => (c, o),
            _ => {
                return Err(OriginError::Malformed(
                    "crs and origin must both be present",
                ));
            }
        };
        let epsg = match crs {
            AttrValue::Text(t) => t
                .strip_prefix("EPSG:")
                .filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
                .and_then(|n| n.parse().ok())
                .filter(|&code: &u32| code > 0)
                .ok_or(OriginError::Malformed(
                    "crs must be \"EPSG:<positive code>\"",
                ))?,
            _ => return Err(OriginError::Malformed("crs must be text")),
        };
        let origin = match origin {
            AttrValue::List(l) if l.len() == 3 => {
                let mut o = [0.0; 3];
                for (slot, v) in o.iter_mut().zip(l) {
                    match v {
                        AttrValue::Float(f) if f.is_finite() => *slot = *f,
                        _ => return Err(OriginError::Malformed("origin must be 3 finite floats")),
                    }
                }
                o
            }
            _ => return Err(OriginError::Malformed("origin must be a list of 3 floats")),
        };
        Ok(Some(Self { epsg, origin }))
    }

    /// Stores the origin in `doc`, replacing any previous one.
    pub fn write(&self, doc: &mut Document) -> Result<(), OriginError> {
        if self.origin.iter().any(|v| !v.is_finite()) {
            return Err(OriginError::Malformed("origin must be finite"));
        }
        if self.epsg == 0 {
            return Err(OriginError::Malformed("EPSG code must be positive"));
        }
        let attr = |e| OriginError::Attr(format!("{e:?}"));
        doc.attr_set(
            AttrTarget::Document,
            NS,
            KEY_CRS,
            AttrValue::Text(format!("EPSG:{}", self.epsg)),
        )
        .map_err(attr)?;
        let list = self.origin.iter().map(|v| AttrValue::Float(*v)).collect();
        doc.attr_set(AttrTarget::Document, NS, KEY_ORIGIN, AttrValue::List(list))
            .map_err(attr)?;
        Ok(())
    }

    /// World (project CRS) to project-local. Rejects points outside
    /// ±[`PROJECT_BOUND_M`]. A world → local → world round trip is off by at
    /// most 1 ulp of max(|world|, |origin|) per axis (≈ 1.9e-9 m at
    /// Y = 9.15e6).
    pub fn to_local(&self, world: [f64; 3]) -> Result<Point3, OriginError> {
        let l = [
            world[0] - self.origin[0],
            world[1] - self.origin[1],
            world[2] - self.origin[2],
        ];
        check_local(l)?;
        Ok(Point3::new(l[0], l[1], l[2]))
    }

    /// Project-local to world (project CRS).
    pub fn to_world(&self, local: Point3) -> [f64; 3] {
        [
            local.x + self.origin[0],
            local.y + self.origin[1],
            local.z + self.origin[2],
        ]
    }
}

/// Project-level check: `Err` unless every component is finite and within
/// ±[`PROJECT_BOUND_M`]. This does NOT make a point safe for one Hew
/// `Document`; geometry there must stay within [`KERNEL_FRAME_BOUND_M`].
pub fn check_local(l: [f64; 3]) -> Result<(), OriginError> {
    if l.iter()
        .all(|v| v.is_finite() && v.abs() <= PROJECT_BOUND_M)
    {
        Ok(())
    } else {
        Err(OriginError::OutOfBound(l))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UTM49S: ProjectOrigin = ProjectOrigin {
        epsg: 32749,
        origin: [430_000.123, 9_150_000.456, 12.5],
    };

    #[test]
    fn origin_round_trips_bit_exactly_through_hew_file() {
        let mut doc = Document::new();
        assert_eq!(ProjectOrigin::read(&doc), Ok(None));
        UTM49S.write(&mut doc).unwrap();
        let doc = Document::load(&doc.save()).expect("reload");
        let back = ProjectOrigin::read(&doc).unwrap().unwrap();
        assert_eq!(back.epsg, 32749);
        assert_eq!(
            back.origin.map(f64::to_bits),
            UTM49S.origin.map(f64::to_bits)
        );
    }

    #[test]
    fn rewrite_replaces_previous_origin() {
        let mut doc = Document::new();
        UTM49S.write(&mut doc).unwrap();
        let other = ProjectOrigin {
            epsg: 28992,
            origin: [155_000.0, 463_000.0, 0.0],
        };
        other.write(&mut doc).unwrap();
        assert_eq!(ProjectOrigin::read(&doc), Ok(Some(other)));
    }

    #[test]
    fn malformed_attributes_are_errors_not_defaults() {
        let mut doc = Document::new();
        UTM49S.write(&mut doc).unwrap();
        doc.attr_set(
            AttrTarget::Document,
            NS,
            KEY_CRS,
            AttrValue::Text("WGS84".into()),
        )
        .unwrap();
        assert!(matches!(
            ProjectOrigin::read(&doc),
            Err(OriginError::Malformed(_))
        ));
        UTM49S.write(&mut doc).unwrap();
        doc.attr_set(
            AttrTarget::Document,
            NS,
            KEY_ORIGIN,
            AttrValue::List(vec![AttrValue::Float(1.0)]),
        )
        .unwrap();
        assert!(matches!(
            ProjectOrigin::read(&doc),
            Err(OriginError::Malformed(_))
        ));
    }

    #[test]
    fn half_written_origin_is_an_error() {
        let mut doc = Document::new();
        doc.attr_set(
            AttrTarget::Document,
            NS,
            KEY_CRS,
            AttrValue::Text("EPSG:32749".into()),
        )
        .unwrap();
        assert!(matches!(
            ProjectOrigin::read(&doc),
            Err(OriginError::Malformed(_))
        ));
        for bad in ["EPSG:", "EPSG:-1", "EPSG:+32749", "EPSG:0", "epsg:32749"] {
            let mut doc = Document::new();
            UTM49S.write(&mut doc).unwrap();
            doc.attr_set(
                AttrTarget::Document,
                NS,
                KEY_CRS,
                AttrValue::Text(bad.into()),
            )
            .unwrap();
            assert!(ProjectOrigin::read(&doc).is_err(), "{bad} accepted");
        }
    }

    #[test]
    fn non_finite_origin_or_zero_epsg_is_refused() {
        let bad = ProjectOrigin {
            epsg: 32749,
            origin: [f64::NAN, 0.0, 0.0],
        };
        assert!(bad.write(&mut Document::new()).is_err());
        let bad = ProjectOrigin {
            epsg: 0,
            origin: [0.0; 3],
        };
        assert!(bad.write(&mut Document::new()).is_err());
    }

    #[test]
    fn local_bound_is_enforced() {
        let o = UTM49S.origin;
        assert!(
            UTM49S
                .to_local([o[0] + PROJECT_BOUND_M, o[1] - PROJECT_BOUND_M, o[2]])
                .is_ok()
        );
        assert!(
            UTM49S
                .to_local([o[0] + PROJECT_BOUND_M + 1.0, o[1], o[2]])
                .is_err()
        );
        assert!(UTM49S.to_local([f64::INFINITY, o[1], o[2]]).is_err());
        assert!(check_local([0.0, f64::NAN, 0.0]).is_err());
    }

    use proptest::prelude::*;

    fn finite() -> impl Strategy<Value = f64> {
        any::<f64>().prop_filter("finite", |v| v.is_finite())
    }

    fn ulp(x: f64) -> f64 {
        let a = x.abs();
        f64::from_bits(a.to_bits() + 1) - a
    }

    proptest! {
        /// Any valid origin (incl. -0.0 and subnormals) survives save/load bit-exactly.
        #[test]
        fn any_origin_round_trips_bit_exactly(epsg in 1u32.., o in [finite(), finite(), finite()]) {
            let mut doc = Document::new();
            ProjectOrigin { epsg, origin: o }.write(&mut doc).unwrap();
            let back = ProjectOrigin::read(&Document::load(&doc.save()).unwrap()).unwrap().unwrap();
            prop_assert_eq!(back.epsg, epsg);
            prop_assert_eq!(back.origin.map(f64::to_bits), o.map(f64::to_bits));
        }

        /// world -> local -> world is off by <= 1 ulp of max(|world|, |origin|).
        #[test]
        fn world_local_world_within_one_ulp(
            o in [-1e7..1e7f64, -1e7..1e7f64, -1e4..1e4f64],
            d in [-PROJECT_BOUND_M..PROJECT_BOUND_M, -PROJECT_BOUND_M..PROJECT_BOUND_M, -1e3..1e3f64],
        ) {
            let po = ProjectOrigin { epsg: 32749, origin: o };
            let w = [o[0] + d[0], o[1] + d[1], o[2] + d[2]];
            // Rounding can push a point just past the bound; that is a refusal, not an error.
            let Ok(l) = po.to_local(w) else { return Ok(()) };
            let back = po.to_world(l);
            for k in 0..3 {
                let tol = ulp(w[k].abs().max(o[k].abs()));
                prop_assert!((back[k] - w[k]).abs() <= tol, "axis {}: {} vs {}", k, back[k], w[k]);
            }
        }
    }
}
