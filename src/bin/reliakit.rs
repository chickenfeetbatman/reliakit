//! ReliaKit command line interface.
//!
//! Subcommands print JSON results to stdout so the engine is
//! scriptable. Input life data: CSV with a header row containing
//! `time` and optionally `failed` (1/0, true/false) columns.
//! Leading comment lines starting with `#` and blank lines are
//! skipped in every CSV input, matching the bundled datasets.

use clap::{Parser, Subcommand};
use reliakit::alt as alt_mod;
use reliakit::bayes;
use reliakit::competing;
use reliakit::degradation;
use reliakit::distributions as dist_mod;
use reliakit::doe;
use reliakit::faulttree::{self, FtNode};
use reliakit::flowchart;
use reliakit::fmea;
use reliakit::fracas;
use reliakit::growth;
use reliakit::importance;
use reliakit::markov::MarkovModel;
use reliakit::prediction::{self, PartEntry};
use reliakit::quickcalc;
use reliakit::standards217::{self, FactorInput};
use reliakit::standby;
use reliakit::system;
use reliakit::warranty::{self, WarrantyRecord};
use reliakit::wizard;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::process::exit;

#[derive(Parser)]
#[command(name = "reliakit", version, about = "ReliaKit reliability engineering CLI")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Fit a life distribution to CSV data (columns: time, failed).
    Fit {
        /// CSV file path.
        file: Option<String>,
        /// CSV file path (alternative to the positional file).
        #[arg(long)]
        csv: Option<String>,
        /// weibull2, weibull3, lognormal, normal, exponential, gamma, gumbel,
        /// loglogistic, logistic, gompertz, gengamma, weibull1, exponential2,
        /// mixed_weibull.
        #[arg(long, default_value = "weibull2")]
        dist: String,
        /// rank (median rank regression) instead of mle.
        #[arg(long)]
        rank: bool,
        /// Fixed shape beta, used only for weibull1.
        #[arg(long)]
        beta: Option<f64>,
    },
    /// Reliability of a named distribution at time t.
    Rel {
        dist: String,
        /// Mission time.
        t: f64,
        /// Parameters as key=value pairs, e.g. beta=1.5 eta=1000.
        params: Vec<String>,
    },
    /// Evaluate an RBD JSON file at mission time t (omit t for pure
    /// R-constant diagrams).
    Rbd {
        file: String,
        #[arg(long)]
        t: Option<f64>,
    },
    /// Monte Carlo availability of an RBD JSON file.
    Availability {
        file: String,
        #[arg(long, default_value_t = 10000.0)]
        horizon: f64,
        #[arg(long, default_value_t = 200)]
        sims: usize,
        #[arg(long, default_value_t = 42)]
        seed: u64,
    },
    /// Crow-AMSAA / Duane reliability growth from a CSV of failure times
    /// (single column with header, or first column).
    Growth {
        file: Option<String>,
        /// CSV file path (alternative to the positional file).
        #[arg(long)]
        csv: Option<String>,
        #[arg(long)]
        t_end: Option<f64>,
    },
    /// Acceleration factor: Arrhenius or inverse power law.
    Af {
        /// arrhenius: values are ea_ev t_use_c t_stress_c.
        /// ipl: values are exponent v_use v_stress.
        model: String,
        values: Vec<f64>,
    },
    /// MLE accelerated life test fit. CSV header: stress,time,failed
    /// with an optional second stress column stress2 for eyring.
    AltMle {
        file: Option<String>,
        /// CSV file path (alternative to the positional file).
        #[arg(long)]
        csv: Option<String>,
        /// arrhenius, ipl or eyring.
        #[arg(long, default_value = "arrhenius")]
        model: String,
        /// Mission time for the reliability readout.
        #[arg(long, default_value_t = 1000.0)]
        t: f64,
        /// Use stress as a comma list, e.g. 55 or 55,220 for eyring.
        /// Defaults to the first data row stress.
        #[arg(long)]
        use_stress: Option<String>,
        /// Two-sided bound level alpha for reliability bounds.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
    },
    /// Competing risks fit. CSV header: time,mode (empty mode means
    /// censored).
    Competing {
        file: Option<String>,
        /// CSV file path (alternative to the positional file).
        #[arg(long)]
        csv: Option<String>,
        /// Life distribution fitted per mode.
        #[arg(long, default_value = "weibull2")]
        dist: String,
        /// Mission time for reliability and CIF readouts.
        #[arg(long, default_value_t = 1000.0)]
        t: f64,
    },
    /// Standby redundancy reliability: quadrature, the system module
    /// evaluator, and a Monte Carlo cross-check.
    Standby {
        /// Life distribution of one unit.
        #[arg(long, default_value = "weibull2")]
        dist: String,
        /// Total units (1 active plus standbys).
        #[arg(long, default_value_t = 2)]
        n: usize,
        /// Dormancy factor in [0, 1]: 0 cold, 1 hot.
        #[arg(long, default_value_t = 0.0)]
        dormancy: f64,
        /// Switch reliability per switch.
        #[arg(long = "switch", default_value_t = 1.0)]
        switch_rel: f64,
        /// Mission time.
        #[arg(long, default_value_t = 1000.0)]
        t: f64,
        /// Parameters as key=value pairs, e.g. beta=1.5 eta=1000.
        params: Vec<String>,
    },
    /// Monte Carlo flowchart (process network) simulation from a JSON
    /// file.
    Flow {
        file: String,
        #[arg(long, default_value_t = 10000)]
        sims: usize,
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
    /// FMEA worksheet: read the FmeaRow CSV format, print rows sorted
    /// by descending RPN as JSON.
    Fmea {
        file: String,
    },
    /// Warranty analysis: Nevada table and returns forecast from a
    /// sales CSV and a returns CSV (sale_period,return_period).
    Warranty {
        /// Sales CSV: one integer per line, or period,count rows.
        #[arg(long)]
        sales: String,
        /// Returns CSV with header sale_period,return_period.
        #[arg(long)]
        returns: String,
        /// Current period index.
        #[arg(long, default_value_t = 0)]
        current: usize,
        /// Forecast horizon in periods.
        #[arg(long, default_value_t = 12)]
        horizon: usize,
    },
    /// Fault tree: top probability and minimal cut sets from a JSON
    /// file in the FtNode serde format.
    Faulttree {
        file: String,
    },
    /// Parts-count prediction from a JSON array of part entries.
    /// Keys: name, base_rate_per_million_hours (or base_rate),
    /// quantity, stress_factor, quality_factor.
    Predict {
        file: String,
        /// Mission time in hours.
        #[arg(long, default_value_t = 1000.0)]
        t: f64,
    },
    /// Two-level factorial DOE effects from a 2^k design.
    Doe {
        /// Number of factors.
        #[arg(long)]
        k: usize,
        /// Responses in Yates order as a comma list (2^k values).
        #[arg(long)]
        y: Option<String>,
        /// CSV file with the responses in the first column (alternative
        /// to --y). Header and comment lines are skipped.
        #[arg(long)]
        csv: Option<String>,
    },
    /// Distribution wizard: fit every rankable distribution to CSV
    /// life data (columns: time, failed) and print the ranking by
    /// AIC, with BIC and Anderson-Darling alongside.
    Wizard {
        /// CSV file path.
        file: Option<String>,
        /// CSV file path (alternative to the positional file).
        #[arg(long)]
        csv: Option<String>,
    },
    /// Quick calculation pad: life quantities of one distribution at
    /// one mission time, plus a Bx life.
    Qcp {
        /// Distribution name (positional form).
        dist_pos: Option<String>,
        /// Distribution name.
        #[arg(long = "dist")]
        dist: Option<String>,
        /// Mission time.
        #[arg(long)]
        t: f64,
        /// Failure fraction for the Bx life.
        #[arg(long, default_value_t = 0.10)]
        bx: f64,
        /// Parameters: comma separated key=value pairs, or plain
        /// values in dist_spec order. Repeatable.
        #[arg(long = "params", value_name = "PARAMS")]
        params_opt: Vec<String>,
        /// Parameters in the same forms, as trailing values.
        #[arg(value_name = "PARAM")]
        params_pos: Vec<String>,
    },
    /// Reliability importance measures (Birnbaum, Fussell-Vesely,
    /// criticality) for every block of an RBD JSON file.
    Importance {
        /// RBD JSON file path (positional form).
        file_pos: Option<String>,
        /// RBD JSON file path.
        #[arg(long = "json")]
        json: Option<String>,
        /// RBD JSON file path (alternative flag).
        #[arg(long = "file")]
        file_flag: Option<String>,
        /// Mission time.
        #[arg(long)]
        t: Option<f64>,
    },
    /// Markov chain analysis from a JSON file with states and
    /// transitions: {"states": [...], "transitions":
    /// [{"from": 0, "to": 1, "rate": 0.01}]}. Indices or state
    /// names both work for from and to.
    Markov {
        /// JSON file path (positional form).
        file_pos: Option<String>,
        /// JSON file path.
        #[arg(long = "json")]
        json: Option<String>,
        /// Absorbing state index. Repeatable, or a comma list.
        #[arg(long, value_delimiter = ',')]
        absorbing: Vec<usize>,
        /// Times for transient probabilities, as a comma list.
        /// Repeatable.
        #[arg(long, value_delimiter = ',')]
        times: Vec<f64>,
        /// Start state for the absorption and transient readouts.
        #[arg(long, default_value_t = 0)]
        start: usize,
    },
    /// MIL-HDBK-217F part stress prediction for one category of the
    /// libraries/mil217 data files.
    Predict217 {
        /// Category name, e.g. resistors, or a category JSON path.
        #[arg(long)]
        category: String,
        /// Inputs as key=value pairs, repeatable. Numeric values are
        /// table variables or pi factors; other values are named
        /// handbook labels such as a style. Keys follow the category
        /// JSON field names, e.g. style=RL temperature_c=60
        /// power_stress=0.5.
        #[arg(long = "input", value_name = "KEY=VALUE")]
        input: Vec<String>,
        /// Environment symbol, e.g. GF. Sets the environment input.
        #[arg(long)]
        env: Option<String>,
        /// Quality level as printed for the category. Sets the
        /// quality input.
        #[arg(long)]
        quality: Option<String>,
    },
    /// SR-332 style prediction methods (clean-room calculators; all
    /// rates and factors are supplied by the caller).
    Sr332 {
        #[command(subcommand)]
        method: Sr332Method,
    },
    /// 62380-style mission profile failure rate from user supplied
    /// phases. This is a mission profile framework in the style of
    /// IEC 62380; it holds no data from that standard.
    Mission {
        /// Base failure rate, in the same units as the result.
        #[arg(long = "base-rate")]
        base_rate: f64,
        /// Activation energy in eV for the temperature factors.
        #[arg(long, default_value_t = 0.35)]
        ea: f64,
        /// Reference temperature in degrees C.
        #[arg(long = "ref-temp", default_value_t = 25.0)]
        ref_temp: f64,
        /// Cycling contribution per cycle, as a multiple of the
        /// base rate spread over the profile hours.
        #[arg(long = "cycling-factor", default_value_t = 0.0)]
        cycling_factor: f64,
        /// One phase as name:hours:temp_c:cycles:dormant:
        /// dormant_factor. The dormant fields may be omitted;
        /// dormant is true, false, 1 or 0 and dormant_factor
        /// defaults to 1. Repeatable.
        #[arg(long = "phase", value_name = "PHASE")]
        phase: Vec<String>,
    },
    /// Reliability allocation for a series system: equal shares, or
    /// weighted shares of the log-reliability budget.
    Allocate {
        /// System target reliability.
        #[arg(long)]
        target: f64,
        /// Number of subsystems for equal allocation.
        #[arg(long)]
        n: Option<usize>,
        /// Weights as a comma list, for weighted allocation.
        #[arg(long)]
        weights: Option<String>,
    },
    /// Component library prediction: load a library CSV (columns:
    /// name, base_rate_per_million_hours, quantity, stress_factor,
    /// quality_factor) and print part rates and the system total.
    Library {
        /// Library CSV file path (positional form).
        file_pos: Option<String>,
        /// Library CSV file path.
        #[arg(long = "csv")]
        csv: Option<String>,
        /// Mission time in hours for the system reliability readout.
        #[arg(long)]
        t: Option<f64>,
    },
    /// FRACAS log summary: counts by status and by failure mode,
    /// plus MTBF for a given total operating hours figure.
    Fracas {
        /// Incidents CSV file path (positional form).
        file_pos: Option<String>,
        /// Incidents CSV file path.
        #[arg(long = "csv")]
        csv: Option<String>,
        /// Total operating hours for the MTBF readout.
        #[arg(long = "total-hours")]
        total_hours: Option<f64>,
    },
    /// Degradation analysis from a CSV with header unit,time,value.
    /// Per-unit paths are fitted and extrapolated to the failure
    /// threshold for pseudo-failure times.
    Degradation {
        /// CSV file path.
        file: Option<String>,
        /// CSV file path (alternative to the positional file).
        #[arg(long)]
        csv: Option<String>,
        /// Failure threshold for the measured value.
        #[arg(long)]
        threshold: f64,
        /// Path model: linear, exponential or power.
        #[arg(long, default_value = "exponential")]
        model: String,
        /// Life distribution fitted to the pseudo-failure times.
        #[arg(long, default_value = "weibull2")]
        dist: String,
        /// The measured value decreases toward failure (default is
        /// increasing toward failure).
        #[arg(long)]
        decreasing: bool,
    },
    /// Run the built-in self test (known closed-form values).
    Selftest,
}

