//! Tests for the v0.2.0 expansion stream: distribution wizard,
//! QCP, reliability importance, DOE expansions, growth models and
//! ALT expansions. Data are synthetic with fixed seeds, and the
//! expected values are hand values stated in each test.

use reliakit::alt::{
    alt_loglinear_use_reliability, fit_alt_mle_loglinear, step_stress_reliability,
    step_stress_reliability_eta_ref,
};
use reliakit::distributions::{self, params_of, FitResult};
use reliakit::doe;
use reliakit::growth::{fit_gompertz, fit_lloyd_lipow};
use reliakit::importance::importance_measures;
use reliakit::quickcalc::quick_calc;
use reliakit::rng::Rng64;
use reliakit::wizard::{anderson_darling, rank_distributions, RANKABLE_DISTS};
use serde_json::json;
use std::collections::BTreeMap;

fn rel_close(got: f64, want: f64, rel: f64) -> bool {
    (got - want).abs() <= rel * want.abs().max(1.0)
}

// ---------------- wizard ----------------

#[test]
fn wizard_weibull_ranks_first_on_weibull_data() {
    // Seeded Weibull data: beta 2, eta 1000, n 150, complete.
    let mut rng = Rng64::new(20261105);
    let times: Vec<f64> = (0..150).map(|_| rng.weibull(2.0, 1000.0)).collect();
    let ranks = rank_distributions(
        &times,
        None,
        &["exponential", "weibull2", "lognormal", "normal"],
    );
    assert_eq!(ranks.len(), 4);
    assert!(ranks.iter().all(|r| r.success), "ranks: {:?}", ranks);
    assert_eq!(
        ranks[0].dist, "weibull2",
        "expected weibull2 first, ranking: {:?}",
        ranks.iter().map(|r| (&r.dist, r.aic)).collect::<Vec<_>>()
    );
    let weibull_aic = ranks[0].aic;
    let exp = ranks.iter().find(|r| r.dist == "exponential").unwrap();
    assert!(
        exp.aic > weibull_aic,
        "exponential AIC {} should be worse than weibull2 AIC {}",
        exp.aic,
        weibull_aic
    );
    // AIC and BIC formulas and parameter counts.
    let w = &ranks[0];
    assert_eq!(w.n_params, 2);
    assert_eq!(exp.n_params, 1);
    assert!((w.aic - (2.0 * 2.0 - 2.0 * w.loglik)).abs() < 1e-9);
    assert!((w.bic - (2.0 * (150.0f64).ln() - 2.0 * w.loglik)).abs() < 1e-9);
    assert!(w.ad_statistic.is_some());
    // RANKABLE_DISTS is the stated list, and ranking over it does
    // not panic even for names not yet fittable: failures sort last.
    assert_eq!(RANKABLE_DISTS.len(), 13);
    assert!(RANKABLE_DISTS.contains(&"weibull2"));
    assert!(!RANKABLE_DISTS.contains(&"weibull1"));
    let all = rank_distributions(&times, None, RANKABLE_DISTS);
    assert_eq!(all.len(), RANKABLE_DISTS.len());
    let first_failure = all.iter().position(|r| !r.success).unwrap_or(all.len());
    assert!(all[first_failure..].iter().all(|r| !r.success));
    assert!(all[..first_failure].iter().all(|r| r.success));
}

fn manual_fit(dist: &str, params: &[(&str, f64)], times: &[f64]) -> FitResult {
    FitResult {
        dist: dist.to_string(),
        params: params_of(params),
        loglik: 0.0,
        n: times.len(),
        n_fail: times.len(),
        method: "manual".to_string(),
        cov: None,
        se: BTreeMap::new(),
        times: times.to_vec(),
        failed: vec![true; times.len()],
        success: true,
    }
}

