//! Exact numeric bounds for range contracts.
//!
//! A bound is stored as its value times 10^6 ("micros"), exact when the
//! literal has at most 6 decimal places. Otherwise it is rounded in the
//! conservative direction for its role (`docs/design/dataflow-v2.md`,
//! "Exact numeric bounds"): a requirement rounds to the stricter side, a
//! guarantee to the weaker side.

/// Value times 10^6.
pub type Micros = i128;

pub const SCALE: i128 = 1_000_000;

/// Which side of a range a bound is on, and who states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundKind {
    /// `x >= b` required by a target (rounds up: stricter).
    RequiredMin,
    /// `x <= b` required by a target (rounds down: stricter).
    RequiredMax,
    /// `x >= b` guaranteed by a source (rounds down: weaker).
    GuaranteedMin,
    /// `x <= b` guaranteed by a source (rounds up: weaker).
    GuaranteedMax,
}

impl BoundKind {
    fn rounds_up(self) -> bool {
        matches!(self, BoundKind::RequiredMin | BoundKind::GuaranteedMax)
    }

    pub fn is_min(self) -> bool {
        matches!(self, BoundKind::RequiredMin | BoundKind::GuaranteedMin)
    }
}

/// Micros of an integer.
pub fn from_int(v: i64) -> Micros {
    v as i128 * SCALE
}

