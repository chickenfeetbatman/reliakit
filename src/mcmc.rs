//! Bayesian MCMC for life-data models (self-contained).
//!
//! Random-walk Metropolis-Hastings with adaptive proposal scaling
//! during warmup, multiple chains, split R-hat and effective sample
//! size diagnostics. Models: weibull2 and lognormal with
//! right-censored likelihoods, plus a hierarchical Weibull variant
//! (common shape, group scales with a normal hyperprior on log eta).
//!
//! Priors (weakly informative, on transformed parameters):
//! log beta ~ N(0, 1.5^2); log eta ~ N(log median time, 2^2);
//! lognormal mu ~ N(log median time, 2^2), log sigma ~ N(0, 1);
//! hierarchical mu_log_eta ~ N(log median time, 2^2), log tau ~ N(0, 1).

use crate::rng::Rng64;
use crate::specfunc;

fn loglik_weibull(theta: &[f64], times: &[f64], failed: &[bool]) -> f64 {
    let (lb, le) = (theta[0], theta[1]);
    let beta = lb.exp();
    let eta = le.exp();
    let mut ll = 0.0;
    for (i, &t) in times.iter().enumerate() {
        if failed[i] {
            ll += beta.ln() - beta * le + (beta - 1.0) * t.ln() - (t / eta).powf(beta);
        } else {
            ll += -(t / eta).powf(beta);
        }
    }
    ll
}

fn loglik_lognormal(theta: &[f64], times: &[f64], failed: &[bool]) -> f64 {
    let (mu, ls) = (theta[0], theta[1]);
    let sigma = ls.exp();
    let mut ll = 0.0;
    for (i, &t) in times.iter().enumerate() {
        let z = (t.ln() - mu) / sigma;
        if failed[i] {
            ll += -0.5 * z * z - (2.0 * std::f64::consts::PI).ln() / 2.0 - t.ln() - ls;
        } else {
            let s = specfunc::norm_sf(z);
            if s <= 0.0 {
                return -1e100;
            }
            ll += s.ln();
        }
    }
    ll
}

/// Run multiple chains of adaptive random-walk Metropolis.
/// logpost takes the transformed parameter vector.
pub fn run_chains_adaptive<F: Fn(&[f64]) -> f64>(
    logpost: &F,
    x0s: &[Vec<f64>],
    n_draws: usize,
    warmup: usize,
    seed: u64,
) -> (Vec<Vec<Vec<f64>>>, Vec<f64>) {
    let mut master = Rng64::new(seed);
    let mut all_chains = Vec::new();
    let mut acc_rates = Vec::new();
    for x0 in x0s {
        let mut rng = master.fork();
        let mut x = x0.clone();
        let mut lp = logpost(&x);
        let mut log_step = 0.0f64;
        let mut draws: Vec<Vec<f64>> = Vec::with_capacity(n_draws);
        let mut acc_warm = 0usize;
        let mut acc_post = 0usize;
        for it in 0..(warmup + n_draws) {
            let step = log_step.exp();
            let prop: Vec<f64> = x.iter().map(|xi| xi + rng.normal() * step).collect();
            let lp_prop = logpost(&prop);
            if rng.open01().ln() < lp_prop - lp {
                x = prop;
                lp = lp_prop;
                if it < warmup {
                    acc_warm += 1;
                } else {
                    acc_post += 1;
                }
            }
            if it < warmup && (it + 1) % 50 == 0 {
                let rate = acc_warm as f64 / 50.0;
                acc_warm = 0;
                log_step += 0.5 * (rate - 0.35);
                log_step = log_step.clamp(-4.0, 2.0);
            }
            if it >= warmup {
                draws.push(x.clone());
            }
        }
        all_chains.push(draws);
        acc_rates.push(acc_post as f64 / n_draws.max(1) as f64);
    }
    (all_chains, acc_rates)
}