#[derive(Subcommand)]
enum Sr332Method {
    /// Method I device rate: generic rate times pi_q, pi_s and pi_t.
    /// pi_t comes from --pi-t, or from --temp and --ref-temp through
    /// the Arrhenius temperature factor.
    Method1 {
        /// Generic failure rate supplied by the caller.
        #[arg(long)]
        generic: f64,
        /// Quality factor pi_q.
        #[arg(long = "pi-q", default_value_t = 1.0)]
        pi_q: f64,
        /// Stress factor pi_s.
        #[arg(long = "pi-s", default_value_t = 1.0)]
        pi_s: f64,
        /// Temperature factor pi_t, given directly.
        #[arg(long = "pi-t")]
        pi_t: Option<f64>,
        /// Use temperature in degrees C, to compute pi_t.
        #[arg(long)]
        temp: Option<f64>,
        /// Reference temperature in degrees C, to compute pi_t.
        #[arg(long = "ref-temp")]
        ref_temp: Option<f64>,
        /// Activation energy in eV for the pi_t computation.
        #[arg(long, default_value_t = prediction::SR332_DEFAULT_ACTIVATION_ENERGY_EV)]
        ea: f64,
    },
    /// Equipment rate: pi_e applied once to the quantity weighted
    /// sum of device rates.
    Equipment {
        /// Parts as qty:rate pairs in a comma list, e.g. 3:1.8,4:0.25.
        #[arg(long)]
        parts: String,
        /// Environment factor pi_e.
        #[arg(long = "pi-e", default_value_t = 1.0)]
        pi_e: f64,
    },
    /// Method II extrapolation from observed conditions to target
    /// conditions by the temperature and stress factor ratios.
    Method2 {
        /// Failure rate observed at the observed conditions.
        #[arg(long = "observed-rate")]
        observed_rate: f64,
        /// Temperature factor at the observed conditions.
        #[arg(long = "pi-t-obs", default_value_t = 1.0)]
        pi_t_obs: f64,
        /// Temperature factor at the target conditions.
        #[arg(long = "pi-t-target", default_value_t = 1.0)]
        pi_t_target: f64,
        /// Stress factor at the observed conditions.
        #[arg(long = "pi-s-obs", default_value_t = 1.0)]
        pi_s_obs: f64,
        /// Stress factor at the target conditions.
        #[arg(long = "pi-s-target", default_value_t = 1.0)]
        pi_s_target: f64,
    },
    /// Two-sided chi-square confidence bounds on a constant failure
    /// rate from a failure count and exposure hours.
    Ci {
        /// Number of failures observed.
        #[arg(long)]
        failures: u64,
        /// Exposure in hours.
        #[arg(long, alias = "exposure-hours")]
        hours: f64,
        /// Confidence level, e.g. 0.95.
        #[arg(long, default_value_t = 0.95)]
        confidence: f64,
    },
    /// Method III combination: weighted average of a predicted rate
    /// and an observed rate.
    Method3 {
        /// Predicted rate (Method I or Method II style).
        #[arg(long)]
        predicted: f64,
        /// Weight on the predicted rate.
        #[arg(long = "predicted-weight", default_value_t = 1.0)]
        predicted_weight: f64,
        /// Observed field or test rate.
        #[arg(long)]
        observed: f64,
        /// Weight on the observed rate.
        #[arg(long = "observed-weight", default_value_t = 1.0)]
        observed_weight: f64,
    },
}

