//! Validation tests for v0.3.2. Fixture record: validation/fixtures.md.
//!
//! Every expected value comes from the fixture record: a hand
//! computation, a closed form, the known generating parameters of a
//! constructed sample, or a second computation path written in this
//! file. No expectation is copied from the function under test.

use reliakit::alt::acceleration_factor_arrhenius;
use reliakit::distributions::{
    cdf, fit_mle, loglik, params_of, pdf, quantile, reliability, Params,
};
use reliakit::standby::standby_closed_form_exp;
use reliakit::system::{
    cold_standby_exponential, ctmc_steady_state, evaluate_rbd, kofn_reliability,
    simulate_availability, single_block_availability, standby_ctmc,
};
use serde_json::json;

// ---------- helpers ----------

/// Composite Simpson integral of the pdf from a to b, written for
/// this test as the independent quadrature path.
fn simpson_pdf(dist: &str, p: &Params, a: f64, b: f64, n: usize) -> f64 {
    let n = if n.is_multiple_of(2) { n } else { n + 1 };
    let h = (b - a) / n as f64;
    let mut sum = pdf(dist, p, a) + pdf(dist, p, b);
    for i in 1..n {
        let x = a + i as f64 * h;
        sum += if i % 2 == 0 { 2.0 } else { 4.0 } * pdf(dist, p, x);
    }
    sum * h / 3.0
}

/// Deterministic quantile sample at Benard plotting positions from
/// a known generating distribution.
fn quantile_sample(dist: &str, p: &Params, n: usize) -> Vec<f64> {
    (1..=n)
        .map(|i| {
            let prob = (i as f64 - 0.3) / (n as f64 + 0.4);
            quantile(dist, p, prob)
        })
        .collect()
}

fn rel_close(actual: f64, expected: f64, rel_tol: f64) -> bool {
    (actual - expected).abs() <= rel_tol * expected.abs().max(1e-300)
}

// ---------- Family 1: distribution consistency ----------

fn check_distribution(dist: &str, p: &Params, support_start: f64) {
    for prob in [0.25f64, 0.50, 0.75] {
        let t = quantile(dist, p, prob);
        assert!(t > support_start, "{dist}: quantile {t} not above support");
        // Quantile inverts the cdf.
        let back = cdf(dist, p, t);
        assert!(
            (back - prob).abs() <= 1e-8,
            "{dist}: cdf(quantile({prob})) = {back}"
        );
        // The pdf integrates to the cdf (Simpson path in this test).
        let area = simpson_pdf(dist, p, support_start, t, 20000);
        let want = cdf(dist, p, t) - cdf(dist, p, support_start);
        assert!(
            (area - want).abs() <= 2e-3,
            "{dist}: integral to {t} is {area}, cdf gives {want}"
        );
        // Reliability is 1 - cdf.
        assert!(
            (reliability(dist, p, t) - (1.0 - cdf(dist, p, t))).abs() <= 1e-12,
            "{dist}: reliability identity failed at {t}"
        );
    }
}

#[test]
fn validation_distribution_consistency() {
    check_distribution(
        "weibull2",
        &params_of(&[("beta", 2.0), ("eta", 1000.0)]),
        0.0,
    );
    check_distribution("lognormal", &params_of(&[("mu", 7.0), ("sigma", 0.5)]), 0.0);
    check_distribution("gamma", &params_of(&[("alpha", 2.5), ("beta", 400.0)]), 0.0);
    check_distribution(
        "loglogistic",
        &params_of(&[("alpha", 1000.0), ("beta", 2.0)]),
        0.0,
    );
    check_distribution(
        "exponential2",
        &params_of(&[("theta", 500.0), ("gamma", 100.0)]),
        100.0,
    );
    check_distribution(
        "gengamma",
        &params_of(&[("a", 1000.0), ("d", 2.0), ("p", 1.5)]),
        0.0,
    );
}