/// Split R-hat per parameter. chains: (chains, draws, d).
pub fn split_rhat(chains: &[Vec<Vec<f64>>]) -> Vec<f64> {
    let c = chains.len();
    let n = chains[0].len();
    let d = chains[0][0].len();
    let half = n / 2;
    // split chains: 2C chains of length half
    let mut out = Vec::new();
    for j in 0..d {
        let mut split: Vec<Vec<f64>> = Vec::new();
        for chain in chains.iter().take(c) {
            split.push(chain[..half].iter().map(|row| row[j]).collect());
            split.push(chain[half..2 * half].iter().map(|row| row[j]).collect());
        }
        let m = split[0].len() as f64;
        let means: Vec<f64> = split.iter().map(|s| s.iter().sum::<f64>() / m).collect();
        let w: f64 = split
            .iter()
            .zip(&means)
            .map(|(s, mu)| s.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / (m - 1.0))
            .sum::<f64>()
            / split.len() as f64;
        let grand = means.iter().sum::<f64>() / means.len() as f64;
        let b = m
            * means
                .iter()
                .map(|mu| (mu - grand).powi(2))
                .sum::<f64>()
            / (means.len() - 1) as f64;
        let var_plus = (1.0 - 1.0 / m) * w + b / m;
        out.push(if w > 0.0 { (var_plus / w).sqrt() } else { f64::NAN });
    }
    out
}

/// Effective sample size per parameter (autocorrelation based, with
/// truncation once the average autocorrelation falls below 0.05).
pub fn ess(chains: &[Vec<Vec<f64>>]) -> Vec<f64> {
    let c = chains.len();
    let n = chains[0].len();
    let d = chains[0][0].len();
    let mut out = Vec::new();
    for j in 0..d {
        let mut rhos = Vec::new();
        for lag in 1..(200usize.min(n - 1)) {
            let mut num = 0.0;
            let mut den = 0.0;
            for chain in chains {
                let col: Vec<f64> = chain.iter().map(|row| row[j]).collect();
                let mu = col.iter().sum::<f64>() / col.len() as f64;
                for i in 0..(col.len() - lag) {
                    num += (col[i] - mu) * (col[i + lag] - mu);
                }
                den += col.iter().map(|x| (x - mu).powi(2)).sum::<f64>();
            }
            rhos.push(if den > 0.0 { num / den } else { 0.0 });
            if lag > 5 && *rhos.last().unwrap() < 0.05 {
                break;
            }
        }
        let tau = 1.0 + 2.0 * rhos.iter().map(|r| r.max(0.0)).sum::<f64>();
        out.push((c * n) as f64 / tau.max(1.0));
    }
    out
}

#[derive(Debug, Clone)]
pub struct McMcResult {
    pub model: String,
    /// (chains, draws, d) on the natural parameter scale.
    pub chains: Vec<Vec<Vec<f64>>>,
    pub param_names: Vec<String>,
    pub accept_rates: Vec<f64>,
    pub times: Vec<f64>,
    pub failed: Vec<bool>,
}

#[derive(Debug, Clone)]
pub struct ParamSummary {
    pub mean: f64,
    pub sd: f64,
    pub q05: f64,
    pub q50: f64,
    pub q95: f64,
}

#[derive(Debug, Clone)]
pub struct CredibleInterval {
    pub mean: f64,
    pub lower: f64,
    pub upper: f64,
    pub median: f64,
}

fn quantile_sorted(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let idx = q * (sorted.len() - 1) as f64;
    let lo = idx.floor() as usize;
    let hi = (lo + 1).min(sorted.len() - 1);
    let frac = idx - lo as f64;
    sorted[lo] * (1.0 - frac) + sorted[hi] * frac
}

impl McMcResult {
    /// Flattened samples: (n_samples, d).
    pub fn samples(&self) -> Vec<Vec<f64>> {
        self.chains.iter().flat_map(|c| c.iter().cloned()).collect()
    }

    pub fn summary(&self) -> Vec<(String, ParamSummary)> {
        let s = self.samples();
        let d = self.param_names.len();
        let mut out = Vec::new();
        for j in 0..d {
            let mut col: Vec<f64> = s.iter().map(|row| row[j]).collect();
            let mean = col.iter().sum::<f64>() / col.len() as f64;
            let sd = (col.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
                / col.len() as f64)
                .sqrt();
            col.sort_by(|a, b| a.partial_cmp(b).unwrap());
            out.push((
                self.param_names[j].clone(),
                ParamSummary {
                    mean,
                    sd,
                    q05: quantile_sorted(&col, 0.05),
                    q50: quantile_sorted(&col, 0.50),
                    q95: quantile_sorted(&col, 0.95),
                },
            ));
        }
        out
    }

    pub fn rhat(&self) -> Vec<f64> {
        // split-R-hat needs chains on the transformed scale to match the
        // Python diagnostics exactly in spirit; the Python code computes
        // diagnostics on natural-scale chains, and so does this port.
        split_rhat(&self.chains)
    }

    pub fn ess_values(&self) -> Vec<f64> {
        ess(&self.chains)
    }

