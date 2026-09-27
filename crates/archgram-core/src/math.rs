//! The few elementary functions colour conversion needs, written with only
//! the operations IEEE 754 rounds exactly (`+ - * /`, `sqrt`, `floor`,
//! `round`) and bit manipulation, so every machine gets the same bits
//! (ARCHITECTURE.md, Invariants). The platform's `powf`, `exp`, `ln`, `sin`
//! and `cos` may differ in their last digit from one libm to the next;
//! clippy refuses them in this crate (`clippy.toml`).
//!
//! Accurate to within a few units in the last place over the ranges colour
//! needs, far below the half of 1/255 that a channel rounds to.

use std::f64::consts::{FRAC_PI_2, LN_2, SQRT_2};

/// `ln 2` split in two, the first part exact in few bits, so `k · ln 2`
/// loses nothing in range reduction (fdlibm's `ln2_hi`, `ln2_lo`).
const LN2_HI: f64 = f64::from_bits(0x3fe6_2e42_fee0_0000);
const LN2_LO: f64 = f64::from_bits(0x3dea_39ef_3579_3c76);

/// `2^k` for a whole `k` in the normal range.
fn two_to(k: i32) -> f64 {
    debug_assert!((-1022..=1023).contains(&k));
    #[allow(clippy::cast_sign_loss)]
    f64::from_bits(((i64::from(k) + 1023) as u64) << 52)
}

/// `e^x`.
#[must_use]
pub fn exp(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x > 709.0 {
        return f64::INFINITY;
    }
    if x < -708.0 {
        return 0.0;
    }
    // x = k·ln 2 + r, |r| ≤ ln 2 / 2; e^r by its Taylor series.
    let k = (x / LN_2).round();
    let r = (x - k * LN2_HI) - k * LN2_LO;
    let mut term = 1.0;
    let mut sum = 1.0;
    for n in 1..=18 {
        term *= r / f64::from(n);
        sum += term;
    }
    #[allow(clippy::cast_possible_truncation)]
    let k = k as i32;
    sum * two_to(k)
}

/// `ln x` for `x > 0`.
#[must_use]
pub fn ln(x: f64) -> f64 {
    if x.is_nan() || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return f64::NEG_INFINITY;
    }
    if x.is_infinite() {
        return x;
    }
    // Subnormals: bring into the normal range first.
    let (x, shift) = if x < f64::MIN_POSITIVE {
        (x * two_to(54), -54)
    } else {
        (x, 0)
    };
    // x = m · 2^e with m in [√½, √2).
    let bits = x.to_bits();
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let mut e = ((bits >> 52) & 0x7ff) as i32 - 1023 + shift;
    let mut m = f64::from_bits((bits & 0x000f_ffff_ffff_ffff) | 0x3ff0_0000_0000_0000);
    if m > SQRT_2 {
        m /= 2.0;
        e += 1;
    }
    // ln m = 2·atanh(s), s = (m − 1)/(m + 1), |s| < 0.18.
    let s = (m - 1.0) / (m + 1.0);
    let s2 = s * s;
    let mut power = s;
    let mut sum = 0.0;
    for k in 0..16 {
        sum += power / f64::from(2 * k + 1);
        power *= s2;
    }
    let e = f64::from(e);
    e * LN2_HI + (e * LN2_LO + 2.0 * sum)
}

/// `x^y` for `x ≥ 0`; `0^y` is 0 for `y > 0`.
#[must_use]
pub fn pow(x: f64, y: f64) -> f64 {
    if x == 0.0 {
        return if y > 0.0 { 0.0 } else { 1.0 };
    }
    exp(y * ln(x))
}

/// The sine and cosine of `degrees`.
#[must_use]
pub fn sin_cos_degrees(degrees: f64) -> (f64, f64) {
    // Into [0, 360), then to the nearest quarter turn: at most 45° off it.
    let d = degrees - 360.0 * (degrees / 360.0).floor();
    let quarter = (d / 90.0).round();
    let r = (d - 90.0 * quarter) * (FRAC_PI_2 / 90.0);
    let r2 = r * r;
    // Taylor series, |r| ≤ π/4.
    let (mut sin, mut cos) = (0.0, 0.0);
    let (mut s_term, mut c_term) = (r, 1.0);
    for n in 0..12 {
        sin += s_term;
        cos += c_term;
        let k = f64::from(2 * n + 2);
        s_term *= -r2 / (k * (k + 1.0));
        c_term *= -r2 / ((k - 1.0) * k);
    }
    #[allow(clippy::cast_possible_truncation)]
    match (quarter as i64).rem_euclid(4) {
        0 => (sin, cos),
        1 => (cos, -sin),
        2 => (-sin, -cos),
        _ => (-cos, sin),
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 4.0 * f64::EPSILON * b.abs().max(1.0)
    }

    #[test]
    fn exp_and_ln_match_the_platform_to_the_last_digits() {
        for i in -400..=400 {
            let x = f64::from(i) / 37.0;
            assert!(close(exp(x), x.exp()), "exp {x}: {} against {}", exp(x), x.exp());
        }
        for i in 1..=2000 {
            let x = f64::from(i) / 997.0;
            assert!(close(ln(x), x.ln()), "ln {x}: {} against {}", ln(x), x.ln());
        }
        assert!(close(ln(1e-310), 1e-310_f64.ln()));
        assert!(close(ln(1e300), 1e300_f64.ln()));
    }

    #[test]
    fn pow_covers_the_transfer_curves() {
        for i in 0..=255 {
            let c = f64::from(i) / 255.0;
            for y in [2.4, 1.0 / 2.4, 3.0, 1.0 / 3.0] {
                let (ours, theirs) = (pow(c, y), c.powf(y));
                assert!((ours - theirs).abs() < 1e-14, "{c}^{y}: {ours} against {theirs}");
            }
        }
    }

    #[test]
    fn sine_and_cosine_in_degrees() {
        for i in -1440..=1440 {
            let d = f64::from(i) / 2.0;
            let (s, c) = sin_cos_degrees(d);
            let r = d.to_radians();
            assert!((s - r.sin()).abs() < 1e-14, "sin {d}");
            assert!((c - r.cos()).abs() < 1e-14, "cos {d}");
        }
    }
}
