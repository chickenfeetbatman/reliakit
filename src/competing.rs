//! Competing-risks / failure-mode analysis.
//!
//! Each unit can fail from one of several distinct failure modes, and
//! the modes compete: the first mode to occur is the one observed,
//! and the other modes are pre-empted for that unit. A separate
//! life distribution (all modes share the same distribution family,
//! selected by the caller) is fitted for each mode, treating
//! failures from that mode as events and every other unit - units
//! that failed from a different mode and units that were suspended
//! (censored) - as censored at their observed times.
//!
//! Independence assumption: the analysis assumes the competing
//! failure modes are statistically independent. Under that
//! assumption the overall (all-mode) reliability is the product of
//! the per-mode reliabilities, and the cumulative incidence function
//! for a mode can be computed from the per-mode hazards and the
//! overall survival function. If the modes are dependent, the
//! per-mode fits still describe the observed cause-specific hazards,
//! but the product form for overall reliability and the incidence
//! integrals should not be interpreted as what would happen if a
//! mode were removed.
//!
//! Masked data handling: in real field data the failure mode is
//! sometimes not identified for a failed unit. In this module a
//! unit whose mode is recorded as "unknown" (a masked failure,
//! mode not identified) is treated as its own mode category named
//! "unknown". It is fitted like any other mode, contributes its own
//! hazard to the overall reliability product, and gets its own
//! cumulative incidence curve. Callers who prefer a different
//! treatment should recode or drop masked units before fitting.

use crate::distributions::{fit_mle, FitResult};
use std::collections::BTreeSet;

/// Fitted competing-risks model: one life-distribution fit per
/// failure mode, all using the distribution family `dist`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompetingRisks {
    pub dist: String,
    pub modes: Vec<(String, FitResult)>,
}

/// Fit a competing-risks model.
///
/// `times[i]` is the observed time for unit `i` and `events[i]`
/// describes what happened at that time: `None` means the unit was
/// suspended (censored) at `times[i]`, and `Some(mode)` means the
/// unit failed from failure mode `mode`. A mode string of
/// "unknown" is treated as its own mode category (see the module
/// documentation for the masked-data handling).
///
/// For each distinct mode, `dist` is fitted with that mode's
/// failures as events and all other units (other modes plus
/// suspended units) as censored at their times. Modes are returned
/// in sorted (alphabetical) order for determinism.
///
/// Returns an error if `times` and `events` have different lengths,
/// if there are no observations, if there are no failures at all,
/// or if any per-mode fit fails.
pub fn fit_competing_risks(
    times: &[f64],
    events: &[Option<&str>],
    dist: &str,
) -> Result<CompetingRisks, String> {
    if times.len() != events.len() {
        return Err(format!(
            "times and events length mismatch: {} vs {}",
            times.len(),
            events.len()
        ));
    }
    if times.is_empty() {
        return Err("need at least one observation to fit competing risks".into());
    }
    if times.iter().any(|t| !t.is_finite()) {
        return Err("times must be finite".into());
    }
    let distinct: BTreeSet<String> = events
        .iter()
        .filter_map(|e| e.map(|s| s.to_string()))
        .collect();
    if distinct.is_empty() {
        return Err("need at least one failure to fit competing risks".into());
    }
    let mut modes: Vec<(String, FitResult)> = Vec::with_capacity(distinct.len());
    for mode in &distinct {
        let failed: Vec<bool> = events
            .iter()
            .map(|e| matches!(e, Some(m) if *m == mode.as_str()))
            .collect();
        let fit = fit_mle(dist, times, Some(&failed))
            .map_err(|e| format!("fit for mode {} failed: {}", mode, e))?;
        modes.push((mode.clone(), fit));
    }
    Ok(CompetingRisks {
        dist: dist.to_string(),
        modes,
    })
}

/// Build a competing-risks model directly from existing per-mode
/// fits. No fitting is performed; `modes` is stored in the order
/// given and `dist` is recorded as the shared distribution family.
pub fn from_fits(dist: &str, modes: Vec<(String, FitResult)>) -> CompetingRisks {
    CompetingRisks {
        dist: dist.to_string(),
        modes,
    }
}

fn find_fit<'a>(cr: &'a CompetingRisks, mode: &str) -> Option<&'a FitResult> {
    cr.modes
        .iter()
        .find(|(name, _)| name.as_str() == mode)
        .map(|(_, fit)| fit)
}

/// Overall reliability at time `t` under independent competing
/// risks: the product over modes of the per-mode reliabilities.
/// Returns 1.0 when there are no modes.
pub fn overall_reliability(cr: &CompetingRisks, t: f64) -> f64 {
    let mut r = 1.0f64;
    for (_, fit) in &cr.modes {
        r *= fit.reliability(t);
    }
    if r.is_finite() {
        r.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Hazard rate of a single failure mode at time `t`, defined as
/// pdf_mode(t) / R_mode(t) using that mode's own fitted
/// distribution. Returns 0.0 if the mode is absent, or if the
/// hazard cannot be evaluated as a finite non-negative number.
pub fn mode_hazard(cr: &CompetingRisks, mode: &str, t: f64) -> f64 {
    let fit = match find_fit(cr, mode) {
        Some(f) => f,
        None => return 0.0,
    };
    let r = fit.reliability(t);
    if !r.is_finite() || r <= 0.0 {
        return 0.0;
    }
    let f = fit.pdf(t);
    if !f.is_finite() || f <= 0.0 {
        return 0.0;
    }
    let h = f / r;
    if h.is_finite() && h >= 0.0 {
        h
    } else {
        0.0
    }
}

/// Cumulative incidence function for a mode at time `t`:
/// CIF_k(t) = integral from 0 to t of S(u) * h_k(u) du, where S is
/// the overall survival function (see `overall_reliability`) and
/// h_k is the hazard of mode `k` (see `mode_hazard`). The integral
/// is evaluated with the composite Simpson rule using 2000
/// intervals (at least 1000, as required for accuracy). Returns
/// 0.0 if the mode is absent or if `t` is not positive.
pub fn cumulative_incidence(cr: &CompetingRisks, mode: &str, t: f64) -> f64 {
    if !t.is_finite() || t <= 0.0 {
        return 0.0;
    }
    if find_fit(cr, mode).is_none() {
        return 0.0;
    }
    // Composite Simpson rule; the number of intervals must be even
    // and at least 1000.
    const N: usize = 2000;
    let step = t / N as f64;
    let integrand = |u: f64| -> f64 {
        let v = overall_reliability(cr, u) * mode_hazard(cr, mode, u);
        if v.is_finite() && v >= 0.0 {
            v
        } else {
            0.0
        }
    };
    let mut sum = integrand(0.0) + integrand(t);
    for i in 1..N {
        let u = step * i as f64;
        let coeff = if i % 2 == 1 { 4.0 } else { 2.0 };
        sum += coeff * integrand(u);
    }
    let value = sum * step / 3.0;
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Cumulative incidence (failure probability by time `t`) for
/// every mode, returned as (mode name, CIF) pairs in the order
/// the modes appear in `cr.modes`.
pub fn failure_probabilities(cr: &CompetingRisks, t: f64) -> Vec<(String, f64)> {
    cr.modes
        .iter()
        .map(|(name, _)| (name.clone(), cumulative_incidence(cr, name, t)))
        .collect()
}
