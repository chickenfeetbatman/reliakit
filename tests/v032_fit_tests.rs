//! Tests for the v0.3.2 fit correctness work: fit status taxonomy,
//! adaptive Weibull bracketing, and the shifted exponential fix.
//!
//! Expected values are stated from independent sources: the
//! clustered Weibull anchor is the SciPy MLE quoted in the external
//! review, the shifted exponential values are hand computed, and the
//! shifted exponential grid check maximises the likelihood
//! independently of the fitting code.

use reliakit::datasets;
use reliakit::distributions::{fit_mle, fit_rank_regression, loglik, FitStatus};
use reliakit::optim::{minimize_scalar_report, ScalarOutcome};

fn rel_diff(a: f64, b: f64) -> f64 {
    (a - b).abs() / b.abs().max(1e-300)
}

// ---- optim reporting ----

#[test]
fn optim_scalar_report_interior_and_bounds() {
    // (x - 0.25)^2 on [-1, 2] has an interior minimum at 0.25.
    let mut f = |x: f64| (x - 0.25) * (x - 0.25);
    let (x, _, outcome) = minimize_scalar_report(&mut f, -1.0, 2.0, 1e-10);
    assert_eq!(outcome, ScalarOutcome::Interior);
    assert!((x - 0.25).abs() < 1e-6, "x={}", x);
    // A strictly decreasing function is minimised at the upper bound.
    let mut g = |x: f64| -x;
    let (x2, _, outcome2) = minimize_scalar_report(&mut g, -1.0, 1.0, 1e-10);
    assert_eq!(outcome2, ScalarOutcome::AtUpperBound);
    assert!((x2 - 1.0).abs() < 1e-6, "x={}", x2);
    // A strictly increasing function is minimised at the lower bound.
    let mut h = |x: f64| x;
    let (x3, _, outcome3) = minimize_scalar_report(&mut h, -1.0, 1.0, 1e-10);
    assert_eq!(outcome3, ScalarOutcome::AtLowerBound);
    assert!((x3 + 1.0).abs() < 1e-6, "x={}", x3);
}

// ---- degenerate and invalid data (item C) ----

#[test]
fn degenerate_all_zero_is_not_a_success() {
    let fit = fit_mle("weibull2", &[0.0, 0.0, 0.0, 0.0], None).unwrap();
    assert!(!fit.success, "all zero data must not report success");
    assert!(
        fit.status == FitStatus::Failed || fit.status == FitStatus::BoundaryOrDegenerate,
        "status={:?}",
        fit.status
    );
    assert_ne!(fit.status_str(), "success");
    assert!(
        fit.status_note.contains("zero"),
        "note should explain the problem: {}",
        fit.status_note
    );
}

#[test]
fn negative_time_is_an_error() {
    let err = fit_mle("weibull2", &[-1.0, 2.0, 3.0, 4.0], None).unwrap_err();
    assert!(err.contains("negative time at index 0"), "err={}", err);
}

#[test]
fn identical_times_are_boundary_or_degenerate() {
    let fit = fit_mle("weibull2", &[1.0, 1.0, 1.0, 1.0], None).unwrap();
    assert!(!fit.success);
    assert_eq!(fit.status, FitStatus::BoundaryOrDegenerate);
    assert_eq!(fit.status_str(), "boundary_or_degenerate");
    assert!(
        fit.status_note.contains("identical"),
        "note={}",
        fit.status_note
    );
    assert!(
        fit.status_note.contains("not identifiable"),
        "note={}",
        fit.status_note
    );
}

#[test]
fn nan_time_is_an_error() {
    let err = fit_mle("weibull2", &[1.0, f64::NAN, 3.0, 4.0], None).unwrap_err();
    assert!(err.contains("non-finite"), "err={}", err);
}

#[test]
fn empty_and_mismatched_inputs_are_errors() {
    let empty: Vec<f64> = Vec::new();
    assert!(fit_mle("weibull2", &empty, None).is_err());
    let times = vec![10.0, 20.0, 30.0];
    let failed = vec![true, false];
    let err = fit_mle("weibull2", &times, Some(&failed)).unwrap_err();
    assert!(err.contains("mismatch"), "err={}", err);
}

#[test]
fn rank_regression_identical_times_are_degenerate() {
    let fit = fit_rank_regression("weibull2", &[1.0, 1.0, 1.0, 1.0], None).unwrap();
    assert!(!fit.success);
    assert_eq!(fit.status, FitStatus::BoundaryOrDegenerate);
}

// ---- Weibull bracketing (item D) ----

#[test]
fn clustered_weibull_matches_scipy_anchor() {
    let times = vec![100.0, 101.0, 102.0, 103.0, 104.0];
    let fit = fit_mle("weibull2", &times, None).unwrap();
    println!(
        "clustered weibull: beta={} eta={} status={:?}",
        fit.params["beta"], fit.params["eta"], fit.status
    );
    assert_eq!(fit.status, FitStatus::Success, "note={}", fit.status_note);
    assert!(fit.success);
    assert!(
        rel_diff(fit.params["beta"], 80.808341) < 0.05,
        "beta={} vs SciPy 80.808341",
        fit.params["beta"]
    );
    assert!(
        rel_diff(fit.params["eta"], 102.694992) < 0.005,
        "eta={} vs SciPy 102.694992",
        fit.params["eta"]
    );
}

