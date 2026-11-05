//! Accelerated life testing (ALTA basics).
//!
//! Life-stress models fitted by least squares on log failure times with
//! a common log-scale scatter (lognormal life at each stress):
//! Arrhenius, inverse power law, and Eyring (temperature + voltage).

use crate::optim;
use crate::specfunc;

pub const K_B_EV: f64 = 8.617333262e-5; // Boltzmann constant, eV/K

/// AF = exp(Ea/k * (1/T_use - 1/T_stress)), temperatures in Celsius.
pub fn acceleration_factor_arrhenius(ea_ev: f64, t_use_c: f64, t_stress_c: f64) -> f64 {
    let tu = t_use_c + 273.15;
    let ts = t_stress_c + 273.15;
    (ea_ev / K_B_EV * (1.0 / tu - 1.0 / ts)).exp()
}

/// AF = (V_stress / V_use)^n.
pub fn acceleration_factor_ipl(n: f64, v_use: f64, v_stress: f64) -> f64 {
    (v_stress / v_use).powf(n)
}

#[derive(Debug, Clone)]
pub struct AltFit {
    pub model: String,
    pub coef: std::collections::BTreeMap<String, f64>,
    pub sigma: f64,
    pub n: usize,
}

impl AltFit {
    pub fn mean_log_life(&self, stress: &[f64]) -> f64 {
        match self.model.as_str() {
            "arrhenius" => {
                let t = if stress[0] < 400.0 { stress[0] + 273.15 } else { stress[0] };
                self.coef["a"] + self.coef["b"] / t
            }
            "ipl" => self.coef["a"] - self.coef["n"] * stress[0].ln(),
            "eyring" => {
                let t = if stress[0] < 400.0 { stress[0] + 273.15 } else { stress[0] };
                self.coef["a"] + self.coef["b"] / t - self.coef["n"] * stress[1].ln()
            }
            "eyring_interaction" => {
                // Convention: mean log life = a + b / T_k - n * ln V
                //   + ab * (1 / T_k) * (-ln V)
                // so the voltage term uses -ln V and the interaction
                // column is (1 / T_k) * (-ln V).
                let t = if stress[0] < 400.0 { stress[0] + 273.15 } else { stress[0] };
                let inv_t = 1.0 / t;
                let neg_ln_v = -stress[1].ln();
                self.coef["a"]
                    + self.coef["b"] * inv_t
                    + self.coef["n"] * neg_ln_v
                    + self.coef["ab"] * inv_t * neg_ln_v
            }
            _ => f64::NAN,
        }
    }

    pub fn reliability(&self, t: f64, stress: &[f64]) -> f64 {
        let mu = self.mean_log_life(stress);
        specfunc::norm_sf((t.ln() - mu) / self.sigma)
    }

    pub fn median_life(&self, stress: &[f64]) -> f64 {
        self.mean_log_life(stress).exp()
    }

    pub fn activation_energy_ev(&self) -> Option<f64> {
        self.coef.get("b").map(|b| b * K_B_EV)
    }
}

