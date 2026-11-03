//! The value box: what a user types while a tool waits for a number.
//!
//! `5`, `2,5` or `2.5` (the comma is a decimal mark, as written in Indonesia),
//! `5;4` or `5x4` for two dimensions, an optional unit `mm`, `cm` or `m`
//! (default m) per value, and a sign for the opposite direction.

/// Parsed input, in meters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Vcb {
    One(f64),
    Two(f64, f64),
}

fn length(text: &str) -> Option<f64> {
    let t = text.trim().to_ascii_lowercase();
    let (number, scale) = if let Some(n) = t.strip_suffix("mm") {
        (n, 1e-3)
    } else if let Some(n) = t.strip_suffix("cm") {
        (n, 1e-2)
    } else if let Some(n) = t.strip_suffix('m') {
        (n, 1.0)
    } else {
        (t.as_str(), 1.0)
    };
    let number = number.trim().replace(',', ".");
    // Plain decimals only: Rust's parser would also take "inf" and "1e3".
    if !number
        .bytes()
        .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'-' | b'+'))
    {
        return None;
    }
    let v: f64 = number.parse().ok()?;
    v.is_finite().then_some(v * scale)
}

/// `None` when the text is not one or two valid lengths.
pub fn parse(text: &str) -> Option<Vcb> {
    let lower = text.to_ascii_lowercase();
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
