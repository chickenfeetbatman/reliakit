//! QA cross-check tests for base features (Stream 6).
//!
//! These tests verify the library against independent methods:
//! numerical differentiation, numerical integration, analytic
//! closed forms, round trips, and exact hand calculations. They
//! complement tests/ported_tests.rs and tests/parity_tests.rs and
//! use only modules that already exist in the base library.

use reliakit::alt::fit_alt;
use reliakit::degradation::{fit_path, pseudo_failure_time};
use reliakit::distributions::{cdf, fit_mle, mean_life, params_of, pdf, quantile};
use reliakit::doe::{analyze_factorial, factorial_design};
use reliakit::faulttree::{minimal_cut_sets, FtNode};
use reliakit::prediction::{
    part_failure_rate_per_million_hours, system_mtbf_hours, PartEntry,
};
use reliakit::rng::Rng64;
use reliakit::specfunc::{beta_ppf, norm_cdf, norm_ppf};
use reliakit::warranty::{forecast_returns, WarrantyRecord};
use std::collections::BTreeSet;

// ---- gamma distribution ----

#[test]
fn qa_gamma_cdf_derivative_matches_pdf() {
    let p = params_of(&[("alpha", 2.5), ("beta", 1.3)]);
    let t = 2.0;
    let h = 1e-4;
    let numeric_derivative = (cdf("gamma", &p, t + h) - cdf("gamma", &p, t - h)) / (2.0 * h);
    let analytic_pdf = pdf("gamma", &p, t);
    assert!(
        (numeric_derivative - analytic_pdf).abs() <= 2e-3,
        "numeric derivative {} vs pdf {}",
        numeric_derivative,
        analytic_pdf
    );
}

#[test]
fn qa_gamma_cdf_at_mean_in_range() {
    let p = params_of(&[("alpha", 2.5), ("beta", 1.3)]);
    let mean = mean_life("gamma", &p);
    assert!((mean - 2.5 * 1.3).abs() <= 1e-12, "mean {}", mean);
    let f_at_mean = cdf("gamma", &p, mean);
    assert!(
        (0.4..=0.65).contains(&f_at_mean),
        "cdf at the mean {} not in [0.4, 0.65]",
        f_at_mean
    );
}

// ---- gumbel distribution ----

#[test]
fn qa_gumbel_cdf_quantile_roundtrip() {
    let p = params_of(&[("mu", 100.0), ("beta", 15.0)]);
    for prob in [0.1, 0.5, 0.9] {
        let q = quantile("gumbel", &p, prob);
        let back = cdf("gumbel", &p, q);
        assert!(
            (back - prob).abs() <= 1e-9,
            "p {} quantile {} cdf back {}",
            prob,
            q,
            back
        );
    }
}

#[test]
fn qa_gumbel_pdf_integrates_to_one() {
    let p = params_of(&[("mu", 100.0), ("beta", 15.0)]);
    let mu = 100.0;
    let beta = 15.0;
    let (a, b) = (mu - 10.0 * beta, mu + 10.0 * beta);
    let steps = 4000usize;
    let h = (b - a) / steps as f64;
    // Simpson rule with an even number of steps.
    let mut sum = pdf("gumbel", &p, a) + pdf("gumbel", &p, b);
    for i in 1..steps {
        let x = a + i as f64 * h;
        let weight = if i % 2 == 0 { 2.0 } else { 4.0 };
        sum += weight * pdf("gumbel", &p, x);
    }
    let integral = sum * h / 3.0;
    assert!(
        (integral - 1.0).abs() <= 2e-3,
        "Simpson integral of gumbel pdf {}",
        integral
    );
}

// ---- specfunc ----

// History: this test was briefly ignored because norm_cdf was built
// on the Abramowitz and Stegun 7.1.26 erf approximation (absolute
// error 1.5e-7, which the 1 / pdf factor amplifies at x = 2.0). The
// coordinator replaced erf and norm_cdf with incomplete-gamma based
// versions accurate to about 1e-14, and the test now runs un-ignored
// with its original tolerance.
#[test]
fn qa_specfunc_norm_ppf_cdf_roundtrip() {
    for x in [-1.5, 0.0, 0.7, 2.0] {
        let back = norm_ppf(norm_cdf(x));
        assert!(
            (back - x).abs() <= 1e-6,
            "x {} cdf {} ppf back {}",
            x,
            norm_cdf(x),
            back
        );
    }
}

