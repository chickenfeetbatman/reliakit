//! Tests for censored MLE ALT fitting and the Eyring interaction
//! LSQ model. All data are synthetic with a fixed seed.

use reliakit::alt::{fit_alt, fit_alt_mle, K_B_EV};
use reliakit::rng::Rng64;

#[test]
fn censored_arrhenius_mle_recovers_ea_and_sigma() {
    // True model: Ea = 0.7 eV, median life 20000 h at 55 C, sigma 0.45.
    let ea_true = 0.7;
    let sigma_true = 0.45;
    let b_true = ea_true / K_B_EV;
    let t_use_k = 55.0 + 273.15;
    let a_true = (20000.0f64).ln() - b_true / t_use_k;

    let mut rng = Rng64::new(20261104);
    let mut stress: Vec<Vec<f64>> = Vec::new();
    let mut times: Vec<f64> = Vec::new();
    let mut failed: Vec<bool> = Vec::new();
    for &t_c in &[85.0f64, 105.0, 125.0] {
        let t_k = t_c + 273.15;
        let mu = a_true + b_true / t_k;
        let median = mu.exp();
        let censor = 1.2 * median;
        for _ in 0..20 {
            let life = rng.lognormal(mu, sigma_true);
            if life <= censor {
                times.push(life);
                failed.push(true);
            } else {
                times.push(censor);
                failed.push(false);
            }
            stress.push(vec![t_c]);
        }
    }
    let n_susp = failed.iter().filter(|f| !**f).count();
    assert!(
        (n_susp as f64) >= 0.10 * (failed.len() as f64),
        "expected at least 10 percent suspended, got {} of {}",
        n_susp,
        failed.len()
    );

    let fit = fit_alt_mle("arrhenius", &stress, &times, Some(&failed)).unwrap();
    let ea = fit.activation_energy_ev().unwrap();
    assert!(
        (ea - ea_true).abs() <= 0.30,
        "Ea recovered {} vs true {}",
        ea,
        ea_true
    );
    assert!(
        (fit.sigma - sigma_true).abs() <= 0.2,
        "sigma recovered {} vs true {}",
        fit.sigma,
        sigma_true
    );
    assert_eq!(fit.n, 60);
    assert_eq!(fit.n_fail, 60 - n_susp);
    assert!(fit.loglik.is_finite());
}

#[test]
fn ipl_mle_recovers_exponent() {
    // True model: log life = a - n ln V, n = 2.0, sigma = 0.3.
    // a is set so the median life at 100 V is 5000 h.
    let n_true = 2.0;
    let sigma_true = 0.3;
    let a_true = (5000.0f64).ln() + n_true * (100.0f64).ln();

    let mut rng = Rng64::new(987654321);
    let mut stress: Vec<Vec<f64>> = Vec::new();
    let mut times: Vec<f64> = Vec::new();
    let mut failed: Vec<bool> = Vec::new();
    for &v in &[50.0f64, 100.0, 200.0] {
        let mu = a_true - n_true * v.ln();
        let median = mu.exp();
        let censor = 2.5 * median;
        for _ in 0..20 {
            let life = rng.lognormal(mu, sigma_true);
            if life <= censor {
                times.push(life);
                failed.push(true);
            } else {
                times.push(censor);
                failed.push(false);
            }
            stress.push(vec![v]);
        }
    }

    let fit = fit_alt_mle("ipl", &stress, &times, Some(&failed)).unwrap();
    let n_hat = fit.coef["n"];
    assert!(
        (n_hat - n_true).abs() <= 0.4,
        "n recovered {} vs true {}",
        n_hat,
        n_true
    );
    assert!(fit.loglik.is_finite());
    // Median life falls as voltage rises.
    assert!(fit.median_life(&[50.0]) > fit.median_life(&[200.0]));
}

