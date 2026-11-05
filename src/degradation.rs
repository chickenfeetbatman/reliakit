//! Degradation analysis: per-unit path fits and pseudo-failure times.

use crate::distributions::{fit_mle, FitResult};
use crate::optim;

pub const MODELS: [&str; 3] = ["linear", "exponential", "power"];

/// Least-squares path fit. Returns (a, b): linear a + b t; exponential
/// exp(a + b t); power exp(a) * t^b.
pub fn fit_path(t: &[f64], y: &[f64], model: &str) -> Result<(f64, f64), String> {
    let design: Vec<Vec<f64>> = match model {
        "linear" => t.iter().map(|ti| vec![1.0, *ti]).collect(),
        "exponential" => {
            if y.iter().any(|v| *v <= 0.0) {
                return Err("exponential path needs positive measurements".into());
            }
            t.iter().map(|ti| vec![1.0, *ti]).collect()
        }
        "power" => {
            if y.iter().any(|v| *v <= 0.0) || t.iter().any(|v| *v <= 0.0) {
                return Err("power path needs positive times and measurements".into());
            }
            t.iter().map(|ti| vec![1.0, ti.ln()]).collect()
        }
        _ => return Err(format!("unknown degradation model {}", model)),
    };
    let target: Vec<f64> = match model {
        "linear" => y.to_vec(),
        _ => y.iter().map(|v| v.ln()).collect(),
    };
    let coef = optim::lstsq(&design, &target).ok_or("path lstsq failed")?;
    Ok((coef[0], coef[1]))
}

pub fn path_value(a: f64, b: f64, t: f64, model: &str) -> f64 {
    match model {
        "linear" => a + b * t,
        "exponential" => (a + b * t).exp(),
        "power" => a.exp() * t.powf(b),
        _ => f64::NAN,
    }
}

/// Analytic crossing time of a fitted path with the failure threshold.
/// Returns f64::INFINITY if the path never crosses in the failure
/// direction.
pub fn pseudo_failure_time(
    a: f64,
    b: f64,
    threshold: f64,
    model: &str,
    increasing: bool,
) -> f64 {
    match model {
        "linear" => {
            if b == 0.0 {
                return f64::INFINITY;
            }
            let tc = (threshold - a) / b;
            if tc > 0.0 && ((b > 0.0) == increasing) {
                tc
            } else {
                f64::INFINITY
            }
        }
        "exponential" => {
            if b == 0.0 || threshold <= 0.0 {
                return f64::INFINITY;
            }
            let tc = (threshold.ln() - a) / b;
            if tc > 0.0 && ((b > 0.0) == increasing) {
                tc
            } else {
                f64::INFINITY
            }
        }
        "power" => {
            if b == 0.0 || threshold <= 0.0 {
                return f64::INFINITY;
            }
            let tc = (threshold / a.exp()).powf(1.0 / b);
            if tc > 0.0 && ((b > 0.0) == increasing) {
                tc
            } else {
                f64::INFINITY
            }
        }
        _ => f64::INFINITY,
    }
}

#[derive(Debug, Clone)]
pub struct DegradationResult {
    pub paths: Vec<(f64, f64)>,
    pub pseudo_times: Vec<f64>,
    pub failed: Vec<bool>,
    pub fit: Option<FitResult>,
    pub model: String,
}

/// Fit each unit's path, extrapolate to the threshold for pseudo-failure
/// times (units that never cross are suspended at their last reading),
/// and fit a life distribution to the pseudo-failures.
pub fn degradation_analysis(
    units: &[(Vec<f64>, Vec<f64>)],
    threshold: f64,
    model: &str,
    dist: &str,
    increasing: bool,
) -> Result<DegradationResult, String> {
    let mut paths = Vec::new();
    let mut pseudo = Vec::new();
    let mut failed = Vec::new();
    for (t, y) in units {
        let (a, b) = fit_path(t, y, model)?;
        paths.push((a, b));
        let tc = pseudo_failure_time(a, b, threshold, model, increasing);
        if tc.is_finite() {
            pseudo.push(tc);
            failed.push(true);
        } else {
            pseudo.push(t.iter().cloned().fold(f64::MIN, f64::max));
            failed.push(false);
        }
    }
    let fit = if failed.iter().any(|f| *f) {
        Some(fit_mle(dist, &pseudo, Some(&failed))?)
    } else {
        None
    };
    Ok(DegradationResult {
        paths,
        pseudo_times: pseudo,
        failed,
        fit,
        model: model.into(),
    })
}
