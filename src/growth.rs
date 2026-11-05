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
