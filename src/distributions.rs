//! Life-data distributions and fitting (Weibull++ equivalent core).
//!
//! Supports 2-parameter and 3-parameter Weibull, lognormal, normal,
//! exponential, gamma and Gumbel distributions, plus the v0.2.0 additions
//! below. Fitting is by maximum likelihood with the exact
//! right-censored (suspended) likelihood, or by median-rank regression
//! where defined. Confidence bounds use the observed Fisher information
//! (inverse Hessian of the negative log-likelihood). Ported from the
//! Python ReliaKit distributions module with the same math.
//!
//! Added distributions and parameterizations:
//! - loglogistic: scale alpha > 0, shape beta > 0.
//!   F(t) = 1 / (1 + (t / alpha)^-beta) for t > 0.
//! - logistic: location mu (real), scale s > 0.
//!   F(t) = 1 / (1 + exp(-(t - mu) / s)).
//! - gompertz: shape a > 0, rate b > 0, in the life-data form with
//!   hazard h(t) = a * exp(b * t), reliability
//!   R(t) = exp(-(a / b) * (exp(b * t) - 1)) and pdf f(t) = h(t) * R(t).
//!   The b -> 0 limit is the exponential distribution; it is not
//!   special-cased.
//! - gengamma: generalized gamma (Stacy form) with scale a > 0 and
//!   shapes d > 0, p > 0. pdf
//!   f(t) = (p / a^d) * t^(d-1) * exp(-(t / a)^p) / Gamma(d / p).
//!   It reduces to the gamma distribution (shape d, scale a) at p = 1
//!   and to the Weibull distribution (shape p, scale a) at d = p.
//! - weibull1: one-parameter Weibull with shape beta > 0 and scale
//!   eta > 0 in the spec so the closed forms work, but fitting needs
//!   beta fixed by the user; see fit_weibull1.
//! - exponential2: two-parameter exponential with mean theta > 0 and
//!   location gamma (support t >= gamma).
//! - mixed_weibull: mixture of two weibull2 subpopulations with weight
//!   w in (0, 1) on subpopulation 1 and params (beta1, eta1) and
//!   (beta2, eta2), in spec order [w, beta1, eta1, beta2, eta2].

use crate::optim;
use crate::specfunc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type Params = BTreeMap<String, f64>;

pub fn params_of(pairs: &[(&str, f64)]) -> Params {
    pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

/// (name, kind) with kind in {"pos", "real", "loc", "prob"} per
/// distribution. "prob" is a probability in (0, 1) handled with a logit
/// transform in the unconstrained fitting space; only mixed_weibull
/// uses it (for the mixture weight w).
pub fn dist_spec(dist: &str) -> Option<Vec<(&'static str, &'static str)>> {
    match dist {
        "weibull2" => Some(vec![("beta", "pos"), ("eta", "pos")]),
        "weibull3" => Some(vec![("beta", "pos"), ("eta", "pos"), ("gamma", "loc")]),
        "lognormal" => Some(vec![("mu", "real"), ("sigma", "pos")]),
        "normal" => Some(vec![("mu", "real"), ("sigma", "pos")]),
        "exponential" => Some(vec![("theta", "pos")]),
        "gamma" => Some(vec![("alpha", "pos"), ("beta", "pos")]),
        "gumbel" => Some(vec![("mu", "real"), ("beta", "pos")]),
        "loglogistic" => Some(vec![("alpha", "pos"), ("beta", "pos")]),
        "logistic" => Some(vec![("mu", "real"), ("s", "pos")]),
        "gompertz" => Some(vec![("a", "pos"), ("b", "pos")]),
        "gengamma" => Some(vec![("a", "pos"), ("d", "pos"), ("p", "pos")]),
        "weibull1" => Some(vec![("beta", "pos"), ("eta", "pos")]),
        "exponential2" => Some(vec![("theta", "pos"), ("gamma", "loc")]),
        "mixed_weibull" => Some(vec![
            ("w", "prob"),
            ("beta1", "pos"),
            ("eta1", "pos"),
            ("beta2", "pos"),
            ("eta2", "pos"),
        ]),
        _ => None,
    }
}

pub fn dist_label(dist: &str) -> &'static str {
    match dist {
        "weibull2" => "Weibull (2P)",
        "weibull3" => "Weibull (3P)",
        "lognormal" => "Lognormal",
        "normal" => "Normal",
        "exponential" => "Exponential",
        "gamma" => "Gamma",
        "gumbel" => "Gumbel (smallest extreme value)",
        "loglogistic" => "Log-logistic",
        "logistic" => "Logistic",
        "gompertz" => "Gompertz",
        "gengamma" => "Generalized gamma",
        "weibull1" => "Weibull (1P)",
        "exponential2" => "Exponential (2P)",
        "mixed_weibull" => "Mixed Weibull (2 subpopulations)",
        _ => "unknown",
    }
}

fn get(p: &Params, k: &str) -> f64 {
    *p.get(k).unwrap_or(&f64::NAN)
}

/// ln(1 + exp(x)) evaluated without overflow for large x.
fn log1pexp(x: f64) -> f64 {
    if x > 0.0 {
        x + (-x).exp().ln_1p()
    } else {
        x.exp().ln_1p()
    }
}

/// Standard logistic CDF 1 / (1 + exp(-z)), stable for large |z|.
fn logistic_cdf_std(z: f64) -> f64 {
    if z >= 0.0 {
        1.0 / (1.0 + (-z).exp())
    } else {
        let e = z.exp();
        e / (1.0 + e)
    }
}

/// Exponential integral E1(x) = integral_x^inf exp(-u) / u du, for
/// x > 0. Series for x < 1 (Numerical Recipes), continued fraction
/// for x >= 1. Used for the Gompertz mean life.
fn expint_e1(x: f64) -> f64 {
    const EULER_GAMMA: f64 = std::f64::consts::EULER_GAMMA;
    if x <= 0.0 {
        return f64::INFINITY;
    }
    if x < 1.0 {
        // E1(x) = -gamma - ln(x) - sum_{k>=1} (-x)^k / (k * k!)
        let mut sum = 0.0;
        let mut term = 1.0; // (-x)^k / k! built iteratively
        for k in 1..200usize {
            term *= -x / k as f64;
            let add = term / k as f64;
            sum += add;
            if add.abs() < 1e-16 * sum.abs().max(1e-300) {
                break;
            }
        }
        -EULER_GAMMA - x.ln() - sum
    } else {
        // Continued fraction (Numerical Recipes expint, E1 branch).
        let tiny = 1e-300;
        let mut b = x + 1.0;
        let mut c = 1.0 / tiny;
        let mut d = 1.0 / b;
        let mut h = d;
        for i in 1..500usize {
            let a = -((i * i) as f64);
            b += 2.0;
            d = 1.0 / (a * d + b);
            c = b + a / c;
            if c == 0.0 {
                c = tiny;
            }
            let del = c * d;
            h *= del;
            if (del - 1.0).abs() < 1e-15 {
                break;
            }
        }
        (-x).exp() * h
    }
}

fn weibull_pdf(beta: f64, eta: f64, t: f64) -> f64 {
    if t <= 0.0 || beta <= 0.0 || eta <= 0.0 {
        return 0.0;
    }
    (beta / eta) * (t / eta).powf(beta - 1.0) * (-(t / eta).powf(beta)).exp()
}

fn weibull_cdf(beta: f64, eta: f64, t: f64) -> f64 {
    if t <= 0.0 {
        0.0
    } else {
        1.0 - (-(t / eta).powf(beta)).exp()
    }
}