    pub fn reliability_samples(&self, t: f64, group: usize) -> Vec<f64> {
        let s = self.samples();
        match self.model.as_str() {
            "weibull2" => s
                .iter()
                .map(|row| (-(t / row[1]).powf(row[0])).exp())
                .collect(),
            "weibull2_hier" => s
                .iter()
                .map(|row| (-(t / row[2 + group]).powf(row[0])).exp())
                .collect(),
            "lognormal" => s
                .iter()
                .map(|row| specfunc::norm_sf((t.ln() - row[0]) / row[1]))
                .collect(),
            _ => vec![],
        }
    }

    pub fn reliability_credible(&self, t: f64, level: f64, group: usize) -> CredibleInterval {
        let mut rs = self.reliability_samples(t, group);
        let mean = rs.iter().sum::<f64>() / rs.len() as f64;
        rs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let a = (1.0 - level) / 2.0;
        CredibleInterval {
            mean,
            lower: quantile_sorted(&rs, a),
            upper: quantile_sorted(&rs, 1.0 - a),
            median: quantile_sorted(&rs, 0.5),
        }
    }

    /// Posterior predictive check: fraction of replicated datasets whose
    /// number of failures by the max observed time is at least the
    /// observed count.
    pub fn posterior_predictive_pvalue(&self, seed: u64) -> f64 {
        let mut rng = Rng64::new(seed);
        let s = self.samples();
        let tmax = self.times.iter().cloned().fold(f64::MIN, f64::max);
        let obs_fail = self
            .times
            .iter()
            .zip(&self.failed)
            .filter(|(t, f)| **f && **t <= tmax)
            .count();
        let n = self.times.len();
        let take = 300.min(s.len());
        let mut hits = 0usize;
        for i in 0..take {
            let row = &s[(i * s.len() / take.max(1)).min(s.len() - 1)];
            let mut count = 0usize;
            for _ in 0..n {
                let sim = if self.model.starts_with("weibull2") {
                    let eta = if self.model == "weibull2" { row[1] } else { row[2] };
                    rng.weibull(row[0], eta)
                } else {
                    rng.lognormal(row[0], row[1])
                };
                if sim <= tmax {
                    count += 1;
                }
            }
            if count >= obs_fail {
                hits += 1;
            }
        }
        hits as f64 / take as f64
    }
}

/// Fit a single-population Bayesian weibull2 or lognormal model.
pub fn fit_bayes(
    dist: &str,
    times: &[f64],
    failed: Option<&[bool]>,
    n_draws: usize,
    warmup: usize,
    n_chains: usize,
    seed: u64,
) -> Result<McMcResult, String> {
    let failed_v: Vec<bool> = match failed {
        Some(f) => f.to_vec(),
        None => vec![true; times.len()],
    };
    let mut ft: Vec<f64> = times
        .iter()
        .zip(&failed_v)
        .filter(|(_, f)| **f)
        .map(|(t, _)| *t)
        .collect();
    if ft.is_empty() {
        ft = times.to_vec();
    }
    ft.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let med = ft[ft.len() / 2];
    let centre = med.max(1e-9).ln();
    let mut rng = Rng64::new(seed);
    let times_v = times.to_vec();
    let failed_c = failed_v.clone();
    let (chains_t, acc, names): (Vec<Vec<Vec<f64>>>, Vec<f64>, Vec<String>) = match dist {
        "weibull2" => {
            let logpost = |th: &[f64]| -> f64 {
                let (lb, le) = (th[0], th[1]);
                if !(-5.0 < lb && lb < 5.0 && centre - 8.0 < le && le < centre + 8.0) {
                    return f64::NEG_INFINITY;
                }
                loglik_weibull(th, &times_v, &failed_c)
                    - 0.5 * (lb / 1.5).powi(2)
                    - 0.5 * ((le - centre) / 2.0).powi(2)
            };
            let x0s: Vec<Vec<f64>> = (0..n_chains)
                .map(|_| vec![rng.normal() * 0.3, centre + rng.normal() * 0.3])
                .collect();
            let (ch, acc) = run_chains_adaptive(&logpost, &x0s, n_draws, warmup, seed);
            (ch, acc, vec!["beta".into(), "eta".into()])
        }
        "lognormal" => {
            let logpost = |th: &[f64]| -> f64 {
                let (mu, ls) = (th[0], th[1]);
                if !(centre - 8.0 < mu && mu < centre + 8.0 && -4.0 < ls && ls < 3.0) {
                    return f64::NEG_INFINITY;
                }
                loglik_lognormal(th, &times_v, &failed_c)
                    - 0.5 * ((mu - centre) / 2.0).powi(2)
                    - 0.5 * (ls / 1.0).powi(2)
            };
            let x0s: Vec<Vec<f64>> = (0..n_chains)
                .map(|_| vec![centre + rng.normal() * 0.3, -0.5 + rng.normal() * 0.3])
                .collect();
            let (ch, acc) = run_chains_adaptive(&logpost, &x0s, n_draws, warmup, seed);
            (ch, acc, vec!["mu".into(), "sigma".into()])
        }
        _ => return Err("MCMC supports weibull2 and lognormal".into()),
    };
    // to natural scale
    let chains: Vec<Vec<Vec<f64>>> = chains_t
        .iter()
        .map(|chain| {
            chain
                .iter()
                .map(|row| {
                    if dist == "weibull2" {
                        vec![row[0].exp(), row[1].exp()]
                    } else {
                        vec![row[0], row[1].exp()]
                    }
                })
                .collect()
        })
        .collect();
    Ok(McMcResult {
        model: dist.into(),
        chains,
        param_names: names,
        accept_rates: acc,
        times: times_v,
        failed: failed_v,
    })
}

