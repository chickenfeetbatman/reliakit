//! Tests for the v0.2.0 added distributions: loglogistic, logistic,
//! gompertz, gengamma, weibull1, exponential2 and mixed_weibull.
//!
//! Closed-form values are computed independently in each test from
//! the defining formula (not by calling the library function under
//! test a second way). Recovery tests use fixed seeds only.

use reliakit::distributions::*;
use reliakit::rng::Rng64;
use reliakit::specfunc;

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

fn p_of(pairs: &[(&str, f64)]) -> Params {
    params_of(pairs)
}

/// Uniform on (0, 1) clamped away from the endpoints, for inversion
/// sampling through the library quantile functions.
fn u_open(rng: &mut Rng64) -> f64 {
    rng.next_f64().clamp(1e-9, 1.0 - 1e-9)
}

// ---- loglogistic ----

#[test]
fn loglogistic_closed_form() {
    let p = p_of(&[("alpha", 1000.0), ("beta", 2.0)]);
    // R(t) = 1 / (1 + (t / alpha)^beta): at t = 500, (0.5)^2 = 0.25,
    // so R = 1 / 1.25 = 0.8 exactly.
    assert!(close(reliability("loglogistic", &p, 500.0), 0.8, 1e-12));
    // R(alpha) = 1 / 2.
    assert!(close(reliability("loglogistic", &p, 1000.0), 0.5, 1e-12));
    // Median is alpha; the 0.8 quantile is alpha * (0.8 / 0.2)^(1/2)
    // = 1000 * 2 = 2000.
    assert!(close(quantile("loglogistic", &p, 0.5), 1000.0, 1e-9));
    assert!(close(quantile("loglogistic", &p, 0.8), 2000.0, 1e-9));
    // pdf at t = alpha: (beta / alpha) * 1^(beta-1) / (1 + 1)^2
    // = (2 / 1000) / 4 = 0.0005.
    assert!(close(pdf("loglogistic", &p, 1000.0), 0.0005, 1e-12));
    // Mean at beta = 2: alpha * (pi / 2) / sin(pi / 2) = 500 pi.
    assert!(close(
        mean_life("loglogistic", &p),
        500.0 * std::f64::consts::PI,
        1e-9
    ));
}

#[test]
fn loglogistic_mle_recovery() {
    let (alpha_t, beta_t) = (1200.0, 2.2);
    let p = p_of(&[("alpha", alpha_t), ("beta", beta_t)]);
    let mut rng = Rng64::new(42);
    let times: Vec<f64> = (0..300)
        .map(|_| quantile("loglogistic", &p, u_open(&mut rng)))
        .collect();
    let fit = fit_mle("loglogistic", &times, None).unwrap();
    println!(
        "loglogistic recovery: alpha={} beta={}",
        fit.params["alpha"], fit.params["beta"]
    );
    assert!(
        (fit.params["alpha"] - alpha_t).abs() < 0.10 * alpha_t,
        "alpha={}",
        fit.params["alpha"]
    );
    assert!(
        (fit.params["beta"] - beta_t).abs() < 0.15 * beta_t,
        "beta={}",
        fit.params["beta"]
    );
}

#[test]
fn loglogistic_rank_regression_recovery() {
    let (alpha_t, beta_t) = (1000.0, 2.0);
    let p = p_of(&[("alpha", alpha_t), ("beta", beta_t)]);
    let mut rng = Rng64::new(9);
    let times: Vec<f64> = (0..200)
        .map(|_| quantile("loglogistic", &p, u_open(&mut rng)))
        .collect();
    let fit = fit_rank_regression("loglogistic", &times, None).unwrap();
    println!(
        "loglogistic RR: alpha={} beta={}",
        fit.params["alpha"], fit.params["beta"]
    );
    assert!(
        (fit.params["alpha"] - alpha_t).abs() < 0.25 * alpha_t,
        "alpha={}",
        fit.params["alpha"]
    );
    assert!(
        (fit.params["beta"] - beta_t).abs() < 0.30 * beta_t,
        "beta={}",
        fit.params["beta"]
    );
}