pub fn pdf(dist: &str, p: &Params, t: f64) -> f64 {
    match dist {
        "weibull2" | "weibull3" => {
            let (beta, eta) = (get(p, "beta"), get(p, "eta"));
            let x = if dist == "weibull3" {
                t - get(p, "gamma")
            } else {
                t
            };
            if x <= 0.0 || beta <= 0.0 || eta <= 0.0 {
                return 0.0;
            }
            (beta / eta) * (x / eta).powf(beta - 1.0) * (-(x / eta).powf(beta)).exp()
        }
        "lognormal" => {
            let (mu, sigma) = (get(p, "mu"), get(p, "sigma"));
            if t <= 0.0 {
                return 0.0;
            }
            let z = (t.ln() - mu) / sigma;
            (-0.5 * z * z).exp() / (t * sigma * (2.0 * std::f64::consts::PI).sqrt())
        }
        "normal" => {
            let (mu, sigma) = (get(p, "mu"), get(p, "sigma"));
            let z = (t - mu) / sigma;
            (-0.5 * z * z).exp() / (sigma * (2.0 * std::f64::consts::PI).sqrt())
        }
        "exponential" => {
            let theta = get(p, "theta");
            if t < 0.0 {
                0.0
            } else {
                (-t / theta).exp() / theta
            }
        }
        "gamma" => {
            let (a, b) = (get(p, "alpha"), get(p, "beta"));
            if t <= 0.0 {
                return 0.0;
            }
            ((a - 1.0) * t.ln() - t / b - a * b.ln() - specfunc::ln_gamma(a)).exp()
        }
        "gumbel" => {
            // scipy gumbel_r (largest extreme value)
            let (mu, beta) = (get(p, "mu"), get(p, "beta"));
            let z = (t - mu) / beta;
            (-z - (-z).exp()).exp() / beta
        }
        "loglogistic" => {
            let (alpha, beta) = (get(p, "alpha"), get(p, "beta"));
            if t <= 0.0 || alpha <= 0.0 || beta <= 0.0 {
                return 0.0;
            }
            // log f = ln beta - ln alpha + (beta - 1)(ln t - ln alpha)
            //         - 2 ln(1 + exp(beta (ln t - ln alpha)))
            let y = beta * (t.ln() - alpha.ln());
            (beta.ln() - alpha.ln() + (beta - 1.0) * (t.ln() - alpha.ln()) - 2.0 * log1pexp(y))
                .exp()
        }
        "logistic" => {
            let (mu, s) = (get(p, "mu"), get(p, "s"));
            if s <= 0.0 {
                return 0.0;
            }
            // Symmetric form with |z| avoids overflow:
            // f = exp(-|z|) / (s (1 + exp(-|z|))^2), z = (t - mu) / s.
            let az = ((t - mu) / s).abs();
            let e = (-az).exp();
            e / (s * (1.0 + e) * (1.0 + e))
        }
        "gompertz" => {
            let (a, b) = (get(p, "a"), get(p, "b"));
            if t < 0.0 || a <= 0.0 || b <= 0.0 {
                return 0.0;
            }
            if b * t > 700.0 {
                return 0.0;
            }
            // f = h R with log f = ln a + b t - (a / b)(exp(b t) - 1).
            (a.ln() + b * t - (a / b) * ((b * t).exp() - 1.0)).exp()
        }
        "gengamma" => {
            let (a, d, pp) = (get(p, "a"), get(p, "d"), get(p, "p"));
            if t <= 0.0 || a <= 0.0 || d <= 0.0 || pp <= 0.0 {
                return 0.0;
            }
            // log f = ln p - d ln a + (d - 1) ln t - (t / a)^p
            //         - ln Gamma(d / p)
            let logf = pp.ln() - d * a.ln() + (d - 1.0) * t.ln()
                - (t / a).powf(pp)
                - specfunc::ln_gamma(d / pp);
            logf.exp()
        }
        "weibull1" => {
            let (beta, eta) = (get(p, "beta"), get(p, "eta"));
            weibull_pdf(beta, eta, t)
        }
        "exponential2" => {
            let (theta, gamma) = (get(p, "theta"), get(p, "gamma"));
            let x = t - gamma;
            if x < 0.0 || theta <= 0.0 {
                0.0
            } else {
                (-x / theta).exp() / theta
            }
        }
        "mixed_weibull" => {
            let w = get(p, "w");
            w * weibull_pdf(get(p, "beta1"), get(p, "eta1"), t)
                + (1.0 - w) * weibull_pdf(get(p, "beta2"), get(p, "eta2"), t)
        }
        _ => f64::NAN,
    }
}

pub fn cdf(dist: &str, p: &Params, t: f64) -> f64 {
    match dist {
        "weibull2" | "weibull3" => {
            let (beta, eta) = (get(p, "beta"), get(p, "eta"));
            let x = if dist == "weibull3" {
                t - get(p, "gamma")
            } else {
                t
            };
            if x <= 0.0 {
                0.0
            } else {
                1.0 - (-(x / eta).powf(beta)).exp()
            }
        }
        "lognormal" => {
            if t <= 0.0 {
                0.0
            } else {
                specfunc::norm_cdf((t.ln() - get(p, "mu")) / get(p, "sigma"))
            }
        }
        "normal" => specfunc::norm_cdf((t - get(p, "mu")) / get(p, "sigma")),
        "exponential" => {
            if t < 0.0 {
                0.0
            } else {
                1.0 - (-t / get(p, "theta")).exp()
            }
        }
        "gamma" => {
            if t <= 0.0 {
                0.0
            } else {
                specfunc::gamma_p(get(p, "alpha"), t / get(p, "beta"))
            }
        }
        "gumbel" => {
            let z = (t - get(p, "mu")) / get(p, "beta");
            (-(-z).exp()).exp()
        }
        "loglogistic" => {
            let (alpha, beta) = (get(p, "alpha"), get(p, "beta"));
            if t <= 0.0 {
                0.0
            } else {
                // F = 1 / (1 + (t / alpha)^-beta), a logistic CDF in ln t.
                logistic_cdf_std(beta * (t.ln() - alpha.ln()))
            }
        }
        "logistic" => {
            let (mu, s) = (get(p, "mu"), get(p, "s"));
            logistic_cdf_std((t - mu) / s)
        }
        "gompertz" => {
            let (a, b) = (get(p, "a"), get(p, "b"));
            if t < 0.0 {
                0.0
            } else if b * t > 700.0 {
                1.0
            } else {
                1.0 - (-(a / b) * ((b * t).exp() - 1.0)).exp()
            }
        }
        "gengamma" => {
            let (a, d, pp) = (get(p, "a"), get(p, "d"), get(p, "p"));
            if t <= 0.0 {
                0.0
            } else {
                let x = (pp * (t.ln() - a.ln())).exp(); // (t / a)^p
                if !x.is_finite() {
                    1.0
                } else {
                    specfunc::gamma_p(d / pp, x)
                }
            }
        }
        "weibull1" => weibull_cdf(get(p, "beta"), get(p, "eta"), t),
        "exponential2" => {
            let (theta, gamma) = (get(p, "theta"), get(p, "gamma"));
            let x = t - gamma;
            if x < 0.0 {
                0.0
            } else {
                1.0 - (-x / theta).exp()
            }
        }
        "mixed_weibull" => {
            let w = get(p, "w");
            w * weibull_cdf(get(p, "beta1"), get(p, "eta1"), t)
                + (1.0 - w) * weibull_cdf(get(p, "beta2"), get(p, "eta2"), t)
        }
        _ => f64::NAN,
    }
}

pub fn reliability(dist: &str, p: &Params, t: f64) -> f64 {
    1.0 - cdf(dist, p, t)
}

pub fn hazard(dist: &str, p: &Params, t: f64) -> f64 {
    let sf = reliability(dist, p, t).max(1e-300);
    pdf(dist, p, t) / sf
}

