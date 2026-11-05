//! Tests for the recursive quadrature standby module.
//!
//! Reference values come from the closed form for exponential units,
//! from the two unit quadrature already in the system module, and
//! from a fixed seed Monte Carlo run.

use reliakit::distributions::{params_of, reliability, Params};
use reliakit::standby::{standby_closed_form_exp, standby_reliability_quad};
use reliakit::system;

fn exp_params() -> Params {
    params_of(&[("theta", 1000.0)])
}

fn weibull_params() -> Params {
    params_of(&[("beta", 1.5), ("eta", 1000.0)])
}

#[test]
fn exponential_cold_standby_matches_closed_forms() {
    let params = exp_params();
    let lam = 1.0 / 1000.0;
    for n in [2usize, 3, 4] {
        for sw in [1.0, 0.95] {
            let quad = standby_reliability_quad(1000.0, "exponential", &params, n, 0.0, sw);
            let closed = standby_closed_form_exp(1000.0, lam, n, sw);
            let system_closed = system::cold_standby_exponential(1000.0, lam, n, sw);
            assert!(
                (quad - closed).abs() < 2e-3,
                "n={} sw={}: quad {} vs closed form {}",
                n,
                sw,
                quad,
                closed
            );
            assert!(
                (quad - system_closed).abs() < 2e-3,
                "n={} sw={}: quad {} vs system cold standby {}",
                n,
                sw,
                quad,
                system_closed
            );
            assert!(
                (closed - system_closed).abs() < 1e-12,
                "n={} sw={}: closed form {} vs system {}",
                n,
                sw,
                closed,
                system_closed
            );
        }
    }
}

#[test]
fn closed_form_two_units_is_poisson_sum() {
    let lam = 1.0 / 1000.0;
    let got = standby_closed_form_exp(1000.0, lam, 2, 0.95);
    let want = (-1.0f64).exp() * (1.0 + 0.95);
    assert!((got - want).abs() < 1e-12, "got {}, want {}", got, want);
}

#[test]
fn two_unit_weibull_matches_system_quadrature() {
    let params = weibull_params();
    for dormancy in [0.0, 0.3] {
        let quad = standby_reliability_quad(500.0, "weibull2", &params, 2, dormancy, 1.0);
        let reference = system::standby_reliability(500.0, "weibull2", &params, 2, dormancy, 1.0);
        assert!(
            (quad - reference).abs() < 3e-3,
            "dormancy={}: quad {} vs system {}",
            dormancy,
            quad,
            reference
        );
    }
}

#[test]
fn three_unit_weibull_matches_monte_carlo() {
    let params = weibull_params();
    let quad = standby_reliability_quad(500.0, "weibull2", &params, 3, 0.2, 1.0);
    let mc = system::standby_reliability_mc(500.0, "weibull2", &params, 3, 0.2, 1.0, 60000, 7);
    assert!(
        (quad - mc).abs() < 0.025,
        "quad {} vs Monte Carlo {}",
        quad,
        mc
    );
}

#[test]
fn ordering_cold_warm_hot_single() {
    let params = weibull_params();
    let single = reliability("weibull2", &params, 500.0);
    let cold = standby_reliability_quad(500.0, "weibull2", &params, 3, 0.0, 1.0);
    let warm = standby_reliability_quad(500.0, "weibull2", &params, 3, 0.3, 1.0);
    let hot = standby_reliability_quad(500.0, "weibull2", &params, 3, 1.0, 1.0);
    assert!(cold >= warm, "cold {} < warm {}", cold, warm);
    assert!(warm >= hot, "warm {} < hot {}", warm, hot);
    assert!(hot > single, "hot {} <= single {}", hot, single);
}

#[test]
fn guards_single_unit_and_hot_standby() {
    let params = weibull_params();
    let single = reliability("weibull2", &params, 500.0);
    let one = standby_reliability_quad(500.0, "weibull2", &params, 1, 0.0, 1.0);
    assert!((one - single).abs() < 1e-12);
    let hot = standby_reliability_quad(500.0, "weibull2", &params, 3, 1.0, 1.0);
    let parallel = 1.0 - (1.0 - single).powi(3);
    assert!((hot - parallel).abs() < 1e-12);
}
