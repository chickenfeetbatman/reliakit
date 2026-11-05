//! Special functions implemented in-crate: log gamma, regularized
//! incomplete gamma and beta, normal CDF and quantile, error function.
//! Algorithms are the standard Numerical Recipes series and continued
//! fraction forms, plus Acklam's rational approximation for the normal
//! quantile. No external statistics crate is needed.

/// Natural log of the gamma function (Lanczos approximation).
pub fn ln_gamma(x: f64) -> f64 {
    const COEF: [f64; 9] = [
        0.99999999999980993,
        676.5203681218851,
        -1259.1392167224028,
        771.32342877765313,
        -176.61502916214059,
        12.507343278686905,
        -0.13857109526572012,
        9.9843695780195716e-6,
        1.5056327351493116e-7,
    ];
    if x < 0.5 {
        // Reflection formula.
        std::f64::consts::PI.ln() - (std::f64::consts::PI * x).sin().ln() - ln_gamma(1.0 - x)
    } else {
        let z = x - 1.0;
        let mut a = COEF[0];
        for (i, c) in COEF.iter().enumerate().skip(1) {
            a += c / (z + i as f64);
        }
        let t = z + 7.5;
        0.5 * (2.0 * std::f64::consts::PI).ln() + (z + 0.5) * t.ln() - t + a.ln()
    }
}

fn gamma_p_series(a: f64, x: f64) -> f64 {
    // Series representation for P(a, x), good for x < a + 1.
    let mut ap = a;
    let mut sum = 1.0 / a;
    let mut del = sum;
    for _ in 0..500 {
        ap += 1.0;
        del *= x / ap;
        sum += del;
        if del.abs() < sum.abs() * 1e-15 {
            break;
        }
    }
    sum * (-x + a * x.ln() - ln_gamma(a)).exp()
}

fn gamma_q_cf(a: f64, x: f64) -> f64 {
    // Continued fraction for Q(a, x), good for x >= a + 1.
    let tiny = 1e-300;
    let mut b = x + 1.0 - a;
    let mut c = 1.0 / tiny;
    let mut d = 1.0 / b.max(tiny);
    let mut h = d;
    for i in 1..500 {
        let an = -(i as f64) * (i as f64 - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < tiny {
            d = tiny;
        }
        c = b + an / c;
        if c.abs() < tiny {
            c = tiny;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < 1e-15 {
            break;
        }
    }
    (-x + a * x.ln() - ln_gamma(a)).exp() * h
}

/// Regularized lower incomplete gamma P(a, x).
pub fn gamma_p(a: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x < a + 1.0 {
        gamma_p_series(a, x)
    } else {
        1.0 - gamma_q_cf(a, x)
    }
}

/// Regularized upper incomplete gamma Q(a, x) = 1 - P(a, x).
pub fn gamma_q(a: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 1.0;
    }
    if x < a + 1.0 {
        1.0 - gamma_p_series(a, x)
    } else {
        gamma_q_cf(a, x)
    }
}

/// Quantile of the gamma distribution by bisection on gamma_p.
pub fn gamma_ppf(a: f64, scale: f64, p: f64) -> f64 {
    if p <= 0.0 {
        return 0.0;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    let mut lo = 0.0;
    let mut hi = (a * scale).max(scale);
    while gamma_p(a, hi / scale) < p && hi < 1e300 {
        hi *= 2.0;
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if gamma_p(a, mid / scale) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

fn beta_cf(a: f64, b: f64, x: f64) -> f64 {
    // Continued fraction for the incomplete beta function.
    let tiny = 1e-300;
    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < tiny {
        d = tiny;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..500 {
        let m2 = 2 * m;
        let mf = m as f64;
        let mut aa = mf * (b - mf) * x / ((qam + m2 as f64) * (a + m2 as f64));
        d = 1.0 + aa * d;
        if d.abs() < tiny {
            d = tiny;
        }
        c = 1.0 + aa / c;
        if c.abs() < tiny {
            c = tiny;
        }
        d = 1.0 / d;
        h *= d * c;
        aa = -(a + mf) * (qab + mf) * x / ((a + m2 as f64) * (qap + m2 as f64));
        d = 1.0 + aa * d;
        if d.abs() < tiny {
            d = tiny;
        }
        c = 1.0 + aa / c;
        if c.abs() < tiny {
            c = tiny;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < 3e-15 {
            break;
        }
    }
    h
}

/// Regularized incomplete beta I_x(a, b).
pub fn beta_i(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let bt = (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b)
        + a * x.ln()
        + b * (1.0 - x).ln())
    .exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        bt * beta_cf(a, b, x) / a
    } else {
        1.0 - bt * beta_cf(b, a, 1.0 - x) / b
    }
}

/// Quantile of the beta distribution by bisection on beta_i.
pub fn beta_ppf(a: f64, b: f64, p: f64) -> f64 {
    if p <= 0.0 {
        return 0.0;
    }
    if p >= 1.0 {
        return 1.0;
    }
    let mut lo = 0.0;
    let mut hi = 1.0;
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if beta_i(a, b, mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Error function (Abramowitz and Stegun 7.1.26, abs error <= 1.5e-7).
pub fn erf(x: f64) -> f64 {
    // erf(x) = sign(x) * P(1/2, x^2); the incomplete gamma routines are
    // accurate to about 1e-14, far better than a fixed rational fit.
    if x == 0.0 {
        return 0.0;
    }
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    sign * gamma_p(0.5, x * x)
}

/// Standard normal CDF.
pub fn norm_cdf(x: f64) -> f64 {
    // Phi(x) via erfc: erfc(z) = Q(1/2, z^2).
    let z = x / std::f64::consts::SQRT_2;
    if x >= 0.0 {
        1.0 - 0.5 * gamma_q(0.5, z * z)
    } else {
        0.5 * gamma_q(0.5, z * z)
    }
}

/// Standard normal survival function.
pub fn norm_sf(x: f64) -> f64 {
    1.0 - norm_cdf(x)
}

/// Standard normal quantile (Acklam's rational approximation).
pub fn norm_ppf(p: f64) -> f64 {
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    const A: [f64; 6] = [
        -3.969683028665376e01,
        2.209460984245205e02,
        -2.759285104469687e02,
        1.383577518672690e02,
        -3.066479806614716e01,
        2.506628277459239e00,
    ];
    const B: [f64; 5] = [
        -5.447609879822406e01,
        1.615858368580409e02,
        -1.556989798598866e02,
        6.680131188771972e01,
        -1.328068155288572e01,
    ];
    const C: [f64; 6] = [
        -7.784894002430293e-03,
        -3.223964580411365e-01,
        -2.400758277161838e00,
        -2.549732539343734e00,
        4.374664141464968e00,
        2.938163982698783e00,
    ];
    const D: [f64; 4] = [
        7.784695709041462e-03,
        3.224671290700398e-01,
        2.445134137142996e00,
        3.754408661907416e00,
    ];
    let p_low = 0.02425;
    let p_high = 1.0 - p_low;
    if p < p_low {
        let q = (-2.0 * p.ln()).sqrt();
        return (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0);
    }
    if p > p_high {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        return -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0);
    }
    let q = p - 0.5;
    let r = q * q;
    (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
        / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
}