#[test]
fn wizard_anderson_darling_hand_case() {
    // Tiny case: exponential theta 100, times [50, 100, 200].
    // Expected value computed here directly from the A^2 formula
    // using distributions::cdf, which validates the summation in
    // anderson_darling itself.
    let times = vec![50.0, 100.0, 200.0];
    let fit = manual_fit("exponential", &[("theta", 100.0)], &times);
    let params = params_of(&[("theta", 100.0)]);
    let n = times.len() as f64;
    let f: Vec<f64> = times
        .iter()
        .map(|t| distributions::cdf("exponential", &params, *t))
        .collect();
    let mut sum = 0.0;
    for i in 1..=times.len() {
        sum += (2 * i - 1) as f64 * (f[i - 1].ln() + (1.0 - f[times.len() - i]).ln());
    }
    let expected = -n - sum / n;
    let got = anderson_darling(&fit).expect("complete sample gives AD");
    assert!(
        (got - expected).abs() < 1e-6,
        "AD got {} expected {}",
        got,
        expected
    );

    // Same sample against a badly wrong theta gives a larger AD.
    let bad = manual_fit("exponential", &[("theta", 10000.0)], &times);
    let ad_bad = anderson_darling(&bad).unwrap();
    assert!(
        got < ad_bad,
        "AD for theta 100 ({}) should be below AD for theta 10000 ({})",
        got,
        ad_bad
    );

    // Censored samples return None, as do samples under n = 2.
    let mut censored = manual_fit("exponential", &[("theta", 100.0)], &times);
    censored.failed[1] = false;
    assert!(anderson_darling(&censored).is_none());
    let tiny = manual_fit("exponential", &[("theta", 100.0)], &[50.0]);
    assert!(anderson_darling(&tiny).is_none());
}

// ---------------- quickcalc ----------------

#[test]
fn quickcalc_weibull_hand_values() {
    // Weibull beta 2, eta 1000 at t 1000.
    let params = params_of(&[("beta", 2.0), ("eta", 1000.0)]);
    let r = quick_calc("weibull2", &params, 1000.0, 0.10).unwrap();
    // R = exp(-(1000/1000)^2) = exp(-1).
    let want_r = (-1.0f64).exp();
    assert!(rel_close(r.reliability, want_r, 1e-9), "R {} vs {}", r.reliability, want_r);
    assert!(rel_close(r.unreliability, 1.0 - want_r, 1e-9));
    // hazard = (beta/eta) * (t/eta)^(beta-1) / 1 at t = eta with
    // the survival factor cancelling the pdf factor: 2/1000 = 0.002.
    assert!(rel_close(r.hazard, 0.002, 1e-9), "hazard {}", r.hazard);
    // B10 = eta * (ln(1 / 0.9))^(1/beta).
    let want_b10 = 1000.0 * (1.0f64 / 0.9).ln().powf(0.5);
    assert!(
        rel_close(r.bx_life, want_b10, 1e-9),
        "B10 {} vs {}",
        r.bx_life,
        want_b10
    );
    // pdf at t = eta: (2/1000) * exp(-1).
    assert!(rel_close(r.pdf, 0.002 * want_r, 1e-9));
    // mean life = eta * Gamma(1.5) = 1000 * sqrt(pi) / 2.
    let want_mean = 1000.0 * std::f64::consts::PI.sqrt() / 2.0;
    assert!(rel_close(r.mean_life, want_mean, 1e-9));
    // Unknown distribution and bad bx are errors.
    assert!(quick_calc("nope", &params, 100.0, 0.1).is_err());
    assert!(quick_calc("weibull2", &params, 100.0, 1.5).is_err());
}

// ---------------- importance ----------------

#[test]
fn importance_series_hand_values() {
    // Series of R 0.9 (a) and R 0.8 (b): R_sys = 0.72.
    let spec = json!({"type":"series","children":[
        {"type":"block","name":"a","R":0.9},
        {"type":"block","name":"b","R":0.8}
    ]});
    let imp = importance_measures(&spec, 100.0).unwrap();
    assert_eq!(imp.len(), 2);
    let a = imp.iter().find(|i| i.name == "a").unwrap();
    // For a: R1 = 0.8, R0 = 0, birnbaum 0.8.
    assert!((a.birnbaum - 0.8).abs() < 1e-9, "birnbaum {}", a.birnbaum);
    // criticality = 0.8 * 0.9 / 0.72 = 1.0.
    assert!((a.criticality - 1.0).abs() < 1e-9, "crit {}", a.criticality);
    // FV = (0.8 - 0.72) / (1 - 0.72) = 0.08 / 0.28.
    assert!(
        (a.fussell_vesely - 0.08 / 0.28).abs() < 1e-9,
        "FV {} vs {}",
        a.fussell_vesely,
        0.08 / 0.28
    );
    let b = imp.iter().find(|i| i.name == "b").unwrap();
    assert!((b.birnbaum - 0.9).abs() < 1e-9);
    assert!((b.criticality - 1.0).abs() < 1e-9);
}