/// JSON number value: non-finite floats (NaN, infinities from
/// failed fits or blank handbook entries) become null.
fn jnum(x: f64) -> Value {
    serde_json::Number::from_f64(x).map(Value::Number).unwrap_or(Value::Null)
}

/// Resolve a file argument given as a positional value or as --csv:
/// --csv wins when both are present.
fn resolve_file_arg(
    positional: &Option<String>,
    csv: &Option<String>,
    what: &str,
) -> Result<String, String> {
    csv.clone()
        .or_else(|| positional.clone())
        .ok_or_else(|| format!("{} needs a CSV file (positional path or --csv)", what))
}

/// Read life data CSV: header with `time` plus `failed` (or the
/// `censored` variant, where 1 means suspended), and an optional
/// `count` column for grouped rows, which is expanded. Comment
/// lines starting with `#` and blank lines are skipped, matching
/// the bundled datasets format.
fn read_csv_life(path: &str) -> Result<(Vec<f64>, Vec<bool>), String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lines = text.lines().filter(|l| {
        let t = l.trim();
        !t.is_empty() && !t.starts_with('#')
    });
    let header = lines.next().ok_or("empty csv")?;
    let cols: Vec<String> = header.split(',').map(|s| s.trim().to_lowercase()).collect();
    let ti = cols
        .iter()
        .position(|c| c == "time" || c == "times" || c == "life" || c == "hours")
        .unwrap_or(0);
    let fi = cols
        .iter()
        .position(|c| c == "failed" || c == "failure" || c == "event");
    let ci = cols.iter().position(|c| c == "censored");
    let ki = cols.iter().position(|c| c == "count");
    let mut times = Vec::new();
    let mut failed = Vec::new();
    for line in lines {
        let cells: Vec<&str> = line.split(',').collect();
        if ti >= cells.len() {
            return Err("bad csv row".into());
        }
        let t: f64 = cells[ti].trim().parse().map_err(|_| "bad time value")?;
        let f = match fi {
            Some(i) => {
                if i >= cells.len() {
                    return Err("bad csv row".into());
                }
                parse_failed_cell(cells[i])
            }
            None => match ci {
                Some(i) => {
                    if i >= cells.len() {
                        return Err("bad csv row".into());
                    }
                    !parse_failed_cell(cells[i])
                }
                None => true,
            },
        };
        let count: usize = match ki {
            Some(i) => {
                if i >= cells.len() {
                    return Err("bad csv row".into());
                }
                cells[i].trim().parse().map_err(|_| "bad count value")?
            }
            None => 1,
        };
        for _ in 0..count {
            times.push(t);
            failed.push(f);
        }
    }
    Ok((times, failed))
}

/// Read the first column of a CSV as numbers. Header rows, comment
/// lines starting with `#` and blank lines are skipped.
fn read_csv_column(path: &str) -> Result<Vec<f64>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let first = line.split(',').next().unwrap_or("").trim();
        if let Ok(v) = first.parse::<f64>() {
            out.push(v);
        }
    }
    Ok(out)
}

/// Generic CSV reader: returns (header names lowercased, data rows as
/// trimmed string cells). Blank lines and comment lines starting
/// with `#` are skipped.
fn read_csv_rows(path: &str) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lines = text.lines().filter(|l| {
        let t = l.trim();
        !t.is_empty() && !t.starts_with('#')
    });
    let header = lines.next().ok_or("empty csv")?;
    let cols: Vec<String> = header.split(',').map(|s| s.trim().to_lowercase()).collect();
    let mut rows = Vec::new();
    for line in lines {
        rows.push(line.split(',').map(|s| s.trim().to_string()).collect::<Vec<String>>());
    }
    Ok((cols, rows))
}

fn col_index(cols: &[String], names: &[&str]) -> Option<usize> {
    cols.iter().position(|c| names.contains(&c.as_str()))
}

fn parse_failed_cell(v: &str) -> bool {
    matches!(v.trim().to_lowercase().as_str(), "1" | "true" | "t" | "yes" | "f" | "failed")
}

fn parse_params(pairs: &[String]) -> Result<dist_mod::Params, String> {
    let mut p = BTreeMap::new();
    for pair in pairs {
        let (k, v) = pair.split_once('=').ok_or("params must be key=value")?;
        p.insert(k.to_string(), v.parse::<f64>().map_err(|_| "bad param value")?);
    }
    Ok(p)
}

/// Flexible parameter parsing for qcp: tokens may be comma
/// separated key=value pairs, or plain numeric values assigned in
/// dist_spec order.
fn parse_params_flexible(dist: &str, tokens: &[String]) -> Result<dist_mod::Params, String> {
    let spec = dist_mod::dist_spec(dist).ok_or_else(|| format!("unknown distribution {}", dist))?;
    let mut pieces: Vec<String> = Vec::new();
    for tok in tokens {
        for piece in tok.split(',') {
            let piece = piece.trim();
            if !piece.is_empty() {
                pieces.push(piece.to_string());
            }
        }
    }
    let mut p = dist_mod::Params::new();
    let mut next_idx = 0usize;
    for piece in &pieces {
        if let Some((k, v)) = piece.split_once('=') {
            p.insert(
                k.trim().to_string(),
                v.trim().parse::<f64>().map_err(|_| "bad param value")?,
            );
        } else if let Ok(v) = piece.parse::<f64>() {
            if next_idx >= spec.len() {
                return Err(format!("too many positional values for {}", dist));
            }
            p.insert(spec[next_idx].0.to_string(), v);
            next_idx += 1;
        } else {
            return Err(format!("bad param token: {}", piece));
        }
    }
    Ok(p)
}

fn parse_comma_f64s(s: &str) -> Result<Vec<f64>, String> {
    s.split(',')
        .map(|x| x.trim().parse::<f64>().map_err(|_| format!("bad number: {}", x)))
        .collect()
}

