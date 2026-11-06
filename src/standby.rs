//! Exact standby reliability for a group of iid units by recursive
//! numerical convolution over virtual age.
//!
//! Let `R` and `f` be the reliability and the pdf of a single unit.
//! Define `G_k(tau, a)` as the reliability, over a further interval of
//! length `tau`, of a group in which one unit is active and `k - 1`
//! identical units wait in standby, given that every unit in the
//! group has already accumulated virtual age `a`. Virtual age grows
//! at full rate for the active unit and at the dormancy rate `d`,
//! with `0 <= d <= 1`, for each waiting unit. A waiting unit that has
//! waited while the active unit ran for time `u` therefore has
//! virtual age `a + d * u` when it is switched in.
//!
//! For a single unit the answer is conditional survival from its
//! virtual age:
//!
//! ```text
//! G_1(tau, a) = R(a + tau) / R(a)
//! ```
//!
//! For `k >= 2` the group survives the interval either because the
//! active unit survives it, which is the `G_1` term, or because the
//! active unit fails at some time `u` inside the interval and the
//! remaining `k - 1` units then survive the rest. Conditioning on
//! the failure time of the active unit gives the recursion
//!
//! ```text
//! G_k(tau, a) = G_1(tau, a)
//!     + integral_0^tau [ f(a + u) / R(a) ] * sw
//!                      * G_{k-1}(tau - u, a + d * u) du
//! ```
//!
//! where `sw` is the switch reliability, charged once per switch.
//! The factor `f(a + u) / R(a)` is the conditional pdf of the active
//! unit failing at `a + u` given survival to `a`, and the new virtual
//! age `a + d * u` inside `G_{k-1}` is exactly the virtual age the
//! waiting units reached while the failed unit ran. The reliability
//! of a group of `n` units over a mission of length `t` starting
//! fresh is `G_n(t, 0)`.
//!
//! Each integral is evaluated with composite Simpson quadrature.
//! Node counts grow with the recursion level of the integral being
//! evaluated, so groups of up to four units stay fast while the
//! nested quadrature keeps errors near 1e-3 for typical life
//! distributions. Two limiting cases bypass the recursion: a single
//! unit returns its own reliability, and dormancy at or above 1 is
//! hot standby, where every unit ages at full rate and the group is
//! a parallel system with reliability `1 - (1 - R)^n`.

use crate::distributions::{self, Params};

/// Number of Simpson intervals used at recursion level `k`.
///
/// The top level gets the finest grid. Deeper levels integrate over
/// shorter remaining intervals, so a coarser grid there costs little
/// accuracy and keeps the nested cost close to linear in the product
/// of the level counts rather than the top count raised to `n - 1`.
fn intervals_for_level(k: usize) -> usize {
    match k {
        2 => 64,
        3 => 112,
        4 => 160,
        // Larger groups are outside the design target. A modest grid
        // keeps the nested recursion from exploding in cost.
        _ => 48,
    }
}

/// `G_k(tau, a)` as defined in the module documentation.
fn g(
    k: usize,
    tau: f64,
    a: f64,
    dist: &str,
    params: &Params,
    dormancy: f64,
    switch_reliability: f64,
) -> f64 {
    if tau <= 0.0 {
        return 1.0;
    }
    let r_a = distributions::reliability(dist, params, a);
    if r_a <= 1e-300 {
        return 0.0;
    }
    let g1 = (distributions::reliability(dist, params, a + tau) / r_a).clamp(0.0, 1.0);
    if k <= 1 {
        return g1;
    }
    let n = intervals_for_level(k);
    let h = tau / n as f64;
    let mut sum = 0.0;
    for i in 0..=n {
        let u = i as f64 * h;
        let weight = if i == 0 || i == n {
            1.0
        } else if i % 2 == 1 {
            4.0
        } else {
            2.0
        };
        let cond_pdf = distributions::pdf(dist, params, a + u) / r_a;
        if cond_pdf == 0.0 {
            continue;
        }
        let rest = g(
            k - 1,
            tau - u,
            a + dormancy * u,
            dist,
            params,
            dormancy,
            switch_reliability,
        );
        sum += weight * cond_pdf * switch_reliability * rest;
    }
    let integral = sum * h / 3.0;
    (g1 + integral).clamp(0.0, 1.0)
}

/// Exact standby reliability of `n_units` iid units at time `t` by
/// recursive numerical convolution (see the module documentation).
///
/// `dormancy` is the rate at which standby units accumulate virtual
/// age while waiting: 0 is cold standby and 1 is hot standby, which
/// reduces to the parallel formula. `switch_reliability` is charged
/// once per switch to a fresh unit.
pub fn standby_reliability_quad(
    t: f64,
    dist: &str,
    params: &Params,
    n_units: usize,
    dormancy: f64,
    switch_reliability: f64,
) -> f64 {
    if n_units == 0 {
        return 0.0;
    }
    if t <= 0.0 {
        return 1.0;
    }
    if n_units <= 1 {
        return distributions::reliability(dist, params, t).clamp(0.0, 1.0);
    }
    if dormancy >= 1.0 {
        let r1 = distributions::reliability(dist, params, t);
        return (1.0 - (1.0 - r1).powi(n_units as i32)).clamp(0.0, 1.0);
    }
    let d = dormancy.max(0.0);
    g(n_units, t, 0.0, dist, params, d, switch_reliability).clamp(0.0, 1.0)
}

/// Cold standby reliability for identical exponential units, in
/// closed form.
///
/// With rate `lam`, mission time `t` and switch reliability `sw`,
/// the number of failures by `t` is Poisson with mean `lam * t`,
/// each switch succeeds with probability `sw`, and the group of
/// `n_units` survives when at most `n_units - 1` failures occur:
///
/// ```text
/// R = exp(-lam * t) * sum_{i=0}^{n-1} (sw * lam * t)^i / i!
/// ```
///
/// For two units this reduces to `exp(-lam * t) * (1 + sw * lam * t)`.
pub fn standby_closed_form_exp(t: f64, lam: f64, n_units: usize, switch_reliability: f64) -> f64 {
    if n_units == 0 {
        return 0.0;
    }
    if t <= 0.0 {
        return 1.0;
    }
    let lt = lam * t;
    let x = switch_reliability * lt;
    let mut total = 0.0;
    let mut term = 1.0;
    for i in 0..n_units {
        if i > 0 {
            term *= x / i as f64;
        }
        total += term;
    }
    ((-lt).exp() * total).clamp(0.0, 1.0)
}
