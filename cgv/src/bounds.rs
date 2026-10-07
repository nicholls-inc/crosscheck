//! Exact numeric bounds for range contracts.
//!
//! A bound is an exact decimal (`Dec`): the literal as written, with
//! exponent notation normalised (`1e-4` is `0.0001`). Contract rows carry it
//! three ways (`docs/design/dataflow-v2.md`, "Exact numeric bounds" and
//! "Round 5"):
//!
//! - `param_min_decimal` / `param_max_decimal`: the exact decimal string;
//! - `param_min_micros` / `param_max_micros`: the value times 10^6, exact
//!   when it has at most 6 decimal places, otherwise rounded in the
//!   conservative direction for its role (a requirement rounds to the
//!   stricter side, a guarantee to the weaker side); NULL when it does not
//!   fit a 64-bit integer;
//! - `param_min_value` / `param_max_value`: the REAL, for legacy readers.

use std::cmp::Ordering;
use std::fmt;

/// Value times 10^6.
pub type Micros = i128;

pub const SCALE: i128 = 1_000_000;

/// Most decimal places kept exactly. Literals with more are not bounds.
const MAX_SCALE: u32 = 30;

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

/// An exact decimal `m × 10^-s`, normalised (no trailing zero digits in
/// `m` when `s > 0`), so structural equality is numeric equality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dec {
    m: i128,
    s: u32,
}

fn pow10(e: u32) -> Option<i128> {
    10i128.checked_pow(e)
}

impl Dec {
    pub const ZERO: Dec = Dec { m: 0, s: 0 };

    fn new(mut m: i128, mut s: u32) -> Dec {
        while s > 0 && m % 10 == 0 {
            m /= 10;
            s -= 1;
        }
        if m == 0 {
            s = 0;
        }
        Dec { m, s }
    }

    pub fn from_int(v: i64) -> Dec {
        Dec::new(v as i128, 0)
    }

    /// The value `micros / 10^6` (for tests and step arithmetic).
    pub fn from_micros(micros: i128) -> Dec {
        Dec::new(micros, 6)
    }

    /// Integer part (towards negative infinity) and the non-negative fraction digits.
    fn split(self) -> (i128, i128) {
        let p = pow10(self.s).expect("scale is bounded");
        (self.m.div_euclid(p), self.m.rem_euclid(p))
    }

    /// The largest integer at most `self`.
    pub fn floor(self) -> Dec {
        Dec::new(self.split().0, 0)
    }

    /// The smallest integer at least `self`.
    pub fn ceil(self) -> Dec {
        let (i, f) = self.split();
        Dec::new(if f == 0 { i } else { i + 1 }, 0)
    }

    /// `self + other`, when it is representable.
    pub fn checked_add(self, other: Dec) -> Option<Dec> {
        let s = self.s.max(other.s);
        let a = self.m.checked_mul(pow10(s - self.s)?)?;
        let b = other.m.checked_mul(pow10(s - other.s)?)?;
        Some(Dec::new(a.checked_add(b)?, s))
    }

    /// The value times 10^6, rounded as `kind` requires when it has more
    /// than 6 decimal places; `None` beyond `i128`.
    pub fn to_micros(self, kind: BoundKind) -> Option<Micros> {
        if self.s <= 6 {
            return self.m.checked_mul(pow10(6 - self.s)?);
        }
        let p = pow10(self.s - 6)?;
        let q = self.m.div_euclid(p); // towards negative infinity
        let exact = self.m.rem_euclid(p) == 0;
        Some(if exact || !kind.rounds_up() { q } else { q + 1 })
    }

    /// The largest integer at most `self`, when it fits an `i64`.
    pub fn to_i64(self) -> Option<i64> {
        i64::try_from(self.split().0).ok()
    }

    /// `m × 10^-s`, when `s` is at most `MAX_SCALE`.
    pub fn from_scaled(m: i128, s: u32) -> Option<Dec> {
        (s <= MAX_SCALE).then(|| Dec::new(m, s))
    }

    /// Nearest `f64` (for the legacy REAL columns).
    pub fn to_f64(self) -> f64 {
        // Via the decimal string: exact parsing, correctly rounded.
        self.to_string().parse().unwrap_or(f64::NAN)
    }
}