fn selftest() -> i32 {
    let mut failures = 0;
    let check = |name: &str, got: f64, want: f64, tol: f64, failures: &mut i32| {
        let ok = (got - want).abs() <= tol;
        if !ok {
            *failures += 1;
        }
        println!(
            "{} {}: got {:.10} want {:.10}",
            if ok { "PASS" } else { "FAIL" },
            name,
            got,
            want
        );
    };
    let mut p = dist_mod::Params::new();
    p.insert("beta".into(), 2.0);
    p.insert("eta".into(), 1000.0);
    check("weibull R(eta)", dist_mod::reliability("weibull2", &p, 1000.0), (-1.0f64).exp(), 1e-10, &mut failures);
    check("series", system::series_reliability(&[0.9, 0.8]), 0.72, 1e-12, &mut failures);
    check("parallel", system::parallel_reliability(&[0.9, 0.8]), 0.98, 1e-12, &mut failures);
    let mut pe = dist_mod::Params::new();
    pe.insert("theta".into(), 1000.0);
    check(
        "cold standby",
        system::standby_reliability(1000.0, "exponential", &pe, 2, 0.0, 1.0),
        (-1.0f64).exp() * 2.0,
        1e-9,
        &mut failures,
    );
    check("arrhenius AF", alt_mod::acceleration_factor_arrhenius(0.7, 25.0, 125.0), 936.5, 20.0, &mut failures);
    let calib = vec![5000.0; 20];
    check("conformal zero-fail", bayes::conformal_reliability_lower(&calib, 1000.0, 0.05), 0.05f64.powf(0.05), 1e-9, &mut failures);
    // Competing risks invariant: for any fitted model the cumulative
    // incidences at t plus the overall reliability at t sum to 1.
    {
        let times = vec![120.0, 340.0, 560.0, 800.0, 950.0, 1300.0, 1700.0, 2200.0, 2600.0, 3100.0];
        let events: Vec<Option<&str>> = vec![
            Some("wear"), Some("electrical"), Some("wear"), None, Some("electrical"),
            Some("wear"), None, Some("electrical"), Some("wear"), None,
        ];
        match competing::fit_competing_risks(&times, &events, "weibull2") {
            Ok(cr) => {
                let t = 1000.0;
                let overall = competing::overall_reliability(&cr, t);
                let cif_sum: f64 = ["wear", "electrical"]
                    .iter()
                    .map(|m| competing::cumulative_incidence(&cr, m, t))
                    .sum();
                check("competing risks CIF + R = 1", cif_sum + overall, 1.0, 0.02, &mut failures);
            }
            Err(e) => {
                println!("FAIL competing risks fit: {}", e);
                failures += 1;
            }
        }
    }
    if failures == 0 {
        println!("selftest: all checks passed");
        0
    } else {
        println!("selftest: {} checks FAILED", failures);
        1
    }
}

fn cmd_alt_mle(
    file: &str,
    model: &str,
    t: f64,
    use_stress: &Option<String>,
    alpha: f64,
) -> Result<Value, String> {
    let (cols, rows) = read_csv_rows(file)?;
    let si = col_index(&cols, &["stress"]).ok_or("csv needs a stress column")?;
    let s2i = col_index(&cols, &["stress2", "stress_2", "voltage"]);
    let ti = col_index(&cols, &["time", "times", "life", "hours"]).ok_or("csv needs a time column")?;
    let fi = col_index(&cols, &["failed", "failure", "event"]);
    let mut stress: Vec<Vec<f64>> = Vec::new();
    let mut times: Vec<f64> = Vec::new();
    let mut failed: Vec<bool> = Vec::new();
    for row in &rows {
        let mut srow = vec![row[si].parse::<f64>().map_err(|_| "bad stress value")?];
        if let Some(i2) = s2i {
            srow.push(row[i2].parse::<f64>().map_err(|_| "bad stress2 value")?);
        }
        stress.push(srow);
        times.push(row[ti].parse::<f64>().map_err(|_| "bad time value")?);
        failed.push(match fi {
            Some(i) => parse_failed_cell(&row[i]),
            None => true,
        });
    }
    let fit = alt_mod::fit_alt_mle(model, &stress, &times, Some(&failed))?;
    let use_s: Vec<f64> = match use_stress {
        Some(s) => parse_comma_f64s(s)?,
        None => stress.first().cloned().unwrap_or_default(),
    };
    let (lo, hi) = fit.reliability_bounds(t, &use_s, alpha);
    Ok(json!({
        "model": model,
        "coef": fit.coef,
        "sigma": fit.sigma,
        "loglik": fit.loglik,
        "activation_energy_ev": fit.activation_energy_ev(),
        "t": t,
        "use_stress": use_s,
        "reliability": fit.reliability(t, &use_s),
        "reliability_bounds": [lo, hi],
        "alpha": alpha,
    }))
}

fn cmd_competing(file: &str, dist: &str, t: f64) -> Result<Value, String> {
    let (cols, rows) = read_csv_rows(file)?;
    let ti = col_index(&cols, &["time", "times", "life", "hours"]).ok_or("csv needs a time column")?;
    let mi = col_index(&cols, &["mode", "failure_mode", "cause"]).ok_or("csv needs a mode column")?;
    let mut times: Vec<f64> = Vec::new();
    let mut mode_strings: Vec<Option<String>> = Vec::new();
    for row in &rows {
        times.push(row[ti].parse::<f64>().map_err(|_| "bad time value")?);
        let m = row[mi].trim().to_string();
        mode_strings.push(if m.is_empty() { None } else { Some(m) });
    }
    let events: Vec<Option<&str>> = mode_strings.iter().map(|o| o.as_deref()).collect();
    let cr = competing::fit_competing_risks(&times, &events, dist)?;
    let modes: Vec<String> = cr.modes.iter().map(|(name, _)| name.clone()).collect();
    let mut per_mode = serde_json::Map::new();
    let mut cif = serde_json::Map::new();
    for (name, fit) in &cr.modes {
        per_mode.insert(
            name.clone(),
            json!({"params": fit.params, "loglik": fit.loglik, "n": fit.n, "n_fail": fit.n_fail}),
        );
        cif.insert(name.clone(), json!(competing::cumulative_incidence(&cr, name, t)));
    }
    Ok(json!({
        "dist": dist,
        "modes": modes,
        "per_mode": Value::Object(per_mode),
        "t": t,
        "overall_reliability": competing::overall_reliability(&cr, t),
        "cumulative_incidence": Value::Object(cif),
        "failure_probabilities": competing::failure_probabilities(&cr, t)
            .iter()
            .map(|(name, p)| json!({"mode": name, "probability": p}))
            .collect::<Vec<Value>>(),
    }))
}

fn cmd_warranty(sales_path: &str, returns_path: &str, current: usize, horizon: usize) -> Result<Value, String> {
    // Sales: one integer per line, or period,count rows (header allowed).
    let sales_text = std::fs::read_to_string(sales_path).map_err(|e| e.to_string())?;
    let mut pairs: Vec<(usize, u32)> = Vec::new();
    let mut singles: Vec<u32> = Vec::new();
    for line in sales_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let cells: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if cells.len() >= 2 {
            if let (Ok(p), Ok(c)) = (cells[0].parse::<usize>(), cells[1].parse::<u32>()) {
                pairs.push((p, c));
            }
        } else if let Ok(v) = cells[0].parse::<u32>() {
            singles.push(v);
        }
    }
    let sales: Vec<u32> = if !pairs.is_empty() {
        pairs.sort();
        let n = pairs.iter().map(|(p, _)| p + 1).max().unwrap_or(0);
        let mut s = vec![0u32; n];
        for (p, c) in pairs {
            s[p] = c;
        }
        s
    } else {
        singles
    };
    // Returns: sale_period,return_period with header.
    let (cols, rows) = read_csv_rows(returns_path)?;
    let spi = col_index(&cols, &["sale_period", "sale", "sold_period"]).ok_or("returns csv needs sale_period")?;
    let rpi = col_index(&cols, &["return_period", "return", "returned_period"]).ok_or("returns csv needs return_period")?;
    let mut records: Vec<WarrantyRecord> = Vec::new();
    for row in &rows {
        records.push(WarrantyRecord {
            sale_period: row[spi].parse::<usize>().map_err(|_| "bad sale_period")?,
            return_period: row[rpi].parse::<usize>().map_err(|_| "bad return_period")?,
        });
    }
    let max_age = records
        .iter()
        .filter(|r| r.return_period >= r.sale_period)
        .map(|r| r.return_period - r.sale_period)
        .max()
        .unwrap_or(0);
    let table = warranty::nevada_table(sales.len(), &records, max_age);
    let forecast = warranty::forecast_returns(&sales, &records, current, horizon)?;
    Ok(json!({
        "nevada_table": table,
        "returns_by_age": warranty::returns_by_age(&table),
        "total_returns": warranty::total_returns(&table),
        "forecast": {
            "beta": forecast.beta,
            "eta": forecast.eta,
            "expected_returns": forecast.expected_returns,
            "units_in_service": forecast.units_in_service,
            "current_period": current,
            "horizon_periods": horizon,
        },
    }))
}