/// Parse a decimal literal (`-12.5`, `1e-4`, `0.1234567`, `1_000`) to
/// micros, rounding as `kind` requires. `None` for anything else
/// (`NaN`, `Infinity`, hex, out of range).
pub fn parse_decimal(text: &str, kind: BoundKind) -> Option<Micros> {
    let t = text.trim().replace('_', "");
    let (neg, t) = match t.strip_prefix('-') {
        Some(rest) => (true, rest.to_string()),
        None => (false, t.strip_prefix('+').unwrap_or(&t).to_string()),
    };
    let (mantissa, exp) = match t.find(['e', 'E']) {
        Some(i) => (&t[..i], t[i + 1..].parse::<i32>().ok()?),
        None => (t.as_str(), 0),
    };
    let (int_part, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if (int_part.is_empty() && frac.is_empty())
        || !int_part.chars().all(|c| c.is_ascii_digit())
        || !frac.chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    // value = digits * 10^(exp - frac.len())
    let digits: String = format!("{int_part}{frac}");
    let digits = digits.trim_start_matches('0');
    let shift = exp as i64 - frac.len() as i64 + 6; // power of ten on `digits` for micros
    if digits.is_empty() {
        return Some(0);
    }
    if digits.len() as i64 + shift > 30 {
        return None; // beyond i128 micros
    }
    let magnitude: i128;
    let mut inexact = false;
    if shift >= 0 {
        let d: i128 = digits.parse().ok()?;
        magnitude = d.checked_mul(10i128.checked_pow(shift as u32)?)?;
    } else {
        let cut = (-shift) as usize;
        if cut >= digits.len() {
            magnitude = 0;
            inexact = true;
        } else {
            let (keep, dropped) = digits.split_at(digits.len() - cut);
            magnitude = keep.parse().ok()?;
            inexact = dropped.chars().any(|c| c != '0');
        }
    }
    // Truncation moved the magnitude towards zero; adjust for the rounding direction.
    let value = if neg { -magnitude } else { magnitude };
    Some(match (inexact, kind.rounds_up(), neg) {
        (false, _, _) => value,
        (true, true, false) => value + 1,
        (true, false, true) => value - 1,
        (true, _, _) => value,
    })
}

/// Micros of a Python float, via its shortest round-trip representation
/// (what `repr` prints, and in practice what the source says).
pub fn from_f64(v: f64, kind: BoundKind) -> Option<Micros> {
    if !v.is_finite() {
        return None;
    }
    parse_decimal(&format!("{v}"), kind)
}

/// A numeric literal bound in source: `5`, `-1`, `0.5`, `Decimal("10")`,
/// `decimal.Decimal("0.01")`, `Decimal(3)`.
pub fn literal_bound(expr: &ruff_python_ast::Expr, kind: BoundKind) -> Option<Micros> {
    use ruff_python_ast::{Expr, Number, UnaryOp};
    match expr {
        Expr::NumberLiteral(n) => match &n.value {
            Number::Int(i) => i.as_i64().map(from_int),
            Number::Float(f) => from_f64(*f, kind),
            Number::Complex { .. } => None,
        },
        Expr::UnaryOp(u) if matches!(u.op, UnaryOp::USub) => {
            // -x rounds the opposite way.
            let flipped = match kind {
                BoundKind::RequiredMin => BoundKind::RequiredMax,
                BoundKind::RequiredMax => BoundKind::RequiredMin,
                BoundKind::GuaranteedMin => BoundKind::GuaranteedMax,
                BoundKind::GuaranteedMax => BoundKind::GuaranteedMin,
            };
            literal_bound(&u.operand, flipped).map(|v| -v)
        }
        Expr::UnaryOp(u) if matches!(u.op, UnaryOp::UAdd) => literal_bound(&u.operand, kind),
        Expr::Call(c) => {
            let is_decimal = match c.func.as_ref() {
                Expr::Name(n) => n.id.as_str() == "Decimal",
                Expr::Attribute(a) => a.attr.as_str() == "Decimal",
                _ => false,
            };
            if !is_decimal || c.arguments.args.len() != 1 || !c.arguments.keywords.is_empty() {
                return None;
            }
            match &c.arguments.args[0] {
                Expr::StringLiteral(s) => parse_decimal(s.value.to_str(), kind),
                other => literal_bound(other, kind),
            }
        }
        _ => None,
    }
}

/// `(legacy REAL value, micros column)` of a bound for a contract row.
/// Micros outside the SQLite integer range are clamped when that is
/// conservative for `kind`, and dropped otherwise.
pub fn columns(b: Option<Micros>, kind: BoundKind) -> (Option<f64>, Option<i64>) {
    let Some(b) = b else { return (None, None) };
    let clamped = match i64::try_from(b) {
        Ok(v) => Some(v),
        // A requirement may be made stricter; a guarantee may only be dropped.
        Err(_) => match kind {
            BoundKind::RequiredMax if b > 0 => Some(i64::MAX),
            BoundKind::RequiredMin if b < 0 => Some(i64::MIN),
            _ => None,
        },
    };
    match clamped {
        Some(m) => (Some(m as f64 / SCALE as f64), Some(m)),
        None => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use BoundKind::*;

    #[test]
    fn test_exact_literals() {
        assert_eq!(parse_decimal("0.5", RequiredMax), Some(500_000));
        assert_eq!(parse_decimal("-1.00", GuaranteedMin), Some(-1_000_000));
        assert_eq!(parse_decimal("10", RequiredMax), Some(10_000_000));
        assert_eq!(parse_decimal("1e-4", GuaranteedMax), Some(100));
        assert_eq!(parse_decimal("1.5E2", GuaranteedMax), Some(150_000_000));
        assert_eq!(parse_decimal("1_000", GuaranteedMax), Some(1_000_000_000));
        assert_eq!(parse_decimal("0", RequiredMin), Some(0));
        assert_eq!(parse_decimal("-0.0", RequiredMin), Some(0));
        assert_eq!(parse_decimal("NaN", RequiredMin), None);
        assert_eq!(parse_decimal("0x10", RequiredMin), None);
        assert_eq!(from_f64(0.7, GuaranteedMax), Some(700_000));
        assert_eq!(from_f64(-0.1, GuaranteedMin), Some(-100_000));
        assert_eq!(from_int(-50), -50_000_000);
    }

    #[test]
    fn test_conservative_rounding() {
        // 0.1234567: a requirement is stricter, a guarantee weaker.
        assert_eq!(parse_decimal("0.1234567", RequiredMax), Some(123_456));
        assert_eq!(parse_decimal("0.1234567", RequiredMin), Some(123_457));
        assert_eq!(parse_decimal("0.1234567", GuaranteedMax), Some(123_457));
        assert_eq!(parse_decimal("0.1234567", GuaranteedMin), Some(123_456));
        assert_eq!(parse_decimal("-0.1234567", RequiredMax), Some(-123_457));
        assert_eq!(parse_decimal("-0.1234567", RequiredMin), Some(-123_456));
        assert_eq!(parse_decimal("-0.1234567", GuaranteedMax), Some(-123_456));
        assert_eq!(parse_decimal("-0.1234567", GuaranteedMin), Some(-123_457));
        assert_eq!(parse_decimal("1e-9", GuaranteedMax), Some(1));
        assert_eq!(parse_decimal("1e-9", GuaranteedMin), Some(0));
    }

    #[test]
    fn test_columns_clamp() {
        assert_eq!(columns(Some(1_500_000), RequiredMax), (Some(1.5), Some(1_500_000)));
        let huge = i64::MAX as i128 * 10;
        assert_eq!(columns(Some(huge), RequiredMax).1, Some(i64::MAX));
        assert_eq!(columns(Some(huge), GuaranteedMax), (None, None));
        assert_eq!(columns(Some(-huge), RequiredMin).1, Some(i64::MIN));
        assert_eq!(columns(None, RequiredMin), (None, None));
    }
}