#[test]
fn qa_specfunc_beta_ppf_closed_form() {
    // Beta(1, m) has cdf 1 - (1 - x)^m, so its 0.95 quantile with
    // m = 20 is 1 - 0.05^(1/20). This is the closed form behind the
    // zero failure conformal bound checked in the ported tests.
    let got = beta_ppf(1.0, 20.0, 0.95);
    let expected = 1.0 - 0.05f64.powf(1.0 / 20.0);
    assert!(
        (got - expected).abs() <= 1e-9,
        "beta_ppf(1, 20, 0.95) {} vs closed form {}",
        got,
        expected
    );
}

// ---- weibull3 fit ----

#[test]
fn qa_weibull3_fit_gamma_below_min_time() {
    let mut rng = Rng64::new(7);
    let times: Vec<f64> = (0..80).map(|_| 100.0 + rng.weibull(2.0, 1000.0)).collect();
    let min_time = times.iter().cloned().fold(f64::INFINITY, f64::min);
    let fit = fit_mle("weibull3", &times, None).expect("weibull3 fit should run");
    let gamma = fit.params["gamma"];
    assert!(
        gamma < min_time,
        "gamma (location) {} not below minimum time {}",
        gamma,
        min_time
    );
}

// ---- degradation: exponential path ----

#[test]
fn qa_degradation_exponential_pseudo_failure_time() {
    // Exact exponential decay y = 100 exp(-0.001 t).
    let t: Vec<f64> = (0..=30).map(|i| i as f64 * 100.0).collect();
    let y: Vec<f64> = t.iter().map(|ti| 100.0 * (-0.001 * ti).exp()).collect();
    let (a, b) = fit_path(&t, &y, "exponential").expect("exponential path fit");
    // Intercept form: y = exp(a + b t) with a = ln(100).
    assert!((a - 100.0f64.ln()).abs() <= 1e-9, "a {}", a);
    assert!((b - (-0.001)).abs() <= 1e-12, "b {}", b);
    let crossing = pseudo_failure_time(a, b, 50.0, "exponential", false);
    let expected = 2.0f64.ln() / 0.001;
    assert!(
        (crossing - expected).abs() / expected <= 0.01,
        "crossing {} vs analytic {}",
        crossing,
        expected
    );
}

// ---- ALT LSQ: Eyring ----

#[test]
fn qa_alt_eyring_lsq_recovers_b_and_orders_reliability() {
    // Noiseless data from a known Eyring mean log life:
    // log life = a + b / T_kelvin - n * ln(V), with sigma 0.
    let (a_true, b_true, n_true) = (-5.0, 5000.0, 1.5);
    let temps_c: [f64; 4] = [50.0, 75.0, 100.0, 125.0];
    let volts: [f64; 3] = [5.0, 10.0, 20.0];
    let mut stress: Vec<Vec<f64>> = Vec::new();
    let mut times: Vec<f64> = Vec::new();
    for tc in temps_c {
        for v in volts {
            let t_kelvin = tc + 273.15;
            let mean_log_life = a_true + b_true / t_kelvin - n_true * v.ln();
            stress.push(vec![tc, v]);
            times.push(mean_log_life.exp());
        }
    }
    let fit = fit_alt("eyring", &stress, &times).expect("eyring LSQ fit");
    let b_fit = fit.coef["b"];
    assert!(
        (b_fit - b_true).abs() / b_true <= 0.01,
        "recovered b {} vs true {}",
        b_fit,
        b_true
    );

    // Reliability ordering across voltage at a fixed temperature:
    // higher voltage must not give higher reliability. The check
    // time is the geometric mean of the two median lives, which
    // lies strictly between them for this noiseless fit.
    let t_fixed = 85.0;
    let med_low_v = fit.median_life(&[t_fixed, 5.0]);
    let med_high_v = fit.median_life(&[t_fixed, 20.0]);
    assert!(med_low_v > med_high_v, "median lives {} vs {}", med_low_v, med_high_v);
    let t_check = (med_low_v * med_high_v).sqrt();
    let r_low_v = fit.reliability(t_check, &[t_fixed, 5.0]);
    let r_high_v = fit.reliability(t_check, &[t_fixed, 20.0]);
    assert!(
        r_low_v > r_high_v,
        "reliability at fixed T: low voltage {} vs high voltage {} at t {}",
        r_low_v,
        r_high_v,
        t_check
    );
}