fn cmd_predict(file: &str, t: f64) -> Result<Value, String> {
    let text = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let arr: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let items = arr.as_array().ok_or("predict file must be a JSON array")?;
    let mut parts: Vec<PartEntry> = Vec::new();
    for item in items {
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("part").to_string();
        let rate = item
            .get("base_rate_per_million_hours")
            .or_else(|| item.get("base_rate"))
            .and_then(|v| v.as_f64())
            .ok_or("part entry needs base_rate_per_million_hours")?;
        let qty = item.get("quantity").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
        let mut p = PartEntry::new(&name, rate, qty);
        if let Some(v) = item.get("stress_factor").and_then(|v| v.as_f64()) {
            p.stress_factor = v;
        }
        if let Some(v) = item.get("quality_factor").and_then(|v| v.as_f64()) {
            p.quality_factor = v;
        }
        parts.push(p);
    }
    Ok(json!({
        "parts": serde_json::to_value(&parts).unwrap_or(Value::Null),
        "system_failure_rate_per_million_hours": prediction::system_failure_rate_per_million_hours(&parts),
        "system_mtbf_hours": prediction::system_mtbf_hours(&parts),
        "t_hours": t,
        "system_reliability": prediction::system_reliability(&parts, t),
    }))
}

fn cmd_wizard(file: &str) -> Result<Value, String> {
    let (times, failed) = read_csv_life(file)?;
    let ranks = wizard::rank_distributions(&times, Some(&failed), wizard::RANKABLE_DISTS);
    let rows: Vec<Value> = ranks
        .iter()
        .enumerate()
        .map(|(i, r)| {
            json!({
                "rank": i + 1,
                "dist": r.dist,
                "loglik": jnum(r.loglik),
                "aic": jnum(r.aic),
                "bic": jnum(r.bic),
                "ad_statistic": r.ad_statistic.map(jnum).unwrap_or(Value::Null),
                "n_params": r.n_params,
                "success": r.success,
            })
        })
        .collect();
    Ok(json!({"file": file, "n": times.len(), "ranking": rows}))
}

fn cmd_importance(
    positional: &Option<String>,
    json_flag: &Option<String>,
    file_flag: &Option<String>,
    t: Option<f64>,
) -> Result<Value, String> {
    let path = json_flag
        .clone()
        .or_else(|| file_flag.clone())
        .or_else(|| positional.clone())
        .ok_or("importance needs an RBD JSON file (positional path or --json)")?;
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let spec: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let tt = t.unwrap_or(0.0);
    let rows = importance::importance_measures(&spec, tt)?;
    let blocks: Vec<Value> = rows
        .iter()
        .map(|b| {
            json!({
                "name": b.name,
                "birnbaum": b.birnbaum,
                "fussell_vesely": b.fussell_vesely,
                "criticality": b.criticality,
            })
        })
        .collect();
    Ok(json!({"file": path, "t": tt, "blocks": blocks}))
}

fn markov_state_index(v: &Value, names: &[String]) -> Result<usize, String> {
    if let Some(i) = v.as_u64() {
        return Ok(i as usize);
    }
    if let Some(s) = v.as_str() {
        if let Some(pos) = names.iter().position(|n| n == s) {
            return Ok(pos);
        }
        if let Ok(i) = s.trim().parse::<usize>() {
            return Ok(i);
        }
        return Err(format!("unknown markov state name: {}", s));
    }
    Err("markov transition endpoint must be an index or a state name".into())
}

fn cmd_markov(
    positional: &Option<String>,
    json_flag: &Option<String>,
    absorbing: &[usize],
    times: &[f64],
    start: usize,
) -> Result<Value, String> {
    let path = json_flag
        .clone()
        .or_else(|| positional.clone())
        .ok_or("markov needs a JSON file (positional path or --json)")?;
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let doc: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let names_val = doc
        .get("states")
        .or_else(|| doc.get("names"))
        .ok_or("markov json needs a states array")?;
    let names: Vec<String> = names_val
        .as_array()
        .ok_or("markov states must be an array")?
        .iter()
        .map(|v| match v.as_str() {
            Some(s) => s.to_string(),
            None => v.to_string(),
        })
        .collect();
    let trans_val = doc
        .get("transitions")
        .and_then(|v| v.as_array())
        .ok_or("markov json needs a transitions array")?;
    let mut transitions: Vec<(usize, usize, f64)> = Vec::new();
    for tr in trans_val {
        if let Some(obj) = tr.as_object() {
            let from = obj.get("from").ok_or("transition needs from")?;
            let to = obj.get("to").ok_or("transition needs to")?;
            let rate = obj
                .get("rate")
                .and_then(|v| v.as_f64())
                .ok_or("transition needs a numeric rate")?;
            transitions.push((
                markov_state_index(from, &names)?,
                markov_state_index(to, &names)?,
                rate,
            ));
        } else if let Some(arr) = tr.as_array() {
            if arr.len() != 3 {
                return Err("array transition needs [from, to, rate]".into());
            }
            let rate = arr[2].as_f64().ok_or("transition rate must be a number")?;
            transitions.push((
                markov_state_index(&arr[0], &names)?,
                markov_state_index(&arr[1], &names)?,
                rate,
            ));
        } else {
            return Err("transition must be an object or a [from, to, rate] array".into());
        }
    }
    let model = MarkovModel::new(names.clone(), &transitions)?;
    let n = model.n_states();
    if start >= n {
        return Err(format!("start state {} out of range for {} states", start, n));
    }
    let steady = model.steady_state()?;
    let mut flags = vec![false; n];
    for &i in absorbing {
        if i >= n {
            return Err(format!("absorbing state {} out of range for {} states", i, n));
        }
        flags[i] = true;
    }
    let (mtta, expected): (Value, Value) = if absorbing.is_empty() {
        (Value::Null, Value::Null)
    } else {
        (
            jnum(model.mean_time_to_absorption(&flags, start)?),
            Value::Array(model.expected_time_in_state(&flags, start)?.iter().map(|v| jnum(*v)).collect()),
        )
    };
    let transient_rows: Vec<Value> = if times.is_empty() {
        Vec::new()
    } else {
        let mut p0 = vec![0.0; n];
        p0[start] = 1.0;
        let rows = model.transient(&p0, times)?;
        times
            .iter()
            .zip(rows.iter())
            .map(|(t, row)| json!({"t": t, "probabilities": row}))
            .collect()
    };
    Ok(json!({
        "file": path,
        "states": names,
        "steady_state": steady,
        "absorbing": absorbing,
        "start": start,
        "mean_time_to_absorption": mtta,
        "expected_time_in_state": expected,
        "transient": transient_rows,
    }))
}

/// Find a category JSON file in libraries/mil217: relative to the
/// current working directory first, then relative to the crate
/// manifest dir.
fn resolve_mil217_category(category: &str) -> Result<PathBuf, String> {
    let raw = PathBuf::from(category);
    if raw.is_file() {
        return Ok(raw);
    }
    let stem = raw
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| category.to_string());
    let with_ext = if stem.ends_with(".json") {
        stem.clone()
    } else {
        format!("{}.json", stem)
    };
    let normalized = with_ext.to_lowercase().replace(' ', "_").replace('-', "_");
    let mut bases: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        bases.push(cwd.join("libraries").join("mil217"));
    }
    bases.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("libraries").join("mil217"));
    for base in &bases {
        for name in [&with_ext, &normalized] {
            let cand = base.join(name);
            if cand.is_file() {
                return Ok(cand);
            }
        }
    }
    Err(format!(
        "cannot find MIL-HDBK-217F category {} in libraries/mil217 (looked in the working directory and the crate directory)",
        category
    ))
}