#[test]
fn validation_support_respected() {
    let cases: Vec<(&str, Params, f64)> = vec![
        (
            "weibull2",
            params_of(&[("beta", 2.0), ("eta", 1000.0)]),
            -10.0,
        ),
        (
            "lognormal",
            params_of(&[("mu", 7.0), ("sigma", 0.5)]),
            -10.0,
        ),
        (
            "gamma",
            params_of(&[("alpha", 2.5), ("beta", 400.0)]),
            -10.0,
        ),
        (
            "loglogistic",
            params_of(&[("alpha", 1000.0), ("beta", 2.0)]),
            -10.0,
        ),
        (
            "gengamma",
            params_of(&[("a", 1000.0), ("d", 2.0), ("p", 1.5)]),
            -10.0,
        ),
    ];
    for (dist, p, below) in &cases {
        assert_eq!(pdf(dist, p, *below), 0.0, "{dist}: pdf below support");
        assert_eq!(cdf(dist, p, *below), 0.0, "{dist}: cdf below support");
    }
    // exponential2: below its location gamma the pdf and cdf are 0.
    let p = params_of(&[("theta", 500.0), ("gamma", 100.0)]);
    assert_eq!(pdf("exponential2", &p, 99.0), 0.0);
    assert_eq!(cdf("exponential2", &p, 99.0), 0.0);
    assert_eq!(reliability("exponential2", &p, 99.0), 1.0);
}

// ---------- Family 2: fitting ----------

fn weibull_sample(n: usize) -> Vec<f64> {
    quantile_sample("weibull2", &params_of(&[("beta", 2.0), ("eta", 1000.0)]), n)
}

#[test]
fn validation_fit_recovers_weibull_complete() {
    let times = weibull_sample(40);
    let fit = fit_mle("weibull2", &times, None).expect("weibull fit");
    assert!(fit.success, "status note: {}", fit.status_note);
    assert!(
        rel_close(fit.params["beta"], 2.0, 0.25),
        "beta {} vs generating 2.0",
        fit.params["beta"]
    );
    assert!(
        rel_close(fit.params["eta"], 1000.0, 0.15),
        "eta {} vs generating 1000",
        fit.params["eta"]
    );
}

#[test]
fn validation_fit_recovers_lognormal_complete() {
    let gen = params_of(&[("mu", 7.0), ("sigma", 0.5)]);
    let times = quantile_sample("lognormal", &gen, 40);
    let fit = fit_mle("lognormal", &times, None).expect("lognormal fit");
    assert!(fit.success, "status note: {}", fit.status_note);
    assert!(
        (fit.params["mu"] - 7.0).abs() <= 0.15,
        "mu {} vs generating 7.0",
        fit.params["mu"]
    );
    assert!(
        rel_close(fit.params["sigma"], 0.5, 0.25),
        "sigma {} vs generating 0.5",
        fit.params["sigma"]
    );
}

#[test]
fn validation_fit_recovers_weibull_censored() {
    let times = weibull_sample(40);
    // Suspend the largest 25 percent at their sample times.
    let failed: Vec<bool> = (0..times.len()).map(|i| i < 30).collect();
    let fit = fit_mle("weibull2", &times, Some(&failed)).expect("censored fit");
    assert!(fit.success, "status note: {}", fit.status_note);
    assert!(
        rel_close(fit.params["beta"], 2.0, 0.35),
        "beta {} vs generating 2.0",
        fit.params["beta"]
    );
    assert!(
        rel_close(fit.params["eta"], 1000.0, 0.25),
        "eta {} vs generating 1000",
        fit.params["eta"]
    );
}

#[test]
fn validation_fit_with_ties_is_finite() {
    let times = vec![
        100.0, 100.0, 200.0, 200.0, 300.0, 300.0, 500.0, 500.0, 800.0, 800.0,
    ];
    let fit = fit_mle("weibull2", &times, None).expect("ties fit");
    assert!(fit.success, "status note: {}", fit.status_note);
    assert!(fit.params["beta"].is_finite() && fit.params["beta"] > 0.0);
    assert!(fit.params["eta"].is_finite() && fit.params["eta"] > 0.0);
    assert!(fit.loglik.is_finite());
}