// ---- DOE: three factors ----

#[test]
fn qa_doe_three_factor_effects() {
    let design = factorial_design(3);
    assert_eq!(design.len(), 8);
    // Model y = 10 + 2 A + 3 B + 0 C + 1.5 A B, evaluated on the
    // design rows. factorial_design uses Yates order with the
    // first factor varying fastest, so column 0 is A, 1 is B,
    // 2 is C.
    let y: Vec<f64> = design
        .iter()
        .map(|row| 10.0 + 2.0 * row[0] + 3.0 * row[1] + 0.0 * row[2] + 1.5 * row[0] * row[1])
        .collect();
    let effects = analyze_factorial(3, &y).expect("factorial analysis");
    let get = |name: &str| {
        effects
            .iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("missing effect {}", name))
            .effect
    };
    // An effect is twice the model coefficient for its term.
    assert!((get("A") - 4.0).abs() <= 1e-9, "A effect {}", get("A"));
    assert!((get("B") - 6.0).abs() <= 1e-9, "B effect {}", get("B"));
    assert!((get("AB") - 3.0).abs() <= 1e-9, "AB effect {}", get("AB"));
    assert!(get("C").abs() <= 1e-9, "C effect {}", get("C"));
}

// ---- prediction: parts stress ----

#[test]
fn qa_prediction_parts_stress_rate_and_mtbf() {
    let mut part = PartEntry::new("device", 2.0, 3);
    part.stress_factor = 1.5;
    part.quality_factor = 0.8;
    // 2.0 * 3 * 1.5 * 0.8 = 7.2 failures per million hours.
    let rate = part_failure_rate_per_million_hours(&part);
    assert!((rate - 7.2).abs() <= 1e-12, "rate {}", rate);
    let mtbf = system_mtbf_hours(&[part]);
    assert!(
        (mtbf - 1e6 / 7.2).abs() <= 1e-6,
        "system mtbf {} vs {}",
        mtbf,
        1e6 / 7.2
    );
}

// ---- fault tree: voting cut sets ----

#[test]
fn qa_faulttree_voting_cut_sets_are_the_three_pairs() {
    let basic = |name: &str| FtNode::Basic {
        name: name.to_string(),
        prob: 0.1,
    };
    let tree = FtNode::Voting {
        name: "top".to_string(),
        k: 2,
        children: vec![basic("a"), basic("b"), basic("c")],
    };
    let cut_sets: BTreeSet<BTreeSet<String>> = minimal_cut_sets(&tree).into_iter().collect();
    let mut expected: BTreeSet<BTreeSet<String>> = BTreeSet::new();
    expected.insert(["a".to_string(), "b".to_string()].into_iter().collect());
    expected.insert(["a".to_string(), "c".to_string()].into_iter().collect());
    expected.insert(["b".to_string(), "c".to_string()].into_iter().collect());
    assert_eq!(cut_sets, expected);
}

// ---- warranty: forecast sanity ----

#[test]
fn qa_warranty_forecast_expected_returns_nonnegative_finite() {
    let sales = vec![100, 100, 100];
    let records = vec![
        WarrantyRecord { sale_period: 0, return_period: 0 },
        WarrantyRecord { sale_period: 0, return_period: 0 },
        WarrantyRecord { sale_period: 0, return_period: 1 },
        WarrantyRecord { sale_period: 0, return_period: 2 },
        WarrantyRecord { sale_period: 1, return_period: 1 },
        WarrantyRecord { sale_period: 1, return_period: 2 },
        WarrantyRecord { sale_period: 1, return_period: 2 },
        WarrantyRecord { sale_period: 2, return_period: 2 },
    ];
    for horizon in [1usize, 2, 3] {
        let forecast = forecast_returns(&sales, &records, 2, horizon)
            .expect("warranty forecast should run");
        assert!(
            forecast.expected_returns.is_finite() && forecast.expected_returns >= 0.0,
            "horizon {} expected returns {}",
            horizon,
            forecast.expected_returns
        );
        assert!(forecast.beta.is_finite() && forecast.beta > 0.0, "beta {}", forecast.beta);
        assert!(forecast.eta.is_finite() && forecast.eta > 0.0, "eta {}", forecast.eta);
        assert!(
            forecast.units_in_service.is_finite() && forecast.units_in_service >= 0.0,
            "units in service {}",
            forecast.units_in_service
        );
    }
}