fn cmd_predict217(
    category: &str,
    input_args: &[String],
    env: &Option<String>,
    quality: &Option<String>,
) -> Result<Value, String> {
    let path = resolve_mil217_category(category)?;
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let cat = standards217::load_category(&text).map_err(|e| e.to_string())?;
    // Split the repeatable --input values into key=value pairs. A
    // comma piece without an = sign is rejoined to the previous
    // pair, so named values that contain a comma (for example the
    // capacitor style "CQ, CQR") survive.
    let mut pair_strings: Vec<String> = Vec::new();
    for raw in input_args {
        for piece in raw.split(',') {
            let piece = piece.trim();
            if piece.is_empty() {
                continue;
            }
            if piece.contains('=') || pair_strings.is_empty() {
                pair_strings.push(piece.to_string());
            } else {
                let last = pair_strings.last_mut().unwrap();
                last.push_str(", ");
                last.push_str(piece);
            }
        }
    }
    let mut inputs: standards217::Inputs = HashMap::new();
    let mut inputs_echo = serde_json::Map::new();
    for pair in &pair_strings {
        let (k, v) = pair
            .split_once('=')
            .ok_or_else(|| format!("--input must be key=value, got {}", pair))?;
        let key = k.trim().to_string();
        let val = v.trim();
        if let Ok(x) = val.parse::<f64>() {
            inputs.insert(key.clone(), FactorInput::num(x));
            inputs_echo.insert(key, json!(x));
        } else {
            inputs.insert(key.clone(), FactorInput::named(val));
            inputs_echo.insert(key, json!(val));
        }
    }
    if let Some(e) = env {
        inputs.insert("environment".to_string(), FactorInput::named(e));
        inputs_echo.insert("environment".to_string(), json!(e));
    }
    if let Some(q) = quality {
        inputs.insert("quality".to_string(), FactorInput::named(q));
        inputs_echo.insert("quality".to_string(), json!(q));
    }
    let lambda_b = standards217::lambda_b_for(&cat, &inputs);
    let lambda_p = standards217::part_failure_rate(&cat, &inputs);
    // Pi factors the public API can resolve directly: factors given
    // as numeric inputs, and named tables looked up through
    // pi_table_value. Series and column tables have no public
    // per-factor resolver, so they are only reflected in lambda_p.
    let mut pi_factors = serde_json::Map::new();
    if let Some(tables) = cat.get("pi_tables").and_then(|t| t.as_object()) {
        let mut names: Vec<&String> = tables.keys().collect();
        names.sort();
        for fname in names {
            if let Some(FactorInput::Num(v)) = inputs.get(fname) {
                pi_factors.insert(fname.clone(), jnum(*v));
                continue;
            }
            let table = &tables[fname];
            if table.get("kind").and_then(|k| k.as_str()) == Some("named") {
                if let Some(input_key) = table.get("input").and_then(|k| k.as_str()) {
                    if let Some(FactorInput::Named(nm)) = inputs.get(input_key) {
                        if let Some(v) = standards217::pi_table_value(&cat, fname, nm) {
                            pi_factors.insert(fname.clone(), jnum(v));
                        }
                    }
                }
            }
        }
    }
    // Caller supplied pi factors that are not tables in this
    // category are multiplied in by part_failure_rate as given.
    for (key, inp) in inputs.iter() {
        if key.starts_with("pi_") && !pi_factors.contains_key(key) {
            if let FactorInput::Num(v) = inp {
                pi_factors.insert(key.clone(), jnum(*v));
            }
        }
    }
    let pi_product = match lambda_b {
        Some(lb) if lb != 0.0 && lb.is_finite() && lambda_p.is_finite() => jnum(lambda_p / lb),
        _ => Value::Null,
    };
    Ok(json!({
        "category": cat.get("category").and_then(|v| v.as_str()).unwrap_or(category),
        "category_file": path.display().to_string(),
        "model": cat.get("model").and_then(|v| v.as_str()).unwrap_or(""),
        "inputs": Value::Object(inputs_echo),
        "lambda_b": lambda_b.map(jnum).unwrap_or(Value::Null),
        "pi_factors": Value::Object(pi_factors),
        "pi_product": pi_product,
        "lambda_p": jnum(lambda_p),
        "unit": "failures per 10^6 hours",
    }))
}

fn cmd_sr332(method: &Sr332Method) -> Result<Value, String> {
    match method {
        Sr332Method::Method1 { generic, pi_q, pi_s, pi_t, temp, ref_temp, ea } => {
            let (pi_t_value, pi_t_source) = if let Some(v) = pi_t {
                (*v, "given")
            } else if let (Some(t), Some(rt)) = (temp, ref_temp) {
                (prediction::sr332_temperature_factor(*ea, *t, *rt), "computed")
            } else if temp.is_some() || ref_temp.is_some() {
                return Err("computing pi_t needs both --temp and --ref-temp".into());
            } else {
                (1.0, "default")
            };
            Ok(json!({
                "method": "method1",
                "generic_rate": generic,
                "pi_q": pi_q,
                "pi_s": pi_s,
                "pi_t": pi_t_value,
                "pi_t_source": pi_t_source,
                "activation_energy_ev": ea,
                "device_rate": prediction::sr332_method1(*generic, *pi_q, *pi_s, pi_t_value),
            }))
        }
        Sr332Method::Equipment { parts, pi_e } => {
            let mut parsed: Vec<(u32, f64)> = Vec::new();
            for piece in parts.split(',') {
                let piece = piece.trim();
                if piece.is_empty() {
                    continue;
                }
                let (q, r) = piece
                    .split_once(':')
                    .ok_or_else(|| format!("--parts entries must be qty:rate, got {}", piece))?;
                parsed.push((
                    q.trim().parse::<u32>().map_err(|_| "bad quantity in --parts")?,
                    r.trim().parse::<f64>().map_err(|_| "bad rate in --parts")?,
                ));
            }
            let part_rows: Vec<Value> = parsed
                .iter()
                .map(|(q, r)| json!({"quantity": q, "device_rate": r, "contribution": *q as f64 * r}))
                .collect();
            Ok(json!({
                "method": "equipment",
                "parts": part_rows,
                "pi_e": pi_e,
                "equipment_rate": prediction::sr332_equipment_rate(&parsed, *pi_e),
            }))
        }
        Sr332Method::Method2 { observed_rate, pi_t_obs, pi_t_target, pi_s_obs, pi_s_target } => Ok(json!({
            "method": "method2",
            "observed_rate": observed_rate,
            "pi_t_observed": pi_t_obs,
            "pi_t_target": pi_t_target,
            "pi_s_observed": pi_s_obs,
            "pi_s_target": pi_s_target,
            "target_rate": prediction::sr332_method2_extrapolate(
                *observed_rate, *pi_t_obs, *pi_t_target, *pi_s_obs, *pi_s_target),
        })),
        Sr332Method::Ci { failures, hours, confidence } => {
            let (lo, hi) = prediction::failure_rate_ci(*failures, *hours, *confidence);
            Ok(json!({
                "method": "ci",
                "failures": failures,
                "exposure_hours": hours,
                "confidence": confidence,
                "rate_per_hour": if *hours != 0.0 { *failures as f64 / *hours } else { f64::NAN },
                "lower": lo,
                "upper": hi,
            }))
        }
        Sr332Method::Method3 { predicted, predicted_weight, observed, observed_weight } => Ok(json!({
            "method": "method3",
            "predicted_rate": predicted,
            "predicted_weight": predicted_weight,
            "observed_rate": observed,
            "observed_weight": observed_weight,
            "combined_rate": prediction::sr332_method3_combine(
                *predicted, *predicted_weight, *observed, *observed_weight),
        })),
    }
}

fn parse_boolish(s: &str) -> Result<bool, String> {
    match s.trim().to_lowercase().as_str() {
        "true" | "1" | "yes" | "y" => Ok(true),
        "false" | "0" | "no" | "n" => Ok(false),
        other => Err(format!("bad dormant flag: {}", other)),
    }
}