impl Ord for Dec {
    fn cmp(&self, other: &Self) -> Ordering {
        let (ai, af) = self.split();
        let (bi, bf) = other.split();
        ai.cmp(&bi).then_with(|| {
            // Both fractions are below 10^s <= 10^MAX_SCALE: aligning them fits i128.
            let s = self.s.max(other.s);
            let a = af * pow10(s - self.s).expect("bounded");
            let b = bf * pow10(s - other.s).expect("bounded");
            a.cmp(&b)
        })
    }
}

impl PartialOrd for Dec {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::ops::Neg for Dec {
    type Output = Dec;
    fn neg(self) -> Dec {
        Dec::new(-self.m, self.s)
    }
}

/// `-?digits(.digits)?`, no exponent: the `param_*_decimal` encoding.
impl fmt::Display for Dec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.m < 0 { "-" } else { "" };
        let digits = self.m.unsigned_abs().to_string();
        if self.s == 0 {
            return write!(f, "{sign}{digits}");
        }
        let s = self.s as usize;
        let padded = if digits.len() <= s {
            format!("{}{digits}", "0".repeat(s - digits.len() + 1))
        } else {
            digits
        };
        let (int, frac) = padded.split_at(padded.len() - s);
        write!(f, "{sign}{int}.{frac}")
    }
}

/// Parse a decimal literal (`-12.5`, `1e-4`, `0.1234567`, `1_000`) exactly.
/// `None` for anything else (`NaN`, `Infinity`, hex) and for values with
/// more than 30 decimal places or beyond 38 significant digits.
pub fn parse_decimal(text: &str) -> Option<Dec> {
    let t = text.trim().replace('_', "");
    let (neg, t) = match t.strip_prefix('-') {
        Some(rest) => (true, rest.to_string()),
        None => (false, t.strip_prefix('+').unwrap_or(&t).to_string()),
    };
    let (mantissa, exp) = match t.find(['e', 'E']) {
        Some(i) => (&t[..i], t[i + 1..].parse::<i64>().ok()?),
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
    let digits = format!("{int_part}{frac}");
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return Some(Dec::ZERO);
    }
    let digits = {
        // Trailing zeros only move the exponent.
        let trimmed = digits.trim_end_matches('0');
        (trimmed, (digits.len() - trimmed.len()) as i64)
    };
    let shift = exp.checked_sub(frac.len() as i64)?.checked_add(digits.1)?;
    if digits.0.len() > 38 {
        return None;
    }
    let d: i128 = digits.0.parse().ok()?;
    let d = if neg { -d } else { d };
    if shift >= 0 {
        let m = d.checked_mul(pow10(u32::try_from(shift).ok()?)?)?;
        Some(Dec::new(m, 0))
    } else {
        let s = u32::try_from(-shift).ok()?;
        (s <= MAX_SCALE).then(|| Dec::new(d, s))
    }
}

/// A bound from an integer.
pub fn from_int(v: i64) -> Dec {
    Dec::from_int(v)
}

/// A Python float, via its shortest round-trip representation (what
/// `repr` prints, and in practice what the source says).
pub fn from_f64(v: f64) -> Option<Dec> {
    if !v.is_finite() {
        return None;
    }
    parse_decimal(&format!("{v}"))
}

/// A numeric literal bound in source: `5`, `-1`, `0.5`, `10_000_000_000_000`,
/// `Decimal("10")`, `decimal.Decimal("0.01")`, `Decimal(3)`.
pub fn literal_bound(expr: &ruff_python_ast::Expr) -> Option<Dec> {
    use ruff_python_ast::{Expr, Number, UnaryOp};
    match expr {
        Expr::NumberLiteral(n) => match &n.value {
            Number::Int(i) => match i.as_i64() {
                Some(v) => Some(from_int(v)),
                // Beyond i64: the literal's digits.
                None => parse_decimal(&i.to_string()),
            },
            Number::Float(f) => from_f64(*f),
            Number::Complex { .. } => None,
        },
        Expr::UnaryOp(u) if matches!(u.op, UnaryOp::USub) => literal_bound(&u.operand).map(|v| -v),
        Expr::UnaryOp(u) if matches!(u.op, UnaryOp::UAdd) => literal_bound(&u.operand),
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
                Expr::StringLiteral(s) => parse_decimal(s.value.to_str()),
                // `Decimal(0.5)` is the float's exact binary value.
                Expr::NumberLiteral(n) if matches!(n.value, Number::Float(_)) => {
                    let Number::Float(f) = n.value else { return None };
                    crate::value_analysis::float_exact(f).and_then(|(_, v)| v)
                }
                other => literal_bound(other),
            }
        }
        _ => None,
    }
}