#[test]
fn importance_parallel_hand_values() {
    // Parallel pair 0.9 and 0.8: R_sys = 1 - 0.1 * 0.2 = 0.98.
    let spec = json!({"type":"parallel","children":[
        {"type":"block","name":"a","R":0.9},
        {"type":"block","name":"b","R":0.8}
    ]});
    let imp = importance_measures(&spec, 50.0).unwrap();
    let a = imp.iter().find(|i| i.name == "a").unwrap();
    // For a: R1 = 1, R0 = 0.8, birnbaum 0.2.
    assert!((a.birnbaum - 0.2).abs() < 1e-9, "birnbaum {}", a.birnbaum);
    // criticality = 0.2 * 0.9 / 0.98 = 0.183673469387755...
    assert!(
        (a.criticality - 0.2 * 0.9 / 0.98).abs() < 1e-9,
        "crit {} vs {}",
        a.criticality,
        0.2 * 0.9 / 0.98
    );
    // FV = (1 - 0.98) / (1 - 0.98) = 1.0.
    assert!((a.fussell_vesely - 1.0).abs() < 1e-9, "FV {}", a.fussell_vesely);
}

#[test]
fn importance_nested_and_default_names() {
    // Nested: series(parallel(a, b), c), all R blocks. Replacement
    // must work through the nested parallel node.
    let spec = json!({"type":"series","children":[
        {"type":"parallel","children":[
            {"type":"block","R":0.9},
            {"type":"block","name":"b","R":0.8}
        ]},
        {"type":"block","name":"c","R":0.95}
    ]});
    let imp = importance_measures(&spec, 10.0).unwrap();
    assert_eq!(imp.len(), 3);
    // The unnamed block gets the default name "block 0".
    let first = imp.iter().find(|i| i.name == "block 0").unwrap();
    // R_sys = 0.98 * 0.95 = 0.931. For the unnamed block:
    // R1 = 1 * 0.95 = 0.95, R0 = 0.8 * 0.95 = 0.76.
    assert!((first.birnbaum - (0.95 - 0.76)).abs() < 1e-9);
    let c = imp.iter().find(|i| i.name == "c").unwrap();
    // For c: R1 = 0.98, R0 = 0, birnbaum 0.98, criticality 1.
    assert!((c.birnbaum - 0.98).abs() < 1e-9);
    assert!((c.criticality - 1.0).abs() < 1e-9);
}

// ---------------- doe: fractional factorial ----------------

#[test]
fn doe_fractional_factorial_2_5_minus_2() {
    let gens: [(char, &str); 2] = [('D', "AB"), ('E', "AC")];
    let design = doe::fractional_factorial(5, &gens);
    assert_eq!(design.len(), 8, "2^(5-2) has 8 runs");
    assert!(design.iter().all(|r| r.len() == 5));
    // Generated columns follow their definitions in every run.
    for row in &design {
        assert_eq!(row[3], row[0] * row[1], "D = A * B in {:?}", row);
        assert_eq!(row[4], row[0] * row[2], "E = A * C in {:?}", row);
    }
    // Defining relation: I = ABD = ACE = BCDE.
    let words = doe::defining_relation(&gens);
    assert!(words.contains(&"ABD".to_string()), "words {:?}", words);
    assert!(words.contains(&"ACE".to_string()), "words {:?}", words);
    assert!(words.contains(&"BCDE".to_string()), "words {:?}", words);
    assert_eq!(words.len(), 3);
    // Shortest word has length 3: resolution III.
    assert_eq!(doe::resolution(&gens), Some(3));
    // Factor A is aliased with BD and CE (and ABCDE).
    let aliases = doe::alias_set(&gens, 'A');
    assert!(aliases.contains(&"BD".to_string()), "aliases {:?}", aliases);
    assert!(aliases.contains(&"CE".to_string()), "aliases {:?}", aliases);
}

// ---------------- doe: Plackett-Burman ----------------

fn assert_pb(n_runs: usize) {
    let design = doe::plackett_burman(n_runs).expect("supported run count");
    assert_eq!(design.len(), n_runs);
    let cols = n_runs - 1;
    assert!(design.iter().all(|r| r.len() == cols));
    for c in 0..cols {
        let plus = design.iter().filter(|r| r[c] > 0.0).count();
        assert_eq!(plus * 2, n_runs, "column {} of PB{} not balanced", c, n_runs);
        for c2 in (c + 1)..cols {
            let dot: f64 = design.iter().map(|r| r[c] * r[c2]).sum();
            assert!(
                dot.abs() < 1e-9,
                "PB{} columns {} and {} dot {}",
                n_runs, c, c2, dot
            );
        }
    }
}

