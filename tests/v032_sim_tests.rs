//! Tests for the v0.3.2 availability simulation work (item K):
//! the simulation count is a caller supplied value and the result
//! carries a Monte Carlo standard error for the mean availability.
//!
//! The standard error is the sample standard deviation of the
//! per-simulation grid up-fractions divided by sqrt(n_sim). The
//! mean itself is still computed from the grid up-counts, so its
//! value does not move when the standard error is added: the
//! Example 3 anchor below was recorded before the change as
//! 0.99875 (bits 0x3feff5c28f5c28f6) and is asserted unchanged.

use reliakit::system::{simulate_availability, single_block_availability};
use serde_json::{json, Value};

fn small_series_spec() -> Value {
    json!({"type": "series", "children": [
        {"type": "block", "name": "a", "mtbf": 1000.0, "mttr": 50.0},
        {"type": "block", "name": "b", "mtbf": 2000.0, "mttr": 20.0}
    ]})
}

fn example3_spec() -> Value {
    serde_json::from_str(r#"{"type":"series","children":[{"type":"block","name":"power supply","dist":"weibull2","params":{"beta":1.4,"eta":8000},"mtbf":6000,"mttr":24},{"type":"standby_group","name":"pumps","n_units":2,"dist":"exponential","params":{"theta":5000},"mtbf":5000,"mttr":8,"dormancy":0.0,"switch_reliability":0.98,"swap_time":0.5},{"type":"parallel","children":[{"type":"block","name":"controller A","dist":"exponential","params":{"theta":12000},"mtbf":12000,"mttr":4},{"type":"block","name":"controller B","dist":"exponential","params":{"theta":12000},"mtbf":12000,"mttr":4}]}]}"#).unwrap()
}

#[test]
fn example3_mean_unchanged_after_std_error_change() {
    // Recorded before the std_error field existed, with the engine
    // defaults: seed 3, n_sim 40, horizon 20000, grid 20.
    let res = simulate_availability(&example3_spec(), 20000.0, 40, 3, 20);
    assert!(
        (res.mean_availability - 0.99875).abs() < 1e-12,
        "mean moved: before 0.99875, after {}",
        res.mean_availability
    );
    assert_eq!(res.mean_availability.to_bits(), 0.99875f64.to_bits());
    // One simulation missed one grid point out of 20, the other 39
    // were up at every grid point, which gives a standard error of
    // exactly 0.00125 for this seed and spec.
    assert!(
        (res.std_error - 0.00125).abs() < 1e-12,
        "std_error={}",
        res.std_error
    );
}

#[test]
fn seed_reproducibility_same_mean_and_std_error() {
    let spec = small_series_spec();
    let first = simulate_availability(&spec, 20000.0, 40, 3, 20);
    let second = simulate_availability(&spec, 20000.0, 40, 3, 20);
    assert_eq!(first.mean_availability, second.mean_availability);
    assert_eq!(first.std_error, second.std_error);
}

#[test]
fn std_error_shrinks_with_more_simulations() {
    let spec = small_series_spec();
    let res40 = simulate_availability(&spec, 20000.0, 40, 3, 20);
    let res160 = simulate_availability(&spec, 20000.0, 160, 3, 20);
    assert!(res40.std_error > 0.0, "SE40={}", res40.std_error);
    assert!(res160.std_error > 0.0, "SE160={}", res160.std_error);
    assert!(
        res160.std_error < res40.std_error * 0.8,
        "SE160={} SE40={}",
        res160.std_error,
        res40.std_error
    );
}

#[test]
fn std_error_finite_and_mean_in_range() {
    let spec = small_series_spec();
    let res = simulate_availability(&spec, 20000.0, 40, 3, 20);
    assert!(res.std_error.is_finite(), "SE={}", res.std_error);
    assert!(res.std_error >= 0.0);
    assert!(
        (0.0..=1.0).contains(&res.mean_availability),
        "mean={}",
        res.mean_availability
    );
    // A single simulation has no sample spread to estimate.
    let single = simulate_availability(&spec, 20000.0, 1, 3, 20);
    assert_eq!(single.std_error, 0.0);
    assert!(single.mean_availability.is_finite());
}

#[test]
fn single_block_mean_within_three_std_errors_of_analytic() {
    // One repairable block: steady state availability is
    // MTTF / (MTTF + MTTR). The simulation starts up at t = 0, so
    // the first grid point is biased up by (1 - A) / n_grid, about
    // 0.0005 here. That bias is small next to the standard error
    // at this horizon, so the 3 SE anchor is valid for this spec.
    let spec = json!({"type": "block", "name": "pump", "mtbf": 1000.0, "mttr": 10.0});
    let res = simulate_availability(&spec, 20000.0, 40, 3, 20);
    let analytic = single_block_availability(1000.0, 10.0);
    assert!(res.std_error > 0.0, "SE={}", res.std_error);
    let diff = (res.mean_availability - analytic).abs();
    assert!(
        diff <= 3.0 * res.std_error,
        "mean={} analytic={} diff={} SE={}",
        res.mean_availability,
        analytic,
        diff,
        res.std_error
    );
}
