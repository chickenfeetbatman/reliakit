//! Ported assertions from the Python ReliaKit test suite
//! (tests/test_reliakit.py, 31 tests), keeping the same known values
//! and tolerances. Random data is generated with the crate RNG under
//! fixed seeds; where the Python tests used numpy draws, the Rust
//! draws differ, so stochastic recovery tests keep the same (loose)
//! tolerances as the Python originals.

use reliakit::alt::*;
use reliakit::bayes::*;
use reliakit::datasets;
use reliakit::degradation::*;
use reliakit::distributions::*;
use reliakit::growth::*;
use reliakit::mcmc::*;
use reliakit::rng::Rng64;
use reliakit::system::*;
use serde_json::json;
use std::collections::BTreeMap;

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

// ---- distributions ----

#[test]
fn test_weibull_reliability_at_eta() {
    let mut p = Params::new();
    p.insert("beta".into(), 2.0);
    p.insert("eta".into(), 1000.0);
    assert!(close(
        reliability("weibull2", &p, 1000.0),
        (-1.0f64).exp(),
        1e-10
    ));
    let mut p2 = Params::new();
    p2.insert("theta".into(), 1000.0);
    assert!(close(
        reliability("exponential", &p2, 1000.0),
        (-1.0f64).exp(),
        1e-12
    ));
}

#[test]
fn test_mle_recovers_weibull() {
    let mut rng = Rng64::new(5);
    let times: Vec<f64> = (0..300).map(|_| rng.weibull(1.5, 1000.0)).collect();
    let fit = fit_mle("weibull2", &times, None).unwrap();
    assert!(
        (fit.params["beta"] - 1.5).abs() < 0.25,
        "beta={}",
        fit.params["beta"]
    );
    assert!(
        (fit.params["eta"] - 1000.0).abs() < 120.0,
        "eta={}",
        fit.params["eta"]
    );
}

#[test]
fn test_censored_fit_and_bounds() {
    let (times, failed) = datasets::bearing_life(2026);
    let fit = fit_mle("weibull2", &times, Some(&failed)).unwrap();
    assert!(fit.b_life(0.10) > 0.0);
    let (lo, hi) = fit.reliability_bounds(1000.0, 0.10);
    assert!(0.0 <= lo && lo <= hi && hi <= 1.0, "lo={} hi={}", lo, hi);
}

#[test]
fn test_rank_regression() {
    let mut rng = Rng64::new(3);
    let times: Vec<f64> = (0..60).map(|_| rng.weibull(2.0, 1000.0)).collect();
    let fit = fit_rank_regression("weibull2", &times, None).unwrap();
    assert!(
        (fit.params["beta"] - 2.0).abs() < 0.6,
        "beta={}",
        fit.params["beta"]
    );
}

#[test]
fn test_median_ranks() {
    assert!(close(median_ranks(10, 1.0), 0.7 / 10.4, 1e-12));
}

#[test]
fn test_kaplan_meier() {
    let times = vec![100.0, 200.0, 200.0, 300.0];
    let failed = vec![true, true, false, true];
    let (ut, surv) = kaplan_meier(&times, Some(&failed));
    // like the Python implementation, the curve starts at (0, 1)
    assert_eq!(ut, vec![0.0, 100.0, 200.0, 300.0]);
    assert!(close(surv[0], 1.0, 1e-12));
    assert!(close(surv[1], 0.75, 1e-12));
    assert!(close(surv[2], 0.50, 1e-12));
    assert!(close(surv[3], 0.0, 1e-12));
}

// ---- bayes / limited data ----

#[test]
fn test_bias_correction_exact() {
    let mut rng = Rng64::new(11);
    let times: Vec<f64> = (0..10).map(|_| rng.weibull(1.5, 1000.0)).collect();
    let fit = fit_mle("weibull2", &times, None).unwrap();
    let corr = bias_corrected_weibull(&fit).unwrap();
    assert!(corr.params["beta"] < fit.params["beta"]);
    let ratio = corr.params["beta"] / fit.params["beta"];
    assert!(close(ratio, 1.0 / (1.0 + 1.37 / 10.0), 1e-9));
}

#[test]
fn test_shrink_normal() {
    let y = vec![7.5, 8.5, 9.5];
    let s = vec![0.4, 0.4, 0.4];
    let res = shrink_normal(&y, &s);
    for ((b, shrunk), yj) in res.b.iter().zip(res.shrunk.iter()).zip(y.iter()) {
        assert!(0.0 <= *b && *b <= 1.0);
        assert!((shrunk - res.mu).abs() <= (yj - res.mu).abs() + 1e-12);
    }
}