#[test]
fn validation_fit_small_sample_n3() {
    let times = vec![120.0, 450.0, 900.0];
    let fit = fit_mle("weibull2", &times, None).expect("n=3 fit");
    assert!(fit.params["beta"].is_finite(), "beta not finite");
    assert!(fit.params["eta"].is_finite(), "eta not finite");
    assert!(fit.loglik.is_finite(), "loglik not finite");
    for (name, se) in &fit.se {
        assert!(se.is_finite(), "standard error for {name} not finite");
    }
}

#[test]
fn validation_fit_extreme_scales_no_nan() {
    let base = weibull_sample(40);
    for (scale, eta_true) in [(1e-9f64, 1e-6f64), (1e5f64, 1e8f64)] {
        let times: Vec<f64> = base.iter().map(|t| t * scale).collect();
        let fit = fit_mle("weibull2", &times, None)
            .unwrap_or_else(|e| panic!("scale {scale}: fit errored: {e}"));
        assert!(
            fit.params["beta"].is_finite() && fit.params["eta"].is_finite(),
            "scale {scale}: non-finite params {:?}",
            fit.params
        );
        assert!(fit.loglik.is_finite(), "scale {scale}: loglik not finite");
        assert!(
            rel_close(fit.params["beta"], 2.0, 0.25),
            "scale {scale}: beta {} vs 2.0",
            fit.params["beta"]
        );
        assert!(
            rel_close(fit.params["eta"], eta_true, 0.15),
            "scale {scale}: eta {} vs {eta_true}",
            fit.params["eta"]
        );
    }
}

#[test]
fn validation_fit_row_order_invariance() {
    let times = weibull_sample(40);
    let mut shuffled = times.clone();
    shuffled.reverse();
    shuffled.rotate_left(7);
    let a = fit_mle("weibull2", &times, None).expect("ordered fit");
    let b = fit_mle("weibull2", &shuffled, None).expect("shuffled fit");
    assert!(
        rel_close(b.params["beta"], a.params["beta"], 1e-9),
        "beta {} vs {}",
        b.params["beta"],
        a.params["beta"]
    );
    assert!(
        rel_close(b.params["eta"], a.params["eta"], 1e-9),
        "eta {} vs {}",
        b.params["eta"],
        a.params["eta"]
    );
}

#[test]
fn validation_fit_unit_rescaling_invariance() {
    let times = weibull_sample(40);
    let scaled: Vec<f64> = times.iter().map(|t| t * 3600.0).collect();
    let a = fit_mle("weibull2", &times, None).expect("hours fit");
    let b = fit_mle("weibull2", &scaled, None).expect("seconds fit");
    assert!(
        rel_close(b.params["beta"], a.params["beta"], 1e-6),
        "beta changed under rescaling: {} vs {}",
        b.params["beta"],
        a.params["beta"]
    );
    assert!(
        rel_close(b.params["eta"], a.params["eta"] * 3600.0, 1e-6),
        "eta did not scale by 3600: {} vs {}",
        b.params["eta"],
        a.params["eta"] * 3600.0
    );
}

// ---------- Family 3: uncertainty, second path ----------