#[test]
fn eyring_interaction_lsq_recovers_coefficients() {
    // Convention check: the generated log life is
    //   30 + 8000 / T_k - 1.5 * ln V + 3000 * (1 / T_k) * (-ln V)
    // which matches the design row [1, 1 / T_k, -ln V,
    // (1 / T_k) * (-ln V)] with coef keys a, b, n, ab, since the
    // mean log life is a + b / T_k + n * (-ln V)
    // + ab * (1 / T_k) * (-ln V) = a + b / T_k - n ln V
    // + ab * (1 / T_k) * (-ln V).
    let mut rng = Rng64::new(424242);
    let mut stress: Vec<Vec<f64>> = Vec::new();
    let mut times: Vec<f64> = Vec::new();
    for &t_c in &[60.0f64, 100.0] {
        let t_k = t_c + 273.15;
        for &v in &[10.0f64, 30.0] {
            let mu = 30.0 + 8000.0 / t_k - 1.5 * v.ln() + 3000.0 * (1.0 / t_k) * (-v.ln());
            for _ in 0..8 {
                let log_life = mu + 0.03 * rng.normal();
                times.push(log_life.exp());
                stress.push(vec![t_c, v]);
            }
        }
    }
    let fit = fit_alt("eyring_interaction", &stress, &times).unwrap();
    let ab = fit.coef["ab"];
    assert!(
        (ab - 3000.0).abs() <= 0.25 * 3000.0,
        "ab recovered {} vs true 3000",
        ab
    );
    let n_hat = fit.coef["n"];
    assert!(
        (n_hat - 1.5).abs() <= 0.5,
        "n recovered {} vs true 1.5",
        n_hat
    );
    // Mean log life at a cell should be near the generated value.
    let t_k = 60.0 + 273.15;
    let mu_true = 30.0 + 8000.0 / t_k - 1.5 * (10.0f64).ln() + 3000.0 * (1.0 / t_k) * (-(10.0f64).ln());
    assert!((fit.mean_log_life(&[60.0, 10.0]) - mu_true).abs() < 0.2);
}

#[test]
fn eyring_interaction_mle_runs_and_bounds_are_ordered() {
    // Same style of generating equation as the LSQ test, with a
    // larger scatter and censoring at 1.5 times the cell median.
    let mut rng = Rng64::new(13579);
    let mut stress: Vec<Vec<f64>> = Vec::new();
    let mut times: Vec<f64> = Vec::new();
    let mut failed: Vec<bool> = Vec::new();
    for &t_c in &[60.0f64, 100.0] {
        let t_k = t_c + 273.15;
        for &v in &[10.0f64, 30.0] {
            let mu = 30.0 + 8000.0 / t_k - 1.5 * v.ln() + 3000.0 * (1.0 / t_k) * (-v.ln());
            let censor = 1.5 * mu.exp();
            for _ in 0..10 {
                let life = (mu + 0.2 * rng.normal()).exp();
                if life <= censor {
                    times.push(life);
                    failed.push(true);
                } else {
                    times.push(censor);
                    failed.push(false);
                }
                stress.push(vec![t_c, v]);
            }
        }
    }

    let fit = fit_alt_mle("eyring_interaction", &stress, &times, Some(&failed)).unwrap();
    assert!(fit.loglik.is_finite(), "loglik {}", fit.loglik);
    assert!(fit.sigma > 0.0 && fit.sigma.is_finite());

    // Reliability decreases as temperature increases at fixed V.
    // At V = 10 the net temperature coefficient b + ab * (-ln V)
    // is positive (8000 - 3000 * ln 10 > 0), so higher temperature
    // means shorter life under the generating model. (At larger V
    // the interaction term can reverse the direction, which is why
    // this check is done at V = 10.)
    let t_eval = fit.median_life(&[80.0, 10.0]);
    let r_low_t = fit.reliability(t_eval, &[60.0, 10.0]);
    let r_high_t = fit.reliability(t_eval, &[100.0, 10.0]);
    assert!(
        r_high_t < r_low_t,
        "reliability at 100 C {} not below reliability at 60 C {}",
        r_high_t,
        r_low_t
    );

    let r = fit.reliability(t_eval, &[80.0, 10.0]);
    let (lo, hi) = fit.reliability_bounds(t_eval, &[80.0, 10.0], 0.05);
    assert!(
        0.0 <= lo && lo <= r && r <= hi && hi <= 1.0,
        "bounds not ordered: lo {} R {} hi {}",
        lo,
        r,
        hi
    );
}
