//! Example 2: limited data / small-sample methods.
//!
//! Reads the multi-lot example data (examples/example2_small_sample_lots.csv,
//! synthetic, 4 lots of 8 units, censored at 4500 h) and runs:
//! per-lot Weibull MLE, shape bias correction, hierarchical
//! empirical-Bayes shrinkage, a split-conformal certified mission
//! time and lower reliability bound (fit on lots 1-3, calibrated on
//! lot 4), and a short Bayesian MCMC run with R-hat and ESS.
//!
//! Run from the repository root:
//!   cargo run --release --example example2_limited_data

use reliakit::bayes::{
    bias_corrected_weibull, conformal_mission_time, conformal_reliability_lower,
    hierarchical_weibull,
};
use reliakit::distributions::fit_mle;
use reliakit::mcmc::fit_bayes;
use std::collections::BTreeMap;

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "examples/example2_small_sample_lots.csv".to_string());
    let text = std::fs::read_to_string(&path).expect("read example2 csv");
    let mut lots: BTreeMap<u32, (Vec<f64>, Vec<bool>)> = BTreeMap::new();
    for (i, line) in text.lines().enumerate() {
        if i == 0 || line.trim().is_empty() {
            continue; // header: lot,time,failed
        }
        let cells: Vec<&str> = line.split(',').collect();
        let lot: u32 = cells[0].trim().parse().unwrap();
        let t: f64 = cells[1].trim().parse().unwrap();
        let f: bool = cells[2].trim() == "1";
        let e = lots.entry(lot).or_default();
        e.0.push(t);
        e.1.push(f);
    }
    let groups: Vec<(Vec<f64>, Vec<bool>)> = lots.values().cloned().collect();
    println!("Example 2: limited data, {} lots", groups.len());
    for (j, (t, f)) in groups.iter().enumerate() {
        let fit = fit_mle("weibull2", t, Some(f)).unwrap();
        let nf = f.iter().filter(|x| **x).count();
        println!(
            "  lot {}: n = {}, failures = {}, MLE beta = {:.3}, eta = {:.1} h",
            j + 1,
            t.len(),
            nf,
            fit.params["beta"],
            fit.params["eta"]
        );
    }

    // Pooled fit and bias correction
    let all_t: Vec<f64> = groups.iter().flat_map(|(t, _)| t.iter().cloned()).collect();
    let all_f: Vec<bool> = groups.iter().flat_map(|(_, f)| f.iter().cloned()).collect();
    let pooled = fit_mle("weibull2", &all_t, Some(&all_f)).unwrap();
    let corrected = bias_corrected_weibull(&pooled).unwrap();
    println!(
        "Pooled MLE: beta = {:.3}, eta = {:.1} h",
        pooled.params["beta"], pooled.params["eta"]
    );
    println!(
        "Bias-corrected: beta = {:.3} (factor {:.3}), eta = {:.1} h",
        corrected.params["beta"],
        corrected.params["beta"] / pooled.params["beta"],
        corrected.params["eta"]
    );

    // Hierarchical empirical-Bayes shrinkage
    let hier = hierarchical_weibull(&groups).unwrap();
    println!(
        "Hierarchical EB: pooled beta = {:.3}, tau2 (log eta) = {:.4}",
        hier.pooled_beta, hier.tau2
    );
    for (j, f) in hier.fits.iter().enumerate() {
        println!(
            "  lot {}: shrunk eta = {:.1} h (shrinkage B = {:.2})",
            j + 1,
            f.params["eta"],
            hier.b[j]
        );
    }

    // Split-conformal: fit on lots 1-2, calibrate on lots 3-4 (16
    // calibration units; with fewer than 9 calibration points a 90
    // percent certified mission time is 0 by construction)
    let train: Vec<(Vec<f64>, Vec<bool>)> = groups[..2].to_vec();
    let tr_t: Vec<f64> = train.iter().flat_map(|(t, _)| t.iter().cloned()).collect();
    let tr_f: Vec<bool> = train.iter().flat_map(|(_, f)| f.iter().cloned()).collect();
    let train_fit = fit_mle("weibull2", &tr_t, Some(&tr_f)).unwrap();
    let calib: Vec<f64> = groups[2..]
        .iter()
        .flat_map(|(t, _)| t.iter().cloned())
        .collect();
    let l = conformal_mission_time(&train_fit, &calib, 0.10);
    let rl = conformal_reliability_lower(&calib, 1000.0, 0.10);
    println!(
        "Split-conformal (fit lots 1-2, calibrate lots 3-4, alpha = 0.10): certified mission time = {:.1} h",
        l
    );
    println!(
        "Conformal lower bound on R(1000 h) = {:.3} (model estimate {:.3})",
        rl,
        train_fit.reliability(1000.0)
    );

    // Short MCMC on the pooled data
    let post = fit_bayes("weibull2", &all_t, Some(&all_f), 1000, 1000, 4, 42).unwrap();
    let summ = post.summary();
    let rhat = post.rhat();
    let essv = post.ess_values();
    let ci = post.reliability_credible(1000.0, 0.90, 0);
    println!(
        "MCMC (4 chains x 1000 draws): beta mean = {:.3}, eta mean = {:.1} h",
        summ[0].1.mean, summ[1].1.mean
    );
    println!(
        "  max R-hat = {:.3}, min ESS = {:.0}",
        rhat.iter().cloned().fold(f64::MIN, f64::max),
        essv.iter().cloned().fold(f64::MAX, f64::min)
    );
    println!(
        "  R(1000 h) posterior mean = {:.3}, 90% credible interval [{:.3}, {:.3}]",
        ci.mean, ci.lower, ci.upper
    );
}
