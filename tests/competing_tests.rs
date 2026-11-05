//! Tests for competing-risks / failure-mode analysis
//! (reliakit::competing).

use reliakit::competing::{
    cumulative_incidence, failure_probabilities, fit_competing_risks, from_fits,
    mode_hazard, overall_reliability, CompetingRisks,
};
use reliakit::distributions::FitResult;
use reliakit::rng::Rng64;
use std::collections::BTreeMap;

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

fn exponential_fit(theta: f64) -> FitResult {
    let mut params = BTreeMap::new();
    params.insert("theta".to_string(), theta);
    FitResult {
        dist: "exponential".to_string(),
        params,
        loglik: 0.0,
        n: 0,
        n_fail: 0,
        method: "manual".to_string(),
        cov: None,
        se: BTreeMap::new(),
        times: Vec::new(),
        failed: Vec::new(),
        success: true,
    }
}

fn hand_model() -> CompetingRisks {
    from_fits(
        "exponential",
        vec![
            ("A".to_string(), exponential_fit(1000.0)),
            ("B".to_string(), exponential_fit(2000.0)),
        ],
    )
}

#[test]
fn test_hand_exponential_overall_reliability() {
    let cr = hand_model();
    // R(1000) = exp(-1000/1000) * exp(-1000/2000) = exp(-1.5)
    let r = overall_reliability(&cr, 1000.0);
    assert!(
        close(r, (-1.5f64).exp(), 1e-9),
        "overall R(1000) = {}, expected {}",
        r,
        (-1.5f64).exp()
    );
}

#[test]
fn test_hand_exponential_cumulative_incidence() {
    let cr = hand_model();
    // Hazards: h_A = 0.001, h_B = 0.0005, total = 0.0015.
    // CIF_A(1000) = (0.001 / 0.0015) * (1 - exp(-1.5))
    let expected_a = (0.001f64 / 0.0015) * (1.0 - (-1.5f64).exp());
    let cif_a = cumulative_incidence(&cr, "A", 1000.0);
    assert!(
        close(cif_a, expected_a, 1e-6),
        "CIF_A(1000) = {}, expected {}",
        cif_a,
        expected_a
    );

    let cif_b = cumulative_incidence(&cr, "B", 1000.0);
    let expected_b = (0.0005f64 / 0.0015) * (1.0 - (-1.5f64).exp());
    assert!(
        close(cif_b, expected_b, 1e-6),
        "CIF_B(1000) = {}, expected {}",
        cif_b,
        expected_b
    );

    // CIFs plus overall survival exhaust the probability.
    let r = overall_reliability(&cr, 1000.0);
    assert!(
        close(cif_a + cif_b + r, 1.0, 1e-6),
        "CIF_A + CIF_B + R = {}, expected 1",
        cif_a + cif_b + r
    );

    // failure_probabilities returns the same CIFs in modes order.
    let fps = failure_probabilities(&cr, 1000.0);
    assert_eq!(fps.len(), 2);
    assert_eq!(fps[0].0, "A");
    assert_eq!(fps[1].0, "B");
    assert!(close(fps[0].1, cif_a, 1e-12));
    assert!(close(fps[1].1, cif_b, 1e-12));

    // Mode hazard for an exponential is the constant rate, and an
    // absent mode has zero hazard and zero incidence.
    assert!(close(mode_hazard(&cr, "A", 1000.0), 0.001, 1e-12));
    assert!(close(mode_hazard(&cr, "B", 500.0), 0.0005, 1e-12));
    assert_eq!(mode_hazard(&cr, "missing", 1000.0), 0.0);
    assert_eq!(cumulative_incidence(&cr, "missing", 1000.0), 0.0);
}

#[test]
fn test_end_to_end_fit_recovers_competing_exponentials() {
    // Each unit draws TA ~ Exp(1000) and TB ~ Exp(2000); the
    // observed time is the minimum, the mode is the argmin, and a
    // unit still alive at 3000 is censored there.
    let mut rng = Rng64::new(42);
    let n = 500;
    let mut times: Vec<f64> = Vec::with_capacity(n);
    let mut events: Vec<Option<&str>> = Vec::with_capacity(n);
    for _ in 0..n {
        let ta = rng.exponential(1000.0);
        let tb = rng.exponential(2000.0);
        let (t_obs, mode) = if ta <= tb { (ta, "A") } else { (tb, "B") };
        if t_obs > 3000.0 {
            times.push(3000.0);
            events.push(None);
        } else {
            times.push(t_obs);
            events.push(Some(mode));
        }
    }

    let cr = fit_competing_risks(&times, &events, "exponential").unwrap();
    assert_eq!(cr.dist, "exponential");
    let names: Vec<&str> = cr.modes.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, vec!["A", "B"], "modes = {:?}", names);

    let expected_r = (-1.5f64).exp();
    let r = overall_reliability(&cr, 1000.0);
    assert!(
        (r - expected_r).abs() < 0.06,
        "overall R(1000) = {}, expected {} within 0.06",
        r,
        expected_r
    );

    let expected_cif_a = (0.001f64 / 0.0015) * (1.0 - (-1.5f64).exp());
    let cif_a = cumulative_incidence(&cr, "A", 1000.0);
    assert!(
        (cif_a - expected_cif_a).abs() < 0.06,
        "CIF_A(1000) = {}, expected {} within 0.06",
        cif_a,
        expected_cif_a
    );
}

#[test]
fn test_unknown_mode_is_its_own_category() {
    // Small hand-made dataset with masked failures recorded as
    // "unknown", plus ordinary A and B failures and suspensions.
    let times: Vec<f64> = vec![
        120.0, 340.0, 560.0, 210.0, 890.0, 430.0, 150.0, 670.0, 980.0, 260.0, 310.0, 720.0,
        180.0, 540.0, 810.0, 390.0, 240.0, 620.0, 470.0, 910.0,
    ];
    let events: Vec<Option<&str>> = vec![
        Some("A"),
        Some("B"),
        Some("unknown"),
        Some("A"),
        None,
        Some("unknown"),
        Some("B"),
        Some("A"),
        None,
        Some("unknown"),
        Some("A"),
        Some("B"),
        Some("unknown"),
        None,
        Some("A"),
        Some("B"),
        Some("unknown"),
        Some("A"),
        None,
        Some("B"),
    ];

    let cr = fit_competing_risks(&times, &events, "exponential").unwrap();
    let names: Vec<&str> = cr.modes.iter().map(|(name, _)| name.as_str()).collect();
    assert!(
        names.contains(&"unknown"),
        "unknown should appear as a mode, modes = {:?}",
        names
    );

    // CIFs over all modes (including unknown) plus overall
    // reliability must sum to 1.
    let t = 500.0;
    let fps = failure_probabilities(&cr, t);
    assert_eq!(fps.len(), cr.modes.len());
    let sum_cif: f64 = fps.iter().map(|(_, cif)| *cif).sum();
    let r = overall_reliability(&cr, t);
    assert!(
        close(sum_cif + r, 1.0, 1e-6),
        "sum CIF + R = {} at t = {}, expected 1",
        sum_cif + r,
        t
    );
}