fn design(model: &str, stress: &[Vec<f64>]) -> Result<(Vec<Vec<f64>>, Vec<&'static str>), String> {
    match model {
        "arrhenius" => {
            let rows: Vec<Vec<f64>> = stress
                .iter()
                .map(|s| {
                    let t = if s[0] < 400.0 { s[0] + 273.15 } else { s[0] };
                    vec![1.0, 1.0 / t]
                })
                .collect();
            Ok((rows, vec!["a", "b"]))
        }
        "ipl" => {
            let rows: Vec<Vec<f64>> = stress.iter().map(|s| vec![1.0, -s[0].ln()]).collect();
            Ok((rows, vec!["a", "n"]))
        }
        "eyring" => {
            let rows: Vec<Vec<f64>> = stress
                .iter()
                .map(|s| {
                    let t = if s[0] < 400.0 { s[0] + 273.15 } else { s[0] };
                    vec![1.0, 1.0 / t, -s[1].ln()]
                })
                .collect();
            Ok((rows, vec!["a", "b", "n"]))
        }
        "eyring_interaction" => {
            // Design row: [1, 1 / T_k, -ln V, (1 / T_k) * (-ln V)].
            // With coef keys a, b, n, ab this gives mean log life
            // a + b / T_k - n * ln V + ab * (1 / T_k) * (-ln V).
            let rows: Vec<Vec<f64>> = stress
                .iter()
                .map(|s| {
                    let t = if s[0] < 400.0 { s[0] + 273.15 } else { s[0] };
                    let inv_t = 1.0 / t;
                    let neg_ln_v = -s[1].ln();
                    vec![1.0, inv_t, neg_ln_v, inv_t * neg_ln_v]
                })
                .collect();
            Ok((rows, vec!["a", "b", "n", "ab"]))
        }
        _ => Err(format!("unknown ALT model {}", model)),
    }
}

/// Least-squares ALT fit. `stress` has one row per unit: [T_C] for
/// Arrhenius, [V] for IPL, [T_C, V] for Eyring. Complete data.
pub fn fit_alt(model: &str, stress: &[Vec<f64>], times: &[f64]) -> Result<AltFit, String> {
    let (x, names) = design(model, stress)?;
    let y: Vec<f64> = times.iter().map(|t| t.ln()).collect();
    let coef = optim::lstsq(&x, &y).ok_or("ALT lstsq failed")?;
    let mut ss = 0.0;
    for (i, row) in x.iter().enumerate() {
        let pred: f64 = row.iter().zip(&coef).map(|(a, b)| a * b).sum();
        ss += (y[i] - pred).powi(2);
    }
    let dof = (y.len() as i64 - x[0].len() as i64).max(1) as f64;
    let sigma = (ss / dof).sqrt();
    let mut map = std::collections::BTreeMap::new();
    for (nm, c) in names.iter().zip(&coef) {
        map.insert(nm.to_string(), *c);
    }
    Ok(AltFit {
        model: model.into(),
        coef: map,
        sigma,
        n: y.len(),
    })
}

// ---------------------------------------------------------------------
// Censored maximum-likelihood ALT fitting.
//
// Log life at a given stress is Normal(mu(stress), sigma) with the
// same design (life-stress relation) as the least-squares fits.
// Failed units contribute the normal log pdf of log time; suspended
// (right-censored) units contribute the log survival probability.
// ---------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AltMleFit {
    pub model: String,
    pub coef: std::collections::BTreeMap<String, f64>,
    pub sigma: f64,
    pub loglik: f64,
    pub cov: Option<Vec<Vec<f64>>>,
    pub n: usize,
    pub n_fail: usize,
}

/// Coefficient names in design order for each model.
fn coef_names(model: &str) -> Option<Vec<&'static str>> {
    match model {
        "arrhenius" => Some(vec!["a", "b"]),
        "ipl" => Some(vec!["a", "n"]),
        "eyring" => Some(vec!["a", "b", "n"]),
        "eyring_interaction" => Some(vec!["a", "b", "n", "ab"]),
        _ => None,
    }
}

/// Mean log life from a coefficient map, for any supported model.
fn mu_from_coef(
    model: &str,
    coef: &std::collections::BTreeMap<String, f64>,
    stress: &[f64],
) -> f64 {
    let get = |k: &str| coef.get(k).copied().unwrap_or(f64::NAN);
    match model {
        "arrhenius" => {
            let t = if stress[0] < 400.0 { stress[0] + 273.15 } else { stress[0] };
            get("a") + get("b") / t
        }
        "ipl" => get("a") - get("n") * stress[0].ln(),
        "eyring" => {
            let t = if stress[0] < 400.0 { stress[0] + 273.15 } else { stress[0] };
            get("a") + get("b") / t - get("n") * stress[1].ln()
        }
        "eyring_interaction" => {
            let t = if stress[0] < 400.0 { stress[0] + 273.15 } else { stress[0] };
            let inv_t = 1.0 / t;
            let neg_ln_v = -stress[1].ln();
            get("a") + get("b") * inv_t + get("n") * neg_ln_v + get("ab") * inv_t * neg_ln_v
        }
        _ => f64::NAN,
    }
}

