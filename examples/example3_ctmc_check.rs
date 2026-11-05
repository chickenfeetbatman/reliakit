//! Example 3 (companion check): CTMC steady-state availability of the
//! cold standby pump group from examples/example3_rbd.json, beside
//! the Monte Carlo availability of the same group, plus the full
//! diagram evaluated through the library model the desktop app uses.
//!
//! Run from the repository root:
//!   cargo run --release --example example3_ctmc_check

use reliakit::system::{ctmc_steady_state, evaluate_rbd, simulate_availability, standby_ctmc};
use serde_json::json;

fn main() {
    // Pumps group alone: 2 units, exponential theta 5000 (lam 0.0002),
    // MTTR 8 h (mu 0.125), cold standby, one repair crew.
    let (q, down) = standby_ctmc(0.0002, 0.125, 2, 0.0, 1);
    let pi = ctmc_steady_state(&q);
    let a_ctmc: f64 = pi
        .iter()
        .enumerate()
        .filter(|(i, _)| !down.contains(i))
        .map(|(_, p)| p)
        .sum();
    let group = json!({"type": "standby_group", "name": "pumps", "n_units": 2,
                       "dist": "exponential", "params": {"theta": 5000.0},
                       "mtbf": 5000.0, "mttr": 8.0, "dormancy": 0.0,
                       "switch_reliability": 1.0, "swap_time": 0.0});
    let mc = simulate_availability(&group, 20000.0, 2000, 8, 50);
    println!(
        "Pump standby group availability: CTMC steady state = {:.7}",
        a_ctmc
    );
    println!(
        "Pump standby group availability: Monte Carlo (2000 runs, horizon 20000 h) = {:.7}, expected downtime {:.3} h per run",
        mc.mean_availability, mc.expected_downtime
    );
    println!("(The CTMC check idealises the group: perfect switching, no swap delay. The diagram itself uses switch reliability 0.98 and a 0.5 h swap time.)");

    // Full Example 3 diagram through the library (same JSON the CLI
    // and the desktop app evaluate).
    let text = std::fs::read_to_string("examples/example3_rbd.json").expect("read example3 json");
    let spec: serde_json::Value = serde_json::from_str(&text).unwrap();
    let r = evaluate_rbd(&spec, Some(1000.0)).unwrap();
    let av = simulate_availability(&spec, 20000.0, 200, 3, 50);
    println!("Full diagram reliability at t = 1000 h: {:.6}", r);
    println!(
        "Full diagram Monte Carlo availability (200 runs, horizon 20000 h): {:.5}, expected downtime {:.1} h",
        av.mean_availability, av.expected_downtime
    );
}
