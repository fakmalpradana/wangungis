//! The value box: what a user types while a tool waits for a number.
//!
//! `5`, `2,5` or `2.5` (the comma is a decimal mark, as written in Indonesia),
//! `5;4` or `5x4` for two dimensions, an optional unit `mm`, `cm` or `m`
//! (default m) per value, and a sign for the opposite direction.
//! After a copy: `x5`, `5x`, `*5` or `5*` for five copies, `/5` or `5/` for five
//! copies spaced evenly up to the copy just made. `8s` for eight sides.
//! `30d` or `30°` for an angle in degrees.

use std::num::NonZeroU32;

/// Parsed input, in meters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Vcb {
    One(f64),
    Two(f64, f64),
    /// Copies each as far as the first.
    Copies(NonZeroU32),
    /// Copies filling the distance to the first.
    Divide(NonZeroU32),
    /// Sides of a circle or polygon.
    Sides(u32),
    /// An angle, in degrees.
    Angle(f64),
}

/// More copies than this in one go is a typing error, not a model.
const MAX_COPIES: u32 = 1_000;

fn count(text: &str) -> Option<NonZeroU32> {
    let t = text.trim();
    if t.is_empty() || !t.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    NonZeroU32::new(t.parse().ok()?).filter(|n| n.get() <= MAX_COPIES)
}

/// `x5`, `5x`, `*5`, `5*`, `/5` or `5/`.
fn array(text: &str) -> Option<Vcb> {
    let t = text.trim();
    let side = |mark: &[char]| t.strip_prefix(mark).or(t.strip_suffix(mark));
    if let Some(n) = side(&['x', '*']) {
        return Some(Vcb::Copies(count(n)?));
    }
    Some(Vcb::Divide(count(side(&['/'])?)?))
}

fn length(text: &str) -> Option<f64> {
    let t = text.trim().to_ascii_lowercase();
    let (number_text, scale) = if let Some(n) = t.strip_suffix("mm") {
        (n, 1e-3)
    } else if let Some(n) = t.strip_suffix("cm") {
        (n, 1e-2)
    } else if let Some(n) = t.strip_suffix('m') {
        (n, 1.0)
    } else {
        (t.as_str(), 1.0)
    };
    Some(number(number_text)? * scale)
}

fn number(text: &str) -> Option<f64> {
    let number = text.trim().replace(',', ".");
    // Plain decimals only: Rust's parser would also take "inf" and "1e3".
    if !number
        .bytes()
        .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'-' | b'+'))
    {
        return None;
    }
    let v: f64 = number.parse().ok()?;
    v.is_finite().then_some(v)
}

/// `None` when the text is not one or two valid lengths.
pub fn parse(text: &str) -> Option<Vcb> {
    let lower = text.to_ascii_lowercase();
    if let Some(v) = array(&lower) {
        return Some(v);
    }
    if let Some(n) = lower.trim().strip_suffix('s') {
        return count(n).map(|n| Vcb::Sides(n.get()));
    }
    if let Some(n) = lower.trim().strip_suffix(['d', '°']) {
        return number(n).map(Vcb::Angle);
    }
    let parts: Vec<&str> = lower.split([';', 'x']).collect();
    match parts[..] {
        [a] => Some(Vcb::One(length(a)?)),
        [a, b] => Some(Vcb::Two(length(a)?, length(b)?)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sides() {
        assert_eq!(parse("8s"), Some(Vcb::Sides(8)));
        assert_eq!(parse(" 48S "), Some(Vcb::Sides(48)));
        for t in ["s", "0s", "-3s", "2,5s", "s8"] {
            assert_eq!(parse(t), None, "{t}");
        }
    }

    #[test]
    fn angles() {
        for (t, want) in [
            ("30d", 30.0),
            ("30°", 30.0),
            (" 2,5D ", 2.5),
            ("-15.5d", -15.5),
            ("0d", 0.0),
        ] {
            assert_eq!(parse(t), Some(Vcb::Angle(want)), "{t}");
        }
        for t in ["d", "°", "30dd", "infd", "1e3d", "5;30d", "30md"] {
            assert_eq!(parse(t), None, "{t}");
        }
    }

    #[test]
    fn array_counts() {
        let n = |v| NonZeroU32::new(v).unwrap();
        for t in ["x5", "5x", "*5", "5*", " x5 ", "X5"] {
            assert_eq!(parse(t), Some(Vcb::Copies(n(5))), "{t}");
        }
        for t in ["/5", "5/"] {
            assert_eq!(parse(t), Some(Vcb::Divide(n(5))), "{t}");
        }
        for t in [
            "x0",
            "/0",
            "x",
            "/",
            "x-1",
            "x1,5",
            "x1001",
            "x99999999999",
            "x5x",
        ] {
            assert_eq!(parse(t), None, "{t}");
        }
        // Two dimensions are not taken for an array.
        assert_eq!(parse("5x4"), Some(Vcb::Two(5.0, 4.0)));
    }

    #[test]
    fn valid_input() {
        for (text, want) in [
            ("5", Vcb::One(5.0)),
            ("2,5", Vcb::One(2.5)),
            ("2.5", Vcb::One(2.5)),
            (" -1,5 m ", Vcb::One(-1.5)),
            ("+3", Vcb::One(3.0)),
            ("250cm", Vcb::One(2.5)),
            ("1500 mm", Vcb::One(1.5)),
            ("0.5M", Vcb::One(0.5)),
            ("5;4", Vcb::Two(5.0, 4.0)),
            ("5x4", Vcb::Two(5.0, 4.0)),
            ("5 X 4", Vcb::Two(5.0, 4.0)),
            ("2,5;4000mm", Vcb::Two(2.5, 4.0)),
            ("-5;4", Vcb::Two(-5.0, 4.0)),
        ] {
            assert_eq!(parse(text), Some(want), "{text:?}");
        }
    }

    #[test]
    fn invalid_input() {
        for text in [
            "", " ", "abc", "5;", ";4", "1;2;3", "1x2x3", "1,2,3", "1.2.3", "inf", "nan", "1e3",
            "m", "-", "5 km", "5m m", "--5",
        ] {
            assert_eq!(parse(text), None, "{text:?}");
        }
    }
}