/// Full parameter vector [coef in design order..., log sigma] from a fit.
fn mle_theta(fit: &AltMleFit) -> Option<Vec<f64>> {
    let names = coef_names(&fit.model)?;
    let mut theta: Vec<f64> = Vec::with_capacity(names.len() + 1);
    for nm in &names {
        theta.push(*fit.coef.get(*nm)?);
    }
    theta.push(fit.sigma.ln());
    Some(theta)
}

/// Reliability at time t from a raw parameter vector.
fn reliability_from_theta(model: &str, theta: &[f64], t: f64, stress: &[f64]) -> f64 {
    let names = match coef_names(model) {
        Some(n) => n,
        None => return f64::NAN,
    };
    if theta.len() != names.len() + 1 {
        return f64::NAN;
    }
    let mut map = std::collections::BTreeMap::new();
    for (nm, v) in names.iter().zip(theta.iter()) {
        map.insert(nm.to_string(), *v);
    }
    let mu = mu_from_coef(model, &map, stress);
    let sigma = theta[theta.len() - 1].exp();
    if !mu.is_finite() || !sigma.is_finite() || sigma <= 0.0 {
        return f64::NAN;
    }
    specfunc::norm_sf((t.ln() - mu) / sigma)
}

impl AltMleFit {
    pub fn mean_log_life(&self, stress: &[f64]) -> f64 {
        mu_from_coef(&self.model, &self.coef, stress)
    }

    pub fn reliability(&self, t: f64, stress: &[f64]) -> f64 {
        let mu = self.mean_log_life(stress);
        specfunc::norm_sf((t.ln() - mu) / self.sigma)
    }

    pub fn median_life(&self, stress: &[f64]) -> f64 {
        self.mean_log_life(stress).exp()
    }

    pub fn activation_energy_ev(&self) -> Option<f64> {
        self.coef.get("b").map(|b| b * K_B_EV)
    }

    /// Delta-method confidence bounds on reliability at time t.
    ///
    /// The numeric gradient of R with respect to the full parameter
    /// vector [coef..., log sigma] gives var(R) = g' cov g. The
    /// interval is built on the logit scale so it stays inside [0, 1]
    /// after clamping. `alpha` is the significance level, so 0.05
    /// gives an approximate 95 percent interval. Without a covariance
    /// matrix the point estimate is returned for both bounds.
    pub fn reliability_bounds(&self, t: f64, stress: &[f64], alpha: f64) -> (f64, f64) {
        let r = self.reliability(t, stress);
        if !r.is_finite() {
            return (r, r);
        }
        let cov = match &self.cov {
            Some(c) => c,
            None => return (r, r),
        };
        let theta = match mle_theta(self) {
            Some(v) => v,
            None => return (r, r),
        };
        let p = theta.len();
        if cov.len() != p || cov.iter().any(|row| row.len() != p) {
            return (r, r);
        }
        // Numeric gradient of R wrt the full parameter vector. The
        // step is scaled to the parameter magnitude because some
        // coefficients (for example the Arrhenius b) are in the
        // thousands while log sigma is near zero.
        let mut grad = vec![0.0; p];
        for i in 0..p {
            let h = 1e-4 * theta[i].abs().max(1.0);
            let mut tp = theta.clone();
            let mut tm = theta.clone();
            tp[i] += h;
            tm[i] -= h;
            let rp = reliability_from_theta(&self.model, &tp, t, stress);
            let rm = reliability_from_theta(&self.model, &tm, t, stress);
            if rp.is_finite() && rm.is_finite() {
                grad[i] = (rp - rm) / (2.0 * h);
            } else {
                grad[i] = 0.0;
            }
        }
        let mut var = 0.0;
        for i in 0..p {
            for j in 0..p {
                var += grad[i] * cov[i][j] * grad[j];
            }
        }
        if !var.is_finite() || var <= 0.0 {
            return (r, r);
        }
        let se = var.sqrt();
        if r <= 0.0 || r >= 1.0 {
            let lo = (r - se).clamp(0.0, 1.0);
            let hi = (r + se).clamp(0.0, 1.0);
            return (lo.min(r), hi.max(r));
        }
        let a = alpha.clamp(1e-6, 0.999999);
        let z = specfunc::norm_ppf(1.0 - 0.5 * a);
        let logit = (r / (1.0 - r)).ln();
        let se_logit = se / (r * (1.0 - r));
        let lo_l = logit - z * se_logit;
        let hi_l = logit + z * se_logit;
        let lo = (1.0 / (1.0 + (-lo_l).exp())).clamp(0.0, 1.0);
        let hi = (1.0 / (1.0 + (-hi_l).exp())).clamp(0.0, 1.0);
        (lo.min(r), hi.max(r))
    }
}