#[test]
fn doe_plackett_burman_12_orthogonal() {
    assert_pb(12);
}

#[test]
fn doe_plackett_burman_20_and_24() {
    assert_pb(20);
    assert_pb(24);
    assert!(doe::plackett_burman(16).is_none());
}

// ---------------- doe: Taguchi ----------------

#[test]
fn doe_taguchi_arrays_orthogonal() {
    let l8 = doe::taguchi_l8();
    assert_eq!(l8.len(), 8);
    assert!(l8.iter().all(|r| r.len() == 7));
    for c in 0..7 {
        let sum: f64 = l8.iter().map(|r| r[c]).sum();
        assert!(sum.abs() < 1e-9, "L8 column {} not balanced", c);
        for c2 in (c + 1)..7 {
            let dot: f64 = l8.iter().map(|r| r[c] * r[c2]).sum();
            assert!(dot.abs() < 1e-9, "L8 columns {} {} dot {}", c, c2, dot);
        }
    }
    let l9 = doe::taguchi_l9();
    assert_eq!(l9.len(), 9);
    assert!(l9.iter().all(|r| r.len() == 4));
    for c in 0..4 {
        for level in [-1.0, 0.0, 1.0] {
            let count = l9.iter().filter(|r| r[c] == level).count();
            assert_eq!(count, 3, "L9 column {} level {}", c, level);
        }
        for c2 in (c + 1)..4 {
            let dot: f64 = l9.iter().map(|r| r[c] * r[c2]).sum();
            assert!(dot.abs() < 1e-9, "L9 columns {} {} dot {}", c, c2, dot);
        }
    }
    assert_eq!(doe::taguchi_orthogonal_array("L8").unwrap().len(), 8);
    assert!(doe::taguchi_orthogonal_array("L27").is_none());
}

#[test]
fn doe_signal_to_noise_hand_values() {
    // Smaller-is-better, y = [1, 2, 3]: mean of squares = 14 / 3.
    let want = -10.0 * (14.0f64 / 3.0).log10();
    assert!((doe::sn_smaller_better(&[1.0, 2.0, 3.0]) - want).abs() < 1e-12);
    // Larger-is-better, y = [1, 2, 4]: mean of 1/y^2
    // = (1 + 1/4 + 1/16) / 3 = 0.4375.
    let want = -10.0 * (0.4375f64).log10();
    assert!((doe::sn_larger_better(&[1.0, 2.0, 4.0]) - want).abs() < 1e-12);
    // Nominal-is-best, y = [2, 3, 4]: mean 3, population variance
    // (divisor n, the convention documented on the function)
    // = (1 + 0 + 1) / 3 = 2/3. SN = 10 log10(9 / (2/3))
    // = 10 log10(13.5).
    let want = 10.0 * (13.5f64).log10();
    assert!((doe::sn_nominal_best(&[2.0, 3.0, 4.0]) - want).abs() < 1e-12);
}

// ---------------- doe: response surface ----------------

#[test]
fn doe_ccd_recovers_exact_quadratic() {
    let runs = doe::central_composite(2, 1.4142135623730951);
    assert_eq!(runs.len(), 4 + 4 + 1, "factorial + axial + 1 center");
    // y = 1 + 2 x1 - x2 + 3 x1^2 + 0 * x2^2 + 0.5 x1 x2.
    let y: Vec<f64> = runs
        .iter()
        .map(|r| {
            1.0 + 2.0 * r[0] - r[1] + 3.0 * r[0] * r[0] + 0.5 * r[0] * r[1]
        })
        .collect();
    let coef = doe::fit_quadratic(&runs, &y).unwrap();
    // Coefficient order: [1, x1, x2, x1^2, x2^2, x1 x2].
    let want = [1.0, 2.0, -1.0, 3.0, 0.0, 0.5];
    assert_eq!(coef.len(), want.len());
    for (g, w) in coef.iter().zip(want.iter()) {
        assert!((g - w).abs() < 1e-6, "coef {} vs want {}", g, w);
    }
    let anova = doe::quadratic_anova(&runs, &y).unwrap();
    assert!(
        anova.ss_residual < 1e-12,
        "residual SS {} for an exact quadratic",
        anova.ss_residual
    );
    assert!(anova.ss_regression > 0.0);
    assert_eq!(anova.df_regression, 5);
    assert_eq!(anova.df_residual, 9 - 6);
    assert!(anova.f_statistic > 1e6 || anova.f_statistic.is_infinite());
    assert!((anova.r_squared - 1.0).abs() < 1e-9);
}