fn cmd_mission(
    base_rate: f64,
    ea: f64,
    ref_temp: f64,
    cycling_factor: f64,
    phase_args: &[String],
) -> Result<Value, String> {
    let mut phases: Vec<prediction::MissionPhase> = Vec::new();
    for raw in phase_args {
        let f: Vec<&str> = raw.split(':').map(|s| s.trim()).collect();
        if f.len() < 4 || f.len() > 6 {
            return Err(format!(
                "phase must be name:hours:temp_c:cycles[:dormant[:dormant_factor]], got {}",
                raw
            ));
        }
        let dormant = if f.len() >= 5 && !f[4].is_empty() {
            parse_boolish(f[4])?
        } else {
            false
        };
        let dormant_factor = if f.len() >= 6 && !f[5].is_empty() {
            f[5].parse::<f64>().map_err(|_| "bad dormant_factor in phase")?
        } else {
            1.0
        };
        phases.push(prediction::MissionPhase {
            name: f[0].to_string(),
            duration_hours: f[1].parse::<f64>().map_err(|_| "bad phase hours")?,
            temp_c: f[2].parse::<f64>().map_err(|_| "bad phase temp_c")?,
            cycles: f[3].parse::<f64>().map_err(|_| "bad phase cycles")?,
            dormant,
            dormant_factor,
        });
    }
    let rate = prediction::iec62380_style_mission_profile_rate(
        base_rate,
        &phases,
        ea,
        ref_temp,
        cycling_factor,
    );
    let phase_rows: Vec<Value> = phases
        .iter()
        .map(|p| {
            json!({
                "name": p.name,
                "duration_hours": p.duration_hours,
                "temp_c": p.temp_c,
                "cycles": p.cycles,
                "dormant": p.dormant,
                "dormant_factor": p.dormant_factor,
            })
        })
        .collect();
    Ok(json!({
        "method": "62380-style mission profile",
        "base_rate": base_rate,
        "activation_energy_ev": ea,
        "ref_temp_c": ref_temp,
        "cycling_factor_per_cycle": cycling_factor,
        "phases": phase_rows,
        "failure_rate": rate,
    }))
}

fn cmd_library(file: &str, t: Option<f64>) -> Result<Value, String> {
    let text = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let parts = prediction::library_from_csv(&text)?;
    let part_rows: Vec<Value> = parts
        .iter()
        .map(|p| {
            json!({
                "name": p.name,
                "base_rate_per_million_hours": p.base_rate_per_million_hours,
                "quantity": p.quantity,
                "stress_factor": p.stress_factor,
                "quality_factor": p.quality_factor,
                "failure_rate_per_million_hours": prediction::part_failure_rate_per_million_hours(p),
            })
        })
        .collect();
    let mut out = json!({
        "file": file,
        "parts": part_rows,
        "system_failure_rate_per_million_hours": prediction::system_failure_rate_per_million_hours(&parts),
        "system_mtbf_hours": jnum(prediction::system_mtbf_hours(&parts)),
    });
    if let Some(tt) = t {
        out["t_hours"] = json!(tt);
        out["system_reliability"] = json!(prediction::system_reliability(&parts, tt));
    }
    Ok(out)
}

fn cmd_fracas(file: &str, total_hours: Option<f64>) -> Result<Value, String> {
    let text = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let incidents = fracas::incidents_from_csv(&text)?;
    let by_status: Vec<Value> = fracas::count_by_status(&incidents)
        .iter()
        .map(|(s, c)| json!({"status": s, "count": c}))
        .collect();
    let by_mode: Vec<Value> = fracas::count_by_mode(&incidents)
        .iter()
        .map(|(m, c)| json!({"mode": m, "count": c}))
        .collect();
    let mtbf = match total_hours {
        Some(h) => fracas::mtbf_hours(h, &incidents).map(jnum).unwrap_or(Value::Null),
        None => Value::Null,
    };
    Ok(json!({
        "file": file,
        "total_incidents": incidents.len(),
        "counts_by_status": by_status,
        "counts_by_mode": by_mode,
        "total_hours": total_hours,
        "mtbf_hours": mtbf,
    }))
}

fn cmd_degradation(
    file: &str,
    threshold: f64,
    model: &str,
    dist: &str,
    decreasing: bool,
) -> Result<Value, String> {
    let (cols, rows) = read_csv_rows(file)?;
    let ui = col_index(&cols, &["unit", "unit_id"]);
    let ti = col_index(&cols, &["time", "times", "hours"]).ok_or("csv needs a time column")?;
    let vi = col_index(&cols, &["value", "measurement"]).ok_or("csv needs a value column")?;
    let mut unit_names: Vec<String> = Vec::new();
    let mut units: Vec<(Vec<f64>, Vec<f64>)> = Vec::new();
    for row in &rows {
        let unit = match ui {
            Some(i) => row[i].clone(),
            None => "1".to_string(),
        };
        let pos = match unit_names.iter().position(|u| *u == unit) {
            Some(p) => p,
            None => {
                unit_names.push(unit);
                units.push((Vec::new(), Vec::new()));
                units.len() - 1
            }
        };
        units[pos].0.push(row[ti].parse::<f64>().map_err(|_| "bad time value")?);
        units[pos].1.push(row[vi].parse::<f64>().map_err(|_| "bad value")?);
    }
    let res = degradation::degradation_analysis(&units, threshold, model, dist, !decreasing)?;
    let path_rows: Vec<Value> = unit_names
        .iter()
        .zip(res.paths.iter())
        .map(|(u, (a, b))| json!({"unit": u, "a": a, "b": b}))
        .collect();
    Ok(json!({
        "file": file,
        "model": res.model,
        "dist": dist,
        "threshold": threshold,
        "increasing": !decreasing,
        "paths": path_rows,
        "pseudo_times": res.pseudo_times,
        "failed": res.failed,
        "fit": serde_json::to_value(&res.fit).unwrap_or(Value::Null),
    }))
}