#[test]
fn test_conformal_zero_failures() {
    let calib = vec![5000.0; 20];
    let r = conformal_reliability_lower(&calib, 1000.0, 0.05);
    assert!(close(r, 0.05f64.powf(1.0 / 20.0), 1e-9), "R={}", r);
}

#[test]
fn test_conformal_mission_time_monotone() {
    let mut rng = Rng64::new(2);
    let times: Vec<f64> = (0..40).map(|_| rng.weibull(1.5, 1000.0)).collect();
    let fit = fit_mle("weibull2", &times, None).unwrap();
    let calib: Vec<f64> = (0..40).map(|_| rng.weibull(1.5, 1000.0)).collect();
    let l20 = conformal_mission_time(&fit, &calib, 0.20);
    let l10 = conformal_mission_time(&fit, &calib, 0.10);
    assert!(l20 >= l10 && l10 >= 0.0);
}

#[test]
fn test_hierarchical_eb() {
    let groups = datasets::small_sample_groups(2026);
    let res = hierarchical_weibull(&groups).unwrap();
    assert_eq!(res.fits.len(), 4);
    assert!(res.tau2 >= 0.0);
    assert!(res.fits.iter().all(|f| f.params["eta"] > 0.0));
}

// ---- mcmc ----

#[test]
fn test_mcmc_weibull_recovery() {
    let mut rng = Rng64::new(21);
    let times: Vec<f64> = (0..60).map(|_| rng.weibull(1.5, 1000.0)).collect();
    let res = fit_bayes("weibull2", &times, None, 900, 900, 4, 99).unwrap();
    let summ = res.summary();
    let beta_mean = summ[0].1.mean;
    let eta_mean = summ[1].1.mean;
    assert!((beta_mean - 1.5).abs() < 0.45, "beta mean {}", beta_mean);
    assert!((eta_mean - 1000.0).abs() < 250.0, "eta mean {}", eta_mean);
    assert!(res.rhat().iter().all(|r| *r < 1.2), "rhat {:?}", res.rhat());
    assert!(
        res.ess_values().iter().all(|e| *e > 50.0),
        "ess {:?}",
        res.ess_values()
    );
    let ci = res.reliability_credible(500.0, 0.9, 0);
    assert!(ci.lower <= ci.mean && ci.mean <= ci.upper);
}

#[test]
fn test_mcmc_hierarchical_shape() {
    let groups = datasets::small_sample_groups(2026);
    let res = fit_bayes_hierarchical_weibull(&groups, 600, 600, 2, 5).unwrap();
    assert_eq!(res.samples()[0].len(), 6);
}

// ---- degradation ----

#[test]
fn test_degradation_crossing() {
    let tc = pseudo_failure_time(100.0, -0.01, 70.0, "linear", false);
    assert!(close(tc, 3000.0, 1e-9));
}

#[test]
fn test_degradation_end_to_end() {
    let units = datasets::degradation_units(2026);
    let res = degradation_analysis(&units, 70.0, "linear", "weibull2", false).unwrap();
    assert!(res.pseudo_times.iter().all(|t| *t > 2000.0));
    assert!(res.fit.unwrap().params["eta"] > 2000.0);
}

// ---- alt ----

#[test]
fn test_arrhenius_af() {
    let af = acceleration_factor_arrhenius(0.7, 25.0, 125.0);
    assert!((af - 936.5).abs() / 936.5 < 0.02, "AF={}", af);
}

#[test]
fn test_ipl_af() {
    assert!(close(
        acceleration_factor_ipl(2.0, 100.0, 200.0),
        4.0,
        1e-12
    ));
}

#[test]
fn test_alt_fit_recovers_ea() {
    let (stress, times) = datasets::alt_dataset(2026);
    let fit = fit_alt("arrhenius", &stress, &times).unwrap();
    let ea = fit.activation_energy_ev().unwrap();
    assert!((ea - 0.7).abs() < 0.25, "Ea={}", ea);
    let r_low = fit.reliability(1000.0, &[55.0]);
    let r_high = fit.reliability(1000.0, &[125.0]);
    assert!(r_low > r_high);
}

// ---- growth ----

#[test]
fn test_crow_amsaa_hand_calc() {
    let times = vec![100.0, 200.0, 300.0];
    let fit = fit_crow_amsaa(&times, Some(300.0));
    let expected_beta = 3.0 / (3.0f64.ln() + 1.5f64.ln() + 1.0f64.ln());
    assert!(close(fit.beta, expected_beta, 1e-9), "beta {}", fit.beta);
    assert!(close(fit.lam, 3.0 / 300.0f64.powf(expected_beta), 1e-9));
}

