//! QCP: the quick calculation pad.
//!
//! Given a distribution and its parameters, report the standard
//! life quantities at one mission time, plus a Bx life.

use crate::distributions::{self, Params};

/// Result of one quick calculation.
#[derive(Debug, Clone)]
pub struct QcpResult {
    pub reliability: f64,
    pub unreliability: f64,
    pub hazard: f64,
    pub mean_life: f64,
    /// Bx life: the time by which a fraction `bx` of units fail.
    pub bx_life: f64,
    pub pdf: f64,
}

/// Quick calculation for `dist` with `params` at time `t`.
///
/// `bx` is a failure fraction in (0, 1), so for example bx = 0.10
/// gives the B10 life, which is the 0.10 quantile of the life
/// distribution.
pub fn quick_calc(dist: &str, params: &Params, t: f64, bx: f64) -> Result<QcpResult, String> {
    if distributions::dist_spec(dist).is_none() {
        return Err(format!("unknown distribution {}", dist));
    }
    if !t.is_finite() || t < 0.0 {
        return Err("mission time t must be finite and non-negative".into());
    }
    if !(bx > 0.0 && bx < 1.0) {
        return Err("bx must be a failure fraction in (0, 1)".into());
    }
    // Every parameter named by the spec must be present and finite.
    for (name, _kind) in distributions::dist_spec(dist).unwrap() {
        match params.get(name) {
            Some(v) if v.is_finite() => {}
            _ => return Err(format!("missing or non-finite parameter {}", name)),
        }
    }
    let reliability = distributions::reliability(dist, params, t);
    Ok(QcpResult {
        reliability,
        unreliability: distributions::cdf(dist, params, t),
        hazard: distributions::hazard(dist, params, t),
        mean_life: distributions::mean_life(dist, params),
        bx_life: distributions::quantile(dist, params, bx),
        pdf: distributions::pdf(dist, params, t),
    })
}
