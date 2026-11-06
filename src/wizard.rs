//! Distribution wizard: best-fit ranking by information criteria.
//!
//! Each candidate distribution is fitted by maximum likelihood, then
//! ranked by AIC. BIC and an Anderson-Darling statistic are reported
//! alongside so a user can see why the ranking came out as it did.

use crate::distributions::{self, FitResult};

/// Distributions offered by the wizard ranking.
///
/// `weibull1` is excluded: it is a one-parameter Weibull with a
/// user-fixed shape beta, so it is not fittable from data alone by
/// the generic MLE path used here. It stays available as a direct
/// calculation distribution, not as a wizard candidate.
pub const RANKABLE_DISTS: &[&str] = &[
    "weibull2",
    "weibull3",
    "lognormal",
    "normal",
    "exponential",
    "gamma",
    "gumbel",
    "loglogistic",
    "logistic",
    "gompertz",
    "gengamma",
    "exponential2",
    "mixed_weibull",
];

/// One row of a best-fit ranking.
#[derive(Debug, Clone)]
pub struct FitRank {
    pub dist: String,
    pub loglik: f64,
    pub aic: f64,
    pub bic: f64,
    pub ad_statistic: Option<f64>,
    pub n_params: usize,
    pub success: bool,
}

/// Anderson-Darling statistic for a fitted distribution.
///
/// A^2 = -n - (1/n) * sum_i (2i - 1) * [ln F(t_i) + ln(1 - F(t_{n+1-i}))]
/// over the sorted failure times, using the fitted CDF.
///
/// Returns None when any observation is censored: the classic
/// Anderson-Darling statistic is defined for a complete ordered
/// sample, and with right-censored times the ordered failure CDF
/// values do not form the complete-sample sequence the formula
/// needs. A censored-data variant would need a different statistic,
/// so this function declines rather than returning a number that
/// is not the standard A^2. Also returns None for fewer than two
/// failures.
pub fn anderson_darling(fit: &FitResult) -> Option<f64> {
    if fit.failed.iter().any(|f| !f) {
        return None;
    }
    let mut times: Vec<f64> = fit
        .times
        .iter()
        .zip(&fit.failed)
        .filter(|(_, f)| **f)
        .map(|(t, _)| *t)
        .collect();
    let n = times.len();
    if n < 2 {
        return None;
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let nf = n as f64;
    // CDF values in sorted order. Clamp away from 0 and 1 so the logs
    // stay finite for extreme fitted values.
    let cdf_vals: Vec<f64> = times
        .iter()
        .map(|t| distributions::cdf(&fit.dist, &fit.params, *t).clamp(1e-15, 1.0 - 1e-15))
        .collect();
    let mut sum = 0.0;
    for i in 1..=n {
        let f_i = cdf_vals[i - 1];
        let f_rev = cdf_vals[n - i];
        sum += (2 * i - 1) as f64 * (f_i.ln() + (1.0 - f_rev).ln());
    }
    Some(-nf - sum / nf)
}

/// Fit each candidate distribution by MLE and rank by AIC.
///
/// AIC = 2k - 2 * loglik and BIC = k * ln(n) - 2 * loglik, where k
/// is the parameter count from `distributions::dist_spec` and n is
/// the number of observations. Results are sorted ascending by AIC.
/// Distributions that cannot be fitted (unknown name, optimiser
/// failure, or a name whose fit only becomes available in a later
/// distributions update) sort last with `success = false`, a NaN
/// loglik and infinite AIC and BIC.
pub fn rank_distributions(times: &[f64], failed: Option<&[bool]>, dists: &[&str]) -> Vec<FitRank> {
    let n = times.len();
    let ln_n = if n > 0 { (n as f64).ln() } else { 0.0 };
    let mut out: Vec<FitRank> = Vec::with_capacity(dists.len());
    for dist in dists {
        let n_params = distributions::dist_spec(dist).map(|s| s.len()).unwrap_or(0);
        match distributions::fit_mle(dist, times, failed) {
            Ok(fit) if fit.success && fit.loglik.is_finite() => {
                let k = n_params as f64;
                out.push(FitRank {
                    dist: dist.to_string(),
                    loglik: fit.loglik,
                    aic: 2.0 * k - 2.0 * fit.loglik,
                    bic: k * ln_n - 2.0 * fit.loglik,
                    ad_statistic: anderson_darling(&fit),
                    n_params,
                    success: true,
                });
            }
            _ => {
                out.push(FitRank {
                    dist: dist.to_string(),
                    loglik: f64::NAN,
                    aic: f64::INFINITY,
                    bic: f64::INFINITY,
                    ad_statistic: None,
                    n_params,
                    success: false,
                });
            }
        }
    }
    out.sort_by(|a, b| {
        // Successful fits first, then ascending AIC.
        b.success.cmp(&a.success).then_with(|| {
            a.aic
                .partial_cmp(&b.aic)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });
    out
}