#[test]
fn doe_box_behnken_shape() {
    let bb3 = doe::box_behnken(3);
    assert_eq!(bb3.len(), 13, "12 pair runs + 1 center for k = 3");
    assert!(bb3.iter().all(|r| r.len() == 3));
    let bb4 = doe::box_behnken(4);
    assert_eq!(bb4.len(), 25, "24 pair runs + 1 center for k = 4");
    for row in bb3.iter().take(12) {
        // Each non-center run has exactly one factor at 0 and the
        // other two at -1 or 1.
        assert_eq!(row.iter().filter(|v| **v == 0.0).count(), 1);
    }
    assert!(doe::box_behnken(2).is_empty());
    // CCD with an explicit center count.
    let ccd = doe::central_composite_with_center(3, 1.0, 5);
    assert_eq!(ccd.len(), 8 + 6 + 5);
}

// ---------------- growth ----------------

#[test]
fn growth_gompertz_recovers_parameters() {
    // Noise-free Gompertz data: a 120, b 0.55, c 0.92.
    let (a_t, b_t, c_t): (f64, f64, f64) = (120.0, 0.55, 0.92);
    let times: Vec<f64> = vec![0.0, 5.0, 10.0, 15.0, 20.0, 30.0, 40.0, 50.0];
    let cum: Vec<f64> = times
        .iter()
        .map(|t| a_t * b_t.powf(c_t.powf(*t)))
        .collect();
    let fit = fit_gompertz(&times, &cum).unwrap();
    assert!(
        (fit.a - a_t).abs() <= 0.10 * a_t,
        "a recovered {} vs true {}",
        fit.a,
        a_t
    );
    for (t, obs) in times.iter().zip(&cum) {
        let pred = fit.predicted(*t);
        assert!(
            (pred - obs).abs() <= 0.05 * obs.max(1.0) + 0.5,
            "prediction at {} is {} vs data {}",
            t,
            pred,
            obs
        );
    }
    assert!(fit.b > 0.0 && fit.b < 1.0 && fit.c > 0.0 && fit.c < 1.0);
}

#[test]
fn growth_lloyd_lipow_recovers_parameters() {
    // Noise-free Lloyd-Lipow data: n_inf 80, alpha 900.
    let times: Vec<f64> = vec![20.0, 30.0, 50.0, 80.0, 120.0, 200.0, 400.0];
    let cum: Vec<f64> = times.iter().map(|t| 80.0 - 900.0 / t).collect();
    let fit = fit_lloyd_lipow(&times, &cum).unwrap();
    assert!(
        (fit.n_inf - 80.0).abs() <= 0.02 * 80.0,
        "n_inf {} vs 80",
        fit.n_inf
    );
    assert!(
        (fit.alpha - 900.0).abs() <= 0.02 * 900.0,
        "alpha {} vs 900",
        fit.alpha
    );
    assert!((fit.predicted(100.0) - 71.0).abs() < 1e-6);
}

// ---------------- alt: loglinear MLE ----------------