pub fn quantile(dist: &str, p: &Params, prob: f64) -> f64 {
    match dist {
        "weibull2" | "weibull3" => {
            let (beta, eta) = (get(p, "beta"), get(p, "eta"));
            let loc = if dist == "weibull3" {
                get(p, "gamma")
            } else {
                0.0
            };
            loc + eta * (-(1.0 - prob).ln()).powf(1.0 / beta)
        }
        "lognormal" => (get(p, "mu") + get(p, "sigma") * specfunc::norm_ppf(prob)).exp(),
        "normal" => get(p, "mu") + get(p, "sigma") * specfunc::norm_ppf(prob),
        "exponential" => -get(p, "theta") * (1.0 - prob).ln(),
        "gamma" => specfunc::gamma_ppf(get(p, "alpha"), get(p, "beta"), prob),
        "gumbel" => get(p, "mu") - get(p, "beta") * (-prob.ln()).ln(),
        "loglogistic" => {
            let (alpha, beta) = (get(p, "alpha"), get(p, "beta"));
            alpha * (prob / (1.0 - prob)).powf(1.0 / beta)
        }
        "logistic" => {
            let (mu, s) = (get(p, "mu"), get(p, "s"));
            mu + s * (prob / (1.0 - prob)).ln()
        }
        "gompertz" => {
            let (a, b) = (get(p, "a"), get(p, "b"));
            // Solve R = 1 - prob in R(t) = exp(-(a / b)(exp(b t) - 1)):
            // t = (1 / b) ln(1 - (b / a) ln(1 - prob)).
            (1.0 / b) * (1.0 - (b / a) * (1.0 - prob).ln()).ln()
        }
        "gengamma" => {
            let (a, d, pp) = (get(p, "a"), get(p, "d"), get(p, "p"));
            // F(t) = P(d / p, (t / a)^p), so the gamma quantile with
            // shape d / p and unit scale gives (t / a)^p.
            let x = specfunc::gamma_ppf(d / pp, 1.0, prob);
            a * x.powf(1.0 / pp)
        }
        "weibull1" => {
            let (beta, eta) = (get(p, "beta"), get(p, "eta"));
            eta * (-(1.0 - prob).ln()).powf(1.0 / beta)
        }
        "exponential2" => {
            let (theta, gamma) = (get(p, "theta"), get(p, "gamma"));
            gamma - theta * (1.0 - prob).ln()
        }
        "mixed_weibull" => {
            // No closed form: bisect on the mixture CDF.
            if prob <= 0.0 {
                return 0.0;
            }
            if prob >= 1.0 {
                return f64::INFINITY;
            }
            let mut lo = 0.0;
            let mut hi = get(p, "eta1").max(get(p, "eta2")).max(1.0);
            while cdf(dist, p, hi) < prob && hi < 1e300 {
                hi *= 2.0;
            }
            for _ in 0..200 {
                let mid = 0.5 * (lo + hi);
                if cdf(dist, p, mid) < prob {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            0.5 * (lo + hi)
        }
        _ => f64::NAN,
    }
}

pub fn mean_life(dist: &str, p: &Params) -> f64 {
    match dist {
        "weibull2" | "weibull3" => {
            let loc = if dist == "weibull3" {
                get(p, "gamma")
            } else {
                0.0
            };
            loc + get(p, "eta") * specfunc::ln_gamma(1.0 + 1.0 / get(p, "beta")).exp()
        }
        "lognormal" => (get(p, "mu") + 0.5 * get(p, "sigma").powi(2)).exp(),
        "normal" => get(p, "mu"),
        "exponential" => get(p, "theta"),
        "gamma" => get(p, "alpha") * get(p, "beta"),
        // scipy gumbel_r mean is mu + beta * EulerGamma
        "gumbel" => get(p, "mu") + get(p, "beta") * std::f64::consts::EULER_GAMMA,
        "loglogistic" => {
            let (alpha, beta) = (get(p, "alpha"), get(p, "beta"));
            if beta > 1.0 {
                // alpha * Gamma(1 + 1 / beta) Gamma(1 - 1 / beta)
                // = alpha * (pi / beta) / sin(pi / beta).
                let x = std::f64::consts::PI / beta;
                alpha * x / x.sin()
            } else {
                f64::INFINITY
            }
        }
        "logistic" => get(p, "mu"),
        "gompertz" => {
            let (a, b) = (get(p, "a"), get(p, "b"));
            // Mean = integral of R(t) = exp(a / b) E1(a / b) / b.
            (a / b).exp() * expint_e1(a / b) / b
        }
        "gengamma" => {
            let (a, d, pp) = (get(p, "a"), get(p, "d"), get(p, "p"));
            // E[T] = a Gamma((d + 1) / p) / Gamma(d / p).
            a * (specfunc::ln_gamma((d + 1.0) / pp) - specfunc::ln_gamma(d / pp)).exp()
        }
        "weibull1" => get(p, "eta") * specfunc::ln_gamma(1.0 + 1.0 / get(p, "beta")).exp(),
        "exponential2" => get(p, "gamma") + get(p, "theta"),
        "mixed_weibull" => {
            let w = get(p, "w");
            let m1 = get(p, "eta1") * specfunc::ln_gamma(1.0 + 1.0 / get(p, "beta1")).exp();
            let m2 = get(p, "eta2") * specfunc::ln_gamma(1.0 + 1.0 / get(p, "beta2")).exp();
            w * m1 + (1.0 - w) * m2
        }
        _ => f64::NAN,
    }
}

fn pack(dist: &str, values: &[f64]) -> Params {
    let spec = dist_spec(dist).unwrap();
    spec.iter()
        .zip(values)
        .map(|((name, _), v)| (name.to_string(), *v))
        .collect()
}

pub fn loglik(dist: &str, values: &[f64], times: &[f64], failed: &[bool]) -> f64 {
    let p = pack(dist, values);
    if dist == "weibull3" {
        let g = get(&p, "gamma");
        // The support bound applies to failure times only. A
        // suspension at or below gamma contributes a factor of 1,
        // because surviving past gamma implies surviving past any
        // earlier censoring time. Bounding gamma by the smallest
        // observed time of any kind would wrongly rule out gamma
        // values between the smallest suspension and the smallest
        // failure.
        let min_failure = times
            .iter()
            .zip(failed)
            .filter(|(_, f)| **f)
            .map(|(t, _)| *t)
            .fold(f64::INFINITY, f64::min);
        if min_failure <= g {
            return f64::NEG_INFINITY;
        }
    }
    if matches!(
        dist,
        "weibull2"
            | "weibull1"
            | "lognormal"
            | "exponential"
            | "gamma"
            | "loglogistic"
            | "gengamma"
            | "mixed_weibull"
    ) && times.iter().cloned().fold(f64::INFINITY, f64::min) <= 0.0
    {
        return f64::NEG_INFINITY;
    }
    if dist == "gengamma" {
        // Direct log-density form: avoids pdf underflow in the tails
        // and uses log Q for the censored terms.
        let (a, d, pp) = (get(&p, "a"), get(&p, "d"), get(&p, "p"));
        if a <= 0.0 || d <= 0.0 || pp <= 0.0 {
            return -1e100;
        }
        let lg = specfunc::ln_gamma(d / pp);
        let mut ll = 0.0;
        for (i, &t) in times.iter().enumerate() {
            if t <= 0.0 {
                return -1e100;
            }
            let x = (pp * (t.ln() - a.ln())).exp(); // (t / a)^p
            if failed[i] {
                if !x.is_finite() {
                    return -1e100;
                }
                ll += pp.ln() - d * a.ln() + (d - 1.0) * t.ln() - x - lg;
            } else {
                if !x.is_finite() {
                    return -1e100;
                }
                let q = specfunc::gamma_q(d / pp, x);
                if q <= 0.0 {
                    return -1e100;
                }
                ll += q.ln();
            }
        }
        return if ll.is_finite() { ll } else { -1e100 };
    }
    if dist == "gompertz" {
        // Direct form: log f = ln a + b t - (a / b)(exp(b t) - 1),
        // log R = -(a / b)(exp(b t) - 1).
        let (a, b) = (get(&p, "a"), get(&p, "b"));
        if a <= 0.0 || b <= 0.0 {
            return -1e100;
        }
        let mut ll = 0.0;
        for (i, &t) in times.iter().enumerate() {
            if t < 0.0 || b * t > 700.0 {
                return -1e100;
            }
            let cum_haz = (a / b) * ((b * t).exp() - 1.0);
            if failed[i] {
                ll += a.ln() + b * t - cum_haz;
            } else {
                ll -= cum_haz;
            }
        }
        return if ll.is_finite() { ll } else { -1e100 };
    }
    let mut ll = 0.0;
    for (i, &t) in times.iter().enumerate() {
        if failed[i] {
            let d = pdf(dist, &p, t);
            if d <= 0.0 || !d.is_finite() {
                return -1e100;
            }
            ll += d.ln();
        } else {
            let s = reliability(dist, &p, t);
            if s <= 0.0 {
                return -1e100;
            }
            ll += s.ln();
        }
    }
    if ll.is_finite() {
        ll
    } else {
        -1e100
    }
}

/// Upper bound for a location parameter: the smallest failure time.
/// Suspensions do not bound the location, because a unit suspended
/// before the support starts still contributes a factor of 1. When
/// there are no failures the smallest observed time is used.
fn loc_bound(times: &[f64], failed: &[bool]) -> f64 {
    let min_failure = times
        .iter()
        .zip(failed)
        .filter(|(_, f)| **f)
        .map(|(t, _)| *t)
        .fold(f64::INFINITY, f64::min);
    if min_failure.is_finite() {
        min_failure
    } else {
        times.iter().cloned().fold(f64::INFINITY, f64::min)
    }
}

fn to_unconstrained(dist: &str, values: &[f64], times: &[f64], failed: &[bool]) -> Vec<f64> {
    let spec = dist_spec(dist).unwrap();
    let tmin = loc_bound(times, failed);
    spec.iter()
        .zip(values)
        .map(|((_, kind), v)| match *kind {
            "pos" => v.max(1e-12).ln(),
            "loc" => (tmin - v).max(1e-9).ln(),
            "prob" => {
                let w = v.clamp(1e-9, 1.0 - 1e-9);
                (w / (1.0 - w)).ln()
            }
            _ => *v,
        })
        .collect()
}

fn from_unconstrained(dist: &str, u: &[f64], times: &[f64], failed: &[bool]) -> Vec<f64> {
    let spec = dist_spec(dist).unwrap();
    let tmin = loc_bound(times, failed);
    spec.iter()
        .zip(u)
        .map(|((_, kind), v)| match *kind {
            "pos" => v.clamp(-30.0, 30.0).exp(),
            "loc" => tmin - v.clamp(-30.0, 30.0).exp(),
            "prob" => logistic_cdf_std(v.clamp(-30.0, 30.0)),
            _ => *v,
        })
        .collect()
}

fn median_of(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 1.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

/// Rough Weibull (beta, eta) start from the mean and standard
/// deviation: beta from the coefficient of variation approximation
/// beta = CV^-1.086, eta set so the Weibull mean matches m.
fn weibull_moment_start(m: f64, s: f64) -> (f64, f64) {
    let cv = if m > 0.0 { (s / m).max(0.05) } else { 1.0 };
    let beta = cv.powf(-1.086).clamp(0.3, 8.0);
    let eta = if m > 0.0 {
        m / specfunc::ln_gamma(1.0 + 1.0 / beta).exp()
    } else {
        1.0
    };
    (beta, eta.max(1e-9))
}

fn initial(dist: &str, times: &[f64], failed: &[bool]) -> Vec<f64> {
    let tf: Vec<f64> = times
        .iter()
        .zip(failed)
        .filter(|(_, f)| **f)
        .map(|(t, _)| *t)
        .collect();
    let src = if tf.is_empty() { times.to_vec() } else { tf };
    let m = src.iter().sum::<f64>() / src.len() as f64;
    let var = if src.len() > 1 {
        src.iter().map(|x| (x - m).powi(2)).sum::<f64>() / src.len() as f64
    } else {
        (m * 0.3).powi(2)
    };
    let s = var.sqrt().max(1e-6);
    match dist {
        "weibull2" => vec![1.0, m],
        "weibull3" => vec![1.0, m, 0.0],
        "lognormal" => vec![m.max(1e-9).ln(), 0.5],
        "normal" => vec![m, s],
        "exponential" => vec![m],
        "gamma" => vec![((m / s).powi(2)).max(0.1), (s * s / m).max(1e-9)],
        "gumbel" => vec![m, (s * 0.78).max(1e-6)],
        "loglogistic" => {
            // The median of the log-logistic is alpha; beta start 1.5.
            let med = median_of(src.clone()).max(1e-9);
            vec![med, 1.5]
        }
        "logistic" => {
            // Var = pi^2 s^2 / 3, so s = sd * sqrt(3) / pi.
            vec![m, (s * 0.5513288954217921).max(1e-6)]
        }
        "gompertz" => {
            // b start 1 / m; a chosen so the cumulative hazard at the
            // mean is about 0.7 (median near the mean):
            // a = b * 0.7 / (exp(b m) - 1) with b m = 1.
            let b0 = 1.0 / m.max(1e-9);
            let a0 = b0 * 0.7 / ((1.0f64).exp() - 1.0);
            vec![a0.max(1e-12), b0]
        }
        "gengamma" => {
            // Start at p = 1, where gengamma is the gamma distribution,
            // with gamma moment matching: d = (m / s)^2, a = s^2 / m.
            // fit_mle adds further starts away from p = 1.
            let d0 = ((m / s).powi(2)).clamp(0.2, 50.0);
            let a0 = (s * s / m).max(1e-9);
            vec![a0, d0, 1.0]
        }
        "weibull1" => vec![1.0, m],
        "exponential2" => vec![m, 0.0],
        "mixed_weibull" => {
            // Split the failure times at their median: the lower half
            // starts subpopulation 1 and the upper half subpopulation 2.
            let med = median_of(src.clone());
            let lo: Vec<f64> = src.iter().cloned().filter(|t| *t <= med).collect();
            let hi: Vec<f64> = src.iter().cloned().filter(|t| *t > med).collect();
            let stats = |v: &[f64]| -> (f64, f64) {
                if v.is_empty() {
                    return (m, s);
                }
                let mm = v.iter().sum::<f64>() / v.len() as f64;
                let vv = v.iter().map(|x| (x - mm).powi(2)).sum::<f64>() / v.len() as f64;
                (mm, vv.sqrt().max(1e-6))
            };
            let (m1, s1) = stats(&lo);
            let (m2, s2) = stats(&hi);
            let (b1, e1) = weibull_moment_start(m1, s1);
            let (b2, e2) = weibull_moment_start(m2, s2);
            vec![0.5, b1, e1, b2, e2]
        }
        _ => vec![],
    }
}

fn numeric_hessian<F: Fn(&[f64]) -> f64>(f: F, x: &[f64]) -> Vec<Vec<f64>> {
    let n = x.len();
    let eps = 1e-4;
    let mut h = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            let ei = eps * x[i].abs().max(1.0);
            let ej = eps * x[j].abs().max(1.0);
            let mut xpp = x.to_vec();
            xpp[i] += ei;
            xpp[j] += ej;
            let mut xpm = x.to_vec();
            xpm[i] += ei;
            xpm[j] -= ej;
            let mut xmp = x.to_vec();
            xmp[i] -= ei;
            xmp[j] += ej;
            let mut xmm = x.to_vec();
            xmm[i] -= ei;
            xmm[j] -= ej;
            h[i][j] = (f(&xpp) - f(&xpm) - f(&xmp) + f(&xmm)) / (4.0 * ei * ej);
        }
    }
    (0..n)
        .map(|i| (0..n).map(|j| 0.5 * (h[i][j] + h[j][i])).collect())
        .collect()
}

/// Classification of a fit outcome. Success means a clean interior
/// fit with finite parameters and a finite log-likelihood.
/// SuccessWithWarnings means the point estimates are usable but
/// something the caller should know about happened, for example
/// standard errors are unavailable or a parameter sits on a support
/// boundary by construction. BoundaryOrDegenerate means the data do
/// not identify a finite interior optimum. Failed means no usable
/// fit was produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FitStatus {
    #[default]
    Success,
    SuccessWithWarnings,
    BoundaryOrDegenerate,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitResult {
    pub dist: String,
    pub params: Params,
    pub loglik: f64,
    pub n: usize,
    pub n_fail: usize,
    pub method: String,
    pub cov: Option<Vec<Vec<f64>>>,
    pub se: BTreeMap<String, f64>,
    pub times: Vec<f64>,
    pub failed: Vec<bool>,
    pub success: bool,
    #[serde(default)]
    pub status: FitStatus,
    #[serde(default)]
    pub status_note: String,
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl FitResult {
    pub fn reliability(&self, t: f64) -> f64 {
        reliability(&self.dist, &self.params, t)
    }
    pub fn unreliability(&self, t: f64) -> f64 {
        cdf(&self.dist, &self.params, t)
    }
    pub fn pdf(&self, t: f64) -> f64 {
        pdf(&self.dist, &self.params, t)
    }
    pub fn hazard(&self, t: f64) -> f64 {
        hazard(&self.dist, &self.params, t)
    }
    pub fn b_life(&self, p: f64) -> f64 {
        quantile(&self.dist, &self.params, p)
    }
    pub fn mean_life(&self) -> f64 {
        mean_life(&self.dist, &self.params)
    }

    pub fn status_str(&self) -> &'static str {
        match self.status {
            FitStatus::Success => "success",
            FitStatus::SuccessWithWarnings => "success_with_warnings",
            FitStatus::BoundaryOrDegenerate => "boundary_or_degenerate",
            FitStatus::Failed => "failed",
        }
    }

    /// Two-sided (1 - alpha) Fisher (delta method) bounds on R(t).
    pub fn reliability_bounds(&self, t: f64, alpha: f64) -> (f64, f64) {
        let cov = match &self.cov {
            Some(c) => c.clone(),
            None => return (f64::NAN, f64::NAN),
        };
        let spec = dist_spec(&self.dist).unwrap();
        let x0: Vec<f64> = spec.iter().map(|(nm, _)| self.params[*nm]).collect();
        let z = specfunc::norm_ppf(1.0 - alpha / 2.0);
        let use_loglog = matches!(
            self.dist.as_str(),
            "weibull2" | "weibull3" | "exponential" | "gamma"
        );
        let gfun = |pr: &Params| -> f64 {
            let r = reliability(&self.dist, pr, t).clamp(1e-12, 1.0 - 1e-12);
            if use_loglog {
                (-r.ln()).ln()
            } else {
                (r / (1.0 - r)).ln()
            }
        };
        let g0 = gfun(&self.params);
        let mut grads = vec![0.0; x0.len()];
        for j in 0..x0.len() {
            let step = 1e-5 * x0[j].abs().max(1.0);
            let mut xp = x0.clone();
            xp[j] += step;
            let mut xm = x0.clone();
            xm[j] -= step;
            grads[j] = (gfun(&pack(&self.dist, &xp)) - gfun(&pack(&self.dist, &xm))) / (2.0 * step);
        }
        let mut var = 0.0;
        for i in 0..x0.len() {
            for j in 0..x0.len() {
                var += grads[i] * cov[i][j] * grads[j];
            }
        }
        let se = var.max(0.0).sqrt();
        let (lo_g, hi_g) = (g0 - z * se, g0 + z * se);
        let (lower, upper) = if use_loglog {
            ((-hi_g.exp()).exp(), (-lo_g.exp()).exp())
        } else {
            (1.0 / (1.0 + (-lo_g).exp()), 1.0 / (1.0 + (-hi_g).exp()))
        };
        (lower.clamp(0.0, 1.0), upper.clamp(0.0, 1.0))
    }

    pub fn reliability_lower_bound(&self, t: f64, alpha: f64) -> f64 {
        self.reliability_bounds(t, 2.0 * alpha).0
    }

    pub fn summary(&self) -> String {
        let mut lines = vec![format!(
            "{} fit by {}: n={}, failures={}, loglik={:.3}",
            dist_label(&self.dist),
            self.method,
            self.n,
            self.n_fail,
            self.loglik
        )];
        for (k, v) in &self.params {
            match self.se.get(k) {
                Some(s) => lines.push(format!("  {} = {:.6}  (se {:.3})", k, v, s)),
                None => lines.push(format!("  {} = {:.6}", k, v)),
            }
        }
        lines.push(format!(
            "  mean life = {:.6}, B10 = {:.6}",
            self.mean_life(),
            self.b_life(0.10)
        ));
        lines.join("\n")
    }
}

/// Covariance and standard errors with a reason when they cannot be
/// produced. A non-finite covariance is dropped entirely, and a
/// non-finite standard error is dropped individually, with the
/// reason returned as a warning string instead of a NaN value.
/// Covariance matrix, standard errors, and a warning when the
/// covariance could not be produced, as returned by
/// `cov_and_se_checked`.
type CovAndSe = (Option<Vec<Vec<f64>>>, BTreeMap<String, f64>, Option<String>);

fn cov_and_se_checked(dist: &str, vals: &[f64], times: &[f64], failed: &[bool]) -> CovAndSe {
    let neg = |v: &[f64]| -loglik(dist, v, times, failed);
    let h = numeric_hessian(neg, vals);
    match optim::matrix_inverse(&h) {
        Some(cov) => {
            if cov.iter().flatten().any(|v| !v.is_finite()) {
                return (
                    None,
                    BTreeMap::new(),
                    Some(
                        "standard errors unavailable: covariance matrix is non-finite".to_string(),
                    ),
                );
            }
            let spec = dist_spec(dist).unwrap();
            let mut se = BTreeMap::new();
            let mut dropped: Vec<String> = Vec::new();
            for (i, (nm, _)) in spec.iter().enumerate() {
                let s = cov[i][i].max(0.0).sqrt();
                if s.is_finite() {
                    se.insert(nm.to_string(), s);
                } else {
                    dropped.push(nm.to_string());
                }
            }
            let warning = if dropped.is_empty() {
                None
            } else {
                Some(format!(
                    "standard errors unavailable for {}: computed standard error is non-finite",
                    dropped.join(", ")
                ))
            };
            (Some(cov), se, warning)
        }
        None => (
            None,
            BTreeMap::new(),
            Some("standard errors unavailable: Hessian is singular".to_string()),
        ),
    }
}

fn success_for(status: FitStatus) -> bool {
    matches!(status, FitStatus::Success | FitStatus::SuccessWithWarnings)
}

fn requires_positive_times(dist: &str) -> bool {
    matches!(
        dist,
        "weibull2"
            | "lognormal"
            | "gamma"
            | "loglogistic"
            | "gengamma"
            | "mixed_weibull"
            | "weibull1"
    )
}

fn fallback_vals(dist: &str, times: &[f64], failed: &[bool]) -> Vec<f64> {
    let spec = dist_spec(dist).unwrap();
    let init = initial(dist, times, failed);
    spec.iter()
        .enumerate()
        .map(|(i, (_, kind))| {
            let v = init.get(i).copied().unwrap_or(f64::NAN);
            match *kind {
                "pos" => {
                    if v.is_finite() && v > 0.0 {
                        v
                    } else {
                        1.0
                    }
                }
                "prob" => {
                    if v.is_finite() && v > 0.0 && v < 1.0 {
                        v
                    } else {
                        0.5
                    }
                }
                _ => {
                    if v.is_finite() {
                        v
                    } else {
                        0.0
                    }
                }
            }
        })
        .collect()
}

fn degenerate_result(
    dist: &str,
    times: &[f64],
    failed: &[bool],
    method: &str,
    status: FitStatus,
    note: String,
) -> FitResult {
    let vals = fallback_vals(dist, times, failed);
    let ll = loglik(dist, &vals, times, failed);
    FitResult {
        dist: dist.into(),
        params: pack(dist, &vals),
        loglik: ll,
        n: times.len(),
        n_fail: failed.iter().filter(|f| **f).count(),
        method: method.into(),
        cov: None,
        se: BTreeMap::new(),
        times: times.to_vec(),
        failed: failed.to_vec(),
        success: false,
        status,
        status_note: note.clone(),
        warnings: vec![note],
    }
}

/// Hard input errors shared by the fit entry points. These are
/// returned as Err because there is no meaningful fit to report.
fn check_hard_errors(times: &[f64], failed: Option<&[bool]>) -> Result<(), String> {
    if times.is_empty() {
        return Err("empty data: need at least one observation to fit".into());
    }
    if let Some(f) = failed {
        if f.len() != times.len() {
            return Err(format!(
                "times/failed length mismatch: times has {} entries but failed has {} entries",
                times.len(),
                f.len()
            ));
        }
    }
    for (i, &t) in times.iter().enumerate() {
        if !t.is_finite() {
            return Err(format!(
                "non-finite time at index {}: time is NaN or infinite",
                i
            ));
        }
        if t < 0.0 {
            return Err(format!("negative time at index {}", i));
        }
    }
    Ok(())
}

/// Degenerate data patterns that produce a classified FitResult
/// rather than an error or a clean success. Returns None when the
/// data are not degenerate in any of these senses.
/// n_estimated is the number of parameters actually estimated, so a
/// one-parameter fit with a fixed shape is not flagged for identical
/// times the way a two-parameter fit is.
fn degenerate_check(
    dist: &str,
    times: &[f64],
    failed: &[bool],
    method: &str,
    n_estimated: usize,
) -> Option<FitResult> {
    if times.iter().all(|&t| t == 0.0) {
        return Some(degenerate_result(
            dist,
            times,
            failed,
            method,
            FitStatus::Failed,
            "all observed times are zero: there is no positive failure time to fit".to_string(),
        ));
    }
    if requires_positive_times(dist) {
        if let Some(i) = times.iter().position(|&t| t == 0.0) {
            return Some(degenerate_result(
                dist,
                times,
                failed,
                method,
                FitStatus::Failed,
                format!("zero time at index {}: {} requires positive times", i, dist),
            ));
        }
    }
    if n_estimated >= 2 && times.iter().all(|&t| t == times[0]) {
        return Some(degenerate_result(
            dist,
            times,
            failed,
            method,
            FitStatus::BoundaryOrDegenerate,
            format!(
                "all observed times are identical ({}): the shape is not identifiable from identical observations; the likelihood increases without bound as the shape grows",
                times[0]
            ),
        ));
    }
    None
}

/// Build a FitResult with post-fit validation. A result is a clean
/// success only when every fitted parameter and the log-likelihood
/// are finite and the positive and probability parameters respect
/// their domains. When a covariance was expected but could not be
/// produced, the fit is downgraded to SuccessWithWarnings with the
/// reason in the warnings.
#[allow(clippy::too_many_arguments)]
fn build_fit(
    dist: &str,
    vals: &[f64],
    times: &[f64],
    failed: &[bool],
    method: &str,
    expect_cov: bool,
    base_status: FitStatus,
    base_note: String,
    extra_warnings: Vec<String>,
) -> FitResult {
    let ll = loglik(dist, vals, times, failed);
    let spec = dist_spec(dist).unwrap();
    let mut status = base_status;
    let mut note = base_note;
    let mut warnings = extra_warnings;
    if !warnings.is_empty() && status == FitStatus::Success {
        status = FitStatus::SuccessWithWarnings;
        if note.is_empty() {
            note = warnings[0].clone();
        }
    }
    let (cov, se) = if expect_cov {
        let (cov, se, warning) = cov_and_se_checked(dist, vals, times, failed);
        if let Some(w) = warning {
            warnings.push(w.clone());
            if status == FitStatus::Success {
                status = FitStatus::SuccessWithWarnings;
            }
            if note.is_empty() {
                note = w;
            }
        }
        (cov, se)
    } else {
        (None, BTreeMap::new())
    };
    let mut failure_reason: Option<String> = None;
    for (i, (nm, kind)) in spec.iter().enumerate() {
        let v = vals.get(i).copied().unwrap_or(f64::NAN);
        if !v.is_finite() {
            failure_reason = Some(format!("fit failed: fitted parameter {} is non-finite", nm));
            break;
        }
        if *kind == "pos" && v <= 0.0 {
            failure_reason = Some(format!(
                "fit failed: fitted parameter {} must be positive but is {}",
                nm, v
            ));
            break;
        }
        if *kind == "prob" && !(v > 0.0 && v < 1.0) {
            failure_reason = Some(format!(
                "fit failed: fitted parameter {} must be in (0, 1) but is {}",
                nm, v
            ));
            break;
        }
    }
    if failure_reason.is_none() && (!ll.is_finite() || ll <= -1e99) {
        failure_reason = Some("fit failed: log-likelihood is non-finite".to_string());
    }
    let (cov, se) = match failure_reason {
        Some(reason) => {
            status = FitStatus::Failed;
            if !warnings.contains(&reason) {
                warnings.push(reason.clone());
            }
            note = reason;
            (None, BTreeMap::new())
        }
        None => (cov, se),
    };
    FitResult {
        dist: dist.into(),
        params: pack(dist, vals),
        loglik: ll,
        n: times.len(),
        n_fail: failed.iter().filter(|f| **f).count(),
        method: method.into(),
        cov,
        se,
        times: times.to_vec(),
        failed: failed.to_vec(),
        success: success_for(status),
        status,
        status_note: note,
        warnings,
    }
}

fn fit_weibull2_profile(times: &[f64], failed: &[bool]) -> FitResult {
    let r = failed.iter().filter(|f| **f).count() as f64;
    let sum_log_tf: f64 = times
        .iter()
        .zip(failed)
        .filter(|(_, f)| **f)
        .map(|(t, _)| t.ln())
        .sum();
    // The profile is evaluated in the log domain. The direct form
    // sum of t^beta overflows for large shapes on large time scales
    // (for example the clustered sample scaled by 3600), which would
    // break the scale invariance of the fit.
    let log_times: Vec<f64> = times.iter().map(|t| t.ln()).collect();
    let log_t_max = log_times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let eta_for_beta = |beta: f64| -> f64 {
        let s: f64 = log_times
            .iter()
            .map(|lt| (beta * (lt - log_t_max)).exp())
            .sum();
        ((beta * log_t_max + s.ln() - r.ln()) / beta).exp()
    };
    let profile = |logbeta: f64| -> f64 {
        let beta = logbeta.exp();
        let eta = eta_for_beta(beta);
        let log_eta = eta.ln();
        let cum: f64 = log_times
            .iter()
            .map(|lt| (beta * (lt - log_eta)).exp())
            .sum();
        let ll = r * (beta.ln() - beta * log_eta) + (beta - 1.0) * sum_log_tf - cum;
        -ll
    };
    // Adaptive bracketing in log beta. The search starts on the
    // historical bracket [-2.5, 3.0] and expands geometrically,
    // doubling the bracket width toward the bound the optimum sits
    // on, up to a cap of |log beta| <= 12. An optimum that is still
    // on a bound at the cap means the profile is still improving
    // there, which is reported as a boundary case instead of a
    // clean fit.
    const LOG_BETA_CAP: f64 = 12.0;
    let mut lo = -2.5f64;
    let mut hi = 3.0f64;
    let mut lb: f64;
    let mut at_cap = false;
    loop {
        let mut pf = profile;
        let (x, _, outcome) = optim::minimize_scalar_report(&mut pf, lo, hi, 1e-10);
        lb = x;
        match outcome {
            optim::ScalarOutcome::Interior => {
                break;
            }
            optim::ScalarOutcome::AtUpperBound => {
                if hi >= LOG_BETA_CAP {
                    at_cap = true;
                    break;
                }
                let width = hi - lo;
                hi = (lo + 2.0 * width).min(LOG_BETA_CAP);
            }
            optim::ScalarOutcome::AtLowerBound => {
                if lo <= -LOG_BETA_CAP {
                    at_cap = true;
                    break;
                }
                let width = hi - lo;
                lo = (hi - 2.0 * width).max(-LOG_BETA_CAP);
            }
            optim::ScalarOutcome::IterationCap => {
                break;
            }
        }
    }
    let beta = lb.exp();
    let eta = eta_for_beta(beta);
    let vals = vec![beta, eta];
    if at_cap {
        let note = "likelihood appears unbounded in shape for this data (observations too tightly clustered to identify a finite shape): profile log-likelihood still increasing at the log beta cap of 12 in absolute value".to_string();
        return build_fit(
            "weibull2",
            &vals,
            times,
            failed,
            "MLE",
            false,
            FitStatus::BoundaryOrDegenerate,
            note.clone(),
            vec![note],
        );
    }
    build_fit(
        "weibull2",
        &vals,
        times,
        failed,
        "MLE",
        true,
        FitStatus::Success,
        String::new(),
        Vec::new(),
    )
}

/// One-parameter Weibull fit with the shape beta fixed by the user.
/// The scale has the closed-form MLE
/// eta = (sum_i t_i^beta / r)^(1 / beta), where the sum runs over all
/// units (failed and suspended) and r is the number of failures.
pub fn fit_weibull1(
    times: &[f64],
    failed: Option<&[bool]>,
    beta: f64,
) -> Result<FitResult, String> {
    if beta <= 0.0 || !beta.is_finite() {
        return Err("weibull1 requires a positive finite beta".into());
    }
    check_hard_errors(times, failed)?;
    if let Some(i) = times.iter().position(|&t| t == 0.0) {
        return Err(format!(
            "zero time at index {}: weibull1 needs positive times",
            i
        ));
    }
    let failed_v: Vec<bool> = match failed {
        Some(f) => f.to_vec(),
        None => vec![true; times.len()],
    };
    let r = failed_v.iter().filter(|f| **f).count();
    if r == 0 {
        return Err("need at least one failure to fit".into());
    }
    let sum: f64 = times.iter().map(|t| t.powf(beta)).sum();
    let eta = (sum / r as f64).powf(1.0 / beta);
    let vals = vec![beta, eta];
    // Beta is fixed, not estimated, so no Fisher covariance is
    // reported for the pair. Identical times are not degenerate
    // here, because only eta is estimated.
    Ok(build_fit(
        "weibull1",
        &vals,
        times,
        &failed_v,
        "MLE (beta fixed)",
        false,
        FitStatus::Success,
        String::new(),
        Vec::new(),
    ))
}

/// Closed-form MLE for the two-parameter exponential. The support
/// start gamma is bounded by the smallest failure time, not by the
/// smallest observed time of any kind: a suspension below gamma
/// contributes a factor of 1 to the likelihood. The profile
/// log-likelihood increases as gamma approaches the smallest
/// failure time from below, so the MLE sits at that boundary point.
/// With gamma at the smallest failure time, theta_hat is S / r,
/// where S sums (t - gamma) over every observation with t above
/// gamma and r is the number of failures.
fn fit_exponential2(times: &[f64], failed: &[bool]) -> FitResult {
    let gamma_hat = times
        .iter()
        .zip(failed)
        .filter(|(_, f)| **f)
        .map(|(t, _)| *t)
        .fold(f64::INFINITY, f64::min);
    let r = failed.iter().filter(|f| **f).count() as f64;
    let s: f64 = times
        .iter()
        .filter(|t| **t > gamma_hat)
        .map(|t| t - gamma_hat)
        .sum();
    let theta_hat = s / r;
    let vals = vec![theta_hat, gamma_hat];
    // The gamma estimator is a boundary estimate, which is
    // non-regular, so no Fisher covariance is reported.
    let warning = "gamma is a boundary (support) estimate at the smallest failure time".to_string();
    build_fit(
        "exponential2",
        &vals,
        times,
        failed,
        "MLE",
        false,
        FitStatus::SuccessWithWarnings,
        warning.clone(),
        vec![warning],
    )
}

/// Mixed Weibull fit by direct maximization of the exact censored
/// log-likelihood with Nelder-Mead on the transformed parameters
/// (logit w, log beta1, log eta1, log beta2, log eta2). Several starts
/// are tried (median split plus perturbed weights, and a split at the
/// quartiles). The returned components are ordered by eta so that
/// subpopulation 1 is the smaller-eta one, which fixes the label
/// switching ambiguity in the output.
fn fit_mixed_weibull(times: &[f64], failed: &[bool]) -> FitResult {
    let mut starts = vec![initial("mixed_weibull", times, failed)];
    for w0 in [0.3, 0.7] {
        let mut st = starts[0].clone();
        st[0] = w0;
        starts.push(st);
    }
    // Quartile split start: lower quartile versus the rest, w = 0.25.
    {
        let mut sorted = times.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let q = sorted[sorted.len() / 4];
        let lo: Vec<f64> = times.iter().cloned().filter(|t| *t <= q).collect();
        let hi: Vec<f64> = times.iter().cloned().filter(|t| *t > q).collect();
        let stats = |v: &[f64], fallback: (f64, f64)| -> (f64, f64) {
            if v.len() < 2 {
                return fallback;
            }
            let mm = v.iter().sum::<f64>() / v.len() as f64;
            let vv = v.iter().map(|x| (x - mm).powi(2)).sum::<f64>() / v.len() as f64;
            (mm, vv.sqrt().max(1e-6))
        };
        let m_all = times.iter().sum::<f64>() / times.len() as f64;
        let (m1, s1) = stats(&lo, (0.5 * m_all, 0.3 * m_all));
        let (m2, s2) = stats(&hi, (m_all, 0.5 * m_all));
        let (b1, e1) = weibull_moment_start(m1, s1);
        let (b2, e2) = weibull_moment_start(m2, s2);
        starts.push(vec![0.25, b1, e1, b2, e2]);
    }
    let mut best: Option<(Vec<f64>, f64)> = None;
    for st in &starts {
        let u0 = to_unconstrained("mixed_weibull", st, times, failed);
        let mut neg = |u: &[f64]| -> f64 {
            let vals = from_unconstrained("mixed_weibull", u, times, failed);
            -loglik("mixed_weibull", &vals, times, failed)
        };
        let step: Vec<f64> = u0.iter().map(|_| 0.3).collect();
        let (x1, _) = optim::nelder_mead(&mut neg, &u0, &step, 8000, 1e-10);
        let (x2, f2) = optim::nelder_mead(&mut neg, &x1, &step, 8000, 1e-12);
        if best.is_none() || f2 < best.as_ref().unwrap().1 {
            best = Some((x2, f2));
        }
    }
    let (bu, _) = best.unwrap();
    let mut vals = from_unconstrained("mixed_weibull", &bu, times, failed);
    // Order components by eta; swap the weight with them.
    if vals[2] > vals[4] {
        vals = vec![1.0 - vals[0], vals[3], vals[4], vals[1], vals[2]];
    }
    build_fit(
        "mixed_weibull",
        &vals,
        times,
        failed,
        "MLE",
        true,
        FitStatus::Success,
        String::new(),
        Vec::new(),
    )
}

/// Maximum likelihood fit with right-censored data. `failed`: true =
/// failure, false = suspended. Defaults to all failures.
pub fn fit_mle(dist: &str, times: &[f64], failed: Option<&[bool]>) -> Result<FitResult, String> {
    if dist_spec(dist).is_none() {
        return Err(format!("unknown distribution {}", dist));
    }
    check_hard_errors(times, failed)?;
    let failed_v: Vec<bool> = match failed {
        Some(f) => f.to_vec(),
        None => vec![true; times.len()],
    };
    if !failed_v.iter().any(|f| *f) {
        return Err("need at least one failure to fit".into());
    }
    if dist == "weibull1" {
        return Err(
            "weibull1 has a fixed shape: call fit_weibull1(times, failed, beta) \
             with the beta value to fix"
                .into(),
        );
    }
    if dist == "exponential2" {
        // Guard with the corrected formula: gamma at the smallest
        // failure time and theta from the time above gamma. A zero
        // or non-finite theta means the failures and any later
        // observations leave no time on test above gamma.
        let gamma_hat = times
            .iter()
            .zip(&failed_v)
            .filter(|(_, f)| **f)
            .map(|(t, _)| *t)
            .fold(f64::INFINITY, f64::min);
        let r = failed_v.iter().filter(|f| **f).count() as f64;
        let s: f64 = times
            .iter()
            .filter(|t| **t > gamma_hat)
            .map(|t| t - gamma_hat)
            .sum();
        let theta_hat = s / r;
        if theta_hat <= 0.0 || !theta_hat.is_finite() {
            return Err(
                "exponential2 fit is degenerate: no time on test above the smallest failure time"
                    .into(),
            );
        }
        return Ok(fit_exponential2(times, &failed_v));
    }
    let n_estimated = dist_spec(dist).unwrap().len();
    if let Some(res) = degenerate_check(dist, times, &failed_v, "MLE", n_estimated) {
        return Ok(res);
    }
    if dist == "weibull2" {
        return Ok(fit_weibull2_profile(times, &failed_v));
    }
    if dist == "mixed_weibull" {
        return Ok(fit_mixed_weibull(times, &failed_v));
    }
    let mut starts = vec![initial(dist, times, &failed_v)];
    if matches!(dist, "weibull3") {
        let mut a = initial(dist, times, &failed_v);
        a[0] = 2.0;
        starts.push(a);
        let mut b = initial(dist, times, &failed_v);
        b[0] = 0.7;
        starts.push(b);
    }
    if dist == "loglogistic" {
        for beta0 in [0.8, 2.5] {
            let mut a = initial(dist, times, &failed_v);
            a[1] = beta0;
            starts.push(a);
        }
    }
    if dist == "gompertz" {
        // Extra starts at half and twice the base rate b0, with a
        // rescaled to keep the cumulative hazard at the mean fixed.
        let base = initial(dist, times, &failed_v);
        for mult in [0.4, 2.5] {
            let mut a = base.clone();
            a[1] = base[1] * mult;
            a[0] = base[0] / mult;
            starts.push(a);
        }
    }
    if dist == "gengamma" {
        // Starts away from the p = 1 gamma start: a Weibull-like start
        // (d = p) and a start with a at the data mean. The scale start
        // is corrected so the gengamma mean matches the data mean at
        // the starting shapes.
        let base = initial(dist, times, &failed_v);
        let m = {
            let tf: Vec<f64> = times
                .iter()
                .zip(&failed_v)
                .filter(|(_, f)| **f)
                .map(|(t, _)| *t)
                .collect();
            let src = if tf.is_empty() { times } else { &tf };
            src.iter().sum::<f64>() / src.len() as f64
        };
        for (d0, p0) in [(2.0, 2.0), (3.0, 1.5), (1.5, 0.7)] {
            let ratio = (specfunc::ln_gamma((d0 + 1.0) / p0) - specfunc::ln_gamma(d0 / p0)).exp();
            starts.push(vec![(m / ratio).max(1e-9), d0, p0]);
        }
        let _ = base;
    }
    let mut best: Option<(Vec<f64>, f64)> = None;
    let mut best_outcome = optim::NelderMeadOutcome::Converged;
    for st in &starts {
        let u0 = to_unconstrained(dist, st, times, &failed_v);
        let mut neg = |u: &[f64]| -> f64 {
            let vals = from_unconstrained(dist, u, times, &failed_v);
            -loglik(dist, &vals, times, &failed_v)
        };
        let step: Vec<f64> = u0.iter().map(|_| 0.3).collect();
        let (x1, _) = optim::nelder_mead(&mut neg, &u0, &step, 5000, 1e-10);
        let (x2, f2, outcome) = optim::nelder_mead_report(&mut neg, &x1, &step, 5000, 1e-12);
        if best.is_none() || f2 < best.as_ref().unwrap().1 {
            best = Some((x2, f2));
            best_outcome = outcome;
        }
    }
    let (bu, _) = best.unwrap();
    let vals = from_unconstrained(dist, &bu, times, &failed_v);
    let extra_warnings = if best_outcome == optim::NelderMeadOutcome::IterationCap {
        vec!["optimizer stopped at the iteration cap without converging".to_string()]
    } else {
        Vec::new()
    };
    Ok(build_fit(
        dist,
        &vals,
        times,
        &failed_v,
        "MLE",
        true,
        FitStatus::Success,
        String::new(),
        extra_warnings,
    ))
}

/// Benard median rank for the k-th failure out of n_total units.
pub fn median_ranks(n_total: usize, failure_order: f64) -> f64 {
    (failure_order - 0.3) / (n_total as f64 + 0.4)
}

/// Median-rank regression (probability-plot least squares) for
/// weibull2, lognormal, normal and loglogistic. Rank regression is
/// not provided for the logistic distribution (its probability plot
/// is not part of this implementation; use fit_mle), nor for gompertz,
/// gengamma, weibull1, exponential2 or mixed_weibull.
pub fn fit_rank_regression(
    dist: &str,
    times: &[f64],
    failed: Option<&[bool]>,
) -> Result<FitResult, String> {
    if !matches!(dist, "weibull2" | "lognormal" | "normal" | "loglogistic") {
        return Err(
            "rank regression implemented for weibull2, lognormal, normal, loglogistic".into(),
        );
    }
    check_hard_errors(times, failed)?;
    let failed_v: Vec<bool> = match failed {
        Some(f) => f.to_vec(),
        None => vec![true; times.len()],
    };
    if !failed_v.iter().any(|f| *f) {
        return Err("need at least one failure to fit".into());
    }
    if let Some(res) = degenerate_check(dist, times, &failed_v, "Rank regression", 2) {
        return Ok(res);
    }
    let mut tf: Vec<f64> = times
        .iter()
        .zip(&failed_v)
        .filter(|(_, f)| **f)
        .map(|(t, _)| *t)
        .collect();
    tf.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let k: Vec<f64> = (1..=tf.len()).map(|i| i as f64).collect();
    let f_emp: Vec<f64> = k.iter().map(|kk| median_ranks(times.len(), *kk)).collect();
    let params: Params = match dist {
        "weibull2" => {
            let x: Vec<f64> = tf.iter().map(|t| t.ln()).collect();
            let y: Vec<f64> = f_emp.iter().map(|f| (-(1.0 - f).ln()).ln()).collect();
            let design: Vec<Vec<f64>> = x.iter().map(|xx| vec![*xx, 1.0]).collect();
            let coef = optim::lstsq(&design, &y).ok_or("lstsq failed")?;
            let (slope, intercept) = (coef[0], coef[1]);
            params_of(&[("beta", slope), ("eta", (-intercept / slope).exp())])
        }
        "loglogistic" => {
            // logit(F) = beta (ln t - ln alpha), so regressing ln t on
            // the logit of the median ranks gives slope 1 / beta and
            // intercept ln alpha.
            let x: Vec<f64> = f_emp.iter().map(|f| (f / (1.0 - f)).ln()).collect();
            let y: Vec<f64> = tf.iter().map(|t| t.ln()).collect();
            let design: Vec<Vec<f64>> = x.iter().map(|xx| vec![*xx, 1.0]).collect();
            let coef = optim::lstsq(&design, &y).ok_or("lstsq failed")?;
            let (slope, intercept) = (coef[0], coef[1]);
            if slope.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
                return Err("loglogistic rank regression gave a non-positive slope".into());
            }
            params_of(&[("alpha", intercept.exp()), ("beta", 1.0 / slope)])
        }
        "lognormal" => {
            let x: Vec<f64> = tf.iter().map(|t| t.ln()).collect();
            let y: Vec<f64> = f_emp.iter().map(|f| specfunc::norm_ppf(*f)).collect();
            let design: Vec<Vec<f64>> = x.iter().map(|xx| vec![*xx, 1.0]).collect();
            let coef = optim::lstsq(&design, &y).ok_or("lstsq failed")?;
            let (slope, intercept) = (coef[0], coef[1]);
            params_of(&[("mu", -intercept / slope), ("sigma", 1.0 / slope)])
        }
        _ => {
            let x = tf.clone();
            let y: Vec<f64> = f_emp.iter().map(|f| specfunc::norm_ppf(*f)).collect();
            let design: Vec<Vec<f64>> = x.iter().map(|xx| vec![*xx, 1.0]).collect();
            let coef = optim::lstsq(&design, &y).ok_or("lstsq failed")?;
            let (slope, intercept) = (coef[0], coef[1]);
            params_of(&[("mu", -intercept / slope), ("sigma", 1.0 / slope)])
        }
    };
    let spec = dist_spec(dist).unwrap();
    let vals: Vec<f64> = spec.iter().map(|(nm, _)| params[*nm]).collect();
    let _ = params;
    Ok(build_fit(
        dist,
        &vals,
        times,
        &failed_v,
        "Rank regression",
        false,
        FitStatus::Success,
        String::new(),
        Vec::new(),
    ))
}