/// Negative log-likelihood of the censored lognormal ALT model.
/// theta is [coef..., log sigma]; x is the design matrix; y is log
/// time; failed marks observed failures (false means suspended).
fn alt_neg_loglik(theta: &[f64], x: &[Vec<f64>], y: &[f64], failed: &[bool]) -> f64 {
    let k = x[0].len();
    if theta.len() != k + 1 {
        return 1e100;
    }
    let log_sigma = theta[k];
    if !log_sigma.is_finite() || log_sigma < -10.0 || log_sigma > 5.0 {
        return 1e100;
    }
    let sigma = log_sigma.exp();
    let coef = &theta[..k];
    let mut nll = 0.0;
    for i in 0..y.len() {
        let mu: f64 = x[i].iter().zip(coef.iter()).map(|(a, b)| a * b).sum();
        if !mu.is_finite() {
            return 1e100;
        }
        let z = (y[i] - mu) / sigma;
        if !z.is_finite() {
            return 1e100;
        }
        if failed[i] {
            // Normal log pdf of log time.
            let log_pdf = -log_sigma - 0.5 * (2.0 * std::f64::consts::PI).ln() - 0.5 * z * z;
            nll -= log_pdf;
        } else {
            let sf = specfunc::norm_sf(z);
            if !(sf > 0.0) || !sf.is_finite() {
                return 1e100;
            }
            nll -= sf.ln();
        }
        if !nll.is_finite() {
            return 1e100;
        }
    }
    nll
}

/// Numeric Hessian of a scalar function at theta. The base step is
/// eps = 1e-4, scaled by the parameter magnitude (with a floor of 1)
/// so large coefficients such as the Arrhenius b get a usable finite
/// difference step.
fn numeric_hessian<F: FnMut(&[f64]) -> f64>(f: &mut F, theta: &[f64]) -> Vec<Vec<f64>> {
    let p = theta.len();
    let eps = 1e-4;
    let steps: Vec<f64> = theta.iter().map(|v| eps * v.abs().max(1.0)).collect();
    let f0 = f(theta);
    let mut hess = vec![vec![0.0; p]; p];
    for i in 0..p {
        let mut tp = theta.to_vec();
        let mut tm = theta.to_vec();
        tp[i] += steps[i];
        tm[i] -= steps[i];
        let fp = f(&tp);
        let fm = f(&tm);
        hess[i][i] = (fp - 2.0 * f0 + fm) / (steps[i] * steps[i]);
        for j in (i + 1)..p {
            let mut tpp = theta.to_vec();
            let mut tpm = theta.to_vec();
            let mut tmp = theta.to_vec();
            let mut tmm = theta.to_vec();
            tpp[i] += steps[i];
            tpp[j] += steps[j];
            tpm[i] += steps[i];
            tpm[j] -= steps[j];
            tmp[i] -= steps[i];
            tmp[j] += steps[j];
            tmm[i] -= steps[i];
            tmm[j] -= steps[j];
            let val = (f(&tpp) - f(&tpm) - f(&tmp) + f(&tmm)) / (4.0 * steps[i] * steps[j]);
            hess[i][j] = val;
            hess[j][i] = val;
        }
    }
    hess
}

