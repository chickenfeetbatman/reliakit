//! Life-data distributions and fitting (Weibull++ equivalent core).
//!
//! Supports 2-parameter and 3-parameter Weibull, lognormal, normal,
//! exponential, gamma and Gumbel distributions. Fitting is by maximum
//! likelihood with the exact right-censored (suspended) likelihood, or by
//! median-rank regression where defined. Confidence bounds use the
//! observed Fisher information (inverse Hessian of the negative
//! log-likelihood). Ported from the Python ReliaKit distributions module
//! with the same math.

use crate::optim;
use crate::specfunc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type Params = BTreeMap<String, f64>;

pub fn params_of(pairs: &[(&str, f64)]) -> Params {
    pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

/// (name, kind) with kind in {"pos", "real", "loc"} per distribution.
pub fn dist_spec(dist: &str) -> Option<Vec<(&'static str, &'static str)>> {
    match dist {
        "weibull2" => Some(vec![("beta", "pos"), ("eta", "pos")]),
        "weibull3" => Some(vec![("beta", "pos"), ("eta", "pos"), ("gamma", "loc")]),
        "lognormal" => Some(vec![("mu", "real"), ("sigma", "pos")]),
        "normal" => Some(vec![("mu", "real"), ("sigma", "pos")]),
        "exponential" => Some(vec![("theta", "pos")]),
        "gamma" => Some(vec![("alpha", "pos"), ("beta", "pos")]),
        "gumbel" => Some(vec![("mu", "real"), ("beta", "pos")]),
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
        _ => "unknown",
    }
}

fn get(p: &Params, k: &str) -> f64 {
    *p.get(k).unwrap_or(&f64::NAN)
}

pub fn pdf(dist: &str, p: &Params, t: f64) -> f64 {
    match dist {
        "weibull2" | "weibull3" => {
            let (beta, eta) = (get(p, "beta"), get(p, "eta"));
            let x = if dist == "weibull3" { t - get(p, "gamma") } else { t };
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
        _ => f64::NAN,
    }
}

pub fn cdf(dist: &str, p: &Params, t: f64) -> f64 {
    match dist {
        "weibull2" | "weibull3" => {
            let (beta, eta) = (get(p, "beta"), get(p, "eta"));
            let x = if dist == "weibull3" { t - get(p, "gamma") } else { t };
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
            let loc = if dist == "weibull3" { get(p, "gamma") } else { 0.0 };
            loc + eta * (-(1.0 - prob).ln()).powf(1.0 / beta)
        }
        "lognormal" => (get(p, "mu") + get(p, "sigma") * specfunc::norm_ppf(prob)).exp(),
        "normal" => get(p, "mu") + get(p, "sigma") * specfunc::norm_ppf(prob),
        "exponential" => -get(p, "theta") * (1.0 - prob).ln(),
        "gamma" => specfunc::gamma_ppf(get(p, "alpha"), get(p, "beta"), prob),
        "gumbel" => get(p, "mu") - get(p, "beta") * (-prob.ln()).ln(),
        _ => f64::NAN,
    }
}

pub fn mean_life(dist: &str, p: &Params) -> f64 {
    match dist {
        "weibull2" | "weibull3" => {
            let loc = if dist == "weibull3" { get(p, "gamma") } else { 0.0 };
            loc + get(p, "eta") * specfunc::ln_gamma(1.0 + 1.0 / get(p, "beta")).exp()
        }
        "lognormal" => (get(p, "mu") + 0.5 * get(p, "sigma").powi(2)).exp(),
        "normal" => get(p, "mu"),
        "exponential" => get(p, "theta"),
        "gamma" => get(p, "alpha") * get(p, "beta"),
        // scipy gumbel_r mean is mu + beta * EulerGamma
        "gumbel" => get(p, "mu") + get(p, "beta") * 0.5772156649015329,
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
        if times.iter().cloned().fold(f64::INFINITY, f64::min) <= g {
            return f64::NEG_INFINITY;
        }
    }
    if matches!(dist, "weibull2" | "lognormal" | "exponential" | "gamma")
        && times.iter().cloned().fold(f64::INFINITY, f64::min) <= 0.0
    {
        return f64::NEG_INFINITY;
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

fn to_unconstrained(dist: &str, values: &[f64], times: &[f64]) -> Vec<f64> {
    let spec = dist_spec(dist).unwrap();
    let tmin = times.iter().cloned().fold(f64::INFINITY, f64::min);
    spec.iter()
        .zip(values)
        .map(|((_, kind), v)| match *kind {
            "pos" => v.max(1e-12).ln(),
            "loc" => (tmin - v).max(1e-9).ln(),
            _ => *v,
        })
        .collect()
}

fn from_unconstrained(dist: &str, u: &[f64], times: &[f64]) -> Vec<f64> {
    let spec = dist_spec(dist).unwrap();
    let tmin = times.iter().cloned().fold(f64::INFINITY, f64::min);
    spec.iter()
        .zip(u)
        .map(|((_, kind), v)| match *kind {
            "pos" => v.clamp(-30.0, 30.0).exp(),
            "loc" => tmin - v.clamp(-30.0, 30.0).exp(),
            _ => *v,
        })
        .collect()
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
    for i in 0..n {
        for j in 0..n {
            let v = 0.5 * (h[i][j] + h[j][i]);
            h[i][j] = v;
            h[j][i] = v;
        }
    }
    h
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

    /// Two-sided (1 - alpha) Fisher (delta method) bounds on R(t).
    pub fn reliability_bounds(&self, t: f64, alpha: f64) -> (f64, f64) {
        let cov = match &self.cov {
            Some(c) => c.clone(),
            None => return (f64::NAN, f64::NAN),
        };
        let spec = dist_spec(&self.dist).unwrap();
        let x0: Vec<f64> = spec.iter().map(|(nm, _)| self.params[*nm]).collect();
        let z = specfunc::norm_ppf(1.0 - alpha / 2.0);
        let use_loglog = matches!(self.dist.as_str(), "weibull2" | "weibull3" | "exponential" | "gamma");
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
            (
                1.0 / (1.0 + (-lo_g).exp()),
                1.0 / (1.0 + (-hi_g).exp()),
            )
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

fn cov_and_se(
    dist: &str,
    vals: &[f64],
    times: &[f64],
    failed: &[bool],
) -> (Option<Vec<Vec<f64>>>, BTreeMap<String, f64>) {
    let neg = |v: &[f64]| -loglik(dist, v, times, failed);
    let h = numeric_hessian(neg, vals);
    match optim::matrix_inverse(&h) {
        Some(cov) => {
            let spec = dist_spec(dist).unwrap();
            let mut se = BTreeMap::new();
            for (i, (nm, _)) in spec.iter().enumerate() {
                se.insert(nm.to_string(), cov[i][i].max(0.0).sqrt());
            }
            (Some(cov), se)
        }
        None => (None, BTreeMap::new()),
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
    let profile = |logbeta: f64| -> f64 {
        let beta = logbeta.exp();
        let eta = (times.iter().map(|t| t.powf(beta)).sum::<f64>() / r).powf(1.0 / beta);
        let ll = r * (beta.ln() - beta * eta.ln()) + (beta - 1.0) * sum_log_tf
            - times.iter().map(|t| (t / eta).powf(beta)).sum::<f64>();
        -ll
    };
    let mut pf = profile;
    let (lb, _) = optim::minimize_scalar(&mut pf, -2.5, 3.0, 1e-10);
    let beta = lb.exp();
    let eta = (times.iter().map(|t| t.powf(beta)).sum::<f64>() / r).powf(1.0 / beta);
    let vals = vec![beta, eta];
    let ll = loglik("weibull2", &vals, times, failed);
    let (cov, se) = cov_and_se("weibull2", &vals, times, failed);
    FitResult {
        dist: "weibull2".into(),
        params: pack("weibull2", &vals),
        loglik: ll,
        n: times.len(),
        n_fail: r as usize,
        method: "MLE".into(),
        cov,
        se,
        times: times.to_vec(),
        failed: failed.to_vec(),
        success: true,
    }
}

/// Maximum likelihood fit with right-censored data. `failed`: true =
/// failure, false = suspended. Defaults to all failures.
pub fn fit_mle(dist: &str, times: &[f64], failed: Option<&[bool]>) -> Result<FitResult, String> {
    if dist_spec(dist).is_none() {
        return Err(format!("unknown distribution {}", dist));
    }
    let failed_v: Vec<bool> = match failed {
        Some(f) => f.to_vec(),
        None => vec![true; times.len()],
    };
    if times.is_empty() || !failed_v.iter().any(|f| *f) {
        return Err("need at least one failure to fit".into());
    }
    if dist == "weibull2" {
        return Ok(fit_weibull2_profile(times, &failed_v));
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
    let mut best: Option<(Vec<f64>, f64)> = None;
    for st in &starts {
        let u0 = to_unconstrained(dist, st, times);
        let mut neg = |u: &[f64]| -> f64 {
            let vals = from_unconstrained(dist, u, times);
            -loglik(dist, &vals, times, &failed_v)
        };
        let step: Vec<f64> = u0.iter().map(|_| 0.3).collect();
        let (x1, _) = optim::nelder_mead(&mut neg, &u0, &step, 5000, 1e-10);
        let (x2, f2) = optim::nelder_mead(&mut neg, &x1, &step, 5000, 1e-12);
        if best.is_none() || f2 < best.as_ref().unwrap().1 {
            best = Some((x2, f2));
        }
    }
    let (bu, _) = best.unwrap();
    let vals = from_unconstrained(dist, &bu, times);
    let params = pack(dist, &vals);
    let ll = loglik(dist, &vals, times, &failed_v);
    let (cov, se) = cov_and_se(dist, &vals, times, &failed_v);
    Ok(FitResult {
        dist: dist.into(),
        params,
        loglik: ll,
        n: times.len(),
        n_fail: failed_v.iter().filter(|f| **f).count(),
        method: "MLE".into(),
        cov,
        se,
        times: times.to_vec(),
        failed: failed_v,
        success: true,
    })
}

/// Benard median rank for the k-th failure out of n_total units.
pub fn median_ranks(n_total: usize, failure_order: f64) -> f64 {
    (failure_order - 0.3) / (n_total as f64 + 0.4)
}

/// Median-rank regression (probability-plot least squares) for
/// weibull2, lognormal and normal.
pub fn fit_rank_regression(
    dist: &str,
    times: &[f64],
    failed: Option<&[bool]>,
) -> Result<FitResult, String> {
    if !matches!(dist, "weibull2" | "lognormal" | "normal") {
        return Err("rank regression implemented for weibull2, lognormal, normal".into());
    }
    let failed_v: Vec<bool> = match failed {
        Some(f) => f.to_vec(),
        None => vec![true; times.len()],
    };
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
    let ll = loglik(dist, &vals, times, &failed_v);
    Ok(FitResult {
        dist: dist.into(),
        params,
        loglik: ll,
        n: times.len(),
        n_fail: tf.len(),
        method: "Rank regression".into(),
        cov: None,
        se: BTreeMap::new(),
        times: times.to_vec(),
        failed: failed_v,
        success: true,
    })
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
    let f_theo: Vec<f64> = times.iter().map(|t| cdf(&fit.dist, &fit.params, *t)).collect();
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