/// The largest magnitude a value with at most `max_digits` digits, at most
/// `decimal_places` of them fractional, can have: `max_digits` nines with the
/// last `decimal_places` after the point (`999.99` for 5 and 2). Without
/// `decimal_places` the value may have no fractional digit and still be
/// accepted, so the limit is `max_digits` whole nines. `None` for a
/// `max_digits` below 1 or above 38 (beyond `i128`), and for a
/// `decimal_places` below 0, above `max_digits` or above 30 (`MAX_SCALE`).
pub fn digits_limit(max_digits: i64, decimal_places: Option<i64>) -> Option<Dec> {
    let places = decimal_places.unwrap_or(0);
    if max_digits < 1 || places < 0 || places > max_digits {
        return None;
    }
    let nines = pow10(u32::try_from(max_digits).ok()?)? - 1;
    Dec::from_scaled(nines, u32::try_from(places).ok()?)
}

/// Narrow `min` and `max` to the magnitudes that `max_digits` and
/// `decimal_places` allow (Django's `DecimalValidator` and pydantic reject
/// more whole digits than `max_digits - decimal_places`). Unchanged when
/// `max_digits` is unknown or `digits_limit` gives no limit.
pub fn narrow_to_digits(
    max_digits: Option<i64>,
    decimal_places: Option<i64>,
    min: &mut Option<Dec>,
    max: &mut Option<Dec>,
) {
    let Some(limit) = max_digits.and_then(|m| digits_limit(m, decimal_places)) else {
        return;
    };
    *min = Some(min.map_or(-limit, |m| m.max(-limit)));
    *max = Some(max.map_or(limit, |m| m.min(limit)));
}

/// The three column values of one bound of a contract row.
#[derive(Debug, Clone, PartialEq)]
pub struct Columns {
    /// Legacy REAL copy.
    pub value: Option<f64>,
    /// Times 10^6, rounded for `kind`; `None` when it does not fit an `i64`.
    pub micros: Option<i64>,
    /// Exact decimal string.
    pub decimal: Option<String>,
}

/// Column values of a bound for a contract row.
pub fn columns(b: Option<Dec>, kind: BoundKind) -> Columns {
    let Some(b) = b else {
        return Columns {
            value: None,
            micros: None,
            decimal: None,
        };
    };
    Columns {
        value: Some(b.to_f64()),
        micros: b.to_micros(kind).and_then(|m| i64::try_from(m).ok()),
        decimal: Some(b.to_string()),
    }
}

/// `Some(micros / 10^6)`, for tests.
#[cfg(test)]
pub fn mu(micros: i128) -> Option<Dec> {
    Some(Dec::from_micros(micros))
}

#[cfg(test)]
mod tests {
    use super::*;
    use BoundKind::*;

    fn d(text: &str) -> Dec {
        parse_decimal(text).unwrap()
    }

    #[test]
    fn test_digits_limit() {
        assert_eq!(digits_limit(5, Some(2)), Some(d("999.99")));
        assert_eq!(digits_limit(5, Some(0)), Some(d("99999")));
        assert_eq!(digits_limit(5, None), Some(d("99999")));
        assert_eq!(digits_limit(3, Some(3)), Some(d("0.999")));
        assert_eq!(digits_limit(1, Some(1)), Some(d("0.9")));
        assert_eq!(digits_limit(30, Some(30)).map(|l| l.to_string()), Some(format!("0.{}", "9".repeat(30))));
        assert_eq!(digits_limit(2, Some(3)), None);
        assert_eq!(digits_limit(0, None), None);
        assert_eq!(digits_limit(5, Some(-1)), None);
        assert_eq!(digits_limit(39, None), None);
        assert_eq!(digits_limit(31, Some(31)), None);
    }

    #[test]
    fn test_narrow_to_digits() {
        let narrowed = |m, places, lo: Option<&str>, hi: Option<&str>| {
            let (mut lo, mut hi) = (lo.map(d), hi.map(d));
            narrow_to_digits(m, places, &mut lo, &mut hi);
            (lo, hi)
        };
        assert_eq!(narrowed(Some(5), Some(2), None, None), (Some(d("-999.99")), Some(d("999.99"))));
        assert_eq!(narrowed(Some(5), Some(2), Some("0"), Some("5000")), (Some(d("0")), Some(d("999.99"))));
        assert_eq!(narrowed(Some(5), Some(2), Some("-5000"), Some("10")), (Some(d("-999.99")), Some(d("10"))));
        assert_eq!(narrowed(None, Some(2), Some("1"), None), (Some(d("1")), None));
        assert_eq!(narrowed(Some(2), Some(3), None, Some("7")), (None, Some(d("7"))));
    }

