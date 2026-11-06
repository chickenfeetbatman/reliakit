//! Reliability growth: Crow-AMSAA and Duane models (RGA basics).

use crate::optim;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrowAmsaaFit {
    pub beta: f64,
    pub lam: f64,
    #[serde(rename = "T")]
    pub t_end: f64,
    pub n_fail: usize,
}

impl CrowAmsaaFit {
    /// Failure intensity rho(t) = lam * beta * t^(beta-1).
    pub fn intensity(&self, t: f64) -> f64 {
        self.lam * self.beta * t.powf(self.beta - 1.0)
    }
    pub fn instantaneous_mtbf(&self, t: f64) -> f64 {
        1.0 / self.intensity(t)
    }
    pub fn cumulative_mtbf(&self, t: f64) -> f64 {
        t / (self.lam * t.powf(self.beta))
    }
}

/// Crow-AMSAA MLE for a time-terminated test ending at T:
/// beta_hat = N / sum ln(T / t_i); lambda_hat = N / T^beta.
pub fn fit_crow_amsaa(failure_times: &[f64], t_end: Option<f64>) -> CrowAmsaaFit {
    let mut t = failure_times.to_vec();
    t.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let t_end = t_end.unwrap_or_else(|| t.iter().cloned().fold(f64::MIN, f64::max));
    let n = t.len();
    let denom: f64 = t.iter().map(|ti| (t_end / ti).ln()).sum();
    let beta = if denom > 0.0 { n as f64 / denom } else { f64::NAN };
    let lam = n as f64 / t_end.powf(beta);
    CrowAmsaaFit {
        beta,
        lam,
        t_end,
        n_fail: n,
    }
}

#[derive(Debug, Clone)]
pub struct DuaneFit {
    pub alpha: f64,
    pub intercept: f64,
    pub times: Vec<f64>,
    pub cum_mtbf: Vec<f64>,
}

impl DuaneFit {
    pub fn cumulative_mtbf_at(&self, t: f64) -> f64 {
        self.intercept.exp() * t.powf(self.alpha)
    }
    pub fn instantaneous_mtbf_at(&self, t: f64) -> f64 {
        self.cumulative_mtbf_at(t) / (1.0 - self.alpha)
    }
}

// ---------------------------------------------------------------------
// Gompertz and Lloyd-Lipow cumulative failure models.
//
// Both fit observed pairs (time, cumulative failures), in the same
// data style as the fits above: `times` are observation times and
// `cum_failures` is the cumulative failure count at each time.
// ---------------------------------------------------------------------

/// Gompertz growth fit: cumulative failures
/// N(t) = a * b^(c^t), with a > 0, 0 < b < 1 and 0 < c < 1.
///
/// As t grows, c^t falls to 0 and N(t) rises toward the asymptote
/// a, the total failure count the population will ever produce.
#[derive(Debug, Clone)]
pub struct GompertzFit {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub times: Vec<f64>,
    pub cum_failures: Vec<f64>,
    /// Sum of squared errors of the fitted curve on the fit data.
    pub sse: f64,
}

impl GompertzFit {
    /// Predicted cumulative failures at time t.
    pub fn predicted(&self, t: f64) -> f64 {
        self.a * self.b.powf(self.c.powf(t))
    }
    /// Alias for `predicted`, matching the naming of the other fits.
    pub fn cumulative_failures_at(&self, t: f64) -> f64 {
        self.predicted(t)
    }
}

fn gompertz_value(a: f64, b: f64, c: f64, t: f64) -> f64 {
    a * b.powf(c.powf(t))
}