/// Censored maximum-likelihood ALT fit.
///
/// `stress` has one row per unit, in the same layout as `fit_alt`.
/// `times` holds failure times for failed units and censoring times
/// for suspended units. `failed` marks failures; when it is None
/// every unit is treated as failed. Optimisation starts from the
/// least-squares fit and runs Nelder-Mead in two rounds.
pub fn fit_alt_mle(
    model: &str,
    stress: &[Vec<f64>],
    times: &[f64],
    failed: Option<&[bool]>,
) -> Result<AltMleFit, String> {
    if stress.is_empty() || stress.len() != times.len() {
        return Err("stress and times must be non-empty and the same length".into());
    }
    if times.iter().any(|t| !t.is_finite() || *t <= 0.0) {
        return Err("all times must be positive and finite".into());
    }
    let failed_vec: Vec<bool> = match failed {
        Some(f) => {
            if f.len() != times.len() {
                return Err("failed must have the same length as times".into());
            }
            f.to_vec()
        }
        None => vec![true; times.len()],
    };
    let n_fail = failed_vec.iter().filter(|f| **f).count();
    if n_fail == 0 {
        return Err("at least one failure is required for an ALT MLE fit".into());
    }
    let (x, names) = design(model, stress)?;
    let y: Vec<f64> = times.iter().map(|t| t.ln()).collect();

    // Starting point from the least-squares fit (censoring times are
    // used as if they were failure times, which is only a start).
    let lsq = fit_alt(model, stress, times)?;
    let mut theta0: Vec<f64> = names.iter().map(|nm| lsq.coef[*nm]).collect();
    theta0.push(lsq.sigma.max(0.05).ln());

    let x_ref = &x;
    let y_ref = &y;
    let f_ref = &failed_vec;
    let mut objective = |theta: &[f64]| -> f64 { alt_neg_loglik(theta, x_ref, y_ref, f_ref) };

    // Round 1.
    let step1: Vec<f64> = theta0
        .iter()
        .enumerate()
        .map(|(i, v)| {
            if i == theta0.len() - 1 {
                0.2
            } else {
                (0.1 * v.abs()).max(0.1)
            }
        })
        .collect();
    let (theta1, _f1) = optim::nelder_mead(&mut objective, &theta0, &step1, 3000, 1e-7);

    // Round 2, starting from the round 1 result with smaller steps.
    let step2: Vec<f64> = theta1
        .iter()
        .enumerate()
        .map(|(i, v)| {
            if i == theta1.len() - 1 {
                0.1
            } else {
                (0.05 * v.abs()).max(0.05)
            }
        })
        .collect();
    let (theta, f_best) = optim::nelder_mead(&mut objective, &theta1, &step2, 3000, 1e-8);

    if !f_best.is_finite() || f_best >= 1e99 {
        return Err("ALT MLE optimisation failed".into());
    }
    let k = names.len();
    let sigma = theta[k].exp();
    let mut map = std::collections::BTreeMap::new();
    for (nm, c) in names.iter().zip(theta.iter()) {
        map.insert(nm.to_string(), *c);
    }

    // Covariance from the inverse numeric Hessian of the negative
    // log-likelihood at the optimum, over the full parameter vector.
    let hess = numeric_hessian(&mut objective, &theta);
    let cov = optim::matrix_inverse(&hess);

    Ok(AltMleFit {
        model: model.into(),
        coef: map,
        sigma,
        loglik: -f_best,
        cov,
        n: times.len(),
        n_fail,
    })
}