// ---- logistic ----

#[test]
fn logistic_closed_form() {
    let p = p_of(&[("mu", 500.0), ("s", 100.0)]);
    // F(mu) = 1 / 2.
    assert!(close(cdf("logistic", &p, 500.0), 0.5, 1e-12));
    // At t = mu + s ln 3: F = 1 / (1 + exp(-ln 3)) = 0.75, R = 0.25.
    let t = 500.0 + 100.0 * 3.0f64.ln();
    assert!(close(reliability("logistic", &p, t), 0.25, 1e-12));
    assert!(close(quantile("logistic", &p, 0.75), t, 1e-9));
    // pdf at mu is 1 / (4 s) = 0.0025.
    assert!(close(pdf("logistic", &p, 500.0), 0.0025, 1e-12));
    // Mean is mu.
    assert!(close(mean_life("logistic", &p), 500.0, 1e-12));
}

#[test]
fn logistic_mle_recovery() {
    let (mu_t, s_t) = (1000.0, 180.0);
    let p = p_of(&[("mu", mu_t), ("s", s_t)]);
    let mut rng = Rng64::new(7);
    let mut times = Vec::new();
    while times.len() < 300 {
        let t = quantile("logistic", &p, u_open(&mut rng));
        if t > 0.0 {
            times.push(t);
        }
    }
    let fit = fit_mle("logistic", &times, None).unwrap();
    println!(
        "logistic recovery: mu={} s={}",
        fit.params["mu"], fit.params["s"]
    );
    assert!((fit.params["mu"] - mu_t).abs() < 60.0, "mu={}", fit.params["mu"]);
    assert!(
        (fit.params["s"] - s_t).abs() < 0.15 * s_t,
        "s={}",
        fit.params["s"]
    );
}

#[test]
fn logistic_rank_regression_not_provided() {
    let times = vec![100.0, 200.0, 300.0, 400.0, 500.0];
    assert!(fit_rank_regression("logistic", &times, None).is_err());
}

// ---- gompertz ----

#[test]
fn gompertz_closed_form() {
    let (a, b) = (0.002, 0.001);
    let p = p_of(&[("a", a), ("b", b)]);
    let t = 300.0;
    // R(t) = exp(-(a / b) (exp(b t) - 1)) from the defining formula.
    let r_expected = (-(a / b) * ((b * t).exp() - 1.0)).exp();
    assert!(close(reliability("gompertz", &p, t), r_expected, 1e-12));
    // Hazard h(t) = a exp(b t).
    let h_expected = a * (b * t).exp();
    assert!(close(hazard("gompertz", &p, t) / h_expected, 1.0, 1e-9));
    // Quantile at prob: (1 / b) ln(1 - (b / a) ln(1 - prob)).
    let prob: f64 = 0.3;
    let q_expected = (1.0 / b) * (1.0 - (b / a) * (1.0 - prob).ln()).ln();
    assert!(close(quantile("gompertz", &p, prob), q_expected, 1e-9));
    // pdf at 0 is the hazard at 0 times R(0) = 1: f(0) = a.
    assert!(close(pdf("gompertz", &p, 0.0), a, 1e-12));
}

#[test]
fn gompertz_pdf_integrates_against_reliability() {
    // Numerical integration of the pdf (Simpson) from 0 to T must
    // match the reliability drop 1 - R(T) from the closed form.
    let (a, b) = (0.002, 0.001);
    let p = p_of(&[("a", a), ("b", b)]);
    let big_t = 800.0;
    let n = 20000usize;
    let h = big_t / n as f64;
    let mut sum = pdf("gompertz", &p, 0.0) + pdf("gompertz", &p, big_t);
    for i in 1..n {
        let w = if i % 2 == 1 { 4.0 } else { 2.0 };
        sum += w * pdf("gompertz", &p, i as f64 * h);
    }
    let integral = sum * h / 3.0;
    let drop = 1.0 - reliability("gompertz", &p, big_t);
    assert!(
        close(integral, drop, 1e-6),
        "integral={} drop={}",
        integral,
        drop
    );
}