/// Fit the Gompertz cumulative failure model by least squares.
///
/// The fit minimises the sum of squared errors between observed
/// and predicted cumulative failures, using Nelder-Mead in a
/// transformed parameter space: log a for the positive asymptote,
/// and logit transforms for b and c so they stay inside (0, 1).
/// Several starts are tried and the best result is refined with a
/// second, smaller simplex.
pub fn fit_gompertz(
    times: &[f64],
    cum_failures: &[f64],
) -> Result<GompertzFit, String> {
    if times.is_empty() || times.len() != cum_failures.len() {
        return Err("times and cum_failures must be non-empty and the same length".into());
    }
    if times.iter().any(|t| !t.is_finite() || *t < 0.0)
        || cum_failures.iter().any(|v| !v.is_finite() || *v < 0.0)
    {
        return Err("times and cum_failures must be finite and non-negative".into());
    }
    let logit = |p: f64| (p / (1.0 - p)).ln();
    let sigmoid = |u: f64| 1.0 / (1.0 + (-u).exp());
    let sse_of = |theta: &[f64]| -> f64 {
        let a = theta[0].exp();
        let b = sigmoid(theta[1]);
        let c = sigmoid(theta[2]);
        if !a.is_finite() || a <= 0.0 {
            return 1e100;
        }
        let mut sse = 0.0;
        for (t, obs) in times.iter().zip(cum_failures) {
            let pred = gompertz_value(a, b, c, *t);
            if !pred.is_finite() {
                return 1e100;
            }
            sse += (obs - pred) * (obs - pred);
        }
        if sse.is_finite() {
            sse
        } else {
            1e100
        }
    };

    let max_cum = cum_failures
        .iter()
        .cloned()
        .fold(0.0f64, f64::max)
        .max(1.0);
    let mut best: Option<(Vec<f64>, f64)> = None;
    for a_start in [max_cum * 1.05, max_cum * 1.5, max_cum * 2.5] {
        for b_start in [0.3f64, 0.55, 0.8] {
            for c_start in [0.7f64, 0.9, 0.98] {
                let theta0 = vec![a_start.ln(), logit(b_start), logit(c_start)];
                let step = vec![0.3, 0.4, 0.4];
                let mut obj = sse_of;
                let (theta, f) =
                    optim::nelder_mead(&mut obj, &theta0, &step, 2000, 1e-10);
                if best.is_none() || f < best.as_ref().unwrap().1 {
                    best = Some((theta, f));
                }
            }
        }
    }
    let (theta_best, _) = best.unwrap();
    // Refine from the best start with a smaller simplex.
    let step2 = vec![0.05, 0.05, 0.05];
    let mut obj2 = sse_of;
    let (theta, f_best) =
        optim::nelder_mead(&mut obj2, &theta_best, &step2, 3000, 1e-12);
    if !f_best.is_finite() || f_best >= 1e99 {
        return Err("Gompertz optimisation failed".into());
    }
    let fit = GompertzFit {
        a: theta[0].exp(),
        b: sigmoid(theta[1]),
        c: sigmoid(theta[2]),
        times: times.to_vec(),
        cum_failures: cum_failures.to_vec(),
        sse: f_best,
    };
    Ok(fit)
}

/// Lloyd-Lipow growth fit: cumulative failures
/// N(t) = n_inf - alpha / t, with n_inf the asymptotic total
/// failure count and alpha > 0 for a growing curve.
#[derive(Debug, Clone)]
pub struct LloydLipowFit {
    pub n_inf: f64,
    pub alpha: f64,
    pub times: Vec<f64>,
    pub cum_failures: Vec<f64>,
}

impl LloydLipowFit {
    /// Predicted cumulative failures at time t (t must be positive).
    pub fn predicted(&self, t: f64) -> f64 {
        self.n_inf - self.alpha / t
    }
    /// Alias for `predicted`, matching the naming of the other fits.
    pub fn cumulative_failures_at(&self, t: f64) -> f64 {
        self.predicted(t)
    }
}

/// Fit the Lloyd-Lipow model by least squares.
///
/// The model is linear in its parameters: regressing cumulative
/// failures on 1 / t gives intercept n_inf and slope -alpha, so the
/// fit is an ordinary least squares solved with `optim::lstsq`.
/// Times must be positive, since 1 / t is a regressor.
pub fn fit_lloyd_lipow(
    times: &[f64],
    cum_failures: &[f64],
) -> Result<LloydLipowFit, String> {
    if times.is_empty() || times.len() != cum_failures.len() {
        return Err("times and cum_failures must be non-empty and the same length".into());
    }
    if times.iter().any(|t| !t.is_finite() || *t <= 0.0) {
        return Err("all times must be positive and finite for Lloyd-Lipow".into());
    }
    let design: Vec<Vec<f64>> = times.iter().map(|t| vec![1.0, 1.0 / t]).collect();
    let coef = optim::lstsq(&design, cum_failures).ok_or("Lloyd-Lipow lstsq failed")?;
    Ok(LloydLipowFit {
        n_inf: coef[0],
        alpha: -coef[1],
        times: times.to_vec(),
        cum_failures: cum_failures.to_vec(),
    })
}

/// Duane model: log-log regression of cumulative MTBF on time.
pub fn fit_duane(failure_times: &[f64]) -> Result<DuaneFit, String> {
    let mut t = failure_times.to_vec();
    t.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let cum: Vec<f64> = t
        .iter()
        .enumerate()
        .map(|(i, ti)| ti / (i + 1) as f64)
        .collect();
    let design: Vec<Vec<f64>> = t.iter().map(|ti| vec![ti.ln(), 1.0]).collect();
    let y: Vec<f64> = cum.iter().map(|c| c.ln()).collect();
    let coef = optim::lstsq(&design, &y).ok_or("Duane lstsq failed")?;
    Ok(DuaneFit {
        alpha: coef[0],
        intercept: coef[1],
        times: t,
        cum_mtbf: cum,
    })
}