/// Hierarchical Bayesian Weibull: common shape, group scales.
/// Natural-scale chains are [beta, tau, eta_1, ..., eta_G].
pub fn fit_bayes_hierarchical_weibull(
    groups: &[(Vec<f64>, Vec<bool>)],
    n_draws: usize,
    warmup: usize,
    n_chains: usize,
    seed: u64,
) -> Result<McMcResult, String> {
    let g = groups.len();
    let all_times: Vec<f64> = groups.iter().flat_map(|(t, _)| t.iter().cloned()).collect();
    let mut sorted = all_times.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let centre = sorted[sorted.len() / 2].ln();
    let data: Vec<(Vec<f64>, Vec<bool>)> = groups.to_vec();
    let logpost = |th: &[f64]| -> f64 {
        let (lb, mu, ltau) = (th[0], th[1], th[2]);
        let le = &th[3..];
        if !(-5.0 < lb && lb < 5.0 && centre - 8.0 < mu && mu < centre + 8.0 && -6.0 < ltau && ltau < 3.0) {
            return f64::NEG_INFINITY;
        }
        if le.iter().any(|v| *v < centre - 10.0 || *v > centre + 10.0) {
            return f64::NEG_INFINITY;
        }
        let tau = ltau.exp();
        let mut lp = -0.5 * (lb / 1.5).powi(2)
            - 0.5 * ((mu - centre) / 2.0).powi(2)
            - 0.5 * ltau.powi(2);
        for v in le {
            let z = (v - mu) / tau;
            lp += -0.5 * z * z - tau.ln() - 0.5 * (2.0 * std::f64::consts::PI).ln();
        }
        for (j, (t, f)) in data.iter().enumerate() {
            lp += loglik_weibull(&[lb, le[j]], t, f);
        }
        lp
    };
    let mut rng = Rng64::new(seed);
    let mut x0s = Vec::new();
    for _ in 0..n_chains {
        let mut x0 = vec![
            rng.normal() * 0.3,
            centre + rng.normal() * 0.2,
            -0.5 + rng.normal() * 0.3,
        ];
        for _ in 0..g {
            x0.push(centre + rng.normal() * 0.3);
        }
        x0s.push(x0);
    }
    let (chains_t, acc) = run_chains_adaptive(&logpost, &x0s, n_draws, warmup, seed);
    let chains: Vec<Vec<Vec<f64>>> = chains_t
        .iter()
        .map(|chain| {
            chain
                .iter()
                .map(|row| {
                    let mut nat = vec![row[0].exp(), row[2].exp()];
                    for j in 0..g {
                        nat.push(row[3 + j].exp());
                    }
                    nat
                })
                .collect()
        })
        .collect();
    let mut names = vec!["beta".to_string(), "tau".to_string()];
    for j in 0..g {
        names.push(format!("eta_{}", j + 1));
    }
    let all_failed: Vec<bool> = groups.iter().flat_map(|(_, f)| f.iter().cloned()).collect();
    Ok(McMcResult {
        model: "weibull2_hier".into(),
        chains,
        param_names: names,
        accept_rates: acc,
        times: all_times,
        failed: all_failed,
    })
}
