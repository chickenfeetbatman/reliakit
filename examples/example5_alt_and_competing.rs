//! Example 5: censored ALT maximum-likelihood fit and competing risks.
//!
//! Part A: the synthetic ALT dataset (85/105/125 C) with an imposed
//! censoring time, fitted by true censored maximum likelihood
//! (Arrhenius model), with the use-level reliability at 55 C.
//! Part B: a competing-risks fit on synthetic two-mode data
//! (bearing wear vs seal leak), with cause-specific cumulative
//! incidence at a mission time.
//!
//! Run from the repository root:
//!   cargo run --release --example example5_alt_and_competing

use reliakit::alt::fit_alt_mle;
use reliakit::competing::{cumulative_incidence, fit_competing_risks, overall_reliability};
use reliakit::rng::Rng64;

fn main() {
    // Part A: censored ALT MLE. Synthetic data generated here with
    // the same recipe as reliakit::datasets::alt_dataset but n = 40
    // per stress (Ea 0.7 eV, median life 20000 h at 55 C, sigma 0.45).
    let mut rng_a = Rng64::new(2026);
    let mut stress: Vec<Vec<f64>> = Vec::new();
    let mut times: Vec<f64> = Vec::new();
    for t_c in [85.0, 105.0, 125.0] {
        let af = reliakit::alt::acceleration_factor_arrhenius(0.7, 55.0, t_c);
        let med = 20000.0 / af;
        for _ in 0..40 {
            times.push(rng_a.lognormal(med.ln(), 0.45));
            stress.push(vec![t_c]);
        }
    }
    let censor = 3000.0;
    let mut ct = Vec::new();
    let mut cf = Vec::new();
    for &t in &times {
        cf.push(t <= censor);
        ct.push(t.min(censor));
    }
    let n_fail = cf.iter().filter(|f| **f).count();
    let fit = fit_alt_mle("arrhenius", &stress, &ct, Some(&cf)).unwrap();
    println!(
        "Example 5A: censored Arrhenius MLE ({} of {} units failed before censoring at {:.0} h)",
        n_fail,
        ct.len(),
        censor
    );
    println!(
        "  Ea = {:.3} eV (data generated with 0.7 eV), sigma = {:.3}, loglik = {:.2}",
        fit.activation_energy_ev().unwrap_or(f64::NAN),
        fit.sigma,
        fit.loglik
    );
    let (lo, hi) = fit.reliability_bounds(20000.0, &[55.0], 0.10);
    println!(
        "  R(20000 h at 55 C) = {:.3}, 90% bounds [{:.3}, {:.3}], median life at 55 C = {:.0} h (data generated with 20000 h)",
        fit.reliability(20000.0, &[55.0]),
        lo,
        hi,
        fit.median_life(&[55.0])
    );

    // Part B: competing risks, synthetic two-mode data
    let mut rng = Rng64::new(11);
    let mut times = Vec::new();
    let mut events: Vec<Option<&str>> = Vec::new();
    for _ in 0..300 {
        let wear = rng.weibull(1.6, 4000.0);
        let seal = rng.weibull(1.2, 6500.0);
        let (t, mode) = if wear < seal {
            (wear, "wear")
        } else {
            (seal, "seal")
        };
        if t <= 5000.0 {
            times.push(t);
            events.push(Some(mode));
        } else {
            times.push(5000.0);
            events.push(None);
        }
    }
    let cr = fit_competing_risks(&times, &events, "weibull2").unwrap();
    println!("Example 5B: competing risks (weibull2 per mode)");
    for (mode, f) in &cr.modes {
        println!(
            "  mode {}: beta = {:.3}, eta = {:.1} h, failures = {}",
            mode, f.params["beta"], f.params["eta"], f.n_fail
        );
    }
    let t = 2000.0;
    println!(
        "  overall R({:.0} h) = {:.3}; P(fail by wear) = {:.3}, P(fail by seal) = {:.3}",
        t,
        overall_reliability(&cr, t),
        cumulative_incidence(&cr, "wear", t),
        cumulative_incidence(&cr, "seal", t)
    );
}
