//! Synthetic example datasets. All data here is SYNTHETIC, generated
//! with fixed seeds, and labelled as such. No real field data is
//! included. Generators mirror the Python ReliaKit datasets (same
//! distributions and parameters; the Rust RNG stream differs from
//! numpy, so draws are not identical to the Python CSVs).

use crate::rng::Rng64;

/// Synthetic bearing life test: 24 units, Weibull(1.8, 4200 h), test
/// truncated at 6000 h. Returns (times, failed). SYNTHETIC.
pub fn bearing_life(seed: u64) -> (Vec<f64>, Vec<bool>) {
    let mut rng = Rng64::new(seed);
    let mut times = Vec::new();
    let mut failed = Vec::new();
    for _ in 0..24 {
        let t = rng.weibull(1.8, 4200.0);
        failed.push(t <= 6000.0);
        times.push(t.min(6000.0));
    }
    (times, failed)
}

/// Synthetic limited-data example: 4 production lots, n = 8 each, lot
/// scale variation (true log eta ~ N(log 3000, 0.25^2), beta 1.6),
/// censoring at 4500 h. SYNTHETIC.
pub fn small_sample_groups(seed: u64) -> Vec<(Vec<f64>, Vec<bool>)> {
    let mut rng = Rng64::new(seed);
    let mut groups = Vec::new();
    for _ in 0..4 {
        let eta = (3000.0f64.ln() + 0.25 * rng.normal()).exp();
        let mut times = Vec::new();
        let mut failed = Vec::new();
        for _ in 0..8 {
            let t = rng.weibull(1.6, eta);
            failed.push(t <= 4500.0);
            times.push(t.min(4500.0));
        }
        groups.push((times, failed));
    }
    groups
}

/// Synthetic LED lumen degradation: 12 units, linear paths toward a 70
/// percent output threshold. Returns (times, output_pct) per unit.
/// SYNTHETIC.
pub fn degradation_units(seed: u64) -> Vec<(Vec<f64>, Vec<f64>)> {
    let mut rng = Rng64::new(seed);
    let t: Vec<f64> = (0..7).map(|i| i as f64 * 500.0).collect();
    let mut units = Vec::new();
    for _ in 0..12 {
        let slope = -0.0085 + 0.0015 * rng.normal();
        let y: Vec<f64> = t
            .iter()
            .map(|ti| 100.0 + slope * ti + 0.6 * rng.normal())
            .collect();
        units.push((t.clone(), y));
    }
    units
}

/// Synthetic ALT: electronic module at 85, 105, 125 C, lognormal life
/// with Ea = 0.7 eV and median life 20000 h at 55 C. Returns
/// (stress_rows, times) with one [T_C] row per unit. SYNTHETIC.
pub fn alt_dataset(seed: u64) -> (Vec<Vec<f64>>, Vec<f64>) {
    use crate::alt::K_B_EV;
    let mut rng = Rng64::new(seed);
    let ea = 0.7;
    let mut stresses = Vec::new();
    let mut times = Vec::new();
    for t_c in [85.0, 105.0, 125.0] {
        let tu = 55.0 + 273.15;
        let ts = t_c + 273.15;
        let af = (ea / K_B_EV * (1.0 / tu - 1.0 / ts)).exp();
        let med = 20000.0 / af;
        for _ in 0..10 {
            times.push(rng.lognormal(med.ln(), 0.45));
            stresses.push(vec![t_c]);
        }
    }
    (stresses, times)
}

/// Synthetic reliability growth failure times (Crow-AMSAA, beta 0.6 in
/// truth), test length 5000 h, about 40 events. SYNTHETIC.
pub fn growth_dataset(seed: u64) -> Vec<f64> {
    let mut rng = Rng64::new(seed);
    let (beta, t_end): (f64, f64) = (0.6, 5000.0);
    let lam = 40.0 / t_end.powf(beta);
    let mut u: Vec<f64> = (0..40).map(|_| rng.next_f64()).collect();
    u.sort_by(|a, b| a.partial_cmp(b).unwrap());
    u.iter()
        .map(|ui| (ui * lam * t_end.powf(beta) / lam).powf(1.0 / beta))
        .filter(|t| *t <= t_end)
        .collect()
}