#[test]
fn alt_loglinear_weibull_recovers_b1_and_monotone_use_reliability() {
    // True model: log eta = b0 + b1 / T + b2 ln S + b3 ln S / T
    // with b0 -8, b1 4500, b2 -0.7, b3 800, Weibull beta 2.5.
    // Stresses: T in {300, 360, 430} K, S in {1, 4}. At S = 1 the
    // ln S terms vanish, which pins down b1 cleanly.
    let (b0, b1, b2, b3, beta) = (-8.0, 4500.0, -0.7, 800.0, 2.5);
    let mut rng = Rng64::new(777001);
    let mut times: Vec<f64> = Vec::new();
    let mut stress: Vec<Vec<f64>> = Vec::new();
    for &temp_k in &[300.0f64, 360.0, 430.0] {
        for &s in &[1.0f64, 4.0] {
            let log_eta = b0 + b1 / temp_k + b2 * s.ln() + b3 * s.ln() / temp_k;
            let eta = log_eta.exp();
            for _ in 0..60 {
                times.push(rng.weibull(beta, eta));
                stress.push(vec![temp_k, s]);
            }
        }
    }
    let fit = fit_alt_mle_loglinear(&times, None, &stress, "weibull2").unwrap();
    assert_eq!(fit.n, 360);
    assert_eq!(fit.n_fail, 360);
    assert!(fit.loglik.is_finite());
    let b1_hat = fit.coef["b1"];
    assert!(b1_hat > 0.0, "b1 sign: recovered {}", b1_hat);
    assert!(
        (b1_hat - b1).abs() <= 0.35 * b1,
        "b1 recovered {} vs true {} (35 percent tolerance)",
        b1_hat,
        b1
    );
    assert!(
        (fit.beta - beta).abs() <= 0.35 * beta,
        "beta recovered {} vs true {}",
        fit.beta,
        beta
    );
    // Use-condition reliability falls as temperature rises at fixed
    // stress and age: the effective temperature coefficient
    // b1 + b3 ln S is positive at both stress levels here.
    for &s in &[1.0f64, 4.0] {
        let t_eval = fit.median_life(360.0, s);
        let r_low_t = alt_loglinear_use_reliability(&fit, 300.0, s, t_eval);
        let r_high_t = alt_loglinear_use_reliability(&fit, 430.0, s, t_eval);
        assert!(
            r_high_t < r_low_t,
            "at S {}: R(430 K) {} not below R(300 K) {}",
            s,
            r_high_t,
            r_low_t
        );
        assert!((0.0..=1.0).contains(&r_low_t));
    }
    // Lognormal variant runs on the same data layout.
    let fit_ln = fit_alt_mle_loglinear(&times, None, &stress, "lognormal").unwrap();
    assert!(fit_ln.loglik.is_finite() && fit_ln.sigma > 0.0);
    assert!(fit_alt_mle_loglinear(&times, None, &stress, "gamma").is_err());
}

// ---------------- alt: step stress ----------------

#[test]
fn alt_step_stress_one_step_equals_plain_weibull() {
    // One step: S 2 for 1000 h, beta 1.7, K 800, n 1.5.
    // eta(2) = 800 * 2^(-1.5).
    let steps = [(2.0, 1000.0)];
    let eta = 800.0 * 2.0f64.powf(-1.5);
    let params = params_of(&[("beta", 1.7), ("eta", eta)]);
    for &t in &[10.0, 100.0, 500.0, 999.0] {
        let got = step_stress_reliability(t, &steps, 1.7, 800.0, 1.5);
        let want = distributions::reliability("weibull2", &params, t);
        assert!(
            (got - want).abs() < 1e-12,
            "t {}: step stress {} vs plain Weibull {}",
            t,
            got,
            want
        );
    }
    // The eta_ref form states the same model: eta_ref 800 at
    // s_ref 1 gives K 800.
    let got = step_stress_reliability_eta_ref(100.0, &steps, 1.7, 800.0, 1.0, 1.5);
    let want = distributions::reliability("weibull2", &params, 100.0);
    assert!((got - want).abs() < 1e-12);
}

#[test]
fn alt_step_stress_two_step_hand_example() {
    // Hand example from the documented Nelson equations.
    // beta 2, K 1000, n 1: eta(1) = 1000, eta(2) = 500.
    // Step 1: S 1 for 100 h. End virtual age 100, cumulative
    // hazard term (100 / 1000)^2 = 0.01.
    // Equivalent start age in step 2: tau = 500 * 100 / 1000 = 50.
    // At t 150 (50 h into step 2): virtual age 100, so
    // R = exp(-(100 / 500)^2) = exp(-0.04).
    let steps = [(1.0, 100.0), (2.0, 200.0)];
    let got = step_stress_reliability(150.0, &steps, 2.0, 1000.0, 1.0);
    let want = (-0.04f64).exp();
    assert!(
        (got - want).abs() < 1e-9,
        "two-step R {} vs hand value {}",
        got,
        want
    );
    // Inside step 1 the model is the plain Weibull at S 1.
    let got1 = step_stress_reliability(50.0, &steps, 2.0, 1000.0, 1.0);
    let want1 = (-(50.0f64 / 1000.0).powi(2)).exp();
    assert!((got1 - want1).abs() < 1e-12);
    // At the step boundary t 100, both segment readings agree:
    // exp(-(100/1000)^2) = exp(-0.01).
    let got_b = step_stress_reliability(100.0, &steps, 2.0, 1000.0, 1.0);
    assert!((got_b - (-0.01f64).exp()).abs() < 1e-12);
}
