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
                let t = if stress[0] < 400.0 {
                    stress[0] + 273.15
                } else {
                    stress[0]
                };
                self.coef["a"] + self.coef["b"] / t
            }
            "ipl" => self.coef["a"] - self.coef["n"] * stress[0].ln(),
            "eyring" => {
                let t = if stress[0] < 400.0 {
                    stress[0] + 273.15
                } else {
                    stress[0]
                };
                self.coef["a"] + self.coef["b"] / t - self.coef["n"] * stress[1].ln()
            }
            "eyring_interaction" => {
                // Convention: mean log life = a + b / T_k - n * ln V
                //   + ab * (1 / T_k) * (-ln V)
                // so the voltage term uses -ln V and the interaction
                // column is (1 / T_k) * (-ln V).
                let t = if stress[0] < 400.0 {
                    stress[0] + 273.15
                } else {
                    stress[0]
                };
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
            let t = if stress[0] < 400.0 {
                stress[0] + 273.15
            } else {
                stress[0]
            };
            get("a") + get("b") / t
        }
        "ipl" => get("a") - get("n") * stress[0].ln(),
        "eyring" => {
            let t = if stress[0] < 400.0 {
                stress[0] + 273.15
            } else {
                stress[0]
            };
            get("a") + get("b") / t - get("n") * stress[1].ln()
        }
        "eyring_interaction" => {
            let t = if stress[0] < 400.0 {
                stress[0] + 273.15
            } else {
                stress[0]
            };
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
            if sf <= 0.0 || !sf.is_finite() {
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

// ---------------------------------------------------------------------
// Generalized Eyring / loglinear MLE with exactly two stresses.
//
// The stress layout differs from the older fits in this file: each
// row is [temperature in Kelvin, non-thermal stress S]. Temperature
// is used as Kelvin exactly as given, with no Celsius conversion.
// The life-stress relation is
//
//   log(eta) = b0 + b1 * (1 / T) + b2 * ln(S) + b3 * (ln(S) / T)
//
// so b1 is the Arrhenius coefficient, b2 is the non-thermal stress
// coefficient, and b3 is their interaction.
//
// For the Weibull life distribution ("weibull2"), eta is the
// Weibull scale at that stress and a single shape beta is shared
// by all units. For the lognormal ("lognormal"), log life is
// Normal(log(eta), sigma) with a single shared sigma.
//
// Optimisation uses scaled regressors x1 = 1000 / T and
// x3 = ln(S) * 1000 / T internally, so the optimised coefficients
// c1 = b1 / 1000 and c3 = b3 / 1000 are of moderate size. Reported
// coefficients are always the unscaled b values above.
// ---------------------------------------------------------------------

/// Fitted generalized Eyring / loglinear ALT model.
#[derive(Debug, Clone)]
pub struct AltLoglinearFit {
    /// Life distribution name: "weibull2" or "lognormal".
    pub dist: String,
    /// Coefficients with keys "b0", "b1", "b2", "b3", in the
    /// log(eta) equation stated at the head of this section.
    pub coef: std::collections::BTreeMap<String, f64>,
    /// Weibull shape beta. Only set for the Weibull fit; NaN for
    /// the lognormal fit.
    pub beta: f64,
    /// Lognormal sigma. Only the primary scale for the lognormal
    /// fit; for the Weibull fit this holds 1 / beta, the standard
    /// deviation of log life under a Weibull model.
    pub sigma: f64,
    pub loglik: f64,
    pub n: usize,
    pub n_fail: usize,
}

impl AltLoglinearFit {
    /// log(eta) at temperature `temp_k` (Kelvin) and non-thermal
    /// stress `stress`.
    pub fn log_eta(&self, temp_k: f64, stress: f64) -> f64 {
        let b = |k: &str| self.coef.get(k).copied().unwrap_or(f64::NAN);
        let ln_s = stress.ln();
        b("b0") + b("b1") / temp_k + b("b2") * ln_s + b("b3") * ln_s / temp_k
    }

    /// eta at temperature `temp_k` (Kelvin) and stress `stress`.
    pub fn eta(&self, temp_k: f64, stress: f64) -> f64 {
        self.log_eta(temp_k, stress).exp()
    }

    /// Reliability at age `t`, temperature `temp_k` (Kelvin) and
    /// non-thermal stress `stress`.
    pub fn use_reliability(&self, temp_k: f64, stress: f64, t: f64) -> f64 {
        alt_loglinear_use_reliability(self, temp_k, stress, t)
    }

    /// Reliability in the call style of the other fits in this
    /// file: `stress` is [temperature in Kelvin, non-thermal stress].
    pub fn reliability(&self, t: f64, stress: &[f64]) -> f64 {
        self.use_reliability(stress[0], stress[1], t)
    }

    pub fn median_life(&self, temp_k: f64, stress: f64) -> f64 {
        match self.dist.as_str() {
            "weibull2" => self.eta(temp_k, stress) * (std::f64::consts::LN_2).powf(1.0 / self.beta),
            _ => self.eta(temp_k, stress),
        }
    }
}

/// Use-condition reliability from a loglinear fit: the reliability
/// at age `t` under temperature `temp_k` (Kelvin) and non-thermal
/// stress `stress`.
pub fn alt_loglinear_use_reliability(
    fit: &AltLoglinearFit,
    temp_k: f64,
    stress: f64,
    t: f64,
) -> f64 {
    if t <= 0.0 {
        return 1.0;
    }
    let eta = fit.eta(temp_k, stress);
    if !eta.is_finite() || eta <= 0.0 {
        return f64::NAN;
    }
    match fit.dist.as_str() {
        "weibull2" => (-(t / eta).powf(fit.beta)).exp(),
        "lognormal" => specfunc::norm_sf((t.ln() - eta.ln()) / fit.sigma),
        _ => f64::NAN,
    }
}

/// Scaled design row [1, 1000 / T, ln S, ln S * 1000 / T] and the
/// unscaled row terms for one [T Kelvin, S] stress pair.
fn loglinear_scaled_row(temp_k: f64, stress: f64) -> [f64; 4] {
    let ln_s = stress.ln();
    let x1 = 1000.0 / temp_k;
    [1.0, x1, ln_s, ln_s * x1]
}

/// Negative log-likelihood for the loglinear model.
/// theta is [b0, c1, b2, c3, log scale] in scaled coefficients,
/// where the last entry is log beta for Weibull (scale holds beta
/// directly, see the caller) or log sigma for lognormal. The
/// caller fixes the convention by the closure it builds; this
/// helper works on prebuilt scaled design rows.
fn loglinear_neg_loglik(
    dist: &str,
    theta: &[f64],
    rows: &[[f64; 4]],
    times: &[f64],
    failed: &[bool],
) -> f64 {
    if theta.len() != 5 {
        return 1e100;
    }
    let coef = &theta[..4];
    let mut nll = 0.0;
    if dist == "weibull2" {
        // theta[4] is log beta.
        if !theta[4].is_finite() || theta[4] < -5.0 || theta[4] > 5.0 {
            return 1e100;
        }
        let beta = theta[4].exp();
        for i in 0..times.len() {
            let log_eta: f64 = rows[i].iter().zip(coef).map(|(a, b)| a * b).sum();
            if !log_eta.is_finite() || log_eta.abs() > 700.0 {
                return 1e100;
            }
            let log_t = times[i].ln();
            let z = (log_t - log_eta) * beta;
            // (t / eta)^beta, computed on the log scale.
            if z > 700.0 {
                if failed[i] {
                    return 1e100;
                }
                // Survival is effectively zero: log R is about
                // -exp(z), a very large negative log-likelihood
                // contribution.
                return 1e100;
            }
            let pow_term = z.exp();
            if failed[i] {
                // log pdf = log beta + (beta - 1) log t
                //           - beta log eta - (t/eta)^beta
                let log_pdf = theta[4] + (beta - 1.0) * log_t - beta * log_eta - pow_term;
                nll -= log_pdf;
            } else {
                // log R = -(t/eta)^beta
                nll += pow_term;
            }
            if !nll.is_finite() {
                return 1e100;
            }
        }
        nll
    } else {
        // Lognormal: theta[4] is log sigma.
        if !theta[4].is_finite() || theta[4] < -10.0 || theta[4] > 5.0 {
            return 1e100;
        }
        let log_sigma = theta[4];
        let sigma = log_sigma.exp();
        for i in 0..times.len() {
            let mu: f64 = rows[i].iter().zip(coef).map(|(a, b)| a * b).sum();
            if !mu.is_finite() {
                return 1e100;
            }
            let z = (times[i].ln() - mu) / sigma;
            if !z.is_finite() {
                return 1e100;
            }
            if failed[i] {
                let log_pdf = -log_sigma - 0.5 * (2.0 * std::f64::consts::PI).ln() - 0.5 * z * z;
                nll -= log_pdf;
            } else {
                let sf = specfunc::norm_sf(z);
                if sf <= 0.0 || !sf.is_finite() {
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
}

/// Censored maximum-likelihood fit of the generalized Eyring /
/// loglinear model with exactly two stress columns.
///
/// `stress` has one row per unit: [temperature in Kelvin,
/// non-thermal stress S]. `times` holds failure times for failed
/// units and censoring times for suspended units; `failed` marks
/// failures, and None means every unit failed. `dist` is
/// "weibull2" (required model) or "lognormal".
pub fn fit_alt_mle_loglinear(
    times: &[f64],
    failed: Option<&[bool]>,
    stress: &[Vec<f64>],
    dist: &str,
) -> Result<AltLoglinearFit, String> {
    if dist != "weibull2" && dist != "lognormal" {
        return Err(format!(
            "loglinear ALT MLE supports weibull2 and lognormal, not {}",
            dist
        ));
    }
    if times.is_empty() || stress.len() != times.len() {
        return Err("stress and times must be non-empty and the same length".into());
    }
    if times.iter().any(|t| !t.is_finite() || *t <= 0.0) {
        return Err("all times must be positive and finite".into());
    }
    for row in stress {
        if row.len() != 2 {
            return Err("each stress row must have exactly two columns [T Kelvin, S]".into());
        }
        if !row[0].is_finite() || row[0] <= 0.0 {
            return Err("temperature column must be positive Kelvin".into());
        }
        if !row[1].is_finite() || row[1] <= 0.0 {
            return Err("non-thermal stress column must be positive".into());
        }
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
        return Err("at least one failure is required for a loglinear ALT MLE fit".into());
    }

    let rows: Vec<[f64; 4]> = stress
        .iter()
        .map(|s| loglinear_scaled_row(s[0], s[1]))
        .collect();

    // Starting coefficients from least squares of log time on the
    // scaled design. For both life distributions the slopes of
    // log time on the stress terms estimate the life-stress slopes
    // up to a constant absorbed by the intercept, which is a good
    // start. The residual scatter starts the scale parameter.
    let design: Vec<Vec<f64>> = rows.iter().map(|r| r.to_vec()).collect();
    let y: Vec<f64> = times.iter().map(|t| t.ln()).collect();
    let lsq_coef = optim::lstsq(&design, &y).ok_or("loglinear ALT start lstsq failed")?;
    let mut ss = 0.0;
    for (i, row) in design.iter().enumerate() {
        let pred: f64 = row.iter().zip(&lsq_coef).map(|(a, b)| a * b).sum();
        ss += (y[i] - pred).powi(2);
    }
    let dof = (y.len() as i64 - 4).max(1) as f64;
    let resid_sd = (ss / dof).sqrt().max(0.05);

    let scale0 = if dist == "weibull2" {
        // Log life under a Weibull has sd pi / (sqrt(6) * beta).
        let beta0 = (std::f64::consts::PI / (6.0f64.sqrt() * resid_sd)).clamp(0.1, 50.0);
        beta0.ln()
    } else {
        resid_sd.ln()
    };
    let mut theta0 = lsq_coef.clone();
    theta0.push(scale0);

    let rows_ref = &rows;
    let times_ref = &times;
    let failed_ref = &failed_vec;
    let dist_ref = dist;
    let mut objective = |theta: &[f64]| -> f64 {
        loglinear_neg_loglik(dist_ref, theta, rows_ref, times_ref, failed_ref)
    };

    let step1: Vec<f64> = vec![0.3, 0.3, 0.2, 0.2, 0.2];
    let (theta1, _f1) = optim::nelder_mead(&mut objective, &theta0, &step1, 4000, 1e-8);
    let step2: Vec<f64> = vec![0.05, 0.05, 0.05, 0.05, 0.05];
    let (theta, f_best) = optim::nelder_mead(&mut objective, &theta1, &step2, 4000, 1e-10);
    if !f_best.is_finite() || f_best >= 1e99 {
        return Err("loglinear ALT MLE optimisation failed".into());
    }

    // Unscale: b1 = c1 * 1000 and b3 = c3 * 1000.
    let mut map = std::collections::BTreeMap::new();
    map.insert("b0".to_string(), theta[0]);
    map.insert("b1".to_string(), theta[1] * 1000.0);
    map.insert("b2".to_string(), theta[2]);
    map.insert("b3".to_string(), theta[3] * 1000.0);
    let (beta, sigma) = if dist == "weibull2" {
        let b = theta[4].exp();
        (b, 1.0 / b)
    } else {
        (f64::NAN, theta[4].exp())
    };
    Ok(AltLoglinearFit {
        dist: dist.to_string(),
        coef: map,
        beta,
        sigma,
        loglik: -f_best,
        n: times.len(),
        n_fail,
    })
}

// ---------------------------------------------------------------------
// Step-stress testing: Nelson cumulative damage model.
//
// The life model at a fixed stress S is Weibull with a common
// shape beta and an inverse power law scale
//
//   eta(S) = K * S^(-n)
//
// so K is the scale at unit stress (S = 1), and larger stress
// gives a smaller eta when n is positive.
//
// A step profile is a list of (stress, duration) segments run in
// order. Nelson cumulative damage carries damage across a stress
// change by matching the CDF, not the clock: at the end of
// segment i, a unit has virtual age a_i (its elapsed time into
// segment i plus the equivalent starting age carried into that
// segment) and cumulative hazard term (a_i / eta_i)^beta. The
// equivalent starting age in segment i + 1 is the age tau with
// (tau / eta_{i+1})^beta equal to that same term, namely
//
//   tau = eta_{i+1} * a_i / eta_i.
//
// Inside a segment, reliability at elapsed time u into the
// segment is exp(-((tau + u) / eta_i)^beta). After the last listed
// segment ends, the stress is held at the last segment level, so
// a one-segment profile is exactly a plain Weibull reliability at
// that stress for every t.
// ---------------------------------------------------------------------

/// Reliability at age `t` under a step-stress profile.
///
/// `steps` are (stress, duration) segments in run order. `beta` is
/// the common Weibull shape, and the inverse power law scale is
/// eta(S) = `k_ipl` * S^(-`n_ipl`).
pub fn step_stress_reliability(
    t: f64,
    steps: &[(f64, f64)],
    beta: f64,
    k_ipl: f64,
    n_ipl: f64,
) -> f64 {
    if steps.is_empty() || t < 0.0 || beta <= 0.0 || k_ipl <= 0.0 {
        return f64::NAN;
    }
    if steps
        .iter()
        .any(|(s, d)| !s.is_finite() || *s <= 0.0 || !d.is_finite() || *d < 0.0)
    {
        return f64::NAN;
    }
    let eta = |s: f64| k_ipl * s.powf(-n_ipl);
    // Equivalent starting age in the current segment.
    let mut tau = 0.0f64;
    let mut elapsed_before = 0.0f64;
    let last = steps.len() - 1;
    for (i, (stress, duration)) in steps.iter().enumerate() {
        let eta_i = eta(*stress);
        let in_segment = t - elapsed_before;
        if in_segment <= *duration || i == last {
            // Age t falls in this segment, or past the end of the
            // profile where the last stress is held.
            let virtual_age = tau + in_segment.max(0.0);
            return (-(virtual_age / eta_i).powf(beta)).exp();
        }
        // The whole segment runs. Carry its end damage into the
        // next segment.
        let end_virtual = tau + duration;
        let eta_next = eta(steps[i + 1].0);
        tau = eta_next * end_virtual / eta_i;
        elapsed_before += duration;
    }
    f64::NAN
}

/// CDF at age `t` under the same step-stress model as
/// `step_stress_reliability`.
pub fn step_stress_cdf(t: f64, steps: &[(f64, f64)], beta: f64, k_ipl: f64, n_ipl: f64) -> f64 {
    1.0 - step_stress_reliability(t, steps, beta, k_ipl, n_ipl)
}

/// Step-stress reliability stated with a reference scale instead
/// of the IPL constant: eta(S) = `eta_ref` * (S / `s_ref`)^(-n).
/// This is the same model with K = eta_ref * s_ref^n.
pub fn step_stress_reliability_eta_ref(
    t: f64,
    steps: &[(f64, f64)],
    beta: f64,
    eta_ref: f64,
    s_ref: f64,
    n_ipl: f64,
) -> f64 {
    let k_ipl = eta_ref * s_ref.powf(n_ipl);
    step_stress_reliability(t, steps, beta, k_ipl, n_ipl)
}
