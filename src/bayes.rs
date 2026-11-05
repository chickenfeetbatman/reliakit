//! Limited-data methods for small samples and pooled lots.
//!
//! 1. Weibull shape bias correction (first-order, Hirose-style).
//! 2. Hierarchical empirical-Bayes shrinkage (normal-normal on the log
//!    scale, DerSimonian-Laird estimation of the group variance).
//! 3. Split-conformal and distribution-free lower reliability bounds.

use crate::distributions::{cdf, fit_mle, quantile, FitResult};
use crate::optim;
use crate::specfunc;

/// First-order multiplicative bias factor for the Weibull shape MLE:
/// divide by (1 + 1.37 / r), r = number of failures.
pub fn weibull_shape_bias_factor(n_failures: usize) -> f64 {
    let r = n_failures.max(1) as f64;
    1.0 / (1.0 + 1.37 / r)
}

/// Return a copy of a weibull2 FitResult with bias-corrected shape. Eta
/// is re-estimated by one-dimensional MLE at the corrected shape.
pub fn bias_corrected_weibull(fit: &FitResult) -> Result<FitResult, String> {
    if fit.dist != "weibull2" {
        return Err("bias correction implemented for weibull2 fits".into());
    }
    let beta_c = fit.params["beta"] * weibull_shape_bias_factor(fit.n_fail);
    let times = &fit.times;
    let failed = &fit.failed;
    let neg_loglik_logeta = |logeta: f64| -> f64 {
        let eta = logeta.exp();
        let mut ll = 0.0;
        for (i, &t) in times.iter().enumerate() {
            if failed[i] {
                ll += beta_c.ln() - beta_c * eta.ln() + (beta_c - 1.0) * t.ln()
                    - (t / eta).powf(beta_c);
            } else {
                ll += -(t / eta).powf(beta_c);
            }
        }
        -ll
    };
    let mut f = neg_loglik_logeta;
    let centre = fit.params["eta"].ln();
    let (logeta, _) = optim::minimize_scalar(&mut f, centre - 3.0, centre + 3.0, 1e-10);
    let eta_c = logeta.exp();
    let mut out = fit.clone();
    out.params
        .insert("beta".into(), beta_c);
    out.params.insert("eta".into(), eta_c);
    out.method = "Bias-corrected MLE".into();
    let vals = vec![beta_c, eta_c];
    out.loglik = crate::distributions::loglik("weibull2", &vals, times, failed);
    Ok(out)
}

#[derive(Debug, Clone)]
pub struct ShrinkResult {
    pub shrunk: Vec<f64>,
    pub post_sd: Vec<f64>,
    pub b: Vec<f64>,
    pub mu: f64,
    pub tau2: f64,
    pub q: f64,
}

/// Normal-normal empirical-Bayes shrinkage (DerSimonian-Laird).
pub fn shrink_normal(estimates: &[f64], ses: &[f64]) -> ShrinkResult {
    let y = estimates;
    let v: Vec<f64> = ses.iter().map(|s| s * s).collect();
    let g = y.len();
    if g < 2 {
        return ShrinkResult {
            shrunk: y.to_vec(),
            post_sd: ses.to_vec(),
            b: vec![0.0; g],
            mu: if g == 1 { y[0] } else { 0.0 },
            tau2: 0.0,
            q: 0.0,
        };
    }
    let w: Vec<f64> = v.iter().map(|vi| 1.0 / vi).collect();
    let sw: f64 = w.iter().sum();
    let mu_fe: f64 = w.iter().zip(y).map(|(wi, yi)| wi * yi).sum::<f64>() / sw;
    let q: f64 = w
        .iter()
        .zip(y)
        .map(|(wi, yi)| wi * (yi - mu_fe).powi(2))
        .sum();
    let denom = sw - w.iter().map(|wi| wi * wi).sum::<f64>() / sw;
    let tau2 = if denom > 0.0 {
        ((q - (g - 1) as f64) / denom).max(0.0)
    } else {
        0.0
    };
    if tau2 == 0.0 {
        let sd = (1.0 / sw).sqrt();
        return ShrinkResult {
            shrunk: vec![mu_fe; g],
            post_sd: vec![sd; g],
            b: vec![1.0; g],
            mu: mu_fe,
            tau2,
            q,
        };
    }
    let b: Vec<f64> = v.iter().map(|vi| vi / (vi + tau2)).collect();
    let shrunk: Vec<f64> = y
        .iter()
        .zip(&b)
        .map(|(yi, bi)| (1.0 - bi) * yi + bi * mu_fe)
        .collect();
    let post_sd: Vec<f64> = b
        .iter()
        .zip(&v)
        .map(|(bi, vi)| ((1.0 - bi) * vi).sqrt())
        .collect();
    ShrinkResult {
        shrunk,
        post_sd,
        b,
        mu: mu_fe,
        tau2,
        q,
    }
}