#[test]
fn gompertz_mle_recovery() {
    let (a_t, b_t) = (0.0008, 0.0012);
    let p = p_of(&[("a", a_t), ("b", b_t)]);
    let mut rng = Rng64::new(11);
    let times: Vec<f64> = (0..400)
        .map(|_| quantile("gompertz", &p, u_open(&mut rng)))
        .collect();
    let fit = fit_mle("gompertz", &times, None).unwrap();
    println!(
        "gompertz recovery: a={} b={}",
        fit.params["a"], fit.params["b"]
    );
    assert!(
        (fit.params["a"] - a_t).abs() < 0.25 * a_t,
        "a={}",
        fit.params["a"]
    );
    assert!(
        (fit.params["b"] - b_t).abs() < 0.20 * b_t,
        "b={}",
        fit.params["b"]
    );
}

// ---- gengamma ----

#[test]
fn gengamma_closed_form() {
    // a = 1000, d = 2, p = 2: CDF is P(d / p, (t / a)^p) = P(1, x)
    // = 1 - exp(-x); at t = a, x = 1.
    let p = p_of(&[("a", 1000.0), ("d", 2.0), ("p", 2.0)]);
    assert!(close(
        cdf("gengamma", &p, 1000.0),
        1.0 - (-1.0f64).exp(),
        1e-12
    ));
    // pdf at t = a from the defining formula with Gamma(1) = 1:
    // f = (p / a^d) t^(d-1) exp(-(t / a)^p) = (2 / 1000^2) * 1000 * e^-1.
    let f_expected = (2.0 / 1000.0f64.powi(2)) * 1000.0 * (-1.0f64).exp();
    assert!(close(pdf("gengamma", &p, 1000.0), f_expected, 1e-12));
    // Mean: a Gamma((d + 1) / p) / Gamma(d / p) = 1000 Gamma(1.5)
    // = 1000 sqrt(pi) / 2.
    assert!(close(
        mean_life("gengamma", &p),
        1000.0 * std::f64::consts::PI.sqrt() / 2.0,
        1e-9
    ));
    // d = p = 1 is the exponential with mean a: median a ln 2.
    let pe = p_of(&[("a", 500.0), ("d", 1.0), ("p", 1.0)]);
    assert!(close(
        quantile("gengamma", &pe, 0.5),
        500.0 * 2.0f64.ln(),
        1e-6
    ));
}

#[test]
fn gengamma_reduces_to_gamma_at_p1() {
    let g = p_of(&[("a", 700.0), ("d", 3.5), ("p", 1.0)]);
    let gam = p_of(&[("alpha", 3.5), ("beta", 700.0)]);
    for t in [200.0, 700.0, 1500.0, 3000.0] {
        assert!(
            close(cdf("gengamma", &g, t), cdf("gamma", &gam, t), 1e-12),
            "cdf mismatch at t={}",
            t
        );
        let (fg, fa) = (pdf("gengamma", &g, t), pdf("gamma", &gam, t));
        assert!(close(fg, fa, 1e-10), "pdf mismatch at t={}: {} vs {}", t, fg, fa);
    }
}

#[test]
fn gengamma_reduces_to_weibull_at_d_eq_p() {
    let g = p_of(&[("a", 900.0), ("d", 2.3), ("p", 2.3)]);
    let w = p_of(&[("beta", 2.3), ("eta", 900.0)]);
    for t in [300.0, 900.0, 1400.0, 2500.0] {
        assert!(
            close(cdf("gengamma", &g, t), cdf("weibull2", &w, t), 1e-12),
            "cdf mismatch at t={}",
            t
        );
        let (fg, fa) = (pdf("gengamma", &g, t), pdf("weibull2", &w, t));
        assert!(close(fg, fa, 1e-10), "pdf mismatch at t={}: {} vs {}", t, fg, fa);
    }
}