#[test]
fn weibull_fit_is_scale_invariant() {
    let times = vec![100.0, 101.0, 102.0, 103.0, 104.0];
    let scaled: Vec<f64> = times.iter().map(|t| t * 3600.0).collect();
    let a = fit_mle("weibull2", &times, None).unwrap();
    let b = fit_mle("weibull2", &scaled, None).unwrap();
    assert!(
        rel_diff(b.params["beta"], a.params["beta"]) < 1e-6,
        "beta {} vs {}",
        b.params["beta"],
        a.params["beta"]
    );
    assert!(
        rel_diff(b.params["eta"], a.params["eta"] * 3600.0) < 1e-6,
        "eta {} vs {}",
        b.params["eta"],
        a.params["eta"] * 3600.0
    );
}

#[test]
fn weibull_shape_below_old_lower_bracket() {
    // Deterministic complete sample from Weibull beta 0.05, eta 1000
    // through the inverse CDF at Benard plotting positions.
    let n = 100usize;
    let (beta_true, eta_true) = (0.05f64, 1000.0f64);
    let times: Vec<f64> = (1..=n)
        .map(|i| {
            let p = (i as f64 - 0.3) / (n as f64 + 0.4);
            eta_true * (-(1.0 - p).ln()).powf(1.0 / beta_true)
        })
        .collect();
    let fit = fit_mle("weibull2", &times, None).unwrap();
    println!("low shape weibull: beta={}", fit.params["beta"]);
    assert!(
        fit.params["beta"] < 0.082,
        "beta={} should be below 0.082",
        fit.params["beta"]
    );
    assert!(
        fit.params["beta"] > beta_true / 2.0 && fit.params["beta"] < beta_true * 2.0,
        "beta={} should be within a factor of 2 of {}",
        fit.params["beta"],
        beta_true
    );
}

// ---- shifted exponential (item G) ----

#[test]
fn exponential2_suspension_below_first_failure() {
    // Suspension at 1, failures at 10 and 20. The smallest failure
    // time is 10, so gamma is 10 and S = 0 + 10 with r = 2 gives
    // theta = 5. The old rule used the overall minimum, gamma 1 and
    // theta 14.
    let times = vec![1.0, 10.0, 20.0];
    let failed = vec![false, true, true];
    let fit = fit_mle("exponential2", &times, Some(&failed)).unwrap();
    assert!(
        (fit.params["gamma"] - 10.0).abs() < 1e-12,
        "gamma={}",
        fit.params["gamma"]
    );
    assert!(
        (fit.params["theta"] - 5.0).abs() < 1e-12,
        "theta={}",
        fit.params["theta"]
    );
    assert!(fit.success);
    assert!(
        fit.warnings
            .iter()
            .any(|w| w.contains("boundary") && w.contains("smallest failure")),
        "warnings={:?}",
        fit.warnings
    );
    // The corrected fit has a higher log-likelihood than the old
    // gamma 1, theta 14 point, both through the stored value and
    // through a direct loglik evaluation.
    let old_ll = loglik("exponential2", &[14.0, 1.0], &times, &failed);
    let new_ll = loglik("exponential2", &[5.0, 10.0], &times, &failed);
    assert!(
        new_ll > old_ll,
        "new loglik {} should exceed old {}",
        new_ll,
        old_ll
    );
    assert!((fit.loglik - new_ll).abs() < 1e-9);
    assert!(fit.loglik > old_ll);
    // Independent grid maximisation over gamma below the smallest
    // failure time, with theta profiled out at each gamma.
    let r = 2.0f64;
    let mut grid_best = f64::NEG_INFINITY;
    let mut g = 0.0f64;
    while g < 10.0 {
        let s: f64 = times.iter().filter(|t| **t > g).map(|t| t - g).sum();
        let theta = s / r;
        let ll = loglik("exponential2", &[theta, g], &times, &failed);
        if ll > grid_best {
            grid_best = ll;
        }
        g += 0.001;
    }
    assert!(
        grid_best <= fit.loglik + 1e-9,
        "grid best {} should not beat the fit {}",
        grid_best,
        fit.loglik
    );
    assert!(
        fit.loglik - grid_best < 0.01,
        "grid best {} should agree with the fit {}",
        grid_best,
        fit.loglik
    );
}

#[test]
fn weibull3_loglik_allows_gamma_above_a_suspension() {
    // A suspension at 1 must not bound gamma: with failures at 10
    // and 20, gamma 5 is a valid support start and the likelihood
    // is finite there.
    let times = vec![1.0, 10.0, 20.0];
    let failed = vec![false, true, true];
    let ll = loglik("weibull3", &[2.0, 100.0, 5.0], &times, &failed);
    assert!(ll.is_finite() && ll > -1e99, "loglik={}", ll);
}

// ---- ordinary fit stays a clean success ----

#[test]
fn ordinary_bearing_fit_still_succeeds() {
    let (times, failed) = datasets::bearing_life(2026);
    let fit = fit_mle("weibull2", &times, Some(&failed)).unwrap();
    assert_eq!(fit.status, FitStatus::Success, "note={}", fit.status_note);
    assert!(fit.success);
    assert_eq!(fit.status_str(), "success");
    assert!(fit.params.values().all(|v| v.is_finite()));
    assert!(fit.loglik.is_finite());
    assert!(fit.se.values().all(|v| v.is_finite()));
}