    #[test]
    fn test_exact_literals() {
        assert_eq!(d("0.5"), Dec::from_micros(500_000));
        assert_eq!(d("-1.00"), Dec::from_int(-1));
        assert_eq!(d("10"), Dec::from_int(10));
        assert_eq!(d("1e-4").to_string(), "0.0001");
        assert_eq!(d("1.5E2").to_string(), "150");
        assert_eq!(d("1_000").to_string(), "1000");
        assert_eq!(d("0"), Dec::ZERO);
        assert_eq!(d("-0.0"), Dec::ZERO);
        assert_eq!(d("0.1234567").to_string(), "0.1234567");
        assert_eq!(d("-0.050").to_string(), "-0.05");
        assert_eq!(d("12e3").to_string(), "12000");
        assert_eq!(d("1E+2").to_string(), "100");
        assert_eq!(d("10000000000000").to_string(), "10000000000000");
        assert_eq!(parse_decimal("NaN"), None);
        assert_eq!(parse_decimal("0x10"), None);
        assert_eq!(parse_decimal("1e-40"), None);
        assert_eq!(from_f64(0.7), Some(d("0.7")));
        assert_eq!(from_f64(-0.1), Some(d("-0.1")));
        assert_eq!(from_int(-50).to_string(), "-50");
    }

    #[test]
    fn test_order_floor_ceil_add() {
        assert!(d("0.1234567") > d("0.123456"));
        assert!(d("-0.5") < d("-0.49"));
        assert!(d("1e20") > d("99999999999999999999.99999999"));
        assert_eq!(d("2.5").floor(), d("2"));
        assert_eq!(d("-2.5").floor(), d("-3"));
        assert_eq!(d("2.5").ceil(), d("3"));
        assert_eq!(d("2").ceil(), d("2"));
        assert_eq!(d("0.1").checked_add(d("0.25")), Some(d("0.35")));
        assert_eq!((-d("0.35")).to_string(), "-0.35");
    }

    #[test]
    fn test_conservative_micros() {
        // 0.1234567: a requirement is stricter, a guarantee weaker.
        assert_eq!(d("0.1234567").to_micros(RequiredMax), Some(123_456));
        assert_eq!(d("0.1234567").to_micros(RequiredMin), Some(123_457));
        assert_eq!(d("0.1234567").to_micros(GuaranteedMax), Some(123_457));
        assert_eq!(d("0.1234567").to_micros(GuaranteedMin), Some(123_456));
        assert_eq!(d("-0.1234567").to_micros(RequiredMax), Some(-123_457));
        assert_eq!(d("-0.1234567").to_micros(RequiredMin), Some(-123_456));
        assert_eq!(d("-0.1234567").to_micros(GuaranteedMax), Some(-123_456));
        assert_eq!(d("-0.1234567").to_micros(GuaranteedMin), Some(-123_457));
        assert_eq!(d("1e-9").to_micros(GuaranteedMax), Some(1));
        assert_eq!(d("1e-9").to_micros(GuaranteedMin), Some(0));
        assert_eq!(d("0.5").to_micros(RequiredMax), Some(500_000));
    }

    #[test]
    fn test_columns() {
        let c = columns(Some(d("1.5")), RequiredMax);
        assert_eq!(
            c,
            Columns {
                value: Some(1.5),
                micros: Some(1_500_000),
                decimal: Some("1.5".to_string())
            }
        );
        // 10^13 overflows i64 micros: micros NULL, the decimal stays exact.
        let c = columns(Some(d("-10000000000000")), RequiredMin);
        assert_eq!((c.micros, c.decimal.as_deref()), (None, Some("-10000000000000")));
        assert_eq!(c.value, Some(-1e13));
        let c = columns(Some(d("0.1234567")), GuaranteedMax);
        assert_eq!((c.micros, c.decimal.as_deref()), (Some(123_457), Some("0.1234567")));
        assert_eq!(columns(None, RequiredMin).decimal, None);
    }
}