#[test]
fn gengamma_mle_recovery() {
    let (a_t, d_t, p_t) = (1000.0, 3.0, 1.5);
    let p = p_of(&[("a", a_t), ("d", d_t), ("p", p_t)]);
    let mut rng = Rng64::new(13);
    let times: Vec<f64> = (0..1000)
        .map(|_| quantile("gengamma", &p, u_open(&mut rng)))
        .collect();
    let fit = fit_mle("gengamma", &times, None).unwrap();
    println!(
        "gengamma recovery: a={} d={} p={}",
        fit.params["a"], fit.params["d"], fit.params["p"]
    );
    // Tolerances: the three gengamma parameters are strongly
    // correlated, which inflates the scale standard error to roughly
    // 10 percent even at n = 1000 (two seeds gave +11 percent at
    // n = 500 and -10 percent here), so all three use 15 percent.
    assert!(
        (fit.params["a"] - a_t).abs() < 0.15 * a_t,
        "a={}",
        fit.params["a"]
    );
    assert!(
        (fit.params["d"] - d_t).abs() < 0.15 * d_t,
        "d={}",
        fit.params["d"]
    );
    assert!(
        (fit.params["p"] - p_t).abs() < 0.15 * p_t,
        "p={}",
        fit.params["p"]
    );
}

// ---- weibull1 ----

#[test]
fn weibull1_closed_form() {
    let p = p_of(&[("beta", 1.7), ("eta", 800.0)]);
    // R(eta) = exp(-1).
    assert!(close(reliability("weibull1", &p, 800.0), (-1.0f64).exp(), 1e-12));
    // B10 = eta (-ln 0.9)^(1 / beta).
    let b10 = 800.0 * (-(0.9f64).ln()).powf(1.0 / 1.7);
    assert!(close(quantile("weibull1", &p, 0.10), b10, 1e-9));
}

#[test]
fn weibull1_eta_closed_form() {
    // All failed, beta = 2: eta = ((100^2 + 200^2 + 300^2 + 400^2) / 4)^(1/2)
    // = (300000 / 4)^(1/2) = sqrt(75000).
    let times = vec![100.0, 200.0, 300.0, 400.0];
    let fit = fit_weibull1(&times, None, 2.0).unwrap();
    assert!(close(fit.params["eta"], 75000.0f64.sqrt(), 1e-9));
    assert!(close(fit.params["beta"], 2.0, 0.0));
    // One suspension: the sum still runs over all units but r = 4
    // failures out of 5: eta = (550000 / 4)^(1/2) = sqrt(137500).
    let times2 = vec![100.0, 200.0, 300.0, 400.0, 500.0];
    let failed2 = vec![true, true, true, false, true];
    let fit2 = fit_weibull1(&times2, Some(&failed2), 2.0).unwrap();
    assert!(close(fit2.params["eta"], 137500.0f64.sqrt(), 1e-9));
    assert_eq!(fit2.n_fail, 4);
}

#[test]
fn weibull1_fit_mle_requires_fixed_beta() {
    let times = vec![100.0, 200.0, 300.0];
    let err = fit_mle("weibull1", &times, None).unwrap_err();
    assert!(err.contains("fit_weibull1"), "err={}", err);
}

// ---- exponential2 ----

#[test]
fn exponential2_closed_form() {
    let p = p_of(&[("theta", 250.0), ("gamma", 100.0)]);
    // R(gamma) = 1; R(gamma + theta) = exp(-1); F below gamma is 0.
    assert!(close(reliability("exponential2", &p, 100.0), 1.0, 1e-12));
    assert!(close(reliability("exponential2", &p, 350.0), (-1.0f64).exp(), 1e-12));
    assert!(close(cdf("exponential2", &p, 50.0), 0.0, 0.0));
    assert!(close(pdf("exponential2", &p, 50.0), 0.0, 0.0));
    // Median = gamma + theta ln 2; mean = gamma + theta.
    assert!(close(
        quantile("exponential2", &p, 0.5),
        100.0 + 250.0 * 2.0f64.ln(),
        1e-9
    ));
    assert!(close(mean_life("exponential2", &p), 350.0, 1e-12));
}

