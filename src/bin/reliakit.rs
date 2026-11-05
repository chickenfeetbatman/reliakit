//! ReliaKit command line interface.
//!
//! Subcommands print JSON results to stdout so the engine is
//! scriptable. Input life data: CSV with a header row containing
//! `time` and optionally `failed` (1/0, true/false) columns.

use clap::{Parser, Subcommand};
use reliakit::alt as alt_mod;
use reliakit::bayes;
use reliakit::competing;
use reliakit::distributions as dist_mod;
use reliakit::doe;
use reliakit::faulttree::{self, FtNode};
use reliakit::flowchart;
use reliakit::fmea;
use reliakit::growth;
use reliakit::prediction::{self, PartEntry};
use reliakit::standby;
use reliakit::system;
use reliakit::warranty::{self, WarrantyRecord};
use serde_json::{json, Value};
use std::collections::BTreeMap;
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
        file: String,
        /// weibull2, weibull3, lognormal, normal, exponential, gamma, gumbel.
        #[arg(long, default_value = "weibull2")]
        dist: String,
        /// rank (median rank regression) instead of mle.
        #[arg(long)]
        rank: bool,
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
        file: String,
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
        file: String,
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
        file: String,
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
        y: String,
    },
    /// Run the built-in self test (known closed-form values).
    Selftest,
}

fn read_csv_life(path: &str) -> Result<(Vec<f64>, Vec<bool>), String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header = lines.next().ok_or("empty csv")?;
    let cols: Vec<String> = header.split(',').map(|s| s.trim().to_lowercase()).collect();
    let ti = cols
        .iter()
        .position(|c| c == "time" || c == "times" || c == "life" || c == "hours")
        .unwrap_or(0);
    let fi = cols
        .iter()
        .position(|c| c == "failed" || c == "failure" || c == "event");
    let mut times = Vec::new();
    let mut failed = Vec::new();
    for line in lines {
        let cells: Vec<&str> = line.split(',').collect();
        let t: f64 = cells[ti].trim().parse().map_err(|_| "bad time value")?;
        times.push(t);
        let f = match fi {
            Some(i) => {
                let v = cells[i].trim().to_lowercase();
                matches!(v.as_str(), "1" | "true" | "t" | "yes" | "f" | "failed")
            }
            None => true,
        };
        failed.push(f);
    }
    Ok((times, failed))
}

fn read_csv_column(path: &str) -> Result<Vec<f64>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let first = line.split(',').next().unwrap_or("").trim();
        if i == 0 {
            if first.parse::<f64>().is_err() {
                continue; // header
            }
        }
        if let Ok(v) = first.parse::<f64>() {
            out.push(v);
        }
    }
    Ok(out)
}

/// Generic CSV reader: returns (header names lowercased, data rows as
/// trimmed string cells). Blank lines are skipped.
fn read_csv_rows(path: &str) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
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

fn main() {
    let cli = Cli::parse();
    let result: Result<Value, String> = (|| {
        match &cli.cmd {
            Cmd::Selftest => std::process::exit(selftest()),
            Cmd::Fit { file, dist, rank } => {
                let (times, failed) = read_csv_life(file)?;
                let fit = if *rank {
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
            Cmd::Growth { file, t_end } => {
                let times = read_csv_column(file)?;
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
            Cmd::AltMle { file, model, t, use_stress, alpha } => {
                cmd_alt_mle(file, model, *t, use_stress, *alpha)
            }
            Cmd::Competing { file, dist, t } => cmd_competing(file, dist, *t),
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
            Cmd::Doe { k, y } => {
                let ys = parse_comma_f64s(y)?;
                let design = doe::factorial_design(*k);
                let effects = doe::analyze_factorial(*k, &ys)?;
                let eff: Vec<Value> = effects
                    .iter()
                    .map(|e| json!({"name": e.name, "terms": e.terms, "effect": e.effect, "sum_squares": e.sum_squares}))
                    .collect();
                Ok(json!({"k": k, "design": design, "effects": eff}))
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