/// (times, empirical F, theoretical F) points for a probability plot.
pub fn probability_plot_data(fit: &FitResult) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut times: Vec<f64> = fit
        .times
        .iter()
        .zip(&fit.failed)
        .filter(|(_, f)| **f)
        .map(|(t, _)| *t)
        .collect();
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let f_emp: Vec<f64> = (1..=times.len())
        .map(|k| median_ranks(fit.n, k as f64))
        .collect();
    let f_theo: Vec<f64> = times
        .iter()
        .map(|t| cdf(&fit.dist, &fit.params, *t))
        .collect();
    (times, f_emp, f_theo)
}

/// Kaplan-Meier survival estimate. Returns (t_unique, survival).
pub fn kaplan_meier(times: &[f64], failed: Option<&[bool]>) -> (Vec<f64>, Vec<f64>) {
    let failed_v: Vec<bool> = match failed {
        Some(f) => f.to_vec(),
        None => vec![true; times.len()],
    };
    let mut order: Vec<usize> = (0..times.len()).collect();
    order.sort_by(|&a, &b| times[a].partial_cmp(&times[b]).unwrap());
    let t_s: Vec<f64> = order.iter().map(|&i| times[i]).collect();
    let f_s: Vec<bool> = order.iter().map(|&i| failed_v[i]).collect();
    let mut uniq: Vec<f64> = t_s
        .iter()
        .zip(&f_s)
        .filter(|(_, f)| **f)
        .map(|(t, _)| *t)
        .collect();
    uniq.dedup();
    let mut surv = 1.0;
    let mut out_t = vec![0.0];
    let mut out_s = vec![1.0];
    let mut n_at_risk = times.len() as f64;
    for tt in uniq {
        let d = t_s
            .iter()
            .zip(&f_s)
            .filter(|(t, f)| **t == tt && **f)
            .count() as f64;
        let c = t_s
            .iter()
            .zip(&f_s)
            .filter(|(t, f)| **t == tt && !**f)
            .count() as f64;
        surv *= 1.0 - d / n_at_risk;
        out_t.push(tt);
        out_s.push(surv);
        n_at_risk -= d + c;
    }
    (out_t, out_s)
}
