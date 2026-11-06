//! Chart series as pure data.
//!
//! These are the series computations from the Slint build's
//! plotters charts (gui/src/charts.rs), ported without any drawing:
//! each function returns named point series that a host renders
//! however it likes (canvas, SVG, or a chart library). A series
//! `kind` is "line" for a connected curve, "points" for markers,
//! and "line-points" where the old chart drew both.

use crate::work::{fnum, parse_numbers, ItemKind, Work};
use reliakit::degradation;
use reliakit::distributions as dist_mod;
use reliakit::growth;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Series {
    pub name: String,
    pub kind: String,
    pub points: Vec<(f64, f64)>,
}

fn line(name: impl Into<String>, points: Vec<(f64, f64)>) -> Series {
    Series {
        name: name.into(),
        kind: "line".to_string(),
        points,
    }
}
fn points(name: impl Into<String>, pts: Vec<(f64, f64)>) -> Series {
    Series {
        name: name.into(),
        kind: "points".to_string(),
        points: pts,
    }
}

/// Life Data chart: the fitted reliability curve, or the
/// probability plot when `prob` is set (empirical F against fitted
/// F, with the y = x reference line the old chart drew dashed).
pub fn life_series(fit: &dist_mod::FitResult, prob: bool) -> Vec<Series> {
    if prob {
        let (_times, f_emp, f_theo) = dist_mod::probability_plot_data(fit);
        let pts: Vec<(f64, f64)> = f_emp.iter().zip(&f_theo).map(|(a, b)| (*a, *b)).collect();
        return vec![
            line("y = x", vec![(0.0, 0.0), (1.0, 1.0)]),
            points("Failures", pts),
        ];
    }
    let tmax = fit.times.iter().cloned().fold(1.0f64, f64::max) * 1.15;
    let pts: Vec<(f64, f64)> = (0..=120)
        .map(|i| {
            let t = tmax * i as f64 / 120.0;
            (t, fit.reliability(t))
        })
        .collect();
    vec![line(dist_mod::dist_label(&fit.dist).to_string(), pts)]
}

/// Degradation chart data: the failure threshold line, each unit's
/// raw measurements as points, and each unit's fitted path line.
pub fn degradation_series(
    units: &[(Vec<f64>, Vec<f64>)],
    result: &degradation::DegradationResult,
    model: &str,
    threshold: f64,
) -> Vec<Series> {
    let mut tmax = 1.0f64;
    for (t, _y) in units {
        for v in t {
            tmax = tmax.max(*v);
        }
    }
    let mut out = vec![line(
        "Threshold",
        vec![(0.0, threshold), (tmax * 1.1, threshold)],
    )];
    for (i, (t, y)) in units.iter().enumerate() {
        let raw: Vec<(f64, f64)> = t.iter().zip(y).map(|(a, b)| (*a, *b)).collect();
        out.push(points(format!("Unit {} data", i + 1), raw));
        if let Some((a, b)) = result.paths.get(i) {
            let tt = t.iter().cloned().fold(1.0f64, f64::max) * 1.2;
            let pts: Vec<(f64, f64)> = (0..=40)
                .map(|k| {
                    let x = tt * k as f64 / 40.0;
                    (x, degradation::path_value(*a, *b, x, model))
                })
                .collect();
            out.push(line(format!("Unit {} fit", i + 1), pts));
        }
    }
    out
}

/// Growth chart data: Crow-AMSAA expected cumulative failures as a
/// line, observed failures as a step-like point series.
pub fn growth_series(fit: &growth::CrowAmsaaFit, times: &[f64]) -> Vec<Series> {
    let mut sorted = times.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let obs: Vec<(f64, f64)> = sorted
        .iter()
        .enumerate()
        .map(|(i, t)| (*t, (i + 1) as f64))
        .collect();
    let tmax = fit.t_end.max(1.0);
    let (lam, beta) = (fit.lam, fit.beta);
    let expected: Vec<(f64, f64)> = (0..=60)
        .map(|k| {
            let t = tmax * k as f64 / 60.0;
            (t, lam * t.powf(beta))
        })
        .collect();
    vec![
        line("Crow-AMSAA expected", expected),
        points("Observed failures", obs),
    ]
}

/// ALT chart data: reliability at use stress, as computed by the
/// work engine into `alt_series`.
pub fn alt_series(series: &[(f64, f64)]) -> Vec<Series> {
    if series.is_empty() {
        return Vec::new();
    }
    vec![line("Least squares fit", series.to_vec())]
}

/// Markov chart data: one transient probability curve per state,
/// as computed by the work engine into `mk_series`.
pub fn markov_series(series: &[(String, Vec<(f64, f64)>)]) -> Vec<Series> {
    series
        .iter()
        .map(|(name, pts)| line(name.clone(), pts.clone()))
        .collect()
}

/// The chart for the currently selected item, as a titled bundle.
/// `title` matches the old plotters captions. Empty when the
/// current screen has no chart or no data yet.
pub fn current_chart(w: &Work) -> (String, Vec<Series>) {
    let kind = w.selected.and_then(|i| w.items.get(i)).map(|it| it.kind);
    match kind {
        Some(ItemKind::LifeData) => match &w.life_fit {
            Some(fit) => (
                if w.life_show_prob {
                    "Empirical vs fitted probability"
                } else {
                    "Reliability curve"
                }
                .to_string(),
                life_series(fit, w.life_show_prob),
            ),
            None => (String::new(), Vec::new()),
        },
        Some(ItemKind::Degradation) => match &w.deg_data {
            Some(res) => (
                "Degradation paths".to_string(),
                degradation_series(
                    &w.deg_parse_units(),
                    res,
                    &w.deg_model,
                    fnum(&w.deg_threshold, 70.0),
                ),
            ),
            None => (String::new(), Vec::new()),
        },
        Some(ItemKind::Alt) => {
            if w.alt_series.is_empty() {
                (String::new(), Vec::new())
            } else {
                (
                    "Reliability at use stress".to_string(),
                    alt_series(&w.alt_series),
                )
            }
        }
        Some(ItemKind::Growth) => match &w.growth_fit {
            Some(fit) => (
                "Reliability growth".to_string(),
                growth_series(fit, &parse_numbers(&w.growth_times)),
            ),
            None => (String::new(), Vec::new()),
        },
        Some(ItemKind::Markov) => {
            if w.mk_series.is_empty() {
                (String::new(), Vec::new())
            } else {
                (
                    "Transient state probabilities".to_string(),
                    markov_series(&w.mk_series),
                )
            }
        }
        _ => (String::new(), Vec::new()),
    }
}