#[test]
fn validation_fisher_se_matches_independent_hessian() {
    let times = weibull_sample(40);
    let failed = vec![true; times.len()];
    let fit = fit_mle("weibull2", &times, None).expect("fit for SE check");
    let beta = fit.params["beta"];
    let eta = fit.params["eta"];
    let se_beta_lib = fit.se["beta"];
    let se_eta_lib = fit.se["eta"];

    // Independent path: central finite differences of the library
    // loglik (the likelihood definition) with this test's own steps,
    // and this test's own 2 by 2 inverse.
    let nll = |b: f64, e: f64| -loglik("weibull2", &[b, e], &times, &failed);
    let hb = 1e-3 * beta.abs().max(1.0);
    let he = 1e-3 * eta.abs().max(1.0);
    let f0 = nll(beta, eta);
    let hbb = (nll(beta + hb, eta) - 2.0 * f0 + nll(beta - hb, eta)) / (hb * hb);
    let hee = (nll(beta, eta + he) - 2.0 * f0 + nll(beta, eta - he)) / (he * he);
    let hbe = (nll(beta + hb, eta + he) - nll(beta + hb, eta - he) - nll(beta - hb, eta + he)
        + nll(beta - hb, eta - he))
        / (4.0 * hb * he);
    let det = hbb * hee - hbe * hbe;
    assert!(det > 0.0, "independent Hessian not positive definite");
    let se_beta_ind = (hee / det).sqrt();
    let se_eta_ind = (hbb / det).sqrt();
    assert!(
        rel_close(se_beta_lib, se_beta_ind, 0.15),
        "beta SE: library {se_beta_lib} vs independent {se_beta_ind}"
    );
    assert!(
        rel_close(se_eta_lib, se_eta_ind, 0.15),
        "eta SE: library {se_eta_lib} vs independent {se_eta_ind}"
    );
}

// ---------- Family 5: RBD closed forms ----------

#[test]
fn validation_rbd_series_parallel_exponential_closed_form() {
    let b1 =
        json!({"type": "block", "name": "a", "dist": "exponential", "params": {"theta": 1000.0}});
    let b2 =
        json!({"type": "block", "name": "b", "dist": "exponential", "params": {"theta": 2000.0}});
    let series = json!({"type": "series", "children": [b1.clone(), b2.clone()]});
    let parallel = json!({"type": "parallel", "children": [b1, b2]});
    let r1 = (-0.5f64).exp();
    let r2 = (-0.25f64).exp();
    let got_series = evaluate_rbd(&series, Some(500.0)).expect("series");
    assert!(
        (got_series - r1 * r2).abs() <= 1e-12,
        "series {got_series} vs hand {}",
        r1 * r2
    );
    let got_parallel = evaluate_rbd(&parallel, Some(500.0)).expect("parallel");
    let want_parallel = 1.0 - (1.0 - r1) * (1.0 - r2);
    assert!(
        (got_parallel - want_parallel).abs() <= 1e-12,
        "parallel {got_parallel} vs hand {want_parallel}"
    );
}

#[test]
fn validation_rbd_kofn_binomial_sum() {
    let rs = [0.9f64, 0.8, 0.7];
    // Hand enumeration: every subset of size 2 or more.
    let mut want = 0.0;
    for mask in 0..8usize {
        let bits = (0..3).filter(|i| (mask >> i) & 1 == 1).count();
        if bits >= 2 {
            let mut term = 1.0;
            for (i, r) in rs.iter().enumerate() {
                term *= if (mask >> i) & 1 == 1 { *r } else { 1.0 - r };
            }
            want += term;
        }
    }
    let got = kofn_reliability(&rs, 2);
    assert!((got - want).abs() <= 1e-12, "kofn {got} vs hand {want}");
    let spec = json!({"type": "kofn", "k": 2, "children": [
        {"type": "block", "name": "a", "R": 0.9},
        {"type": "block", "name": "b", "R": 0.8},
        {"type": "block", "name": "c", "R": 0.7}
    ]});
    let got_spec = evaluate_rbd(&spec, None).expect("kofn spec");
    assert!(
        (got_spec - want).abs() <= 1e-12,
        "kofn spec {got_spec} vs hand {want}"
    );
}