#[test]
fn test_duane() {
    let times = datasets::growth_dataset(2026);
    let fit = fit_duane(&times).unwrap();
    assert!(fit.alpha > 0.0 && fit.alpha < 1.5);
    assert!(fit.instantaneous_mtbf_at(5000.0) > 0.0);
}

// ---- system ----

#[test]
fn test_series_parallel() {
    assert!(close(series_reliability(&[0.9, 0.8]), 0.72, 1e-12));
    assert!(close(parallel_reliability(&[0.9, 0.8]), 0.98, 1e-12));
}

#[test]
fn test_kofn() {
    assert!(close(kofn_reliability(&[0.9, 0.9, 0.9], 2), 0.972, 1e-12));
}

#[test]
fn test_nested_rbd() {
    let spec = json!({"type": "series", "children": [
        {"type": "block", "R": 0.99, "name": "a"},
        {"type": "parallel", "children": [
            {"type": "block", "R": 0.9, "name": "b"},
            {"type": "block", "R": 0.9, "name": "c"}]}]});
    let r = evaluate_rbd(&spec, None).unwrap();
    assert!(close(r, 0.9801, 1e-12));
}

#[test]
fn test_cold_standby_exponential() {
    let mut p = Params::new();
    p.insert("theta".into(), 1000.0);
    let r = standby_reliability(1000.0, "exponential", &p, 2, 0.0, 1.0);
    assert!(close(r, (-1.0f64).exp() * 2.0, 1e-9), "R={}", r);
}

#[test]
fn test_standby_ordering() {
    let mut p = Params::new();
    p.insert("beta".into(), 1.5);
    p.insert("eta".into(), 1000.0);
    let cold = standby_reliability(500.0, "weibull2", &p, 2, 0.0, 1.0);
    let warm = standby_reliability(500.0, "weibull2", &p, 2, 0.3, 1.0);
    let hot = standby_reliability(500.0, "weibull2", &p, 2, 1.0, 1.0);
    let single = reliability("weibull2", &p, 500.0);
    assert!(
        cold >= warm && warm >= hot && hot > single,
        "cold={} warm={} hot={} single={}",
        cold,
        warm,
        hot,
        single
    );
}

#[test]
fn test_block_availability() {
    assert!(close(
        single_block_availability(1000.0, 100.0),
        1000.0 / 1100.0,
        1e-12
    ));
}

#[test]
fn test_ctmc_single() {
    let (q, down) = standby_ctmc(0.001, 0.1, 1, 0.0, 1);
    let pi = ctmc_steady_state(&q);
    let a: f64 = pi
        .iter()
        .enumerate()
        .filter(|(i, _)| !down.contains(i))
        .map(|(_, p)| p)
        .sum();
    assert!(close(a, 0.1 / 0.101, 1e-9), "A={}", a);
}

#[test]
fn test_mc_availability_single_block() {
    let spec = json!({"type": "block", "name": "pump", "mtbf": 1000.0, "mttr": 10.0});
    let res = simulate_availability(&spec, 20000.0, 60, 3, 50);
    assert!(
        (res.mean_availability - 1000.0 / 1010.0).abs() < 0.02,
        "A={}",
        res.mean_availability
    );
    assert!(res.expected_downtime > 0.0);
}

#[test]
fn test_mc_availability_standby_vs_ctmc() {
    let spec = json!({"type": "standby_group", "n_units": 2, "dist": "exponential",
                      "params": {"theta": 1000.0}, "mtbf": 1000.0, "mttr": 10.0,
                      "dormancy": 0.0});
    let res = simulate_availability(&spec, 20000.0, 60, 8, 50);
    let (q, down) = standby_ctmc(0.001, 0.1, 2, 0.0, 1);
    let pi = ctmc_steady_state(&q);
    let a_ctmc: f64 = pi
        .iter()
        .enumerate()
        .filter(|(i, _)| !down.contains(i))
        .map(|(_, p)| p)
        .sum();
    assert!(
        (res.mean_availability - a_ctmc).abs() < 0.02,
        "MC={} CTMC={}",
        res.mean_availability,
        a_ctmc
    );
}

#[test]
fn test_posterior_system_reliability() {
    let spec = json!({"type": "series", "children": [
        {"type": "block", "name": "a"}, {"type": "block", "name": "b"}]});
    let mut draws = BTreeMap::new();
    draws.insert("a".to_string(), vec![0.9; 50]);
    draws.insert("b".to_string(), vec![0.9; 50]);
    let out = posterior_system_reliability(&spec, &draws, 100.0).unwrap();
    assert!(out.iter().all(|r| close(*r, 0.81, 1e-12)));
}