#[derive(Debug, Clone)]
pub struct HierarchicalResult {
    pub fits: Vec<FitResult>,
    pub mu_log_eta: f64,
    pub tau2: f64,
    pub b: Vec<f64>,
    pub pooled_beta: f64,
    pub group_fits: Vec<FitResult>,
}

/// Hierarchical empirical-Bayes Weibull fit across groups: per-group
/// MLE, shape pooled by inverse-variance weighting on the log scale,
/// log eta shrunk toward the group mean.
pub fn hierarchical_weibull(
    groups: &[(Vec<f64>, Vec<bool>)],
) -> Result<HierarchicalResult, String> {
    let mut fits = Vec::new();
    for (t, f) in groups {
        fits.push(fit_mle("weibull2", t, Some(f))?);
    }
    let log_eta: Vec<f64> = fits.iter().map(|f| f.params["eta"].ln()).collect();
    let se_log_eta: Vec<f64> = fits
        .iter()
        .map(|f| match &f.cov {
            Some(c) => (c[1][1].max(0.0)).sqrt() / f.params["eta"],
            None => 0.5,
        })
        .collect();
    let res = shrink_normal(&log_eta, &se_log_eta);
    let log_beta: Vec<f64> = fits.iter().map(|f| f.params["beta"].ln()).collect();
    let se_log_beta: Vec<f64> = fits
        .iter()
        .map(|f| match &f.cov {
            Some(c) => (c[0][0].max(0.0)).sqrt() / f.params["beta"],
            None => 0.5,
        })
        .collect();
    let w: Vec<f64> = se_log_beta
        .iter()
        .map(|s| 1.0 / (s * s).max(1e-9))
        .collect();
    let sw: f64 = w.iter().sum();
    let pooled_beta: f64 =
        (w.iter().zip(&log_beta).map(|(wi, lb)| wi * lb).sum::<f64>() / sw).exp();
    let mut out = Vec::new();
    for (j, f) in fits.iter().enumerate() {
        let mut nf = f.clone();
        nf.params.insert("beta".into(), pooled_beta);
        nf.params.insert("eta".into(), res.shrunk[j].exp());
        nf.method = "Hierarchical EB".into();
        out.push(nf);
    }
    Ok(HierarchicalResult {
        fits: out,
        mu_log_eta: res.mu,
        tau2: res.tau2,
        b: res.b,
        pooled_beta,
        group_fits: fits,
    })
}

/// Split-conformal certified mission time: returns L with
/// P(T_new >= L) >= 1 - alpha over calibration sample and future unit.
pub fn conformal_mission_time(fit: &FitResult, calib_times: &[f64], alpha: f64) -> f64 {
    let m = calib_times.len();
    let mut s: Vec<f64> = calib_times
        .iter()
        .map(|t| cdf(&fit.dist, &fit.params, *t))
        .collect();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let j = ((m + 1) as f64 * alpha).floor() as i64;
    if j <= 0 || m == 0 {
        return 0.0;
    }
    let q = s[(j - 1) as usize].clamp(1e-12, 1.0 - 1e-12);
    quantile(&fit.dist, &fit.params, q)
}

/// Distribution-free lower bound on R(t) at confidence 1 - alpha
/// (exact Clopper-Pearson form on the calibration failure count).
pub fn conformal_reliability_lower(calib_times: &[f64], t: f64, alpha: f64) -> f64 {
    let m = calib_times.len();
    if m == 0 {
        return 0.0;
    }
    let n_fail = calib_times.iter().filter(|ct| **ct <= t).count();
    let f_upper = if n_fail == 0 {
        1.0 - alpha.powf(1.0 / m as f64)
    } else {
        specfunc::beta_ppf(
            (n_fail + 1) as f64,
            (m - n_fail) as f64,
            1.0 - alpha,
        )
    };
    (1.0 - f_upper).clamp(0.0, 1.0)
}

/// Conformal upper bound on F(t) via PIT order statistics (the count
/// form of the same construction).
pub fn conformal_pit_upper_cdf(fit: &FitResult, calib_times: &[f64], t: f64, alpha: f64) -> f64 {
    let _ = (fit, t);
    1.0 - conformal_reliability_lower(calib_times, t, alpha)
}