#[test]
fn validation_standby_cold_closed_form() {
    // Hand Poisson sum with lam * t = 1 (theta 1000, t 1000).
    for (n, sw) in [(2usize, 1.0f64), (2, 0.95), (3, 1.0), (3, 0.95)] {
        let mut sum = 0.0;
        let mut term = 1.0;
        for i in 0..n {
            if i > 0 {
                term *= sw * 1.0 / i as f64;
            }
            sum += term;
        }
        let want = (-1.0f64).exp() * sum;
        let a = standby_closed_form_exp(1000.0, 0.001, n, sw);
        let b = cold_standby_exponential(1000.0, 0.001, n, sw);
        assert!(
            (a - want).abs() <= 1e-12,
            "standby mod n={n} sw={sw}: {a} vs {want}"
        );
        assert!(
            (b - want).abs() <= 1e-12,
            "system n={n} sw={sw}: {b} vs {want}"
        );
    }
    // The n = 2, sw = 1 anchor also used in tests/standby_tests.rs.
    assert!(
        (standby_closed_form_exp(1000.0, 0.001, 2, 1.0) - (-1.0f64).exp() * 2.0).abs() <= 1e-12
    );
}

// ---------- Family 6: availability ----------

#[test]
fn validation_availability_ctmc_single_unit() {
    let (lam, mu) = (0.001f64, 0.01f64);
    let (q, down) = standby_ctmc(lam, mu, 1, 0.0, 1);
    let pi = ctmc_steady_state(&q);
    let availability: f64 = pi
        .iter()
        .enumerate()
        .filter(|(i, _)| !down.contains(i))
        .map(|(_, p)| *p)
        .sum();
    let want = mu / (lam + mu);
    assert!(
        (availability - want).abs() <= 1e-9,
        "CTMC availability {availability} vs {want}"
    );
    // The same number in MTTF / (MTTF + MTTR) form.
    assert!((want - 1000.0 / 1100.0).abs() <= 1e-12);
    assert!((single_block_availability(1000.0, 100.0) - want).abs() <= 1e-12);
}

#[test]
fn validation_availability_monte_carlo_agrees_with_ctmc() {
    // AvailabilityResult carries std_error (standard error of the
    // per-simulation time-averaged availability), so the check is
    // the 3 standard error agreement recorded in fixtures.md.
    let spec = json!({"type": "block", "name": "unit", "dist": "exponential",
        "params": {"theta": 1000.0}, "mtbf": 1000.0, "mttr": 100.0});
    let res = simulate_availability(&spec, 20000.0, 200, 3, 20);
    let want = 1000.0 / 1100.0;
    assert!(
        res.std_error > 0.0 && res.std_error.is_finite(),
        "std_error should be positive and finite, got {}",
        res.std_error
    );
    assert!(
        (res.mean_availability - want).abs() <= 3.0 * res.std_error,
        "Monte Carlo mean availability {} vs CTMC {want}: diff {} exceeds 3 SE ({})",
        res.mean_availability,
        (res.mean_availability - want).abs(),
        3.0 * res.std_error
    );
    assert!(res.mean_availability > 0.0 && res.mean_availability < 1.0);
    assert!(res.expected_downtime >= 0.0 && res.expected_downtime.is_finite());
}

// ---------- Family 8: ALT ----------

#[test]
fn validation_alt_arrhenius_celsius_to_kelvin() {
    // Hand computation in the test from the stated constants.
    let k = 8.617e-5f64;
    let ea = 0.7f64;
    let t_use_k = 25.0 + 273.15;
    let t_stress_k = 85.0 + 273.15;
    let want = (ea / k * (1.0 / t_use_k - 1.0 / t_stress_k)).exp();
    // Anchor the hand value itself: it is near 95.8.
    assert!(
        (want - 95.8).abs() / 95.8 < 0.01,
        "hand AF {want} not near 95.8"
    );
    let got = acceleration_factor_arrhenius(ea, 25.0, 85.0);
    assert!(
        (got - want).abs() / want <= 0.01,
        "library AF {got} vs hand {want} (Celsius inputs converted to Kelvin)"
    );
    // A Kelvin-as-Celsius mistake would be far outside tolerance:
    // treating 25 and 85 as Kelvin gives a wildly different factor,
    // so agreement here confirms the conversion really happens.
    let wrong = (ea / k * (1.0 / 25.0 - 1.0 / 85.0)).exp();
    assert!(
        (got - wrong).abs() / wrong > 0.5,
        "AF matches the no-conversion value"
    );
}
