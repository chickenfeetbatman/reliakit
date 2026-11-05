//! Benchmark workloads mirroring the Python ReliaKit benchmark script
//! (same workloads and parameters; the RNG streams differ, the
//! distributions and sizes are the same).

use reliakit::bayes::conformal_reliability_lower;
use reliakit::distributions::fit_mle;
use reliakit::mcmc::fit_bayes;
use reliakit::rng::Rng64;
use reliakit::system::simulate_availability;
use serde_json::json;
use std::time::Instant;

fn main() {
    // 1) Monte Carlo availability: single pump, horizon 20000, 300 runs
    let spec = json!({"type": "block", "name": "pump", "mtbf": 1000.0, "mttr": 10.0});
    let t0 = Instant::now();
    let res = simulate_availability(&spec, 20000.0, 300, 3, 50);
    println!(
        "availability_mc  mean_A={:.6}  time={:.3}s",
        res.mean_availability,
        t0.elapsed().as_secs_f64()
    );

    // 2) MCMC: 60 Weibull draws, 4 chains x (2000 warmup + 2000 draws)
    let mut rng = Rng64::new(21);
    let times: Vec<f64> = (0..60).map(|_| rng.weibull(1.5, 1000.0)).collect();
    let t0 = Instant::now();
    let fit = fit_bayes("weibull2", &times, None, 2000, 2000, 4, 99).unwrap();
    let elapsed = t0.elapsed().as_secs_f64();
    let summ = fit.summary();
    println!(
        "mcmc  beta_mean={:.4}  time={:.3}s",
        summ[0].1.mean, elapsed
    );

    // 3) Conformal mini-study: 200 replicates, fit + conformal bound each
    let mut rng = Rng64::new(7);
    let t0 = Instant::now();
    let mut cover = 0usize;
    let true_r = (-(500.0f64 / 1000.0).powf(1.5)).exp();
    for _ in 0..200 {
        let tr: Vec<f64> = (0..40).map(|_| rng.weibull(1.5, 1000.0)).collect();
        let _fit = fit_mle("weibull2", &tr, None).unwrap();
        let calib: Vec<f64> = (0..40).map(|_| rng.weibull(1.5, 1000.0)).collect();
        let rl = conformal_reliability_lower(&calib, 500.0, 0.10);
        if rl <= true_r {
            cover += 1;
        }
    }
    println!(
        "conformal_study  coverage={}/200  time={:.3}s",
        cover,
        t0.elapsed().as_secs_f64()
    );
}