fn main() {
    let cli = Cli::parse();
    let result: Result<Value, String> = (|| {
        match &cli.cmd {
            Cmd::Selftest => std::process::exit(selftest()),
            Cmd::Fit { file, csv, dist, rank, beta } => {
                let path = resolve_file_arg(file, csv, "fit")?;
                let (times, failed) = read_csv_life(&path)?;
                let fit = if dist == "weibull1" {
                    let b = beta.ok_or("weibull1 requires --beta <value>")?;
                    dist_mod::fit_weibull1(&times, Some(&failed), b)?
                } else if *rank {
                    dist_mod::fit_rank_regression(dist, &times, Some(&failed))?
                } else {
                    dist_mod::fit_mle(dist, &times, Some(&failed))?
                };
                Ok(serde_json::to_value(&fit).unwrap())
            }
            Cmd::Rel { dist, t, params } => {
                let p = parse_params(params)?;
                Ok(json!({
                    "dist": dist,
                    "t": t,
                    "reliability": dist_mod::reliability(dist, &p, *t),
                    "unreliability": dist_mod::cdf(dist, &p, *t),
                    "hazard": dist_mod::hazard(dist, &p, *t),
                    "b10": dist_mod::quantile(dist, &p, 0.10),
                    "mean_life": dist_mod::mean_life(dist, &p),
                }))
            }
            Cmd::Rbd { file, t } => {
                let text = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
                let spec: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
                let spec = system::rbd_from_json(spec)?;
                let r = system::evaluate_rbd(&spec, *t)?;
                Ok(json!({"reliability": r, "t": t}))
            }
            Cmd::Availability { file, horizon, sims, seed } => {
                let text = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
                let spec: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
                let res = system::simulate_availability(&spec, *horizon, *sims, *seed, 50);
                Ok(json!({
                    "mean_availability": res.mean_availability,
                    "expected_downtime": res.expected_downtime,
                    "mean_failures": res.mean_failures,
                    "mean_repairs": res.mean_repairs,
                }))
            }
            Cmd::Growth { file, csv, t_end } => {
                let path = resolve_file_arg(file, csv, "growth")?;
                let times = read_csv_column(&path)?;
                let ca = growth::fit_crow_amsaa(&times, *t_end);
                let du = growth::fit_duane(&times).map_err(|e| e)?;
                Ok(json!({
                    "crow_amsaa": {"beta": ca.beta, "lambda": ca.lam,
                        "instantaneous_mtbf_at_T": ca.instantaneous_mtbf(ca.t_end)},
                    "duane": {"alpha": du.alpha,
                        "instantaneous_mtbf_at_T": du.instantaneous_mtbf_at(ca.t_end)},
                }))
            }
            Cmd::Af { model, values } => {
                if values.len() != 3 {
                    return Err("need exactly 3 values".into());
                }
                let af = match model.as_str() {
                    "arrhenius" => alt_mod::acceleration_factor_arrhenius(values[0], values[1], values[2]),
                    "ipl" => alt_mod::acceleration_factor_ipl(values[0], values[1], values[2]),
                    _ => return Err("model must be arrhenius or ipl".into()),
                };
                Ok(json!({"model": model, "acceleration_factor": af}))
            }
            Cmd::AltMle { file, csv, model, t, use_stress, alpha } => {
                let path = resolve_file_arg(file, csv, "alt-mle")?;
                cmd_alt_mle(&path, model, *t, use_stress, *alpha)
            }
            Cmd::Competing { file, csv, dist, t } => {
                let path = resolve_file_arg(file, csv, "competing")?;
                cmd_competing(&path, dist, *t)
            }
            Cmd::Standby { dist, n, dormancy, switch_rel, t, params } => {
                let p = parse_params(params)?;
                Ok(json!({
                    "dist": dist,
                    "t": t,
                    "n": n,
                    "dormancy": dormancy,
                    "switch_reliability": switch_rel,
                    "reliability_quad": standby::standby_reliability_quad(*t, dist, &p, *n, *dormancy, *switch_rel),
                    "reliability_system": system::standby_reliability(*t, dist, &p, *n, *dormancy, *switch_rel),
                    "reliability_mc": system::standby_reliability_mc(*t, dist, &p, *n, *dormancy, *switch_rel, 20000, 1),
                    "mc_sims": 20000,
                    "mc_seed": 1,
                }))
            }
            Cmd::Flow { file, sims, seed } => {
                let text = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
                let spec: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
                let res = flowchart::simulate_flow(&spec, *sims, *seed)?;
                Ok(json!({
                    "mean_time": res.mean_time,
                    "sd_time": res.sd_time,
                    "p10": res.p10,
                    "p50": res.p50,
                    "p90": res.p90,
                    "min_time": res.min_time,
                    "max_time": res.max_time,
                    "mean_cost": res.mean_cost,
                    "n_sim": res.n_sim,
                }))
            }
            Cmd::Fmea { file } => {
                let text = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
                let mut rows = fmea::rows_from_csv(&text)?;
                fmea::sort_by_rpn_desc(&mut rows);
                let out: Vec<Value> = rows
                    .iter()
                    .map(|r| {
                        json!({
                            "item": r.item,
                            "function": r.function,
                            "failure_mode": r.failure_mode,
                            "effect": r.effect,
                            "cause": r.cause,
                            "severity": r.severity,
                            "occurrence": r.occurrence,
                            "detection": r.detection,
                            "rpn": r.rpn(),
                            "recommended_action": r.recommended_action,
                            "revised_severity": r.revised_severity,
                            "revised_occurrence": r.revised_occurrence,
                            "revised_detection": r.revised_detection,
                            "revised_rpn": r.revised_rpn(),
                        })
                    })
                    .collect();
                Ok(json!({"rows": out}))
            }
            Cmd::Warranty { sales, returns, current, horizon } => {
                cmd_warranty(sales, returns, *current, *horizon)
            }
            Cmd::Faulttree { file } => {
                let text = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
                let node: FtNode = serde_json::from_str(&text).map_err(|e| e.to_string())?;
                let cuts: Vec<Vec<String>> = faulttree::minimal_cut_sets(&node)
                    .iter()
                    .map(|s| s.iter().cloned().collect())
                    .collect();
                Ok(json!({
                    "top_probability": faulttree::top_probability(&node),
                    "minimal_cut_sets": cuts,
                }))
            }
            Cmd::Predict { file, t } => cmd_predict(file, *t),
            Cmd::Doe { k, y, csv } => {
                let ys = if let Some(path) = csv {
                    read_csv_column(path)?
                } else if let Some(y) = y {
                    parse_comma_f64s(y)?
                } else {
                    return Err("doe needs responses via --y or --csv".into());
                };
                let design = doe::factorial_design(*k);
                let effects = doe::analyze_factorial(*k, &ys)?;
                let eff: Vec<Value> = effects
                    .iter()
                    .map(|e| json!({"name": e.name, "terms": e.terms, "effect": e.effect, "sum_squares": e.sum_squares}))
                    .collect();
                Ok(json!({"k": k, "design": design, "effects": eff}))
            }
            Cmd::Wizard { file, csv } => {
                let path = resolve_file_arg(file, csv, "wizard")?;
                cmd_wizard(&path)
            }
            Cmd::Qcp { dist_pos, dist, t, bx, params_opt, params_pos } => {
                let d = dist
                    .clone()
                    .or_else(|| dist_pos.clone())
                    .ok_or("qcp needs a distribution (--dist or positional)")?;
                let mut tokens: Vec<String> = params_opt.clone();
                // When the distribution came from --dist, a value in
                // the positional dist slot is a parameter token.
                if dist.is_some() {
                    if let Some(extra) = dist_pos {
                        tokens.push(extra.clone());
                    }
                }
                tokens.extend(params_pos.iter().cloned());
                let p = parse_params_flexible(&d, &tokens)?;
                let r = quickcalc::quick_calc(&d, &p, *t, *bx)?;
                Ok(json!({
                    "dist": d,
                    "params": p,
                    "t": t,
                    "bx": bx,
                    "reliability": r.reliability,
                    "unreliability": r.unreliability,
                    "hazard": r.hazard,
                    "mean_life": r.mean_life,
                    "bx_life": r.bx_life,
                    "pdf": r.pdf,
                }))
            }
            Cmd::Importance { file_pos, json, file_flag, t } => {
                cmd_importance(file_pos, json, file_flag, *t)
            }
            Cmd::Markov { file_pos, json, absorbing, times, start } => {
                cmd_markov(file_pos, json, absorbing, times, *start)
            }
            Cmd::Predict217 { category, input, env, quality } => {
                cmd_predict217(category, input, env, quality)
            }
            Cmd::Sr332 { method } => cmd_sr332(method),
            Cmd::Mission { base_rate, ea, ref_temp, cycling_factor, phase } => {
                cmd_mission(*base_rate, *ea, *ref_temp, *cycling_factor, phase)
            }
            Cmd::Allocate { target, n, weights } => {
                let (method, targets) = if let Some(w) = weights {
                    let ws = parse_comma_f64s(w)?;
                    ("weighted", prediction::allocate_weighted(*target, &ws))
                } else if let Some(n) = n {
                    ("equal", prediction::allocate_equal(*target, *n))
                } else {
                    return Err("allocate needs --n for equal allocation or --weights".into());
                };
                let product: f64 = targets.iter().product();
                Ok(json!({
                    "target": target,
                    "method": method,
                    "subsystem_targets": targets,
                    "product": product,
                }))
            }
            Cmd::Library { file_pos, csv, t } => {
                let path = resolve_file_arg(file_pos, csv, "library")?;
                cmd_library(&path, *t)
            }
            Cmd::Fracas { file_pos, csv, total_hours } => {
                let path = resolve_file_arg(file_pos, csv, "fracas")?;
                cmd_fracas(&path, *total_hours)
            }
            Cmd::Degradation { file, csv, threshold, model, dist, decreasing } => {
                let path = resolve_file_arg(file, csv, "degradation")?;
                cmd_degradation(&path, *threshold, model, dist, *decreasing)
            }
        }
    })();
    match result {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
        Err(e) => {
            eprintln!("error: {}", e);
            exit(1);
        }
    }
}
