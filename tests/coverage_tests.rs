//! Coordinator-added coverage tests closing the gaps the QA audit
//! found in FEATURE_TEST_LEDGER.md: lognormal and normal
//! distributions, the degradation power path, and the inverse power
//! law least-squares ALT fit.

use reliakit::alt::fit_alt;
use reliakit::degradation::pseudo_failure_time;
use reliakit::distributions::{cdf, fit_mle, quantile, reliability, Params};
use reliakit::rng::Rng64;

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn lognormal_known_values_and_mle() {
    let mut p = Params::new();
    p.insert("mu".into(), 6.0);
    p.insert("sigma".into(), 0.5);
    // median = exp(mu): cdf(exp(mu)) = 0.5, R = 0.5
    assert!(close(cdf("lognormal", &p, 6.0f64.exp()), 0.5, 1e-12));
    assert!(close(reliability("lognormal", &p, 6.0f64.exp()), 0.5, 1e-12));
    // quantile roundtrip
    let q = quantile("lognormal", &p, 0.9);
    assert!(close(cdf("lognormal", &p, q), 0.9, 1e-9));
    // MLE recovery on seeded data
    let mut rng = Rng64::new(17);
    let times: Vec<f64> = (0..300).map(|_| rng.lognormal(6.0, 0.5)).collect();
    let fit = fit_mle("lognormal", &times, None).unwrap();
    assert!((fit.params["mu"] - 6.0).abs() < 0.08, "mu {}", fit.params["mu"]);
    assert!(
        (fit.params["sigma"] - 0.5).abs() < 0.06,
        "sigma {}",
        fit.params["sigma"]
    );
}

#[test]
fn normal_known_values_and_mle() {
    let mut p = Params::new();
    p.insert("mu".into(), 100.0);
    p.insert("sigma".into(), 15.0);
    assert!(close(cdf("normal", &p, 100.0), 0.5, 1e-12));
    // 90th percentile of the standard normal is 1.2815515655446004
    let q = quantile("normal", &p, 0.9);
    assert!(close(q, 100.0 + 15.0 * 1.2815515655446004, 1e-6), "q {}", q);
    let mut rng = Rng64::new(23);
    let times: Vec<f64> = (0..300)
        .map(|_| 100.0 + 15.0 * rng.normal())
        .collect();
    let fit = fit_mle("normal", &times, None).unwrap();
    assert!((fit.params["mu"] - 100.0).abs() < 3.0, "mu {}", fit.params["mu"]);
    assert!(
        (fit.params["sigma"] - 15.0).abs() < 2.0,
        "sigma {}",
        fit.params["sigma"]
    );
}

#[test]
fn degradation_power_path_crossing() {
    // power path y = exp(a) * t^b with exp(a) = 2, b = 0.5:
    // crossing y = 8 at t = (8/2)^(1/0.5) = 16 exactly
    let tc = pseudo_failure_time(2.0f64.ln(), 0.5, 8.0, "power", true);
    assert!(close(tc, 16.0, 1e-9), "tc {}", tc);
    // decreasing power path never crosses upward threshold direction
    let tc2 = pseudo_failure_time(2.0f64.ln(), -0.5, 8.0, "power", true);
    assert!(tc2.is_infinite());
}

#[test]
fn alt_ipl_lsq_fit_recovers_exponent() {
    // noiseless data: log life = 12 - 2 ln V, three voltages, reps
    let mut stress = Vec::new();
    let mut times = Vec::new();
    for v in [50.0f64, 100.0, 200.0] {
        for _ in 0..4 {
            let loglife = 12.0 - 2.0 * v.ln();
            times.push(loglife.exp());
            stress.push(vec![v]);
        }
    }
    let fit = fit_alt("ipl", &stress, &times).unwrap();
    assert!(
        (fit.coef["n"] - 2.0).abs() < 1e-6,
        "n {}",
        fit.coef["n"]
    );
    assert!(fit.sigma < 1e-6, "sigma {}", fit.sigma);
    // median life falls as voltage rises: exp(12 - 2 ln V)
    assert!(fit.median_life(&[50.0]) > fit.median_life(&[200.0]));
    assert!(
        (fit.median_life(&[100.0]) - (12.0f64 - 2.0 * 100.0f64.ln()).exp()).abs() < 1e-3,
        "median {}",
        fit.median_life(&[100.0])
    );
}