#[test]
fn exponential2_mle_closed_form() {
    // All failed: gamma_hat = min = 50,
    // theta_hat = ((0 + 30 + 70 + 150)) / 4 = 62.5.
    let times = vec![50.0, 80.0, 120.0, 200.0];
    let fit = fit_mle("exponential2", &times, None).unwrap();
    assert!(close(fit.params["gamma"], 50.0, 1e-12));
    assert!(close(fit.params["theta"], 62.5, 1e-12));
    // With the 120 observation suspended, r = 3 and the sum is over
    // all units: theta_hat = 250 / 3.
    let failed = vec![true, true, false, true];
    let fit2 = fit_mle("exponential2", &times, Some(&failed)).unwrap();
    assert!(close(fit2.params["gamma"], 50.0, 1e-12));
    assert!(close(fit2.params["theta"], 250.0 / 3.0, 1e-12));
}

// ---- mixed_weibull ----

#[test]
fn mixed_weibull_closed_form() {
    let p = p_of(&[
        ("w", 0.3),
        ("beta1", 1.5),
        ("eta1", 400.0),
        ("beta2", 2.0),
        ("eta2", 2000.0),
    ]);
    let t = 600.0;
    // R(t) = w exp(-(t / eta1)^beta1) + (1 - w) exp(-(t / eta2)^beta2).
    let r_expected = 0.3 * (-(t / 400.0f64).powf(1.5)).exp()
        + 0.7 * (-(t / 2000.0f64).powf(2.0)).exp();
    assert!(close(reliability("mixed_weibull", &p, t), r_expected, 1e-12));
    assert!(close(cdf("mixed_weibull", &p, t), 1.0 - r_expected, 1e-12));
    // Mean is the weighted sum of the component means.
    let m_expected = 0.3 * 400.0 * specfunc::ln_gamma(1.0 + 1.0 / 1.5).exp()
        + 0.7 * 2000.0 * specfunc::ln_gamma(1.0 + 1.0 / 2.0).exp();
    assert!(close(mean_life("mixed_weibull", &p), m_expected, 1e-9));
    // The bisected quantile inverts the CDF.
    let q = quantile("mixed_weibull", &p, 0.4);
    assert!(close(cdf("mixed_weibull", &p, q), 0.4, 1e-9));
}

#[test]
fn mixed_weibull_mle_recovery() {
    let (w_t, b1_t, e1_t, b2_t, e2_t) = (0.4, 1.2, 500.0, 2.5, 3000.0);
    let mut rng = Rng64::new(17);
    let mut times = Vec::new();
    for _ in 0..1000 {
        if rng.next_f64() < w_t {
            times.push(rng.weibull(b1_t, e1_t));
        } else {
            times.push(rng.weibull(b2_t, e2_t));
        }
    }
    let fit = fit_mle("mixed_weibull", &times, None).unwrap();
    // The fit orders components by eta, so component 1 is the
    // eta = 500 subpopulation (handles label switching).
    println!(
        "mixed recovery: w={} beta1={} eta1={} beta2={} eta2={}",
        fit.params["w"],
        fit.params["beta1"],
        fit.params["eta1"],
        fit.params["beta2"],
        fit.params["eta2"]
    );
    assert!(fit.params["eta1"] < fit.params["eta2"]);
    assert!(
        (fit.params["w"] - w_t).abs() < 0.08,
        "w={}",
        fit.params["w"]
    );
    assert!(
        (fit.params["eta1"] - e1_t).abs() < 0.15 * e1_t,
        "eta1={}",
        fit.params["eta1"]
    );
    assert!(
        (fit.params["eta2"] - e2_t).abs() < 0.15 * e2_t,
        "eta2={}",
        fit.params["eta2"]
    );
    assert!(
        (fit.params["beta1"] - b1_t).abs() < 0.35 * b1_t,
        "beta1={}",
        fit.params["beta1"]
    );
    assert!(
        (fit.params["beta2"] - b2_t).abs() < 0.35 * b2_t,
        "beta2={}",
        fit.params["beta2"]
    );
}
