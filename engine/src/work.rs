//! Working state and computation for the Slint front end.
//!
//! This module is a port of the v0.2.0 desktop app's non-UI logic:
//! the project model, the per-item state capture and restore (the
//! reliakit-project-2 JSON format, unchanged), the eight examples,
//! the bundled dataset loading, every screen's computation, and the
//! six data flows between items. All math goes through the reliakit
//! library, exactly as before; only the presentation layer changed.

use reliakit::alt as alt_mod;
use reliakit::bayes;
use reliakit::competing;
use reliakit::datasets;
use reliakit::degradation;
use reliakit::distributions as dist_mod;
use reliakit::doe;
use reliakit::faulttree::{self, FtNode};
use reliakit::flowchart;
use reliakit::fmea::{self, DvprEntry, FmeaRow, FmeaType, FmeaWorksheet};
use reliakit::fracas::{self, Incident, IncidentStatus};
use reliakit::growth;
use reliakit::importance::{self, BlockImportance};
use reliakit::markov::MarkovModel;
use reliakit::mcmc;
use reliakit::prediction::{self, MissionPhase, PartEntry};
use reliakit::quickcalc;
use reliakit::rcm::{Consequence, RcmMode};
use reliakit::standards217::{self, FactorInput};
use reliakit::system;
use reliakit::warranty::{self, WarrantyRecord};
use reliakit::wizard::{self, FitRank};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const EXAMPLE3_JSON: &str = r#"{"type":"series","children":[{"type":"block","name":"power supply","dist":"weibull2","params":{"beta":1.4,"eta":8000},"mtbf":6000,"mttr":24},{"type":"standby_group","name":"pumps","n_units":2,"dist":"exponential","params":{"theta":5000},"mtbf":5000,"mttr":8,"dormancy":0.0,"switch_reliability":0.98,"swap_time":0.5},{"type":"parallel","children":[{"type":"block","name":"controller A","dist":"exponential","params":{"theta":12000},"mtbf":12000,"mttr":4},{"type":"block","name":"controller B","dist":"exponential","params":{"theta":12000},"mtbf":12000,"mttr":4}]}]}"#;

// ---------------- text parsing helpers ----------------

pub fn fmt_num(v: f64) -> String {
    format!("{}", v)
}

#[derive(Clone, Debug)]
pub struct ValidationError {
    pub field: String,
    pub token: String,
    pub reason: String,
}
impl ValidationError {
    pub fn new(field: &str, token: &str, reason: &str) -> Self {
        Self {
            field: field.to_string(),
            token: token.to_string(),
            reason: reason.to_string(),
        }
    }
    pub fn message(&self) -> String {
        format!(
            "Invalid value '{}' in {}: {}",
            self.token, self.field, self.reason
        )
    }
}
pub type VResult<T> = Result<T, ValidationError>;

fn check_number_token(tok: &str, field: &str) -> VResult<f64> {
    let low = tok.trim().to_ascii_lowercase();
    if low.is_empty() {
        return Err(ValidationError::new(field, tok, "empty required field"));
    }
    if matches!(
        low.as_str(),
        "nan" | "+nan" | "-nan" | "inf" | "+inf" | "-inf" | "infinity" | "+infinity" | "-infinity"
    ) {
        return Err(ValidationError::new(
            field,
            tok,
            "non-finite value is not allowed",
        ));
    }
    match tok.trim().parse::<f64>() {
        Ok(v) if v.is_finite() => Ok(v),
        Ok(_) => Err(ValidationError::new(
            field,
            tok,
            "non-finite value (overflow)",
        )),
        Err(_) => Err(ValidationError::new(field, tok, "not a number")),
    }
}
pub fn valid_f64(s: &str, field: &str) -> VResult<f64> {
    if s.trim().is_empty() {
        return Err(ValidationError::new(field, "", "empty required field"));
    }
    check_number_token(s.trim(), field)
}
pub fn valid_f64_nonneg(s: &str, field: &str) -> VResult<f64> {
    let v = valid_f64(s, field)?;
    if v < 0.0 {
        return Err(ValidationError::new(
            field,
            s.trim(),
            "negative value is not allowed",
        ));
    }
    Ok(v)
}
pub fn valid_int(s: &str, field: &str) -> VResult<i64> {
    let tr = s.trim();
    if tr.is_empty() {
        return Err(ValidationError::new(field, "", "empty required field"));
    }
    let low = tr.to_ascii_lowercase();
    if matches!(
        low.as_str(),
        "nan" | "inf" | "-inf" | "+inf" | "infinity" | "-infinity" | "+infinity"
    ) {
        return Err(ValidationError::new(
            field,
            tr,
            "non-finite value is not allowed",
        ));
    }
    if let Ok(v) = tr.parse::<i64>() {
        return Ok(v);
    }
    // fractional or malformed
    if let Ok(v) = tr.parse::<f64>() {
        if !v.is_finite() {
            return Err(ValidationError::new(
                field,
                tr,
                "non-finite value (overflow)",
            ));
        }
        return Err(ValidationError::new(
            field,
            tr,
            "fractional value in integer field",
        ));
    }
    Err(ValidationError::new(field, tr, "not an integer"))
}
pub fn valid_usize(s: &str, field: &str) -> VResult<usize> {
    let v = valid_int(s, field)?;
    if v < 0 {
        return Err(ValidationError::new(
            field,
            s.trim(),
            "negative value is not allowed",
        ));
    }
    Ok(v as usize)
}
pub fn valid_parse_numbers(text: &str, field: &str) -> VResult<Vec<f64>> {
    if text.trim().is_empty() {
        return Err(ValidationError::new(field, "", "empty required field"));
    }
    let mut out = Vec::new();
    for tok in text.split(|c: char| c == ',' || c == ';' || c.is_whitespace()) {
        let tok = tok.trim();
        if tok.is_empty() {
            continue;
        }
        out.push(check_number_token(tok, field)?);
    }
    if out.is_empty() {
        return Err(ValidationError::new(field, "", "empty required field"));
    }
    Ok(out)
}
pub fn valid_parse_numbers_nonneg(text: &str, field: &str) -> VResult<Vec<f64>> {
    let v = valid_parse_numbers(text, field)?;
    for (i, x) in v.iter().enumerate() {
        if *x < 0.0 {
            let toks: Vec<&str> = text
                .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
                .filter(|s| !s.trim().is_empty())
                .collect();
            return Err(ValidationError::new(
                field,
                toks.get(i).copied().unwrap_or(""),
                "negative time is not allowed",
            ));
        }
    }
    Ok(v)
}
pub fn valid_parse_life(text: &str, field: &str) -> VResult<(Vec<f64>, Vec<bool>)> {
    if text.trim().is_empty() {
        return Err(ValidationError::new(field, "", "empty required field"));
    }
    let mut times = Vec::new();
    let mut failed = Vec::new();
    for tok in text.split(|c: char| c == ',' || c == ';' || c.is_whitespace()) {
        let tok = tok.trim();
        if tok.is_empty() {
            continue;
        }
        let (num, cens) = if let Some(s) = tok.strip_suffix(['c', 'C', '+']) {
            (s, true)
        } else {
            (tok, false)
        };
        if num.is_empty() {
            return Err(ValidationError::new(field, tok, "invalid censoring flag"));
        }
        let v = check_number_token(num, field).map_err(|mut e| {
            e.token = tok.to_string();
            e
        })?;
        if v < 0.0 {
            return Err(ValidationError::new(
                field,
                tok,
                "negative time is not allowed",
            ));
        }
        times.push(v);
        failed.push(!cens);
    }
    if times.is_empty() {
        return Err(ValidationError::new(field, "", "empty required field"));
    }
    Ok((times, failed))
}
pub fn valid_parse_params(text: &str, field: &str) -> VResult<dist_mod::Params> {
    let mut p = dist_mod::Params::new();
    for pair in text.split(|c: char| c == ',' || c == ';' || c.is_whitespace()) {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        match pair.split_once('=') {
            Some((k, v)) => {
                let val = check_number_token(v.trim(), field).map_err(|mut e| {
                    e.token = pair.to_string();
                    e
                })?;
                p.insert(k.trim().to_string(), val);
            }
            None => {
                return Err(ValidationError::new(field, pair, "expected name=value"));
            }
        }
    }
    Ok(p)
}
#[derive(Clone, Debug)]
pub struct FitStatusInfo {
    pub status: String,
    pub note: String,
    pub warnings: Vec<String>,
}
pub fn fit_status_info(fit: &dist_mod::FitResult) -> FitStatusInfo {
    FitStatusInfo {
        status: fit.status_str().to_string(),
        note: fit.status_note.clone(),
        warnings: fit.warnings.clone(),
    }
}
pub fn clean_val(v: f64, reason: &str) -> String {
    if v.is_finite() {
        format!("{:.4}", v)
    } else {
        format!("unavailable: {}", reason)
    }
}

pub fn parse_numbers(text: &str) -> Vec<f64> {
    text.split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|s| !s.trim().is_empty())
        .filter_map(|s| s.trim().parse::<f64>().ok())
        .collect()
}

/// Life data text: one value per token; a trailing 'c' or '+' marks a
/// censored (suspended) unit.
pub fn parse_life(text: &str) -> (Vec<f64>, Vec<bool>) {
    let mut times = Vec::new();
    let mut failed = Vec::new();
    for tok in text.split(|c: char| c == ',' || c == ';' || c.is_whitespace()) {
        let tok = tok.trim();
        if tok.is_empty() {
            continue;
        }
        let (num, cens) = if let Some(stripped) = tok.strip_suffix(['c', 'C', '+']) {
            (stripped, true)
        } else {
            (tok, false)
        };
        if let Ok(v) = num.parse::<f64>() {
            times.push(v);
            failed.push(!cens);
        }
    }
    (times, failed)
}

pub fn format_life(times: &[f64], failed: &[bool]) -> String {
    times
        .iter()
        .zip(failed)
        .map(|(t, f)| {
            if *f {
                fmt_num(*t)
            } else {
                format!("{}c", fmt_num(*t))
            }
        })
        .collect::<Vec<String>>()
        .join(", ")
}

pub fn parse_params_text(text: &str) -> dist_mod::Params {
    let mut p = BTreeMap::new();
    for pair in text.split(|c: char| c == ',' || c == ';' || c.is_whitespace()) {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        if let Some((k, v)) = pair.split_once('=') {
            if let Ok(x) = v.trim().parse::<f64>() {
                p.insert(k.trim().to_string(), x);
            }
        }
    }
    p
}

pub fn params_to_json(p: &dist_mod::Params) -> Value {
    let mut m = serde_json::Map::new();
    for (k, v) in p {
        m.insert(k.clone(), json!(v));
    }
    Value::Object(m)
}

pub fn params_to_text(v: &Value) -> String {
    match v.as_object() {
        Some(obj) => obj
            .iter()
            .filter_map(|(k, val)| val.as_f64().map(|x| format!("{}={}", k, x)))
            .collect::<Vec<String>>()
            .join(", "),
        None => String::new(),
    }
}

pub fn fnum(s: &str, default: f64) -> f64 {
    s.trim().parse::<f64>().unwrap_or(default)
}

pub fn inum(s: &str, default: usize) -> usize {
    s.trim().parse::<usize>().unwrap_or(default)
}

pub fn opt_num(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        t.parse::<f64>().ok()
    }
}

/// Split a stored result string back into (label, value) rows for
/// display as cards. Lines are "Label: value" as produced by the
/// compute functions; a line without a colon becomes a plain line.
pub fn rows_of(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| match l.split_once(": ") {
            Some((a, b)) => (a.to_string(), b.to_string()),
            None => (String::new(), l.to_string()),
        })
        .collect()
}

pub fn rows_text(rows: &[(String, String)]) -> String {
    rows.iter()
        .map(|(a, b)| {
            if a.is_empty() {
                b.clone()
            } else {
                format!("{}: {}", a, b)
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

// ---------------- directories and bundled data ----------------

pub fn datasets_dir() -> PathBuf {
    let p = PathBuf::from("datasets");
    if p.is_dir() {
        return p;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("datasets")
}

pub fn mil217_dir() -> PathBuf {
    let p = PathBuf::from("libraries").join("mil217");
    if p.is_dir() {
        return p;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("libraries")
        .join("mil217")
}

/// The 16 bundled dataset files documented in datasets/README.md.
pub const BUNDLED_CSV: [&str; 16] = [
    "reliasoft_alt_circuit_boards.csv",
    "reliasoft_alt_degradation_led.csv",
    "reliasoft_alt_paper_clips.csv",
    "reliasoft_alt_tensile.csv",
    "reliasoft_alt_two_stress.csv",
    "reliasoft_beta_shape_example.csv",
    "reliasoft_degradation_mosfet.csv",
    "reliasoft_degradation_turbine_blades.csv",
    "reliasoft_growth_crow_amsaa.csv",
    "reliasoft_growth_test_fix_test.csv",
    "reliasoft_life_data_example.csv",
    "reliasoft_unbiasing_complete.csv",
    "reliasoft_unbiasing_suspended.csv",
    "reliasoft_warranty_nevada.csv",
    "reliasoft_warranty_time_to_failure.csv",
    "smrd_bearingcage.csv",
];

pub const MIL217_FILES: [&str; 18] = [
    "capacitors.json",
    "circuit_breakers.json",
    "coils.json",
    "connections.json",
    "connector_sockets.json",
    "connectors.json",
    "elapsed_time_meters.json",
    "electronic_filters.json",
    "fuses.json",
    "interconnection_assemblies_pth.json",
    "miscellaneous_parts.json",
    "quartz_crystals.json",
    "relays_mechanical.json",
    "relays_solid_state.json",
    "resistors.json",
    "switches.json",
    "synchros_and_resolvers.json",
    "transformers.json",
];

/// All distributions the library can evaluate, in spec order.
pub const ALL_DISTS: [&str; 14] = [
    "weibull2",
    "weibull3",
    "lognormal",
    "normal",
    "exponential",
    "gamma",
    "gumbel",
    "loglogistic",
    "logistic",
    "gompertz",
    "gengamma",
    "weibull1",
    "exponential2",
    "mixed_weibull",
];

/// Split CSV text into (header, rows), skipping leading '#' comment
/// lines and blank lines. Fields are split on commas and trimmed;
/// the bundled files use no quoted fields.
pub fn csv_table(text: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let mut lines = text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'));
    let header: Vec<String> = match lines.next() {
        Some(h) => h.split(',').map(|c| c.trim().to_lowercase()).collect(),
        None => return (Vec::new(), Vec::new()),
    };
    let rows: Vec<Vec<String>> = lines
        .map(|l| l.split(',').map(|c| c.trim().to_string()).collect())
        .collect();
    (header, rows)
}

pub fn col_idx(header: &[String], name: &str) -> Option<usize> {
    header.iter().position(|h| h == name)
}

// ---------------- project model ----------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ItemKind {
    LifeData,
    LimitedData,
    Mcmc,
    Degradation,
    Alt,
    Growth,
    Competing,
    Rbd,
    FaultTree,
    Fmea,
    Warranty,
    Prediction,
    Doe,
    Flowchart,
    Markov,
    Qcp,
    Rcm,
    Fracas,
}

pub const KIND_DEFS: [(ItemKind, &str, &str); 18] = [
    (ItemKind::LifeData, "Life Data", "life"),
    (ItemKind::LimitedData, "Limited Data", "limited"),
    (ItemKind::Mcmc, "MCMC", "mcmc"),
    (ItemKind::Degradation, "Degradation", "degradation"),
    (ItemKind::Alt, "ALT", "alt"),
    (ItemKind::Growth, "Growth", "growth"),
    (ItemKind::Competing, "Competing Risks", "competing"),
    (ItemKind::Rbd, "RBD", "rbd"),
    (ItemKind::FaultTree, "Fault Tree", "faulttree"),
    (ItemKind::Fmea, "FMEA", "fmea"),
    (ItemKind::Warranty, "Warranty", "warranty"),
    (ItemKind::Prediction, "Prediction", "prediction"),
    (ItemKind::Doe, "DOE", "doe"),
    (ItemKind::Flowchart, "Flowchart", "flowchart"),
    (ItemKind::Markov, "Markov", "markov"),
    (ItemKind::Qcp, "QCP", "qcp"),
    (ItemKind::Rcm, "RCM", "rcm"),
    (ItemKind::Fracas, "FRACAS", "fracas"),
];

impl ItemKind {
    pub fn label(self) -> &'static str {
        KIND_DEFS
            .iter()
            .find(|(k, _, _)| *k == self)
            .map(|(_, l, _)| *l)
            .unwrap_or("")
    }
    pub fn icon(self) -> &'static str {
        KIND_DEFS
            .iter()
            .find(|(k, _, _)| *k == self)
            .map(|(_, _, i)| *i)
            .unwrap_or("life")
    }
    pub fn key(self) -> &'static str {
        // Screen key used by the .slint shell.
        match self {
            ItemKind::LifeData => "life",
            ItemKind::LimitedData => "limited",
            ItemKind::Mcmc => "mcmc",
            ItemKind::Degradation => "degradation",
            ItemKind::Alt => "alt",
            ItemKind::Growth => "growth",
            ItemKind::Competing => "competing",
            ItemKind::Rbd => "rbd",
            ItemKind::FaultTree => "faulttree",
            ItemKind::Fmea => "fmea",
            ItemKind::Warranty => "warranty",
            ItemKind::Prediction => "prediction",
            ItemKind::Doe => "doe",
            ItemKind::Flowchart => "flowchart",
            ItemKind::Markov => "markov",
            ItemKind::Qcp => "qcp",
            ItemKind::Rcm => "rcm",
            ItemKind::Fracas => "fracas",
        }
    }
    pub fn from_label(s: &str) -> Option<ItemKind> {
        KIND_DEFS
            .iter()
            .find(|(_, l, _)| *l == s)
            .map(|(k, _, _)| *k)
    }
    pub fn from_key(s: &str) -> Option<ItemKind> {
        let key = s.to_lowercase().replace([' ', '-'], "");
        KIND_DEFS
            .iter()
            .find(|(k, l, _)| {
                l.to_lowercase().replace([' ', '-'], "") == key
                    || format!("{:?}", k).to_lowercase() == key
            })
            .map(|(k, _, _)| *k)
    }
}

#[derive(Clone)]
pub struct ProjectItem {
    pub id: u64,
    pub kind: ItemKind,
    pub name: String,
    pub state: Value,
}

/// Which bundled dataset file loads into which item kind, following
/// the categories in datasets/README.md.
pub fn dataset_kind(file: &str) -> ItemKind {
    if file.contains("_alt_") && !file.contains("degradation") {
        ItemKind::Alt
    } else if file.contains("degradation") {
        ItemKind::Degradation
    } else if file.contains("growth") {
        ItemKind::Growth
    } else if file.contains("warranty_nevada") {
        ItemKind::Warranty
    } else {
        ItemKind::LifeData
    }
}

/// Pure project serialization: one JSON value for the whole project.
pub fn project_to_value(
    project_name: &str,
    items: &[ProjectItem],
    selected: Option<usize>,
) -> Value {
    json!({
        "format": "reliakit-project-2",
        "project_name": project_name,
        "selected_index": selected,
        "items": items.iter().map(|it| json!({
            "id": it.id,
            "kind": format!("{:?}", it.kind),
            "name": it.name,
            "state": it.state,
        })).collect::<Vec<Value>>(),
    })
}

/// Inverse of project_to_value. Returns None if the value is not a
/// v2 project (legacy files are handled by the caller).
pub fn project_from_value(v: &Value) -> Option<(String, Vec<ProjectItem>, Option<usize>)> {
    if v.get("format").and_then(|x| x.as_str()) != Some("reliakit-project-2") {
        return None;
    }
    let name = v
        .get("project_name")
        .and_then(|x| x.as_str())
        .unwrap_or("Untitled project")
        .to_string();
    let mut items: Vec<ProjectItem> = Vec::new();
    for (i, iv) in v
        .get("items")
        .and_then(|x| x.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[])
        .iter()
        .enumerate()
    {
        let kind = iv
            .get("kind")
            .and_then(|x| x.as_str())
            .and_then(ItemKind::from_key)?;
        items.push(ProjectItem {
            id: iv
                .get("id")
                .and_then(|x| x.as_u64())
                .unwrap_or((i + 1) as u64),
            kind,
            name: iv
                .get("name")
                .and_then(|x| x.as_str())
                .unwrap_or(kind.label())
                .to_string(),
            state: iv.get("state").cloned().unwrap_or(json!({})),
        });
    }
    let sel = v
        .get("selected_index")
        .and_then(|x| x.as_u64())
        .map(|i| i as usize)
        .filter(|i| *i < items.len());
    Some((name, items, sel))
}

// ---------------- RCM row (UI-side record, same JSON shape) ----------------

#[derive(Clone)]
pub struct RcmRowUi {
    pub asset: String,
    pub function: String,
    pub mode: String,
    pub consequence: String,
    pub monitorable: bool,
    pub age_related: bool,
    pub restorable: bool,
    pub rate: String,
    pub alpha: String,
    pub beta: String,
    pub hours: String,
}

impl RcmRowUi {
    pub fn to_json(&self) -> Value {
        json!({
            "asset": self.asset, "function": self.function, "mode": self.mode,
            "consequence": self.consequence, "monitorable": self.monitorable,
            "age_related": self.age_related, "restorable": self.restorable,
            "rate": self.rate, "alpha": self.alpha, "beta": self.beta, "hours": self.hours,
        })
    }
    pub fn from_json(v: &Value) -> Self {
        let g = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        RcmRowUi {
            asset: g("asset"),
            function: g("function"),
            mode: g("mode"),
            consequence: if g("consequence").is_empty() {
                "Operational".into()
            } else {
                g("consequence")
            },
            monitorable: v
                .get("monitorable")
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
            age_related: v
                .get("age_related")
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
            restorable: v
                .get("restorable")
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
            rate: g("rate"),
            alpha: g("alpha"),
            beta: g("beta"),
            hours: g("hours"),
        }
    }
    pub fn to_mode(&self) -> RcmMode {
        RcmMode {
            asset: self.asset.clone(),
            function: self.function.clone(),
            failure_mode: self.mode.clone(),
            consequence: Consequence::from_str(&self.consequence)
                .unwrap_or(Consequence::Operational),
            condition_monitorable: self.monitorable,
            age_related: self.age_related,
            restorable: self.restorable,
            failure_rate_per_hour: opt_num(&self.rate),
            mode_ratio_alpha: opt_num(&self.alpha),
            effect_probability_beta: opt_num(&self.beta),
            mission_hours: opt_num(&self.hours),
        }
    }
}

// ---------------- working state (one item open at a time) ----------------

pub struct Work {
    pub status: String,
    pub headline: String,
    pub project_path: String,
    // CSV import (shared controls)
    pub csv_pick: String,
    pub csv_path: String,
    // Life Data
    pub life_times: String,
    pub life_dist: String,
    pub life_rank: bool,
    pub life_t: String,
    pub life_result: String,
    pub life_fit: Option<dist_mod::FitResult>,
    pub life_show_prob: bool,
    pub wizard_ranks: Vec<FitRank>,
    pub wizard_note: String,
    // Limited Data
    pub limited_groups: String,
    pub limited_result: String,
    // MCMC
    pub mcmc_times: String,
    pub mcmc_dist: String,
    pub mcmc_draws: String,
    pub mcmc_warmup: String,
    pub mcmc_chains: String,
    pub mcmc_seed: String,
    pub mcmc_t: String,
    pub mcmc_result: String,
    // Degradation
    pub deg_units: String,
    pub deg_threshold: String,
    pub deg_model: String,
    pub deg_dist: String,
    pub deg_increasing: bool,
    pub deg_result: String,
    pub deg_data: Option<degradation::DegradationResult>,
    pub deg_parsed: Vec<(Vec<f64>, Vec<f64>)>,
    // ALT
    pub alt_data: String,
    pub alt_model: String,
    pub alt_use_stress: String,
    pub alt_t: String,
    pub alt_result: String,
    pub alt_series: Vec<(f64, f64)>,
    // Growth
    pub growth_times: String,
    pub growth_result: String,
    pub growth_fit: Option<growth::CrowAmsaaFit>,
    // Competing
    pub comp_data: String,
    pub comp_dist: String,
    pub comp_t: String,
    pub comp_result: String,
    // System RBD
    pub rbd_json: String,
    pub rbd_spec: Value,
    pub rbd_t: String,
    pub rbd_horizon: String,
    pub rbd_sims: String,
    pub rbd_topology: String,
    pub rbd_k: String,
    pub rbd_avail: String,
    pub rbd_path: String,
    pub rbd_status: String,
    pub rbd_armed: Option<String>,
    pub rbd_sel: Option<usize>,
    pub rbd_importance: Vec<BlockImportance>,
    pub rbd_importance_note: String,
    pub ed_name: String,
    pub ed_dist: String,
    pub ed_params: String,
    pub ed_mtbf: String,
    pub ed_mttr: String,
    pub ed_units: String,
    pub ed_k: String,
    pub ed_dormancy: String,
    pub ed_switch: String,
    pub nb_name: String,
    pub nb_dist: String,
    pub nb_params: String,
    pub nb_mtbf: String,
    pub nb_mttr: String,
    pub nb_mode: String,
    pub nb_dormancy: String,
    // Fault tree
    pub ft_json: String,
    pub ft_result: String,
    // FMEA
    pub fmea_rows: Vec<FmeaRow>,
    pub fmea_type: String,
    pub fmea_crit: Vec<[String; 4]>,
    pub fmea_csv: String,
    pub fmea_export_path: String,
    pub fm_item: String,
    pub fm_mode: String,
    pub fm_sev: String,
    pub fm_occ: String,
    pub fm_det: String,
    pub fm_alpha: String,
    pub fm_beta: String,
    pub fm_lambda: String,
    pub fm_hours: String,
    pub fmea_status: String,
    pub dvpr_entries: Vec<DvprEntry>,
    pub dv_id: String,
    pub dv_ref: String,
    pub dv_test: String,
    pub dv_sample: String,
    pub dv_criteria: String,
    // Warranty
    pub war_sales: String,
    pub war_returns: String,
    pub war_current: String,
    pub war_horizon: String,
    pub war_result: String,
    pub war_table: Vec<Vec<u32>>,
    pub war_sale_labels: Vec<String>,
    // Prediction
    pub pred_parts: String,
    pub pred_t: String,
    pub pred_result: String,
    pub pred_source: String,
    pub pred_lib_path: String,
    pub pred_lib_parts: Vec<PartEntry>,
    pub pred_lib_note: String,
    pub pred217_file: String,
    pub pred217_json: Option<Value>,
    pub pred217_named: BTreeMap<String, String>,
    pub pred217_num: BTreeMap<String, String>,
    pub pred217_result: String,
    pub alloc_target: String,
    pub alloc_n: String,
    pub alloc_weights: String,
    pub alloc_result: String,
    pub der_name: String,
    pub der_operating: String,
    pub der_rated: String,
    pub der_max: String,
    pub der_result: String,
    pub sr_generic: String,
    pub sr_pi_q: String,
    pub sr_pi_s: String,
    pub sr_ea: String,
    pub sr_temp: String,
    pub sr_ref_temp: String,
    pub sr_pi_e: String,
    pub sr_qty_rate: String,
    pub sr_obs_rate: String,
    pub sr_pit_obs: String,
    pub sr_pit_tgt: String,
    pub sr_pis_obs: String,
    pub sr_pis_tgt: String,
    pub sr_ci_failures: String,
    pub sr_ci_exposure: String,
    pub sr_ci_conf: String,
    pub sr_m3_pred: String,
    pub sr_m3_pw: String,
    pub sr_m3_obs: String,
    pub sr_m3_ow: String,
    pub mp_base: String,
    pub mp_ea: String,
    pub mp_ref: String,
    pub mp_cyc_factor: String,
    pub mp_phases: String,
    pub pred_clean_result: String,
    // DOE
    pub doe_k: String,
    pub doe_y: String,
    pub doe_result: String,
    // Flowchart
    pub flow_json: String,
    pub flow_sims: String,
    pub flow_seed: String,
    pub flow_result: String,
    // Markov
    pub mk_states: Vec<String>,
    pub mk_new_state: String,
    pub mk_transitions: Vec<[String; 3]>,
    pub mk_absorbing: String,
    pub mk_start: String,
    pub mk_time: String,
    pub mk_result: String,
    pub mk_series: Vec<(String, Vec<(f64, f64)>)>,
    // QCP
    pub qcp_dist: String,
    pub qcp_params: BTreeMap<String, String>,
    pub qcp_t: String,
    pub qcp_bx: String,
    pub qcp_result: String,
    // RCM
    pub rcm_rows: Vec<RcmRowUi>,
    // FRACAS
    pub fracas_rows: Vec<Incident>,
    pub fracas_total_hours: String,
    pub fracas_path: String,
    pub fracas_status: String,
    // Project workspace
    pub project_name: String,
    pub items: Vec<ProjectItem>,
    pub selected: Option<usize>,
    pub next_id: u64,
    pub new_kind: String,
    pub new_name: String,
    pub rename_name: String,
    pub flow_target_id: u64,
    pub flow_block_idx: usize,
    pub fmea_rcm_row: String,
    pub pred_life_dist: String,
    pub pred_life_params: String,
    pub pred_life_t: String,
    pub pred_life_result: String,
    pub rbd_undo: Vec<Value>,
    // v0.3.2 correctness: validation, fit status, fingerprints, warranty notes
    pub validation_error: Option<ValidationError>,
    pub dispatch_warning: Option<(String, String)>,
    pub life_fit_status: String,
    pub life_fit_status_note: String,
    pub life_fit_warnings: Vec<String>,
    pub limited_fit_status: String,
    pub limited_fit_status_note: String,
    pub limited_fit_warnings: Vec<String>,
    pub comp_fit_status: String,
    pub comp_fit_status_note: String,
    pub comp_fit_warnings: Vec<String>,
    pub deg_fit_status: String,
    pub deg_fit_status_note: String,
    pub deg_fit_warnings: Vec<String>,
    pub alt_fit_status: String,
    pub alt_fit_status_note: String,
    pub alt_fit_warnings: Vec<String>,
    pub war_excluded_future: usize,
    pub war_notes: Vec<String>,
    pub result_fingerprints: std::collections::BTreeMap<u64, String>,
    // Presentation-only selection state (not persisted)
    pub rcm_sel: Option<usize>,
    pub fracas_sel: Option<usize>,
    pub mk_sel: Option<usize>,
}

impl Default for Work {
    fn default() -> Self {
        Self::new()
    }
}

impl Work {
    pub fn new() -> Self {
        let (bt, bf) = datasets::bearing_life(2026);
        let life_text = format_life(&bt, &bf);
        let groups = datasets::small_sample_groups(2026);
        let limited_text = groups
            .iter()
            .map(|(t, f)| format_life(t, f))
            .collect::<Vec<String>>()
            .join("\n");
        let deg = datasets::degradation_units(2026);
        let deg_text = deg
            .iter()
            .map(|(t, y)| {
                format!(
                    "{} | {}",
                    t.iter()
                        .map(|v| fmt_num(*v))
                        .collect::<Vec<String>>()
                        .join(","),
                    y.iter()
                        .map(|v| fmt_num(*v))
                        .collect::<Vec<String>>()
                        .join(",")
                )
            })
            .collect::<Vec<String>>()
            .join("\n");
        let (alt_stress, alt_times) = datasets::alt_dataset(2026);
        let alt_text = alt_stress
            .iter()
            .zip(&alt_times)
            .map(|(s, t)| format!("{},{}", fmt_num(s[0]), fmt_num(*t)))
            .collect::<Vec<String>>()
            .join("\n");
        let growth_text = datasets::growth_dataset(2026)
            .iter()
            .map(|v| fmt_num(*v))
            .collect::<Vec<String>>()
            .join(", ");
        let spec: Value = serde_json::from_str(EXAMPLE3_JSON)
            .unwrap_or(json!({"type": "series", "children": []}));
        let mut w = Work {
            status: "Ready.".into(),
            headline: String::new(),
            project_path: "reliakit_project.json".into(),
            csv_pick: BUNDLED_CSV[10].to_string(),
            csv_path: String::new(),
            life_times: life_text.clone(),
            life_dist: "weibull2".into(),
            life_rank: false,
            life_t: "1000".into(),
            life_result: String::new(),
            life_fit: None,
            life_show_prob: false,
            wizard_ranks: Vec::new(),
            wizard_note: String::new(),
            limited_groups: limited_text,
            limited_result: String::new(),
            mcmc_times: life_text,
            mcmc_dist: "weibull2".into(),
            mcmc_draws: "300".into(),
            mcmc_warmup: "300".into(),
            mcmc_chains: "2".into(),
            mcmc_seed: "42".into(),
            mcmc_t: "1000".into(),
            mcmc_result: String::new(),
            deg_units: deg_text,
            deg_threshold: "70".into(),
            deg_model: "linear".into(),
            deg_dist: "weibull2".into(),
            deg_increasing: false,
            deg_result: String::new(),
            deg_data: None,
            deg_parsed: Vec::new(),
            alt_data: alt_text,
            alt_model: "arrhenius".into(),
            alt_use_stress: "55".into(),
            alt_t: "1000".into(),
            alt_result: String::new(),
            alt_series: Vec::new(),
            growth_times: growth_text,
            growth_result: String::new(),
            growth_fit: None,
            comp_data: "120,wear\n340,electrical\n560,wear\n800,\n950,electrical\n1300,wear\n1700,\n2200,electrical".into(),
            comp_dist: "weibull2".into(),
            comp_t: "1000".into(),
            comp_result: String::new(),
            rbd_json: serde_json::to_string_pretty(&spec).unwrap_or_default(),
            rbd_spec: spec,
            rbd_t: "1000".into(),
            rbd_horizon: "20000".into(),
            rbd_sims: "40".into(),
            rbd_topology: "series".into(),
            rbd_k: "2".into(),
            rbd_avail: String::new(),
            rbd_path: "rbd.json".into(),
            rbd_status: String::new(),
            rbd_armed: None,
            rbd_sel: None,
            rbd_importance: Vec::new(),
            rbd_importance_note: String::new(),
            ed_name: String::new(),
            ed_dist: "weibull2".into(),
            ed_params: String::new(),
            ed_mtbf: String::new(),
            ed_mttr: String::new(),
            ed_units: "2".into(),
            ed_k: "1".into(),
            ed_dormancy: "0".into(),
            ed_switch: "1".into(),
            nb_name: "new block".into(),
            nb_dist: "weibull2".into(),
            nb_params: "beta=1.5, eta=5000".into(),
            nb_mtbf: "5000".into(),
            nb_mttr: "10".into(),
            nb_mode: "active".into(),
            nb_dormancy: "0.5".into(),
            ft_json: "{\n  \"Or\": {\n    \"name\": \"top event\",\n    \"children\": [\n      {\"Basic\": {\"name\": \"pump fails\", \"prob\": 0.02}},\n      {\"Basic\": {\"name\": \"valve fails\", \"prob\": 0.03}}\n    ]\n  }\n}".into(),
            ft_result: String::new(),
            fmea_rows: Vec::new(),
            fmea_type: "Design".into(),
            fmea_crit: Vec::new(),
            fmea_csv: String::new(),
            fmea_export_path: "fmea_export.csv".into(),
            fm_item: String::new(),
            fm_mode: String::new(),
            fm_sev: "5".into(),
            fm_occ: "5".into(),
            fm_det: "5".into(),
            fm_alpha: String::new(),
            fm_beta: String::new(),
            fm_lambda: String::new(),
            fm_hours: String::new(),
            fmea_status: String::new(),
            dvpr_entries: Vec::new(),
            dv_id: String::new(),
            dv_ref: String::new(),
            dv_test: String::new(),
            dv_sample: "3".into(),
            dv_criteria: String::new(),
            war_sales: "120, 150, 140, 160".into(),
            war_returns: "0,1\n0,2\n1,2\n1,3\n2,3\n0,3\n2,4\n1,4".into(),
            war_current: "3".into(),
            war_horizon: "6".into(),
            war_result: String::new(),
            war_table: Vec::new(),
            war_sale_labels: Vec::new(),
            pred_parts: "Resistor, 2.5, 10, 1.0, 1.0\nCapacitor, 1.2, 6, 1.1, 1.0\nConnector, 0.8, 4, 1.0, 1.2".into(),
            pred_t: "1000".into(),
            pred_result: String::new(),
            pred_source: "Generic user data".into(),
            pred_lib_path: String::new(),
            pred_lib_parts: Vec::new(),
            pred_lib_note: String::new(),
            pred217_file: "resistors.json".into(),
            pred217_json: None,
            pred217_named: BTreeMap::new(),
            pred217_num: BTreeMap::new(),
            pred217_result: String::new(),
            alloc_target: "0.99".into(),
            alloc_n: "3".into(),
            alloc_weights: "1, 2, 1".into(),
            alloc_result: String::new(),
            der_name: "Resistor".into(),
            der_operating: "0.5".into(),
            der_rated: "1.0".into(),
            der_max: "0.7".into(),
            der_result: String::new(),
            sr_generic: "1.0".into(),
            sr_pi_q: "1.0".into(),
            sr_pi_s: "1.0".into(),
            sr_ea: "0.35".into(),
            sr_temp: "55".into(),
            sr_ref_temp: "25".into(),
            sr_pi_e: "1.0".into(),
            sr_qty_rate: "10,1.0\n5,2.0".into(),
            sr_obs_rate: "1.2".into(),
            sr_pit_obs: "1.0".into(),
            sr_pit_tgt: "1.5".into(),
            sr_pis_obs: "1.0".into(),
            sr_pis_tgt: "1.0".into(),
            sr_ci_failures: "3".into(),
            sr_ci_exposure: "100000".into(),
            sr_ci_conf: "0.90".into(),
            sr_m3_pred: "1.0".into(),
            sr_m3_pw: "1.0".into(),
            sr_m3_obs: "1.4".into(),
            sr_m3_ow: "1.0".into(),
            mp_base: "1.0".into(),
            mp_ea: "0.35".into(),
            mp_ref: "25".into(),
            mp_cyc_factor: "0.001".into(),
            mp_phases: "Operating, 8000, 55, 100, 0, 1.0\nDormant, 760, 25, 0, 1, 0.1".into(),
            pred_clean_result: String::new(),
            pred_life_dist: "weibull2".into(),
            pred_life_params: "beta=1.5, eta=5000".into(),
            pred_life_t: "1000".into(),
            pred_life_result: String::new(),
            doe_k: "3".into(),
            doe_y: "10, 12, 11, 15, 13, 16, 14, 18".into(),
            doe_result: String::new(),
            flow_json: "{\n  \"nodes\": [\n    {\"id\": \"s\", \"type\": \"start\", \"next\": \"a\"},\n    {\"id\": \"a\", \"type\": \"task\", \"name\": \"inspect\", \"time\": {\"dist\": \"fixed\", \"value\": 2.0}, \"cost\": 3.0, \"next\": \"b\"},\n    {\"id\": \"b\", \"type\": \"task\", \"name\": \"assemble\", \"time\": {\"dist\": \"fixed\", \"value\": 5.0}, \"next\": \"e\"},\n    {\"id\": \"e\", \"type\": \"end\"}\n  ]\n}".into(),
            flow_sims: "5000".into(),
            flow_seed: "1".into(),
            flow_result: String::new(),
            mk_states: vec!["Both up".into(), "One down".into(), "System down".into()],
            mk_new_state: String::new(),
            mk_transitions: vec![
                ["Both up".into(), "One down".into(), "0.02".into()],
                ["One down".into(), "Both up".into(), "0.1".into()],
                ["One down".into(), "System down".into(), "0.01".into()],
            ],
            mk_absorbing: "System down".into(),
            mk_start: "Both up".into(),
            mk_time: "100".into(),
            mk_result: String::new(),
            mk_series: Vec::new(),
            qcp_dist: "weibull2".into(),
            qcp_params: BTreeMap::new(),
            qcp_t: "1000".into(),
            qcp_bx: "0.10".into(),
            qcp_result: String::new(),
            rcm_rows: Vec::new(),
            fracas_rows: Vec::new(),
            fracas_total_hours: "10000".into(),
            fracas_path: "fracas.csv".into(),
            fracas_status: String::new(),
            project_name: "Untitled project".into(),
            items: Vec::new(),
            selected: None,
            next_id: 1,
            new_kind: "Life Data".into(),
            new_name: String::new(),
            rename_name: String::new(),
            flow_target_id: 0,
            flow_block_idx: 0,
            fmea_rcm_row: String::new(),
            rbd_undo: Vec::new(),
            rcm_sel: None,
            fracas_sel: None,
            mk_sel: None,
            validation_error: None,
            dispatch_warning: None,
            life_fit_status: String::new(), life_fit_status_note: String::new(), life_fit_warnings: Vec::new(),
            limited_fit_status: String::new(), limited_fit_status_note: String::new(), limited_fit_warnings: Vec::new(),
            comp_fit_status: String::new(), comp_fit_status_note: String::new(), comp_fit_warnings: Vec::new(),
            deg_fit_status: String::new(), deg_fit_status_note: String::new(), deg_fit_warnings: Vec::new(),
            alt_fit_status: String::new(), alt_fit_status_note: String::new(), alt_fit_warnings: Vec::new(),
            war_excluded_future: 0, war_notes: Vec::new(),
            result_fingerprints: std::collections::BTreeMap::new(),
        };
        w.qcp_reset_params();
        w
    }

    pub fn note(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
    }
    pub fn set_validation_error(&mut self, e: ValidationError) {
        self.status = e.message();
        self.validation_error = Some(e);
    }
    pub fn compute_fingerprint(&self, kind: ItemKind) -> String {
        let parts: Vec<String> = match kind {
            ItemKind::LifeData => vec![
                self.life_times.clone(),
                self.life_dist.clone(),
                self.life_t.clone(),
                self.life_rank.to_string(),
                self.life_show_prob.to_string(),
            ],
            ItemKind::LimitedData => vec![self.limited_groups.clone()],
            ItemKind::Mcmc => vec![
                self.mcmc_times.clone(),
                self.mcmc_dist.clone(),
                self.mcmc_draws.clone(),
                self.mcmc_warmup.clone(),
                self.mcmc_chains.clone(),
                self.mcmc_seed.clone(),
                self.mcmc_t.clone(),
            ],
            ItemKind::Degradation => vec![
                self.deg_units.clone(),
                self.deg_threshold.clone(),
                self.deg_model.clone(),
                self.deg_dist.clone(),
                self.deg_increasing.to_string(),
            ],
            ItemKind::Alt => vec![
                self.alt_data.clone(),
                self.alt_model.clone(),
                self.alt_use_stress.clone(),
                self.alt_t.clone(),
            ],
            ItemKind::Growth => vec![self.growth_times.clone()],
            ItemKind::Competing => vec![
                self.comp_data.clone(),
                self.comp_dist.clone(),
                self.comp_t.clone(),
            ],
            ItemKind::Rbd => vec![
                serde_json::to_string(&self.rbd_spec).unwrap_or_default(),
                self.rbd_t.clone(),
                self.rbd_horizon.clone(),
                self.rbd_sims.clone(),
                self.rbd_topology.clone(),
                self.rbd_k.clone(),
            ],
            ItemKind::FaultTree => vec![self.ft_json.clone()],
            ItemKind::Fmea => vec![
                self.fmea_type.clone(),
                serde_json::to_string(&self.fmea_rows).unwrap_or_default(),
            ],
            ItemKind::Warranty => vec![
                self.war_sales.clone(),
                self.war_returns.clone(),
                self.war_current.clone(),
                self.war_horizon.clone(),
            ],
            ItemKind::Prediction => vec![
                self.pred_parts.clone(),
                self.pred_t.clone(),
                self.pred_life_dist.clone(),
                self.pred_life_params.clone(),
                self.pred_life_t.clone(),
                self.alloc_target.clone(),
                self.alloc_n.clone(),
                self.alloc_weights.clone(),
                self.der_operating.clone(),
                self.der_rated.clone(),
                self.der_max.clone(),
                self.pred217_file.clone(),
                serde_json::to_string(&self.pred217_named).unwrap_or_default(),
                serde_json::to_string(&self.pred217_num).unwrap_or_default(),
            ],
            ItemKind::Doe => vec![self.doe_k.clone(), self.doe_y.clone()],
            ItemKind::Flowchart => vec![
                self.flow_json.clone(),
                self.flow_sims.clone(),
                self.flow_seed.clone(),
            ],
            ItemKind::Markov => vec![
                self.mk_states.join(","),
                self.mk_transitions
                    .iter()
                    .map(|tr| tr.join(","))
                    .collect::<Vec<_>>()
                    .join(";"),
                self.mk_start.clone(),
                self.mk_absorbing.clone(),
                self.mk_time.clone(),
            ],
            ItemKind::Qcp => vec![
                self.qcp_dist.clone(),
                self.qcp_t.clone(),
                self.qcp_bx.clone(),
                self.qcp_params
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join(","),
            ],
            ItemKind::Rcm => vec![self
                .rcm_rows
                .iter()
                .map(|r| format!("{}|{}|{}", r.asset, r.mode, r.rate))
                .collect::<Vec<_>>()
                .join(";")],
            ItemKind::Fracas => vec![
                self.fracas_total_hours.clone(),
                self.fracas_rows
                    .iter()
                    .map(|r| format!("{}:{}", r.id, fmt_num(r.hours)))
                    .collect::<Vec<_>>()
                    .join(";"),
            ],
        };
        parts.join("\u{1f}")
    }
    pub fn has_results(&self, kind: ItemKind) -> bool {
        match kind {
            ItemKind::LifeData => {
                self.life_fit.is_some()
                    || !self.life_result.is_empty()
                    || !self.wizard_ranks.is_empty()
            }
            ItemKind::LimitedData => !self.limited_result.is_empty(),
            ItemKind::Mcmc => !self.mcmc_result.is_empty(),
            ItemKind::Degradation => !self.deg_result.is_empty() || self.deg_data.is_some(),
            ItemKind::Alt => !self.alt_result.is_empty(),
            ItemKind::Growth => !self.growth_result.is_empty() || self.growth_fit.is_some(),
            ItemKind::Competing => !self.comp_result.is_empty(),
            ItemKind::Rbd => !self.rbd_avail.is_empty() || !self.rbd_importance.is_empty(),
            ItemKind::FaultTree => !self.ft_result.is_empty(),
            ItemKind::Warranty => !self.war_result.is_empty(),
            ItemKind::Prediction => {
                !self.pred_result.is_empty()
                    || !self.pred_life_result.is_empty()
                    || !self.alloc_result.is_empty()
                    || !self.der_result.is_empty()
                    || !self.pred_clean_result.is_empty()
                    || !self.pred217_result.is_empty()
            }
            ItemKind::Doe => !self.doe_result.is_empty(),
            ItemKind::Flowchart => !self.flow_result.is_empty(),
            ItemKind::Markov => !self.mk_result.is_empty(),
            ItemKind::Qcp => !self.qcp_result.is_empty(),
            _ => false,
        }
    }
    pub fn mark_computed(&mut self) {
        if let Some(i) = self.selected {
            if let Some(it) = self.items.get(i).cloned() {
                let fp = self.compute_fingerprint(it.kind);
                self.result_fingerprints.insert(it.id, fp);
            }
        }
    }
    pub fn is_current_outdated(&self) -> bool {
        if let Some(i) = self.selected {
            if let Some(it) = self.items.get(i) {
                if let Some(stored) = self.result_fingerprints.get(&it.id) {
                    return *stored != self.compute_fingerprint(it.kind)
                        && self.has_results(it.kind);
                }
            }
        }
        false
    }
    pub fn is_source_outdated(&self) -> bool {
        self.is_current_outdated()
    }
    pub fn duplicate_selected_item(&mut self) {
        self.capture_selected();
        let (kind, orig_name, state, orig_id) =
            match self.selected.and_then(|i| self.items.get(i).cloned()) {
                Some(it) => (it.kind, it.name.clone(), it.state.clone(), it.id),
                None => {
                    self.note("No item selected to duplicate.");
                    return;
                }
            };
        let base = format!("{} copy", orig_name);
        let mut name = base.clone();
        let mut n = 2;
        while self.items.iter().any(|it| it.name == name) {
            name = format!("{} {}", base, n);
            n += 1;
        }
        let idx = self.create_item_silent(kind, name.clone(), state);
        // copy fingerprint for the copy
        if let Some(fp) = self.result_fingerprints.get(&orig_id).cloned() {
            let new_id = self.items[idx].id;
            self.result_fingerprints.insert(new_id, fp);
        }
        self.select_index(Some(idx));
        self.note(format!("Duplicated item as {}.", name));
    }

    // ---------------- project workspace ----------------

    fn gs(v: &Value, k: &str, d: &str) -> String {
        v.get(k)
            .and_then(|x| x.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| d.to_string())
    }
    fn gb(v: &Value, k: &str, d: bool) -> bool {
        v.get(k).and_then(|x| x.as_bool()).unwrap_or(d)
    }

    pub fn init_project(&mut self) {
        if !self.items.is_empty() {
            return;
        }
        let st = self.capture_state(ItemKind::LifeData);
        self.items.push(ProjectItem {
            id: 1,
            kind: ItemKind::LifeData,
            name: "Life Data 1".into(),
            state: st,
        });
        self.next_id = 2;
        self.selected = Some(0);
        self.rename_name = "Life Data 1".into();
    }

    pub fn capture_selected(&mut self) {
        if let Some(i) = self.selected {
            if i < self.items.len() {
                let kind = self.items[i].kind;
                let st = self.capture_state(kind);
                self.items[i].state = st;
            }
        }
    }

    pub fn select_index(&mut self, idx: Option<usize>) {
        self.capture_selected();
        self.selected = idx;
        if let Some(i) = idx {
            if let Some(it) = self.items.get(i).cloned() {
                self.apply_state(it.kind, &it.state.clone());
                self.rename_name = it.name.clone();
            }
        }
        self.rcm_sel = None;
        self.fracas_sel = None;
        self.mk_sel = None;
    }

    pub fn create_item_silent(&mut self, kind: ItemKind, name: String, state: Value) -> usize {
        self.capture_selected();
        let id = self.next_id;
        self.next_id += 1;
        self.items.push(ProjectItem {
            id,
            kind,
            name,
            state,
        });
        self.items.len() - 1
    }

    pub fn add_item_from_panel(&mut self) {
        let kind = ItemKind::from_label(&self.new_kind).unwrap_or(ItemKind::LifeData);
        let name = if self.new_name.trim().is_empty() {
            self.default_item_name(kind)
        } else {
            self.new_name.trim().to_string()
        };
        let idx = self.create_item_silent(kind, name.clone(), json!({}));
        self.new_name = String::new();
        self.select_index(Some(idx));
        self.note(format!("Added {} item {}.", kind.label(), name));
    }

    pub fn remove_selected_item(&mut self) {
        if let Some(i) = self.selected {
            if i < self.items.len() {
                let it = self.items.remove(i);
                self.selected = None;
                self.note(format!("Removed item {}.", it.name));
            }
        }
    }

    pub fn rename_selected_item(&mut self) {
        if let Some(i) = self.selected {
            let name = self.rename_name.trim().to_string();
            if !name.is_empty() {
                if let Some(it) = self.items.get_mut(i) {
                    it.name = name.clone();
                }
                self.note(format!("Item renamed to {}.", name));
            }
        }
    }

    pub fn find_or_create_item(&mut self, kind: ItemKind, name: &str) -> usize {
        if let Some(i) = self.items.iter().position(|it| it.kind == kind) {
            return i;
        }
        self.create_item_silent(kind, name.to_string(), json!({}))
    }

    pub fn default_item_name(&self, kind: ItemKind) -> String {
        let n = self.items.iter().filter(|it| it.kind == kind).count() + 1;
        format!("{} {}", kind.label(), n)
    }

    pub fn save_project(&mut self) {
        self.capture_selected();
        let proj = project_to_value(&self.project_name, &self.items, self.selected);
        match std::fs::write(
            &self.project_path,
            serde_json::to_string_pretty(&proj).unwrap_or_default(),
        ) {
            Ok(()) => self.note(format!("Project saved to {}", self.project_path)),
            Err(e) => self.note(format!("Project save error: {}", e)),
        }
    }

    pub fn open_project(&mut self) {
        let text = match std::fs::read_to_string(&self.project_path) {
            Ok(t) => t,
            Err(e) => {
                self.note(format!("Project open error: {}", e));
                return;
            }
        };
        let v: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                self.note(format!("Project JSON error: {}", e));
                return;
            }
        };
        if let Some((name, items, sel)) = project_from_value(&v) {
            self.project_name = name;
            self.next_id = items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
            self.items = items;
            self.selected = None;
            if let Some(i) = sel {
                self.select_index(Some(i));
            }
            self.note(format!("Project opened from {}", self.project_path));
            return;
        }
        // Legacy format (reliakit-project-1): load the three covered
        // parts as items.
        if v.get("life_times").is_some() || v.get("rbd").is_some() || v.get("fmea_rows").is_some() {
            self.items = Vec::new();
            self.selected = None;
            self.next_id = 1;
            if v.get("life_times").is_some() {
                let st = json!({
                    "life_times": v.get("life_times").cloned().unwrap_or(json!("")),
                    "life_dist": v.get("life_dist").cloned().unwrap_or(json!("weibull2")),
                    "life_t": v.get("life_t").cloned().unwrap_or(json!("1000")),
                });
                self.create_item_silent(
                    ItemKind::LifeData,
                    "Life Data (from old project)".into(),
                    st,
                );
            }
            if let Some(rbd) = v.get("rbd") {
                let st = json!({"rbd_spec": rbd.clone(), "rbd_t": "1000"});
                self.create_item_silent(ItemKind::Rbd, "RBD (from old project)".into(), st);
            }
            if v.get("fmea_rows").is_some() {
                let st = json!({
                    "fmea_type": v.get("fmea_type").cloned().unwrap_or(json!("Design")),
                    "fmea_rows": v.get("fmea_rows").cloned().unwrap_or(json!([])),
                    "dvpr": v.get("dvpr").cloned().unwrap_or(json!([])),
                });
                self.create_item_silent(ItemKind::Fmea, "FMEA (from old project)".into(), st);
            }
            self.note(format!(
                "Old format project opened from {} as {} items.",
                self.project_path,
                self.items.len()
            ));
            return;
        }
        self.note("Project file is not a ReliaKit project (v2 or legacy).");
    }

    pub fn new_project(&mut self) {
        self.items = Vec::new();
        self.selected = None;
        self.next_id = 1;
        self.project_name = "Untitled project".into();
        self.headline = String::new();
        self.note("New project. Add an item, or load an example or dataset from the start page.");
    }

    pub fn load_fmea_example(&mut self) {
        let mut r1 = FmeaRow::new("Pump seal", "Seal leaks", 8, 4, 5);
        r1.function = "Seal the pump housing".into();
        r1.effect = "Fluid loss, unit shutdown".into();
        r1.cause = "Seal wear".into();
        let mut r2 = FmeaRow::new("Controller board", "Board overheats", 7, 3, 6);
        r2.function = "Control the drive".into();
        r2.effect = "Loss of control".into();
        r2.cause = "Blocked airflow".into();
        let mut r3 = FmeaRow::new("Power connector", "Connector corrodes", 6, 5, 4);
        r3.function = "Carry power".into();
        r3.effect = "Intermittent power".into();
        r3.cause = "Moisture ingress".into();
        self.fmea_rows = vec![r1, r2, r3];
        self.fmea_crit = vec![
            [String::new(), String::new(), String::new(), String::new()],
            [String::new(), String::new(), String::new(), String::new()],
            [String::new(), String::new(), String::new(), String::new()],
        ];
    }
}

// ---------------- item state capture and restore ----------------

impl Work {
    pub fn capture_state(&self, kind: ItemKind) -> Value {
        match kind {
            ItemKind::LifeData => json!({
                "life_times": self.life_times, "life_dist": self.life_dist,
                "life_rank": self.life_rank, "life_t": self.life_t,
                "life_result": self.life_result, "life_fit_status": self.life_fit_status, "life_fit_status_note": self.life_fit_status_note, "life_fit_warnings": self.life_fit_warnings,
                "life_fit": serde_json::to_value(&self.life_fit).unwrap_or(json!(null)),
                "life_show_prob": self.life_show_prob, "wizard_note": self.wizard_note,
                "wizard_ranks": self.wizard_ranks.iter().map(|r| json!({
                    "dist": r.dist, "loglik": r.loglik, "aic": r.aic, "bic": r.bic,
                    "ad": r.ad_statistic, "success": r.success, "n_params": r.n_params,
                })).collect::<Vec<Value>>(),
            }),
            ItemKind::LimitedData => {
                json!({"limited_groups": self.limited_groups, "limited_result": self.limited_result, "limited_fit_status": self.limited_fit_status, "limited_fit_status_note": self.limited_fit_status_note, "limited_fit_warnings": self.limited_fit_warnings})
            }
            ItemKind::Mcmc => json!({
                "mcmc_times": self.mcmc_times, "mcmc_dist": self.mcmc_dist,
                "mcmc_draws": self.mcmc_draws, "mcmc_warmup": self.mcmc_warmup,
                "mcmc_chains": self.mcmc_chains, "mcmc_seed": self.mcmc_seed,
                "mcmc_t": self.mcmc_t, "mcmc_result": self.mcmc_result,
            }),
            ItemKind::Degradation => json!({
                "deg_units": self.deg_units, "deg_threshold": self.deg_threshold,
                "deg_model": self.deg_model, "deg_dist": self.deg_dist,
                "deg_increasing": self.deg_increasing, "deg_result": self.deg_result, "deg_fit_status": self.deg_fit_status, "deg_fit_status_note": self.deg_fit_status_note, "deg_fit_warnings": self.deg_fit_warnings,
                "deg_parsed": serde_json::to_value(&self.deg_parsed).unwrap_or(json!([])),
                "deg_data": self.deg_data.as_ref().map(|r| json!({
                    "paths": r.paths.iter().map(|(a, b)| json!([a, b])).collect::<Vec<Value>>(),
                    "pseudo_times": r.pseudo_times, "failed": r.failed,
                    "fit": serde_json::to_value(&r.fit).unwrap_or(json!(null)),
                    "model": r.model,
                })),
            }),
            ItemKind::Alt => json!({
                "alt_data": self.alt_data, "alt_model": self.alt_model,
                "alt_use_stress": self.alt_use_stress, "alt_t": self.alt_t, "alt_result": self.alt_result, "alt_fit_status": self.alt_fit_status, "alt_fit_status_note": self.alt_fit_status_note, "alt_fit_warnings": self.alt_fit_warnings,
            }),
            ItemKind::Growth => json!({
                "growth_times": self.growth_times, "growth_result": self.growth_result,
                "growth_fit": serde_json::to_value(&self.growth_fit).unwrap_or(json!(null)),
            }),
            ItemKind::Competing => json!({
                "comp_data": self.comp_data, "comp_dist": self.comp_dist,
                "comp_t": self.comp_t, "comp_result": self.comp_result, "comp_fit_status": self.comp_fit_status, "comp_fit_status_note": self.comp_fit_status_note, "comp_fit_warnings": self.comp_fit_warnings,
            }),
            ItemKind::Rbd => json!({
                "rbd_spec": self.rbd_spec, "rbd_t": self.rbd_t, "rbd_horizon": self.rbd_horizon,
                "rbd_sims": self.rbd_sims,
                "rbd_topology": self.rbd_topology, "rbd_k": self.rbd_k, "rbd_avail": self.rbd_avail,
                "rbd_path": self.rbd_path, "rbd_status": self.rbd_status, "rbd_sel": self.rbd_sel,
                "rbd_importance": self.rbd_importance.iter().map(|b| json!({
                    "name": b.name, "birnbaum": b.birnbaum, "fussell_vesely": b.fussell_vesely, "criticality": b.criticality,
                })).collect::<Vec<Value>>(),
                "rbd_importance_note": self.rbd_importance_note,
                "ed_name": self.ed_name, "ed_dist": self.ed_dist, "ed_params": self.ed_params,
                "ed_mtbf": self.ed_mtbf, "ed_mttr": self.ed_mttr, "ed_units": self.ed_units,
                "ed_k": self.ed_k, "ed_dormancy": self.ed_dormancy, "ed_switch": self.ed_switch,
                "nb_name": self.nb_name, "nb_dist": self.nb_dist, "nb_params": self.nb_params,
                "nb_mtbf": self.nb_mtbf, "nb_mttr": self.nb_mttr, "nb_mode": self.nb_mode,
                "nb_dormancy": self.nb_dormancy,
            }),
            ItemKind::FaultTree => json!({"ft_json": self.ft_json, "ft_result": self.ft_result}),
            ItemKind::Fmea => json!({
                "fmea_type": self.fmea_type,
                "fmea_rows": serde_json::to_value(&self.fmea_rows).unwrap_or(json!([])),
                "fmea_crit": serde_json::to_value(&self.fmea_crit).unwrap_or(json!([])),
                "fmea_csv": self.fmea_csv, "fmea_export_path": self.fmea_export_path,
                "fm_item": self.fm_item, "fm_mode": self.fm_mode, "fm_sev": self.fm_sev,
                "fm_occ": self.fm_occ, "fm_det": self.fm_det, "fm_alpha": self.fm_alpha,
                "fm_beta": self.fm_beta, "fm_lambda": self.fm_lambda, "fm_hours": self.fm_hours,
                "fmea_status": self.fmea_status,
                "dvpr": self.dvpr_entries.iter().map(|e| json!({
                    "id": e.id, "failure_mode_ref": e.failure_mode_ref, "test": e.test,
                    "sample_size": e.sample_size, "acceptance_criteria": e.acceptance_criteria,
                })).collect::<Vec<Value>>(),
                "dv_id": self.dv_id, "dv_ref": self.dv_ref, "dv_test": self.dv_test,
                "dv_sample": self.dv_sample, "dv_criteria": self.dv_criteria,
            }),
            ItemKind::Warranty => json!({
                "war_sales": self.war_sales, "war_returns": self.war_returns,
                "war_current": self.war_current, "war_horizon": self.war_horizon,
                "war_result": self.war_result, "war_excluded_future": self.war_excluded_future, "war_notes": self.war_notes,
                "war_table": serde_json::to_value(&self.war_table).unwrap_or(json!([])),
                "war_sale_labels": self.war_sale_labels,
            }),
            ItemKind::Prediction => json!({
                "pred_parts": self.pred_parts, "pred_t": self.pred_t, "pred_result": self.pred_result,
                "pred_source": self.pred_source, "pred_lib_path": self.pred_lib_path,
                "pred_lib_note": self.pred_lib_note, "pred217_file": self.pred217_file,
                "pred217_json": self.pred217_json.clone().unwrap_or(json!(null)),
                "pred217_named": serde_json::to_value(&self.pred217_named).unwrap_or(json!({})),
                "pred217_num": serde_json::to_value(&self.pred217_num).unwrap_or(json!({})),
                "pred217_result": self.pred217_result,
                "alloc_target": self.alloc_target, "alloc_n": self.alloc_n,
                "alloc_weights": self.alloc_weights, "alloc_result": self.alloc_result,
                "der_name": self.der_name, "der_operating": self.der_operating,
                "der_rated": self.der_rated, "der_max": self.der_max, "der_result": self.der_result,
                "sr_generic": self.sr_generic, "sr_pi_q": self.sr_pi_q, "sr_pi_s": self.sr_pi_s,
                "sr_ea": self.sr_ea, "sr_temp": self.sr_temp, "sr_ref_temp": self.sr_ref_temp,
                "sr_pi_e": self.sr_pi_e, "sr_qty_rate": self.sr_qty_rate, "sr_obs_rate": self.sr_obs_rate,
                "sr_pit_obs": self.sr_pit_obs, "sr_pit_tgt": self.sr_pit_tgt,
                "sr_pis_obs": self.sr_pis_obs, "sr_pis_tgt": self.sr_pis_tgt,
                "sr_ci_failures": self.sr_ci_failures, "sr_ci_exposure": self.sr_ci_exposure,
                "sr_ci_conf": self.sr_ci_conf, "sr_m3_pred": self.sr_m3_pred, "sr_m3_pw": self.sr_m3_pw,
                "sr_m3_obs": self.sr_m3_obs, "sr_m3_ow": self.sr_m3_ow,
                "mp_base": self.mp_base, "mp_ea": self.mp_ea, "mp_ref": self.mp_ref,
                "mp_cyc_factor": self.mp_cyc_factor, "mp_phases": self.mp_phases,
                "pred_clean_result": self.pred_clean_result,
                "pred_life_dist": self.pred_life_dist, "pred_life_params": self.pred_life_params,
                "pred_life_t": self.pred_life_t, "pred_life_result": self.pred_life_result,
            }),
            ItemKind::Doe => {
                json!({"doe_k": self.doe_k, "doe_y": self.doe_y, "doe_result": self.doe_result})
            }
            ItemKind::Flowchart => json!({
                "flow_json": self.flow_json, "flow_sims": self.flow_sims,
                "flow_seed": self.flow_seed, "flow_result": self.flow_result,
            }),
            ItemKind::Markov => json!({
                "mk_states": self.mk_states,
                "mk_transitions": serde_json::to_value(&self.mk_transitions).unwrap_or(json!([])),
                "mk_absorbing": self.mk_absorbing, "mk_start": self.mk_start,
                "mk_time": self.mk_time, "mk_result": self.mk_result,
            }),
            ItemKind::Qcp => json!({
                "qcp_dist": self.qcp_dist,
                "qcp_params": serde_json::to_value(&self.qcp_params).unwrap_or(json!({})),
                "qcp_t": self.qcp_t, "qcp_bx": self.qcp_bx, "qcp_result": self.qcp_result,
            }),
            ItemKind::Rcm => json!({
                "rcm_rows": self.rcm_rows.iter().map(|r| r.to_json()).collect::<Vec<Value>>(),
            }),
            ItemKind::Fracas => json!({
                "fracas_rows": self.fracas_rows.iter().map(|inc| json!({
                    "id": inc.id, "part": inc.part, "failure_mode": inc.failure_mode,
                    "hours": inc.hours, "date": inc.date, "status": inc.status.as_str(),
                    "corrective_action": inc.corrective_action,
                })).collect::<Vec<Value>>(),
                "fracas_total_hours": self.fracas_total_hours,
                "fracas_path": self.fracas_path, "fracas_status": self.fracas_status,
            }),
        }
    }
}

impl Work {
    pub fn apply_state(&mut self, kind: ItemKind, v: &Value) {
        let gs = |k: &str, d: &str| Self::gs(v, k, d);
        match kind {
            ItemKind::LifeData => {
                if v.get("life_times").and_then(|x| x.as_str()).is_some() {
                    self.life_times = gs("life_times", "");
                } else {
                    let (bt, bf) = datasets::bearing_life(2026);
                    self.life_times = format_life(&bt, &bf);
                }
                self.life_dist = gs("life_dist", "weibull2");
                self.life_rank = Self::gb(v, "life_rank", false);
                self.life_t = gs("life_t", "1000");
                self.life_result = gs("life_result", "");
                self.life_fit_status = gs("life_fit_status", "");
                self.life_fit_status_note = gs("life_fit_status_note", "");
                self.life_fit_warnings = v
                    .get("life_fit_warnings")
                    .and_then(|x| serde_json::from_value(x.clone()).ok())
                    .unwrap_or_default();
                self.life_fit = v
                    .get("life_fit")
                    .and_then(|x| serde_json::from_value(x.clone()).ok());
                self.life_show_prob = Self::gb(v, "life_show_prob", false);
                self.wizard_note = gs("wizard_note", "");
                self.wizard_ranks = v
                    .get("wizard_ranks")
                    .and_then(|x| x.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|r| FitRank {
                                dist: Self::gs(r, "dist", ""),
                                loglik: r.get("loglik").and_then(|x| x.as_f64()).unwrap_or(0.0),
                                aic: r.get("aic").and_then(|x| x.as_f64()).unwrap_or(0.0),
                                bic: r.get("bic").and_then(|x| x.as_f64()).unwrap_or(0.0),
                                ad_statistic: r.get("ad").and_then(|x| x.as_f64()),
                                n_params: r.get("n_params").and_then(|x| x.as_u64()).unwrap_or(0)
                                    as usize,
                                success: Self::gb(r, "success", false),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
            }
            ItemKind::LimitedData => {
                if v.get("limited_groups").and_then(|x| x.as_str()).is_some() {
                    self.limited_groups = gs("limited_groups", "");
                } else {
                    let groups = datasets::small_sample_groups(2026);
                    self.limited_groups = groups
                        .iter()
                        .map(|(t, f)| format_life(t, f))
                        .collect::<Vec<String>>()
                        .join("\n");
                }
                self.limited_result = gs("limited_result", "");
                self.limited_fit_status = gs("limited_fit_status", "");
                self.limited_fit_status_note = gs("limited_fit_status_note", "");
                self.limited_fit_warnings = v
                    .get("limited_fit_warnings")
                    .and_then(|x| serde_json::from_value(x.clone()).ok())
                    .unwrap_or_default();
            }
            ItemKind::Mcmc => {
                self.mcmc_times = if v.get("mcmc_times").and_then(|x| x.as_str()).is_some() {
                    gs("mcmc_times", "")
                } else {
                    self.life_times.clone()
                };
                self.mcmc_dist = gs("mcmc_dist", "weibull2");
                self.mcmc_draws = gs("mcmc_draws", "300");
                self.mcmc_warmup = gs("mcmc_warmup", "300");
                self.mcmc_chains = gs("mcmc_chains", "2");
                self.mcmc_seed = gs("mcmc_seed", "42");
                self.mcmc_t = gs("mcmc_t", "1000");
                self.mcmc_result = gs("mcmc_result", "");
            }
            ItemKind::Degradation => {
                if v.get("deg_units").and_then(|x| x.as_str()).is_some() {
                    self.deg_units = gs("deg_units", "");
                } else {
                    let deg = datasets::degradation_units(2026);
                    self.deg_units = deg
                        .iter()
                        .map(|(t, y)| {
                            format!(
                                "{} | {}",
                                t.iter()
                                    .map(|v2| fmt_num(*v2))
                                    .collect::<Vec<String>>()
                                    .join(","),
                                y.iter()
                                    .map(|v2| fmt_num(*v2))
                                    .collect::<Vec<String>>()
                                    .join(",")
                            )
                        })
                        .collect::<Vec<String>>()
                        .join("\n");
                }
                self.deg_threshold = gs("deg_threshold", "70");
                self.deg_model = gs("deg_model", "linear");
                self.deg_dist = gs("deg_dist", "weibull2");
                self.deg_increasing = Self::gb(v, "deg_increasing", false);
                self.deg_result = gs("deg_result", "");
                self.deg_fit_status = gs("deg_fit_status", "");
                self.deg_fit_status_note = gs("deg_fit_status_note", "");
                self.deg_fit_warnings = v
                    .get("deg_fit_warnings")
                    .and_then(|x| serde_json::from_value(x.clone()).ok())
                    .unwrap_or_default();
                self.deg_parsed = v
                    .get("deg_parsed")
                    .and_then(|x| serde_json::from_value(x.clone()).ok())
                    .unwrap_or_default();
                self.deg_data = v.get("deg_data").and_then(|d| {
                    if d.is_null() {
                        return None;
                    }
                    let paths: Vec<(f64, f64)> = d
                        .get("paths")
                        .and_then(|x| serde_json::from_value::<Vec<[f64; 2]>>(x.clone()).ok())
                        .map(|a| a.iter().map(|p| (p[0], p[1])).collect())
                        .unwrap_or_default();
                    let pseudo: Vec<f64> = d
                        .get("pseudo_times")
                        .and_then(|x| serde_json::from_value(x.clone()).ok())?;
                    let failed: Vec<bool> = d
                        .get("failed")
                        .and_then(|x| serde_json::from_value(x.clone()).ok())?;
                    let fit: Option<dist_mod::FitResult> = d
                        .get("fit")
                        .and_then(|x| serde_json::from_value(x.clone()).ok())
                        .flatten();
                    Some(degradation::DegradationResult {
                        paths,
                        pseudo_times: pseudo,
                        failed,
                        fit,
                        model: Self::gs(d, "model", "linear"),
                    })
                });
            }
            ItemKind::Alt => {
                if v.get("alt_data").and_then(|x| x.as_str()).is_some() {
                    self.alt_data = gs("alt_data", "");
                } else {
                    let (alt_stress, alt_times) = datasets::alt_dataset(2026);
                    self.alt_data = alt_stress
                        .iter()
                        .zip(&alt_times)
                        .map(|(s2, t)| format!("{},{}", fmt_num(s2[0]), fmt_num(*t)))
                        .collect::<Vec<String>>()
                        .join("\n");
                }
                self.alt_model = gs("alt_model", "arrhenius");
                self.alt_use_stress = gs("alt_use_stress", "55");
                self.alt_t = gs("alt_t", "1000");
                self.alt_result = gs("alt_result", "");
                self.alt_fit_status = gs("alt_fit_status", "");
                self.alt_fit_status_note = gs("alt_fit_status_note", "");
                self.alt_fit_warnings = v
                    .get("alt_fit_warnings")
                    .and_then(|x| serde_json::from_value(x.clone()).ok())
                    .unwrap_or_default();
                self.alt_series = Vec::new();
            }
            ItemKind::Growth => {
                if v.get("growth_times").and_then(|x| x.as_str()).is_some() {
                    self.growth_times = gs("growth_times", "");
                } else {
                    self.growth_times = datasets::growth_dataset(2026)
                        .iter()
                        .map(|v2| fmt_num(*v2))
                        .collect::<Vec<String>>()
                        .join(", ");
                }
                self.growth_result = gs("growth_result", "");
                self.growth_fit = v
                    .get("growth_fit")
                    .and_then(|x| serde_json::from_value(x.clone()).ok());
            }
            ItemKind::Competing => {
                self.comp_data = gs("comp_data", "120,wear\n340,electrical\n560,wear\n800,\n950,electrical\n1300,wear\n1700,\n2200,electrical");
                self.comp_dist = gs("comp_dist", "weibull2");
                self.comp_t = gs("comp_t", "1000");
                self.comp_result = gs("comp_result", "");
                self.comp_fit_status = gs("comp_fit_status", "");
                self.comp_fit_status_note = gs("comp_fit_status_note", "");
                self.comp_fit_warnings = v
                    .get("comp_fit_warnings")
                    .and_then(|x| serde_json::from_value(x.clone()).ok())
                    .unwrap_or_default();
            }
            ItemKind::Rbd => {
                self.rbd_spec = v
                    .get("rbd_spec")
                    .cloned()
                    .filter(|x| !x.is_null())
                    .unwrap_or_else(|| {
                        serde_json::from_str(EXAMPLE3_JSON)
                            .unwrap_or(json!({"type": "series", "children": []}))
                    });
                self.rbd_t = gs("rbd_t", "1000");
                self.rbd_horizon = gs("rbd_horizon", "20000");
                self.rbd_sims = gs("rbd_sims", "40");
                self.rbd_topology = gs("rbd_topology", "series");
                self.rbd_k = gs("rbd_k", "2");
                self.rbd_avail = gs("rbd_avail", "");
                self.rbd_path = gs("rbd_path", "rbd.json");
                self.rbd_status = gs("rbd_status", "");
                self.rbd_sel = v
                    .get("rbd_sel")
                    .and_then(|x| x.as_u64())
                    .map(|i| i as usize);
                self.rbd_importance = v
                    .get("rbd_importance")
                    .and_then(|x| x.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|b| BlockImportance {
                                name: Self::gs(b, "name", ""),
                                birnbaum: b.get("birnbaum").and_then(|x| x.as_f64()).unwrap_or(0.0),
                                fussell_vesely: b
                                    .get("fussell_vesely")
                                    .and_then(|x| x.as_f64())
                                    .unwrap_or(0.0),
                                criticality: b
                                    .get("criticality")
                                    .and_then(|x| x.as_f64())
                                    .unwrap_or(0.0),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                self.rbd_importance_note = gs("rbd_importance_note", "");
                self.ed_name = gs("ed_name", "");
                self.ed_dist = gs("ed_dist", "weibull2");
                self.ed_params = gs("ed_params", "");
                self.ed_mtbf = gs("ed_mtbf", "");
                self.ed_mttr = gs("ed_mttr", "");
                self.ed_units = gs("ed_units", "2");
                self.ed_k = gs("ed_k", "1");
                self.ed_dormancy = gs("ed_dormancy", "0");
                self.ed_switch = gs("ed_switch", "1");
                self.nb_name = gs("nb_name", "new block");
                self.nb_dist = gs("nb_dist", "weibull2");
                self.nb_params = gs("nb_params", "beta=1.5, eta=5000");
                self.nb_mtbf = gs("nb_mtbf", "5000");
                self.nb_mttr = gs("nb_mttr", "10");
                self.nb_mode = gs("nb_mode", "active");
                self.nb_dormancy = gs("nb_dormancy", "0.5");
                self.rbd_armed = None;
                self.rbd_undo.clear();
                self.ensure_rbd_positions();
                if let Some(sel) = self.rbd_sel {
                    self.load_rbd_editor(sel);
                }
            }
            ItemKind::FaultTree => {
                self.ft_json = gs("ft_json", "{\n  \"Or\": {\n    \"name\": \"top event\",\n    \"children\": [\n      {\"Basic\": {\"name\": \"pump fails\", \"prob\": 0.02}},\n      {\"Basic\": {\"name\": \"valve fails\", \"prob\": 0.03}}\n    ]\n  }\n}");
                self.ft_result = gs("ft_result", "");
            }
            ItemKind::Fmea => {
                self.fmea_type = gs("fmea_type", "Design");
                let had_rows = v
                    .get("fmea_rows")
                    .and_then(|x| x.as_array())
                    .map(|a| !a.is_empty())
                    .unwrap_or(false);
                if had_rows {
                    self.fmea_rows =
                        serde_json::from_value(v.get("fmea_rows").cloned().unwrap_or(json!([])))
                            .unwrap_or_default();
                    self.fmea_crit = v
                        .get("fmea_crit")
                        .and_then(|x| serde_json::from_value(x.clone()).ok())
                        .unwrap_or_else(|| {
                            vec![
                                [String::new(), String::new(), String::new(), String::new()];
                                self.fmea_rows.len()
                            ]
                        });
                    self.dvpr_entries = v
                        .get("dvpr")
                        .and_then(|x| x.as_array())
                        .map(|arr| {
                            arr.iter()
                                .map(|e| DvprEntry {
                                    id: Self::gs(e, "id", ""),
                                    failure_mode_ref: Self::gs(e, "failure_mode_ref", ""),
                                    test: Self::gs(e, "test", ""),
                                    sample_size: e
                                        .get("sample_size")
                                        .and_then(|x| x.as_u64())
                                        .unwrap_or(0)
                                        as u32,
                                    acceptance_criteria: Self::gs(e, "acceptance_criteria", ""),
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                } else {
                    self.load_fmea_example();
                }
                self.fmea_csv = gs("fmea_csv", "");
                self.fmea_export_path = gs("fmea_export_path", "fmea_export.csv");
                self.fm_item = gs("fm_item", "");
                self.fm_mode = gs("fm_mode", "");
                self.fm_sev = gs("fm_sev", "5");
                self.fm_occ = gs("fm_occ", "5");
                self.fm_det = gs("fm_det", "5");
                self.fm_alpha = gs("fm_alpha", "");
                self.fm_beta = gs("fm_beta", "");
                self.fm_lambda = gs("fm_lambda", "");
                self.fm_hours = gs("fm_hours", "");
                self.fmea_status = gs("fmea_status", "");
                self.dv_id = gs("dv_id", "");
                self.dv_ref = gs("dv_ref", "");
                self.dv_test = gs("dv_test", "");
                self.dv_sample = gs("dv_sample", "3");
                self.dv_criteria = gs("dv_criteria", "");
            }
            ItemKind::Warranty => {
                self.war_sales = gs("war_sales", "120, 150, 140, 160");
                self.war_returns = gs("war_returns", "0,1\n0,2\n1,2\n1,3\n2,3\n0,3\n2,4\n1,4");
                self.war_current = gs("war_current", "3");
                self.war_horizon = gs("war_horizon", "6");
                self.war_result = gs("war_result", "");
                self.war_excluded_future = v
                    .get("war_excluded_future")
                    .and_then(|x| x.as_u64())
                    .unwrap_or(0) as usize;
                self.war_notes = v
                    .get("war_notes")
                    .and_then(|x| serde_json::from_value(x.clone()).ok())
                    .unwrap_or_default();
                self.war_table = v
                    .get("war_table")
                    .and_then(|x| serde_json::from_value(x.clone()).ok())
                    .unwrap_or_default();
                self.war_sale_labels = v
                    .get("war_sale_labels")
                    .and_then(|x| serde_json::from_value(x.clone()).ok())
                    .unwrap_or_default();
            }
            ItemKind::Prediction => {
                self.pred_parts = gs("pred_parts", "Resistor, 2.5, 10, 1.0, 1.0\nCapacitor, 1.2, 6, 1.1, 1.0\nConnector, 0.8, 4, 1.0, 1.2");
                self.pred_t = gs("pred_t", "1000");
                self.pred_result = gs("pred_result", "");
                self.pred_source = gs("pred_source", "Generic user data");
                self.pred_lib_path = gs("pred_lib_path", "");
                self.pred_lib_parts = Vec::new();
                self.pred_lib_note = gs("pred_lib_note", "");
                self.pred217_file = gs("pred217_file", "resistors.json");
                self.pred217_json = v.get("pred217_json").cloned().filter(|x| !x.is_null());
                self.pred217_named = v
                    .get("pred217_named")
                    .and_then(|x| serde_json::from_value(x.clone()).ok())
                    .unwrap_or_default();
                self.pred217_num = v
                    .get("pred217_num")
                    .and_then(|x| serde_json::from_value(x.clone()).ok())
                    .unwrap_or_default();
                self.pred217_result = gs("pred217_result", "");
                self.alloc_target = gs("alloc_target", "0.99");
                self.alloc_n = gs("alloc_n", "3");
                self.alloc_weights = gs("alloc_weights", "1, 2, 1");
                self.alloc_result = gs("alloc_result", "");
                self.der_name = gs("der_name", "Resistor");
                self.der_operating = gs("der_operating", "0.5");
                self.der_rated = gs("der_rated", "1.0");
                self.der_max = gs("der_max", "0.7");
                self.der_result = gs("der_result", "");
                self.sr_generic = gs("sr_generic", "1.0");
                self.sr_pi_q = gs("sr_pi_q", "1.0");
                self.sr_pi_s = gs("sr_pi_s", "1.0");
                self.sr_ea = gs("sr_ea", "0.35");
                self.sr_temp = gs("sr_temp", "55");
                self.sr_ref_temp = gs("sr_ref_temp", "25");
                self.sr_pi_e = gs("sr_pi_e", "1.0");
                self.sr_qty_rate = gs("sr_qty_rate", "10,1.0\n5,2.0");
                self.sr_obs_rate = gs("sr_obs_rate", "1.2");
                self.sr_pit_obs = gs("sr_pit_obs", "1.0");
                self.sr_pit_tgt = gs("sr_pit_tgt", "1.5");
                self.sr_pis_obs = gs("sr_pis_obs", "1.0");
                self.sr_pis_tgt = gs("sr_pis_tgt", "1.0");
                self.sr_ci_failures = gs("sr_ci_failures", "3");
                self.sr_ci_exposure = gs("sr_ci_exposure", "100000");
                self.sr_ci_conf = gs("sr_ci_conf", "0.90");
                self.sr_m3_pred = gs("sr_m3_pred", "1.0");
                self.sr_m3_pw = gs("sr_m3_pw", "1.0");
                self.sr_m3_obs = gs("sr_m3_obs", "1.4");
                self.sr_m3_ow = gs("sr_m3_ow", "1.0");
                self.mp_base = gs("mp_base", "1.0");
                self.mp_ea = gs("mp_ea", "0.35");
                self.mp_ref = gs("mp_ref", "25");
                self.mp_cyc_factor = gs("mp_cyc_factor", "0.001");
                self.mp_phases = gs(
                    "mp_phases",
                    "Operating, 8000, 55, 100, 0, 1.0\nDormant, 760, 25, 0, 1, 0.1",
                );
                self.pred_clean_result = gs("pred_clean_result", "");
                self.pred_life_dist = gs("pred_life_dist", "weibull2");
                self.pred_life_params = gs("pred_life_params", "beta=1.5, eta=5000");
                self.pred_life_t = gs("pred_life_t", "1000");
                self.pred_life_result = gs("pred_life_result", "");
            }
            ItemKind::Doe => {
                self.doe_k = gs("doe_k", "3");
                self.doe_y = gs("doe_y", "10, 12, 11, 15, 13, 16, 14, 18");
                self.doe_result = gs("doe_result", "");
            }
            ItemKind::Flowchart => {
                self.flow_json = gs("flow_json", "{\n  \"nodes\": [\n    {\"id\": \"s\", \"type\": \"start\", \"next\": \"a\"},\n    {\"id\": \"a\", \"type\": \"task\", \"name\": \"inspect\", \"time\": {\"dist\": \"fixed\", \"value\": 2.0}, \"cost\": 3.0, \"next\": \"b\"},\n    {\"id\": \"b\", \"type\": \"task\", \"name\": \"assemble\", \"time\": {\"dist\": \"fixed\", \"value\": 5.0}, \"next\": \"e\"},\n    {\"id\": \"e\", \"type\": \"end\"}\n  ]\n}");
                self.flow_sims = gs("flow_sims", "5000");
                self.flow_seed = gs("flow_seed", "1");
                self.flow_result = gs("flow_result", "");
            }
            ItemKind::Markov => {
                self.mk_states = v
                    .get("mk_states")
                    .and_then(|x| serde_json::from_value(x.clone()).ok())
                    .unwrap_or_else(|| {
                        vec!["Both up".into(), "One down".into(), "System down".into()]
                    });
                self.mk_transitions = v
                    .get("mk_transitions")
                    .and_then(|x| serde_json::from_value(x.clone()).ok())
                    .unwrap_or_else(|| {
                        vec![
                            ["Both up".into(), "One down".into(), "0.02".into()],
                            ["One down".into(), "Both up".into(), "0.1".into()],
                            ["One down".into(), "System down".into(), "0.01".into()],
                        ]
                    });
                self.mk_new_state = String::new();
                self.mk_absorbing = gs("mk_absorbing", "System down");
                self.mk_start = gs("mk_start", "Both up");
                self.mk_time = gs("mk_time", "100");
                self.mk_result = gs("mk_result", "");
                self.mk_series = Vec::new();
            }
            ItemKind::Qcp => {
                self.qcp_dist = gs("qcp_dist", "weibull2");
                let had = v
                    .get("qcp_params")
                    .and_then(|x| x.as_object())
                    .map(|m| !m.is_empty())
                    .unwrap_or(false);
                if had {
                    self.qcp_params =
                        serde_json::from_value(v.get("qcp_params").cloned().unwrap_or(json!({})))
                            .unwrap_or_default();
                } else {
                    self.qcp_reset_params();
                }
                self.qcp_t = gs("qcp_t", "1000");
                self.qcp_bx = gs("qcp_bx", "0.10");
                self.qcp_result = gs("qcp_result", "");
            }
            ItemKind::Rcm => {
                if let Some(arr) = v.get("rcm_rows").and_then(|x| x.as_array()) {
                    self.rcm_rows = arr.iter().map(RcmRowUi::from_json).collect();
                } else {
                    self.rcm_rows = vec![RcmRowUi {
                        asset: "Pump".into(),
                        function: "Move fluid".into(),
                        mode: "Seal leaks".into(),
                        consequence: "Operational".into(),
                        monitorable: true,
                        age_related: false,
                        restorable: false,
                        rate: "0.00001".into(),
                        alpha: "0.3".into(),
                        beta: "1.0".into(),
                        hours: "8760".into(),
                    }];
                }
            }
            ItemKind::Fracas => {
                if let Some(arr) = v.get("fracas_rows").and_then(|x| x.as_array()) {
                    self.fracas_rows = arr
                        .iter()
                        .map(|r| Incident {
                            id: Self::gs(r, "id", ""),
                            part: Self::gs(r, "part", ""),
                            failure_mode: Self::gs(r, "failure_mode", ""),
                            hours: r.get("hours").and_then(|x| x.as_f64()).unwrap_or(0.0),
                            date: Self::gs(r, "date", ""),
                            status: IncidentStatus::from_str(&Self::gs(r, "status", "Open"))
                                .unwrap_or(IncidentStatus::Open),
                            corrective_action: Self::gs(r, "corrective_action", ""),
                        })
                        .collect();
                } else {
                    self.fracas_rows = vec![
                        Incident {
                            id: "FR-1".into(),
                            part: "Pump seal".into(),
                            failure_mode: "Leak".into(),
                            hours: 1200.0,
                            date: "2026-01-15".into(),
                            status: IncidentStatus::Closed,
                            corrective_action: "Replaced seal material".into(),
                        },
                        Incident {
                            id: "FR-2".into(),
                            part: "Controller board".into(),
                            failure_mode: "Overheat".into(),
                            hours: 3400.0,
                            date: "2026-03-02".into(),
                            status: IncidentStatus::Analysis,
                            corrective_action: String::new(),
                        },
                    ];
                }
                self.fracas_total_hours = gs("fracas_total_hours", "10000");
                self.fracas_path = gs("fracas_path", "fracas.csv");
                self.fracas_status = gs("fracas_status", "");
            }
        }
    }
}

// ---------------- RBD diagram helpers ----------------

impl Work {
    pub fn leaf_paths(spec: &Value) -> Vec<Vec<usize>> {
        fn walk(node: &Value, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
            let ty = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if ty == "block" || ty == "standby_group" {
                out.push(path.clone());
                return;
            }
            if let Some(ch) = node.get("children").and_then(|v| v.as_array()) {
                for (i, c) in ch.iter().enumerate() {
                    path.push(i);
                    walk(c, path, out);
                    path.pop();
                }
            }
        }
        let mut out = Vec::new();
        walk(spec, &mut Vec::new(), &mut out);
        out
    }

    pub fn node_at(spec: &Value, path: &[usize]) -> Option<Value> {
        let mut cur = spec;
        for &i in path {
            cur = cur.get("children")?.get(i)?;
        }
        Some(cur.clone())
    }

    pub fn replace_at(spec: &Value, path: &[usize], new_node: Value) -> Value {
        if path.is_empty() {
            return new_node;
        }
        let mut root = spec.clone();
        fn rec(node: &mut Value, path: &[usize], new_node: Value) {
            if path.len() == 1 {
                if let Some(ch) = node.get_mut("children").and_then(|v| v.as_array_mut()) {
                    if path[0] < ch.len() {
                        ch[path[0]] = new_node;
                    }
                }
                return;
            }
            if let Some(ch) = node.get_mut("children").and_then(|v| v.as_array_mut()) {
                if path[0] < ch.len() {
                    rec(&mut ch[path[0]], &path[1..], new_node);
                }
            }
        }
        rec(&mut root, path, new_node);
        root
    }

    pub fn remove_at(spec: &Value, path: &[usize]) -> Value {
        if path.is_empty() {
            return json!({"type": "series", "children": []});
        }
        let mut root = spec.clone();
        fn rec(node: &mut Value, path: &[usize]) {
            if path.len() == 1 {
                if let Some(ch) = node.get_mut("children").and_then(|v| v.as_array_mut()) {
                    if path[0] < ch.len() {
                        ch.remove(path[0]);
                    }
                }
                return;
            }
            if let Some(ch) = node.get_mut("children").and_then(|v| v.as_array_mut()) {
                if path[0] < ch.len() {
                    rec(&mut ch[path[0]], &path[1..]);
                }
            }
        }
        rec(&mut root, path);
        root
    }

    pub fn leaf_pos(node: &Value, idx: usize) -> (f32, f32) {
        if let Some(a) = node.get("pos").and_then(|v| v.as_array()) {
            if a.len() == 2 {
                return (
                    a[0].as_f64().unwrap_or(30.0) as f32,
                    a[1].as_f64().unwrap_or(90.0) as f32,
                );
            }
        }
        (30.0 + (idx as f32) * 160.0, 90.0)
    }

    /// Stored pos of a leaf node, if it carries one.
    pub fn stored_pos(node: &Value) -> Option<(f32, f32)> {
        let a = node.get("pos").and_then(|v| v.as_array())?;
        if a.len() == 2 {
            Some((
                a[0].as_f64().unwrap_or(30.0) as f32,
                a[1].as_f64().unwrap_or(90.0) as f32,
            ))
        } else {
            None
        }
    }

    // Topology layout (v0.3.1): slots are generated from the same
    // tree that system::evaluate_rbd evaluates, so the drawing
    // cannot misrepresent parallel structure as a series chain.
    // Leaf size is 128x56. Series children go left to right with a
    // 32px gap, top-aligned. Parallel and kofn children stack
    // vertically with a 28px gap (branch pitch 84), sharing the
    // group's left x. Group sizes are recursive: series width is
    // the sum of child widths plus gaps and height is the max
    // child height; parallel/kofn width is the max child width
    // and height is the sum of child heights plus gaps. The root
    // layout origin is (30, 90).
    pub const RBD_LEAF_W: f32 = 128.0;
    pub const RBD_LEAF_H: f32 = 56.0;
    pub const RBD_SERIES_GAP: f32 = 32.0;
    pub const RBD_BRANCH_GAP: f32 = 28.0;
    pub const RBD_ORIGIN_X: f32 = 30.0;
    pub const RBD_ORIGIN_Y: f32 = 90.0;

    fn rbd_node_size(node: &Value) -> (f32, f32) {
        let ty = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if ty == "block" || ty == "standby_group" {
            return (Self::RBD_LEAF_W, Self::RBD_LEAF_H);
        }
        let children: Vec<Value> = node
            .get("children")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        if children.is_empty() {
            return (0.0, 0.0);
        }
        let sizes: Vec<(f32, f32)> = children.iter().map(Self::rbd_node_size).collect();
        match ty {
            "series" => {
                let w = sizes.iter().map(|s| s.0).sum::<f32>()
                    + Self::RBD_SERIES_GAP * (sizes.len() as f32 - 1.0);
                let h = sizes.iter().map(|s| s.1).fold(0.0f32, f32::max);
                (w, h)
            }
            "parallel" | "kofn" => {
                let w = sizes.iter().map(|s| s.0).fold(0.0f32, f32::max);
                let h = sizes.iter().map(|s| s.1).sum::<f32>()
                    + Self::RBD_BRANCH_GAP * (sizes.len() as f32 - 1.0);
                (w, h)
            }
            _ => (Self::RBD_LEAF_W, Self::RBD_LEAF_H),
        }
    }

    fn rbd_assign_slots(node: &Value, x: f32, y: f32, out: &mut Vec<(f32, f32)>) {
        let ty = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if ty == "block" || ty == "standby_group" {
            out.push((x, y));
            return;
        }
        let children: Vec<Value> = node
            .get("children")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        match ty {
            "series" => {
                let mut cx = x;
                for c in &children {
                    Self::rbd_assign_slots(c, cx, y, out);
                    cx += Self::rbd_node_size(c).0 + Self::RBD_SERIES_GAP;
                }
            }
            "parallel" | "kofn" => {
                let mut cy = y;
                for c in &children {
                    Self::rbd_assign_slots(c, x, cy, out);
                    cy += Self::rbd_node_size(c).1 + Self::RBD_BRANCH_GAP;
                }
            }
            _ => {
                for c in &children {
                    Self::rbd_assign_slots(c, x, y, out);
                }
            }
        }
    }

    /// One layout slot per leaf, in leaf_paths order, with the
    /// root of the spec laid out at (30, 90).
    pub fn rbd_layout_slots(spec: &Value) -> Vec<(f32, f32)> {
        let mut out = Vec::new();
        Self::rbd_assign_slots(spec, Self::RBD_ORIGIN_X, Self::RBD_ORIGIN_Y, &mut out);
        out
    }

    /// Size of a subtree under the topology layout, exposed for
    /// the connector and frame geometry in the dispatcher.
    pub fn rbd_subtree_size(node: &Value) -> (f32, f32) {
        Self::rbd_node_size(node)
    }

    /// Write topology layout slots into leaves that do not carry a
    /// pos yet, so positions survive in the diagram JSON itself.
    /// Leaves that already carry a pos keep it: user placement and
    /// drags win over the layout.
    pub fn ensure_rbd_positions(&mut self) {
        let paths = Self::leaf_paths(&self.rbd_spec);
        let slots = Self::rbd_layout_slots(&self.rbd_spec);
        let mut spec = self.rbd_spec.clone();
        for (i, p) in paths.iter().enumerate() {
            if let Some(mut node) = Self::node_at(&spec, p) {
                if node.get("pos").is_none() {
                    let pos = slots.get(i).copied().unwrap_or((30.0, 90.0));
                    node["pos"] = json!([pos.0, pos.1]);
                    spec = Self::replace_at(&spec, p, node);
                }
            }
        }
        self.rbd_spec = spec;
        self.rbd_json = serde_json::to_string_pretty(&self.rbd_spec).unwrap_or_default();
    }

    pub fn set_rbd_spec(&mut self, spec: Value) {
        if self.rbd_spec != spec && !self.rbd_spec.is_null() {
            self.rbd_undo.push(self.rbd_spec.clone());
            if self.rbd_undo.len() > 50 {
                self.rbd_undo.remove(0);
            }
        }
        self.rbd_spec = spec;
        self.rbd_sel = None;
        self.rbd_importance.clear();
        self.ensure_rbd_positions();
        self.rbd_avail = String::new();
    }

    pub fn rbd_reliability(&self) -> Option<f64> {
        let t = self.rbd_t.trim().parse::<f64>().ok()?;
        system::evaluate_rbd(&self.rbd_spec, Some(t)).ok()
    }

    pub fn load_rbd_editor(&mut self, idx: usize) {
        let paths = Self::leaf_paths(&self.rbd_spec);
        if let Some(p) = paths.get(idx) {
            if let Some(node) = Self::node_at(&self.rbd_spec, p) {
                self.ed_name = node
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("block")
                    .to_string();
                self.ed_dist = node
                    .get("dist")
                    .and_then(|v| v.as_str())
                    .unwrap_or("weibull2")
                    .to_string();
                self.ed_params = params_to_text(&node.get("params").cloned().unwrap_or(json!({})));
                self.ed_mtbf = node
                    .get("mtbf")
                    .and_then(|v| v.as_f64())
                    .map(|x| x.to_string())
                    .unwrap_or_default();
                self.ed_mttr = node
                    .get("mttr")
                    .and_then(|v| v.as_f64())
                    .map(|x| x.to_string())
                    .unwrap_or_default();
                self.ed_units = node
                    .get("n_units")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(2)
                    .to_string();
                self.ed_k = node
                    .get("k")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1)
                    .to_string();
                self.ed_dormancy = node
                    .get("dormancy")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
                    .to_string();
                self.ed_switch = node
                    .get("switch_reliability")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1.0)
                    .to_string();
                self.rbd_sel = Some(idx);
            }
        }
    }

    pub fn insert_into_root_series(&mut self, node: Value) {
        let mut spec = self.rbd_spec.clone();
        if spec.get("type").and_then(|v| v.as_str()) == Some("series") {
            if let Some(ch) = spec.get_mut("children").and_then(|v| v.as_array_mut()) {
                ch.push(node);
            }
        } else {
            spec = json!({"type": "series", "children": [spec, node]});
        }
        self.set_rbd_spec(spec);
    }

    /// Place a palette item at canvas position (x, y).
    pub fn rbd_place_armed(&mut self, x: f32, y: f32) {
        let kind = match self.rbd_armed.clone() {
            Some(k) => k,
            None => return,
        };
        let paths = Self::leaf_paths(&self.rbd_spec);
        // Not over an existing block (128x52 blocks)?
        for (i, p) in paths.iter().enumerate() {
            if let Some(node) = Self::node_at(&self.rbd_spec, p) {
                let (bx, by) = Self::leaf_pos(&node, i);
                if x >= bx && x <= bx + 128.0 && y >= by && y <= by + 52.0 {
                    return;
                }
            }
        }
        let n = paths.len() + 1;
        let bx = (x - 20.0).max(0.0);
        let by = (y - 20.0).max(0.0);
        let mk_block = |name: &str, px: f32, py: f32| {
            json!({"type": "block", "name": name, "dist": "weibull2",
                   "params": {"beta": 1.5, "eta": 5000}, "mtbf": 5000, "mttr": 10,
                   "pos": [px, py]})
        };
        match kind.as_str() {
            "standby_group" => {
                let node = json!({"type": "standby_group", "name": format!("standby {}", n), "n_units": 2,
                                  "dist": "exponential", "params": {"theta": 5000}, "mtbf": 5000, "mttr": 8,
                                  "dormancy": 0.0, "switch_reliability": 0.98, "swap_time": 0.5,
                                  "pos": [bx, by]});
                self.insert_into_root_series(node);
            }
            "parallel" => {
                let node = json!({"type": "parallel", "children": [
                    mk_block(&format!("parallel {} A", n), bx, by),
                    mk_block(&format!("parallel {} B", n), bx, by + 84.0),
                ]});
                self.insert_into_root_series(node);
            }
            "kofn" => {
                let node = json!({"type": "kofn", "k": 2, "children": [
                    mk_block(&format!("kofn {} A", n), bx, by),
                    mk_block(&format!("kofn {} B", n), bx, by + 84.0),
                    mk_block(&format!("kofn {} C", n), bx, by + 168.0),
                ]});
                self.insert_into_root_series(node);
            }
            _ => {
                let node = mk_block(&format!("block {}", n), bx, by);
                self.insert_into_root_series(node);
            }
        }
        self.rbd_status = format!("Placed {} from palette.", kind);
        self.note(self.rbd_status.clone());
        self.rbd_armed = None;
    }

    pub fn rbd_move_block(&mut self, idx: usize, x: f32, y: f32) {
        let paths = Self::leaf_paths(&self.rbd_spec);
        if let Some(p) = paths.get(idx).cloned() {
            if let Some(mut node) = Self::node_at(&self.rbd_spec, &p) {
                node["pos"] = json!([x.max(0.0), y.max(0.0)]);
                let spec = Self::replace_at(&self.rbd_spec, &p, node);
                // During a drag, do not push undo entries or clear the
                // selection; just move the spec.
                self.rbd_spec = spec;
                self.rbd_json = serde_json::to_string_pretty(&self.rbd_spec).unwrap_or_default();
            }
        }
    }

    pub fn rbd_apply_editor(&mut self) {
        let sel = match self.rbd_sel {
            Some(s) => s,
            None => return,
        };
        let paths = Self::leaf_paths(&self.rbd_spec);
        if let Some(p) = paths.get(sel).cloned() {
            if let Some(node) = Self::node_at(&self.rbd_spec, &p) {
                let is_standby = node.get("type").and_then(|v| v.as_str()) == Some("standby_group");
                let mut new_node = node.clone();
                new_node["name"] = json!(self.ed_name);
                new_node["dist"] = json!(self.ed_dist);
                new_node["params"] = params_to_json(&parse_params_text(&self.ed_params));
                if let Some(v) = opt_num(&self.ed_mtbf) {
                    new_node["mtbf"] = json!(v);
                }
                if let Some(v) = opt_num(&self.ed_mttr) {
                    new_node["mttr"] = json!(v);
                }
                if is_standby {
                    new_node["n_units"] = json!(inum(&self.ed_units, 2));
                    if let Some(v) = opt_num(&self.ed_dormancy) {
                        new_node["dormancy"] = json!(v);
                    }
                    if let Some(v) = opt_num(&self.ed_switch) {
                        new_node["switch_reliability"] = json!(v);
                    }
                }
                let spec = Self::replace_at(&self.rbd_spec, &p, new_node);
                self.set_rbd_spec(spec);
                self.rbd_sel = Some(sel);
                self.load_rbd_editor(sel);
                self.rbd_status = "Block properties applied.".into();
                self.note(self.rbd_status.clone());
            }
        }
    }

    pub fn rbd_duplicate_block(&mut self) {
        let sel = match self.rbd_sel {
            Some(s) => s,
            None => return,
        };
        let paths = Self::leaf_paths(&self.rbd_spec);
        if let Some(p) = paths.get(sel).cloned() {
            if let Some(node) = Self::node_at(&self.rbd_spec, &p) {
                let mut copy = node.clone();
                let old_name = copy
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("block")
                    .to_string();
                copy["name"] = json!(format!("{} copy", old_name));
                let pos = Self::leaf_pos(&copy, sel);
                copy["pos"] = json!([pos.0 + 30.0, pos.1 + 60.0]);
                self.insert_into_root_series(copy);
                self.rbd_status = "Block duplicated.".into();
                self.note(self.rbd_status.clone());
            }
        }
    }

    pub fn rbd_delete_block(&mut self) {
        if let Some(sel) = self.rbd_sel {
            let paths = Self::leaf_paths(&self.rbd_spec);
            if let Some(path) = paths.get(sel).cloned() {
                let spec = Self::remove_at(&self.rbd_spec, &path);
                self.set_rbd_spec(spec);
                self.rbd_status = "Block deleted.".into();
                self.note("Block deleted. Ctrl+Z undoes RBD changes.");
            }
        }
    }

    pub fn undo_rbd(&mut self) {
        if let Some(prev) = self.rbd_undo.pop() {
            self.rbd_spec = prev;
            self.rbd_sel = None;
            self.rbd_importance.clear();
            self.ensure_rbd_positions();
            self.note("RBD change undone.");
        } else {
            self.note("Nothing to undo in the RBD editor.");
        }
    }

    pub fn rbd_add_block_form(&mut self) {
        let params = parse_params_text(&self.nb_params);
        let node = if self.nb_mode == "active" {
            json!({"type": "block", "name": self.nb_name, "dist": self.nb_dist,
                   "params": params_to_json(&params),
                   "mtbf": fnum(&self.nb_mtbf, 5000.0), "mttr": fnum(&self.nb_mttr, 10.0)})
        } else {
            let dorm = match self.nb_mode.as_str() {
                "hot standby" => 1.0,
                "cold standby" => 0.0,
                _ => fnum(&self.nb_dormancy, 0.5),
            };
            json!({"type": "standby_group", "name": self.nb_name, "n_units": 2,
                   "dist": self.nb_dist, "params": params_to_json(&params),
                   "mtbf": fnum(&self.nb_mtbf, 5000.0), "mttr": fnum(&self.nb_mttr, 10.0),
                   "dormancy": dorm, "switch_reliability": 0.98, "swap_time": 0.5})
        };
        self.insert_into_root_series(node);
        self.rbd_status = "Block added.".into();
        self.note(self.rbd_status.clone());
    }

    pub fn rbd_apply_topology(&mut self) {
        let mut spec = self.rbd_spec.clone();
        if spec.get("children").is_some() {
            match self.rbd_topology.as_str() {
                "parallel" => spec["type"] = json!("parallel"),
                "k-of-n" => {
                    spec["type"] = json!("kofn");
                    spec["k"] = json!(inum(&self.rbd_k, 2));
                }
                _ => spec["type"] = json!("series"),
            }
            self.set_rbd_spec(spec);
        } else {
            self.rbd_status = "Topology applies to a node with children.".into();
            self.note(self.rbd_status.clone());
        }
    }

    pub fn rbd_load_json_text(&mut self) {
        match serde_json::from_str::<Value>(&self.rbd_json) {
            Ok(v) => match system::rbd_from_json(v) {
                Ok(spec) => {
                    self.set_rbd_spec(spec);
                    self.rbd_status = "Diagram loaded.".into();
                    self.note(self.rbd_status.clone());
                }
                Err(e) => self.rbd_status = format!("Error: {}", e),
            },
            Err(e) => self.rbd_status = format!("JSON error: {}", e),
        }
    }

    pub fn rbd_save_file(&mut self) {
        match std::fs::write(&self.rbd_path, &self.rbd_json) {
            Ok(()) => self.rbd_status = format!("Saved to {}", self.rbd_path),
            Err(e) => self.rbd_status = format!("Save error: {}", e),
        }
    }

    pub fn rbd_load_file(&mut self) {
        match std::fs::read_to_string(&self.rbd_path) {
            Ok(text) => match serde_json::from_str::<Value>(&text) {
                Ok(v) => match system::rbd_from_json(v) {
                    Ok(spec) => {
                        self.set_rbd_spec(spec);
                        self.rbd_status = format!("Loaded {}", self.rbd_path);
                        self.note(self.rbd_status.clone());
                    }
                    Err(e) => self.rbd_status = format!("Error: {}", e),
                },
                Err(e) => self.rbd_status = format!("JSON error: {}", e),
            },
            Err(e) => self.rbd_status = format!("Load error: {}", e),
        }
    }

    // ---------------- QCP helpers ----------------

    pub fn qcp_default(dist: &str, name: &str) -> &'static str {
        match (dist, name) {
            ("gamma", "beta") => "2500",
            ("gumbel", "beta") => "1000",
            (_, "beta") => "1.5",
            (_, "eta") => "5000",
            (_, "eta1") => "3000",
            (_, "eta2") => "8000",
            (_, "beta1") => "1.2",
            (_, "beta2") => "2.0",
            (_, "gamma") => "0",
            ("lognormal", "mu") => "8.0",
            ("gumbel", "mu") => "5000",
            ("logistic", "mu") => "5000",
            ("normal", "mu") => "5000",
            (_, "mu") => "5000",
            (_, "sigma") => "0.5",
            (_, "theta") => "5000",
            ("gamma", "alpha") => "2.0",
            (_, "alpha") => "5000",
            (_, "s") => "1000",
            (_, "a") => "0.0001",
            (_, "b") => "0.0001",
            (_, "d") => "2.0",
            (_, "p") => "1.5",
            (_, "w") => "0.5",
            _ => "1.0",
        }
    }

    pub fn qcp_reset_params(&mut self) {
        self.qcp_params.clear();
        if let Some(spec) = dist_mod::dist_spec(&self.qcp_dist) {
            for (name, _kind) in spec {
                self.qcp_params.insert(
                    name.to_string(),
                    Self::qcp_default(&self.qcp_dist, name).to_string(),
                );
            }
        }
    }
}

// ---------------- examples and datasets ----------------

impl Work {
    pub fn example_title(n: usize) -> &'static str {
        match n {
            1 => "Bearing life data",
            2 => "Limited data lots",
            3 => "RBD with cold standby",
            4 => "FMEA sample",
            5 => "ALT dataset",
            6 => "Growth dataset",
            7 => "Degradation units",
            8 => "Markov preset",
            _ => "Example",
        }
    }
    pub fn load_example_item(&mut self, n: usize) {
        match n {
            1 => {
                let idx = self.create_item_silent(
                    ItemKind::LifeData,
                    format!("Example: {}", Self::example_title(1)),
                    json!({}),
                );
                self.select_index(Some(idx));
                let (t, f) = datasets::bearing_life(2026);
                self.life_times = format_life(&t, &f);
                self.mcmc_times = self.life_times.clone();
                self.life_fit = None;
                self.life_result = "Loaded Example 1 (bearing life data).".into();
                self.note("Example 1 loaded as a new Life Data item.");
            }
            2 => {
                let idx = self.create_item_silent(
                    ItemKind::LimitedData,
                    format!("Example: {}", Self::example_title(2)),
                    json!({}),
                );
                self.select_index(Some(idx));
                let groups = datasets::small_sample_groups(2026);
                self.limited_groups = groups
                    .iter()
                    .map(|(t, f)| format_life(t, f))
                    .collect::<Vec<String>>()
                    .join("\n");
                self.limited_result = "Loaded Example 2 (limited data lots).".into();
                self.note("Example 2 loaded as a new Limited Data item.");
            }
            3 => {
                let idx = self.create_item_silent(
                    ItemKind::Rbd,
                    format!("Example: {}", Self::example_title(3)),
                    json!({}),
                );
                self.select_index(Some(idx));
                let spec: Value = serde_json::from_str(EXAMPLE3_JSON).unwrap_or(json!({}));
                self.set_rbd_spec(spec);
                self.rbd_t = "1000".into();
                self.rbd_status = "Loaded Example 3.".into();
                self.note("Example 3 loaded as a new RBD item.");
            }
            4 => {
                let idx = self.create_item_silent(
                    ItemKind::Fmea,
                    format!("Example: {}", Self::example_title(4)),
                    json!({}),
                );
                self.select_index(Some(idx));
                self.load_fmea_example();
                self.fmea_status = "Loaded Example 4 (3 sample rows).".into();
                self.note("Example 4 loaded as a new FMEA item.");
            }
            5 => {
                let idx = self.create_item_silent(
                    ItemKind::Alt,
                    format!("Example: {}", Self::example_title(5)),
                    json!({}),
                );
                self.select_index(Some(idx));
                let (stress, times) = datasets::alt_dataset(2026);
                self.alt_data = stress
                    .iter()
                    .zip(&times)
                    .map(|(s2, t)| format!("{},{}", fmt_num(s2[0]), fmt_num(*t)))
                    .collect::<Vec<String>>()
                    .join("\n");
                self.alt_result = "Loaded ALT example.".into();
                self.note("Example 5 loaded as a new ALT item.");
            }
            6 => {
                let idx = self.create_item_silent(
                    ItemKind::Growth,
                    format!("Example: {}", Self::example_title(6)),
                    json!({}),
                );
                self.select_index(Some(idx));
                self.growth_times = datasets::growth_dataset(2026)
                    .iter()
                    .map(|v| fmt_num(*v))
                    .collect::<Vec<String>>()
                    .join(", ");
                self.growth_result = "Loaded growth example.".into();
                self.note("Example 6 loaded as a new Growth item.");
            }
            7 => {
                let idx = self.create_item_silent(
                    ItemKind::Degradation,
                    format!("Example: {}", Self::example_title(7)),
                    json!({}),
                );
                self.select_index(Some(idx));
                let deg = datasets::degradation_units(2026);
                self.deg_units = deg
                    .iter()
                    .map(|(t, y)| {
                        format!(
                            "{} | {}",
                            t.iter()
                                .map(|v| fmt_num(*v))
                                .collect::<Vec<String>>()
                                .join(","),
                            y.iter()
                                .map(|v| fmt_num(*v))
                                .collect::<Vec<String>>()
                                .join(",")
                        )
                    })
                    .collect::<Vec<String>>()
                    .join("\n");
                self.deg_result = "Loaded degradation example.".into();
                self.note("Example 7 loaded as a new Degradation item.");
            }
            _ => {
                let idx = self.create_item_silent(
                    ItemKind::Markov,
                    format!("Example: {}", Self::example_title(8)),
                    json!({}),
                );
                self.select_index(Some(idx));
                self.mk_states = vec!["Both up".into(), "One down".into(), "System down".into()];
                self.mk_transitions = vec![
                    ["Both up".into(), "One down".into(), "0.02".into()],
                    ["One down".into(), "Both up".into(), "0.1".into()],
                    ["One down".into(), "System down".into(), "0.01".into()],
                ];
                self.mk_absorbing = "System down".into();
                self.mk_start = "Both up".into();
                self.mk_result =
                    "Loaded Markov preset. MTTF from Both up should be 650 hours.".into();
                self.note("Example 8 loaded as a new Markov item.");
            }
        }
    }

    pub fn load_dataset_item(&mut self, file: &str) {
        let path = datasets_dir().join(file);
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                self.note(format!("Dataset load error: {}", e));
                return;
            }
        };
        let kind = dataset_kind(file);
        let idx = self.create_item_silent(kind, format!("{} ({})", file, kind.label()), json!({}));
        self.select_index(Some(idx));
        self.apply_csv_to_current(&text);
        self.note(format!(
            "Loaded dataset {} as a new {} item.",
            file,
            kind.label()
        ));
    }

    /// Load the bundled dataset chosen in the shared CSV controls
    /// into the current item.
    pub fn csv_load_bundled(&mut self) {
        let path = datasets_dir().join(&self.csv_pick);
        match std::fs::read_to_string(&path) {
            Ok(t) => {
                let pick = self.csv_pick.clone();
                self.apply_csv_to_current(&t);
                self.note(format!("Loaded bundled dataset {}", pick));
            }
            Err(e) => self.note(format!("Dataset load error: {}", e)),
        }
    }

    pub fn csv_load_file(&mut self) {
        match std::fs::read_to_string(&self.csv_path) {
            Ok(t) => {
                let p = self.csv_path.clone();
                self.apply_csv_to_current(&t);
                self.note(format!("Loaded CSV file {}", p));
            }
            Err(e) => self.note(format!("CSV load error: {}", e)),
        }
    }

    /// Apply CSV text to the currently selected item, using the same
    /// column rules as that item view's own CSV loader.
    pub fn apply_csv_to_current(&mut self, text: &str) {
        let kind = match self
            .selected
            .and_then(|i| self.items.get(i))
            .map(|it| it.kind)
        {
            Some(k) => k,
            None => return,
        };
        let (header, rows) = csv_table(text);
        match kind {
            ItemKind::LifeData => {
                let ti = col_idx(&header, "time");
                let fi = col_idx(&header, "failed");
                let ci = col_idx(&header, "censored");
                let ni = col_idx(&header, "count");
                if let (Some(t), Some(f)) = (ti, fi.or(ci)) {
                    let is_failed_col = col_idx(&header, "failed").is_some();
                    let mut times: Vec<f64> = Vec::new();
                    let mut failed: Vec<bool> = Vec::new();
                    for r in &rows {
                        let tv_raw = match r.get(t) {
                            Some(s) if !s.trim().is_empty() => s.clone(),
                            _ => {
                                self.set_validation_error(ValidationError::new(
                                    "csv.time",
                                    "",
                                    "missing CSV cell",
                                ));
                                return;
                            }
                        };
                        let fv_raw = match r.get(f) {
                            Some(s) if !s.trim().is_empty() => s.clone(),
                            _ => {
                                self.set_validation_error(ValidationError::new(
                                    "csv.failed",
                                    "",
                                    "missing CSV cell",
                                ));
                                return;
                            }
                        };
                        let tv = match valid_f64_nonneg(&tv_raw, "csv.time") {
                            Ok(v) => v,
                            Err(e) => {
                                self.set_validation_error(e);
                                return;
                            }
                        };
                        let fv = match valid_f64(&fv_raw, "csv.failed") {
                            Ok(v) => v,
                            Err(e) => {
                                self.set_validation_error(e);
                                return;
                            }
                        };
                        let count = match ni {
                            Some(n) => match r.get(n) {
                                Some(s) if !s.trim().is_empty() => {
                                    match valid_usize(s, "csv.count") {
                                        Ok(v) => v,
                                        Err(e) => {
                                            self.set_validation_error(e);
                                            return;
                                        }
                                    }
                                }
                                _ => 1,
                            },
                            None => 1,
                        };
                        let is_failed = if is_failed_col { fv != 0.0 } else { fv == 0.0 };
                        for _ in 0..count {
                            times.push(tv);
                            failed.push(is_failed);
                        }
                    }
                    if !times.is_empty() {
                        self.life_times = format_life(&times, &failed);
                        self.mcmc_times = self.life_times.clone();
                        self.note(format!("Imported {} life data rows from CSV.", times.len()));
                    } else {
                        self.note("CSV import found no usable time rows.");
                    }
                } else {
                    self.note("Life CSV needs a time column plus failed or censored column.");
                }
            }
            ItemKind::Alt => {
                if let (Some(si), Some(ti)) = (col_idx(&header, "stress"), col_idx(&header, "time"))
                {
                    let fi = col_idx(&header, "failed");
                    let s2i = col_idx(&header, "stress2");
                    let mut lines: Vec<String> = Vec::new();
                    for r in &rows {
                        let s2 = r.get(si).cloned().unwrap_or_default();
                        let t = r.get(ti).cloned().unwrap_or_default();
                        let f = fi
                            .and_then(|i| r.get(i))
                            .cloned()
                            .unwrap_or_else(|| "1".into());
                        match s2i {
                            Some(i2) => lines.push(format!(
                                "{},{},{},{}",
                                s2,
                                r.get(i2).cloned().unwrap_or_default(),
                                t,
                                f
                            )),
                            None => lines.push(format!("{},{},{}", s2, t, f)),
                        }
                    }
                    if !lines.is_empty() {
                        self.alt_data = lines.join("\n");
                        self.note(format!("Imported {} ALT rows from CSV.", lines.len()));
                    }
                } else {
                    self.note("ALT CSV needs stress and time columns.");
                }
            }
            ItemKind::Degradation => {
                if let (Some(ui_i), Some(ti), Some(vi)) = (
                    col_idx(&header, "unit"),
                    col_idx(&header, "time"),
                    col_idx(&header, "value"),
                ) {
                    let mut units: Vec<(Vec<f64>, Vec<f64>)> = Vec::new();
                    let mut order: Vec<String> = Vec::new();
                    for r in &rows {
                        let unit = r.get(ui_i).cloned().unwrap_or_default();
                        if !order.contains(&unit) {
                            order.push(unit.clone());
                            units.push((Vec::new(), Vec::new()));
                        }
                        let k = order.iter().position(|u| *u == unit).unwrap_or(0);
                        let traw = match r.get(ti) {
                            Some(s) if !s.trim().is_empty() => s.clone(),
                            _ => {
                                self.set_validation_error(ValidationError::new(
                                    "csv.time",
                                    "",
                                    "missing CSV cell",
                                ));
                                return;
                            }
                        };
                        let vraw = match r.get(vi) {
                            Some(s) if !s.trim().is_empty() => s.clone(),
                            _ => {
                                self.set_validation_error(ValidationError::new(
                                    "csv.value",
                                    "",
                                    "missing CSV cell",
                                ));
                                return;
                            }
                        };
                        let tv2 = match valid_f64_nonneg(&traw, "csv.time") {
                            Ok(v) => v,
                            Err(e) => {
                                self.set_validation_error(e);
                                return;
                            }
                        };
                        let vv2 = match valid_f64(&vraw, "csv.value") {
                            Ok(v) => v,
                            Err(e) => {
                                self.set_validation_error(e);
                                return;
                            }
                        };
                        units[k].0.push(tv2);
                        units[k].1.push(vv2);
                    }
                    if !units.is_empty() {
                        self.deg_units = units
                            .iter()
                            .map(|(t, y)| {
                                format!(
                                    "{} | {}",
                                    t.iter()
                                        .map(|v2| fmt_num(*v2))
                                        .collect::<Vec<String>>()
                                        .join(","),
                                    y.iter()
                                        .map(|v2| fmt_num(*v2))
                                        .collect::<Vec<String>>()
                                        .join(",")
                                )
                            })
                            .collect::<Vec<String>>()
                            .join("\n");
                        self.note(format!(
                            "Imported degradation data for {} units.",
                            units.len()
                        ));
                    }
                } else {
                    self.note("Degradation CSV needs unit, time, value columns.");
                }
            }
            ItemKind::Growth => {
                if let Some(ti) = col_idx(&header, "time") {
                    for r in &rows {
                        match r.get(ti) {
                            Some(s) if !s.trim().is_empty() => {
                                if let Err(e) = valid_f64_nonneg(s, "csv.time") {
                                    self.set_validation_error(e);
                                    return;
                                }
                            }
                            _ => {
                                self.set_validation_error(ValidationError::new(
                                    "csv.time",
                                    "",
                                    "missing CSV cell",
                                ));
                                return;
                            }
                        }
                    }
                    let times: Vec<String> =
                        rows.iter().filter_map(|r| r.get(ti).cloned()).collect();
                    if !times.is_empty() {
                        self.growth_times = times.join(", ");
                        self.note(format!("Imported {} growth failure times.", times.len()));
                    }
                } else {
                    self.note("Growth CSV needs a time column (time,failures format).");
                }
            }
            ItemKind::Competing => {
                if let Some(ti) = col_idx(&header, "time") {
                    let fi = col_idx(&header, "failed");
                    let ci = col_idx(&header, "cause");
                    let mut lines: Vec<String> = Vec::new();
                    for r in &rows {
                        let t = r.get(ti).cloned().unwrap_or_default();
                        if t.trim().is_empty() {
                            self.set_validation_error(ValidationError::new(
                                "csv.time",
                                "",
                                "missing CSV cell",
                            ));
                            return;
                        }
                        if let Err(e) = valid_f64_nonneg(&t, "csv.time") {
                            self.set_validation_error(e);
                            return;
                        }
                        let failed = fi.and_then(|i| r.get(i)).map(|s| s != "0").unwrap_or(true);
                        let cause = ci.and_then(|i| r.get(i)).cloned().unwrap_or_default();
                        lines.push(format!(
                            "{},{}",
                            t,
                            if failed { cause } else { String::new() }
                        ));
                    }
                    if !lines.is_empty() {
                        self.comp_data = lines.join("\n");
                        self.note(format!("Imported {} competing risks rows.", lines.len()));
                    }
                } else {
                    self.note("Competing risks CSV needs time, failed, cause columns.");
                }
            }
            ItemKind::Warranty => {
                if let (Some(sp), Some(sa), Some(lg), Some(rt)) = (
                    col_idx(&header, "ship_period"),
                    col_idx(&header, "sales"),
                    col_idx(&header, "lag_months"),
                    col_idx(&header, "returns"),
                ) {
                    let mut sales_map: BTreeMap<usize, u32> = BTreeMap::new();
                    let mut records: Vec<String> = Vec::new();
                    for r in &rows {
                        let ship = r
                            .get(sp)
                            .and_then(|s2| s2.parse::<usize>().ok())
                            .unwrap_or(1);
                        let sale = r.get(sa).and_then(|s2| s2.parse::<u32>().ok()).unwrap_or(0);
                        let lag = r
                            .get(lg)
                            .and_then(|s2| s2.parse::<usize>().ok())
                            .unwrap_or(0);
                        let ret = r.get(rt).and_then(|s2| s2.parse::<u32>().ok()).unwrap_or(0);
                        sales_map.insert(ship, sale);
                        for _ in 0..ret {
                            records.push(format!("{},{}", ship - 1, ship - 1 + lag));
                        }
                    }
                    let max_ship = sales_map.keys().max().copied().unwrap_or(0);
                    self.war_sales = (1..=max_ship)
                        .map(|k| sales_map.get(&k).copied().unwrap_or(0).to_string())
                        .collect::<Vec<String>>()
                        .join(", ");
                    self.war_returns = records.join("\n");
                    self.note("Imported Nevada warranty CSV. Run the analysis to see the table.");
                } else if col_idx(&header, "time").is_some() {
                    self.note("That file is time-to-failure warranty format. Load it on the Life Data screen, where the grouped time,failed,count format is handled.");
                } else {
                    self.note(
                        "Warranty CSV needs ship_period, sales, lag_months, returns columns.",
                    );
                }
            }
            _ => {}
        }
    }
}

// ---------------- data flows between items ----------------

/// Pure helper for flow (a): set one leaf of an RBD state to a fitted
/// distribution. Returns the block name on success.
pub fn apply_fit_to_rbd_state(
    state: &mut Value,
    block_idx: usize,
    dist: &str,
    params: &dist_mod::Params,
) -> Result<String, String> {
    let spec = state
        .get("rbd_spec")
        .cloned()
        .filter(|x| !x.is_null())
        .unwrap_or_else(|| serde_json::from_str(EXAMPLE3_JSON).unwrap_or(json!({})));
    let paths = Work::leaf_paths(&spec);
    let path = paths
        .get(block_idx)
        .cloned()
        .ok_or_else(|| "Block index out of range.".to_string())?;
    let mut node = Work::node_at(&spec, &path).ok_or_else(|| "Block not found.".to_string())?;
    let name = node
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("block")
        .to_string();
    node["dist"] = json!(dist);
    node["params"] = params_to_json(params);
    let new_spec = Work::replace_at(&spec, &path, node);
    state["rbd_spec"] = new_spec.clone();
    state["rbd_json"] = json!(serde_json::to_string_pretty(&new_spec).unwrap_or_default());
    Ok(name)
}

impl Work {
    pub fn flow_send_fit_to_qcp(&mut self) {
        let fit = match self.life_fit.clone() {
            Some(f) => f,
            None => {
                self.note("Run a fit first, then send it to QCP.");
                return;
            }
        };
        let idx = self.find_or_create_item(ItemKind::Qcp, "QCP 1");
        let mut params = serde_json::Map::new();
        for (k, val) in &fit.params {
            params.insert(k.clone(), json!(val.to_string()));
        }
        let mut st = self.items[idx].state.clone();
        if !st.is_object() {
            st = json!({});
        }
        st["qcp_dist"] = json!(fit.dist);
        st["qcp_params"] = Value::Object(params);
        st["qcp_t"] = json!(self.life_t);
        let name = self.items[idx].name.clone();
        self.items[idx].state = st;
        self.headline = format!("Fit sent to QCP item {}", name);
        self.note(format!(
            "Sent {} fit to QCP item {}. Open it to run the calculation.",
            fit.dist, name
        ));
    }

    pub fn flow_send_fit_to_prediction(&mut self) {
        let fit = match self.life_fit.clone() {
            Some(f) => f,
            None => {
                self.note("Run a fit first, then send it to Prediction.");
                return;
            }
        };
        let idx = self.find_or_create_item(ItemKind::Prediction, "Prediction 1");
        let params_text = fit
            .params
            .iter()
            .map(|(k, val)| format!("{}={}", k, val))
            .collect::<Vec<String>>()
            .join(", ");
        let mut st = self.items[idx].state.clone();
        if !st.is_object() {
            st = json!({});
        }
        st["pred_life_dist"] = json!(fit.dist);
        st["pred_life_params"] = json!(params_text);
        st["pred_life_t"] = json!(self.life_t);
        let name = self.items[idx].name.clone();
        self.items[idx].state = st;
        self.headline = format!("Fit sent to Prediction item {}", name);
        self.note(format!("Sent {} fit to Prediction item {}. Its life model section shows R(t) for the fitted model.", fit.dist, name));
    }

    /// Block names of the currently targeted RBD item, for the
    /// Life Data send-to panel.
    pub fn flow_target_block_names(&self) -> Vec<String> {
        let st = self
            .items
            .iter()
            .find(|it| it.id == self.flow_target_id)
            .map(|it| it.state.clone())
            .unwrap_or(json!({}));
        let spec = st
            .get("rbd_spec")
            .cloned()
            .filter(|x| !x.is_null())
            .unwrap_or_else(|| serde_json::from_str(EXAMPLE3_JSON).unwrap_or(json!({})));
        Self::leaf_paths(&spec)
            .iter()
            .enumerate()
            .map(|(i, path)| {
                Self::node_at(&spec, path)
                    .and_then(|n| {
                        n.get("name")
                            .and_then(|v| v.as_str())
                            .map(|x| x.to_string())
                    })
                    .unwrap_or_else(|| format!("block {}", i + 1))
            })
            .collect()
    }

    pub fn flow_assign_fit_to_rbd(&mut self) {
        let fit = match self.life_fit.clone() {
            Some(f) => f,
            None => {
                self.note("Run a fit first, then assign it to an RBD block.");
                return;
            }
        };
        let idx = match self
            .items
            .iter()
            .position(|it| it.id == self.flow_target_id)
        {
            Some(i) => i,
            None => {
                self.note("Pick a target RBD item first.");
                return;
            }
        };
        let mut st = self.items[idx].state.clone();
        match apply_fit_to_rbd_state(&mut st, self.flow_block_idx, &fit.dist, &fit.params) {
            Ok(block_name) => {
                let item_name = self.items[idx].name.clone();
                self.items[idx].state = st;
                self.headline = format!("Block {} now uses {}", block_name, fit.dist);
                self.note(format!("Assigned {} fit to block {} in RBD item {}. Its reliability recomputes when you open it.", fit.dist, block_name, item_name));
            }
            Err(e) => self.note(e),
        }
    }

    pub fn flow_fmea_to_rcm(&mut self, row_idx: usize) {
        let row = match self.fmea_rows.get(row_idx).cloned() {
            Some(r) => r,
            None => {
                self.note("Pick a valid FMEA row first.");
                return;
            }
        };
        let idx = self.find_or_create_item(ItemKind::Rcm, "RCM 1");
        let new_row = json!({
            "asset": row.item, "function": row.function, "mode": row.failure_mode,
            "consequence": "Operational", "monitorable": false, "age_related": false,
            "restorable": false, "rate": "", "alpha": "", "beta": "", "hours": "",
        });
        let mut st = self.items[idx].state.clone();
        if !st.is_object() {
            st = json!({});
        }
        let mut rows: Vec<Value> = st
            .get("rcm_rows")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        rows.push(new_row);
        st["rcm_rows"] = json!(rows);
        let name = self.items[idx].name.clone();
        self.items[idx].state = st;
        self.note(format!(
            "Created an RCM row in {} from FMEA row {} ({} / {}).",
            name,
            row_idx + 1,
            row.item,
            row.failure_mode
        ));
    }

    pub fn flow_rcm_row_from_field(&mut self) {
        let idx = inum(&self.fmea_rcm_row, 1).saturating_sub(1);
        self.flow_fmea_to_rcm(idx);
    }

    pub fn flow_fracas_to_life(&mut self) {
        let times: Vec<String> = self
            .fracas_rows
            .iter()
            .map(|inc| fmt_num(inc.hours))
            .collect();
        let n = times.len();
        if n == 0 {
            self.note("No incidents to export. Add an incident first.");
            return;
        }
        let st = json!({
            "life_times": times.join(", "), "life_dist": "weibull2", "life_t": "1000",
            "life_result": "Exported from FRACAS. All incidents are treated as failures; the FRACAS log has no censoring notion.",
        });
        let idx = self.create_item_silent(
            ItemKind::LifeData,
            "FRACAS export (all incidents as failures)".into(),
            st,
        );
        self.select_index(Some(idx));
        self.note(format!(
            "Created a Life Data item from {} FRACAS incidents. All incidents count as failures.",
            n
        ));
    }
}

// ---------------- computations ----------------

impl Work {
    pub fn compute_life_fit(&mut self) -> Vec<(String, String)> {
        let (times, failed) = match valid_parse_life(&self.life_times, "LifeUi.times") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.life_result);
            }
        };
        let t_val = match valid_f64_nonneg(&self.life_t, "LifeUi.t") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.life_result);
            }
        };
        let res = if self.life_rank {
            dist_mod::fit_rank_regression(&self.life_dist, &times, Some(&failed))
        } else {
            dist_mod::fit_mle(&self.life_dist, &times, Some(&failed))
        };
        match res {
            Ok(fit) => {
                let info = fit_status_info(&fit);
                self.life_fit_status = info.status.clone();
                self.life_fit_status_note = info.note.clone();
                self.life_fit_warnings = info.warnings.clone();
                if info.status == "failed" {
                    self.life_fit = None;
                    self.headline = if info.note.is_empty() {
                        "Fit failed".into()
                    } else {
                        info.note.clone()
                    };
                    self.life_result = format!(
                        "Fit status: unavailable: {}",
                        if info.note.is_empty() {
                            "fit failed".into()
                        } else {
                            info.note.clone()
                        }
                    );
                    self.note("Life data fit failed.");
                    return rows_of(&self.life_result);
                }
                let t = t_val;
                let (lo, hi) = fit.reliability_bounds(t, 0.05);
                let mut rows: Vec<(String, String)> = vec![
                    ("Fit".into(), fit.summary().replace('\n', " | ")),
                    (format!("R({:.0})", t), format!("{:.4}", fit.reliability(t))),
                    ("95% bounds".into(), format!("{:.4} to {:.4}", lo, hi)),
                    (
                        "Lower bound (one-sided 95%)".into(),
                        format!("{:.4}", fit.reliability_lower_bound(t, 0.05)),
                    ),
                    ("B10".into(), format!("{:.1}", fit.b_life(0.10))),
                    ("Mean life".into(), format!("{:.1}", fit.mean_life())),
                ];
                for (k, v) in &fit.params {
                    rows.push((format!("Parameter {}", k), format!("{:.6}", v)));
                }
                if info.status == "boundary_or_degenerate" || info.status == "success_with_warnings"
                {
                    rows.insert(
                        0,
                        (
                            "Fit status".into(),
                            format!("{}: {}", info.status, info.note),
                        ),
                    );
                    self.headline = if info.note.is_empty() {
                        format!("R({:.0}) = {:.4}", t, fit.reliability(t))
                    } else {
                        info.note.clone()
                    };
                } else {
                    self.headline = format!("R({:.0}) = {:.4}", t, fit.reliability(t));
                }
                for r in rows.iter_mut() {
                    if r.1.contains("NaN") || r.1.contains("inf") {
                        r.1 = "unavailable: non-finite result".into();
                    }
                }
                self.life_result = rows_text(&rows);
                self.note("Life data fit complete.");
                self.life_fit = Some(fit);
                self.mark_computed();
                rows
            }
            Err(e) => {
                self.life_result = format!("Error: {}", e);
                self.life_fit = None;
                self.life_fit_status = "failed".to_string();
                self.life_fit_status_note = e.clone();
                self.life_fit_warnings = Vec::new();
                self.headline = e.clone();
                self.note("Life data fit failed.");
                rows_of(&self.life_result)
            }
        }
    }

    pub fn compute_wizard(&mut self) {
        let (times, failed) = match valid_parse_life(&self.life_times, "LifeUi.times") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return;
            }
        };
        self.wizard_ranks =
            wizard::rank_distributions(&times, Some(&failed), wizard::RANKABLE_DISTS);
        self.mark_computed();
        self.wizard_note = format!("Ranked {} candidates.", self.wizard_ranks.len());
        self.note("Distribution wizard ranking complete.");
    }

    pub fn wizard_use(&mut self, idx: usize) {
        let r = match self.wizard_ranks.get(idx).cloned() {
            Some(r) if r.success => r,
            _ => return,
        };
        let data = self.life_times.clone();
        let dist = r.dist.clone();
        let idx2 = self.create_item_silent(
            ItemKind::LifeData,
            self.default_item_name(ItemKind::LifeData),
            json!({}),
        );
        self.select_index(Some(idx2));
        self.life_times = data;
        self.life_dist = dist.clone();
        let (times, failed) = parse_life(&self.life_times);
        if let Ok(fit) = dist_mod::fit_mle(&dist, &times, Some(&failed)) {
            self.headline = format!("Wizard fit: {}", dist_mod::dist_label(&fit.dist));
            self.life_result = fit.summary().replace('\n', " | ");
            self.life_fit = Some(fit);
        }
        self.note(format!(
            "Wizard created a new Life Data item with {} preselected and the current data loaded.",
            dist
        ));
    }

    pub fn compute_limited(&mut self) -> Vec<(String, String)> {
        if self.limited_groups.trim().is_empty() {
            self.set_validation_error(ValidationError::new(
                "LimitedUi.groups",
                "",
                "empty required field",
            ));
            return rows_of(&self.limited_result);
        }
        let mut groups: Vec<(Vec<f64>, Vec<bool>)> = Vec::new();
        for l in self.limited_groups.lines().filter(|l| !l.trim().is_empty()) {
            match valid_parse_life(l, "LimitedUi.groups") {
                Ok(g) => groups.push(g),
                Err(e) => {
                    self.set_validation_error(e);
                    return rows_of(&self.limited_result);
                }
            }
        }
        match bayes::hierarchical_weibull(&groups) {
            Ok(h) => {
                let mut rows: Vec<(String, String)> = vec![
                    ("Pooled beta".into(), format!("{:.3}", h.pooled_beta)),
                    ("Mu log eta".into(), format!("{:.3}", h.mu_log_eta)),
                    ("Tau squared".into(), format!("{:.4}", h.tau2)),
                ];
                for (i, f) in h.fits.iter().enumerate() {
                    rows.push((
                        format!("Lot {} shrunk eta", i + 1),
                        format!(
                            "{:.1} (MLE eta {:.1}, shrink B {:.2})",
                            f.params["eta"], h.group_fits[i].params["eta"], h.b[i]
                        ),
                    ));
                }
                if let Some(first) = h.group_fits.first() {
                    if let Ok(bc) = bayes::bias_corrected_weibull(first) {
                        rows.push((
                            "Bias correction, lot 1".into(),
                            format!(
                                "beta {:.3} to {:.3}, eta {:.1} to {:.1} (factor {:.3})",
                                first.params["beta"],
                                bc.params["beta"],
                                first.params["eta"],
                                bc.params["eta"],
                                bayes::weibull_shape_bias_factor(first.n_fail)
                            ),
                        ));
                    }
                    let calib: Vec<f64> =
                        groups.iter().flat_map(|(t, _)| t.iter().cloned()).collect();
                    rows.push((
                        "Conformal mission time (alpha 0.10)".into(),
                        format!("{:.1}", bayes::conformal_mission_time(first, &calib, 0.10)),
                    ));
                    rows.push((
                        "Conformal R lower bound at t=1000 (alpha 0.05)".into(),
                        format!(
                            "{:.3}",
                            bayes::conformal_reliability_lower(&calib, 1000.0, 0.05)
                        ),
                    ));
                }
                if let Some(f0) = h.fits.first() {
                    let info = fit_status_info(f0);
                    self.limited_fit_status = info.status;
                    self.limited_fit_status_note = info.note;
                    self.limited_fit_warnings = info.warnings;
                } else {
                    self.limited_fit_status = "success".into();
                    self.limited_fit_status_note = String::new();
                    self.limited_fit_warnings = Vec::new();
                }
                for r in rows.iter_mut() {
                    if r.1.contains("NaN") || r.1.contains("inf") {
                        r.1 = "unavailable: non-finite result".into();
                    }
                }
                self.limited_result = rows_text(&rows);
                self.note("Hierarchical fit complete.");
                self.mark_computed();
                rows
            }
            Err(e) => {
                self.limited_result = format!("Error: {}", e);
                rows_of(&self.limited_result)
            }
        }
    }

    pub fn compute_mcmc(&mut self) -> Vec<(String, String)> {
        let (times, failed) = match valid_parse_life(&self.mcmc_times, "McmcUi.times") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.mcmc_result);
            }
        };
        let draws = match valid_usize(&self.mcmc_draws, "McmcUi.draws") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.mcmc_result);
            }
        };
        let warmup = match valid_usize(&self.mcmc_warmup, "McmcUi.warmup") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.mcmc_result);
            }
        };
        let chains = match valid_usize(&self.mcmc_chains, "McmcUi.chains") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.mcmc_result);
            }
        };
        let seed_i = match valid_int(&self.mcmc_seed, "McmcUi.seed") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.mcmc_result);
            }
        };
        if seed_i < 0 {
            self.set_validation_error(ValidationError::new(
                "McmcUi.seed",
                &self.mcmc_seed.clone(),
                "negative value is not allowed",
            ));
            return rows_of(&self.mcmc_result);
        }
        let _t_check = match valid_f64_nonneg(&self.mcmc_t, "McmcUi.t") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.mcmc_result);
            }
        };
        let res = mcmc::fit_bayes(
            &self.mcmc_dist,
            &times,
            Some(&failed),
            draws,
            warmup,
            chains,
            seed_i as u64,
        );
        match res {
            Ok(r) => {
                let mut rows: Vec<(String, String)> = Vec::new();
                for (name, ps) in r.summary() {
                    rows.push((
                        format!("{} posterior", name),
                        format!(
                            "mean {:.3}, sd {:.3}, q05 {:.3}, median {:.3}, q95 {:.3}",
                            ps.mean, ps.sd, ps.q05, ps.q50, ps.q95
                        ),
                    ));
                }
                // R-hat and ESS are plain numbers, one labeled row per
                // parameter (never a quoted string array).
                let rhat = r.rhat();
                let ess = r.ess_values();
                for (j, name) in r.param_names.iter().enumerate() {
                    if let Some(v) = rhat.get(j) {
                        rows.push((format!("{} R-hat", name), format!("{:.3}", v)));
                    }
                    if let Some(v) = ess.get(j) {
                        rows.push((format!("{} ESS", name), format!("{:.0}", v)));
                    }
                }
                let t = _t_check;
                let ci = r.reliability_credible(t, 0.90, 0);
                rows.push((
                    format!("R({:.0}) credible", t),
                    format!(
                        "mean {:.3}, 90% CI {:.3} to {:.3}",
                        ci.mean, ci.lower, ci.upper
                    ),
                ));
                rows.push((
                    "Posterior predictive p-value".into(),
                    format!("{:.3}", r.posterior_predictive_pvalue(7)),
                ));
                self.mcmc_result = rows_text(&rows);
                self.headline = format!("R({:.0}) mean {:.3}", t, ci.mean);
                self.note("MCMC run complete.");
                self.mark_computed();
                rows
            }
            Err(e) => {
                self.mcmc_result = format!("Error: {}", e);
                rows_of(&self.mcmc_result)
            }
        }
    }

    pub fn deg_parse_units(&self) -> Vec<(Vec<f64>, Vec<f64>)> {
        let mut units: Vec<(Vec<f64>, Vec<f64>)> = Vec::new();
        for line in self.deg_units.lines() {
            if let Some((a, b)) = line.split_once('|') {
                units.push((parse_numbers(a), parse_numbers(b)));
            }
        }
        units
    }
    pub fn deg_parse_units_valid(&self) -> VResult<Vec<(Vec<f64>, Vec<f64>)>> {
        if self.deg_units.trim().is_empty() {
            return Err(ValidationError::new(
                "DegUi.units",
                "",
                "empty required field",
            ));
        }
        let mut units = Vec::new();
        for line in self.deg_units.lines() {
            if line.trim().is_empty() {
                continue;
            }
            match line.split_once('|') {
                Some((a, b)) => {
                    let tv = valid_parse_numbers_nonneg(a, "DegUi.units")?;
                    let yv = valid_parse_numbers(b, "DegUi.units")?;
                    if tv.len() != yv.len() {
                        return Err(ValidationError::new(
                            "DegUi.units",
                            line,
                            "times and values counts differ",
                        ));
                    }
                    units.push((tv, yv));
                }
                None => {
                    return Err(ValidationError::new(
                        "DegUi.units",
                        line,
                        "expected times | values",
                    ))
                }
            }
        }
        if units.is_empty() {
            return Err(ValidationError::new(
                "DegUi.units",
                "",
                "empty required field",
            ));
        }
        Ok(units)
    }

    pub fn compute_degradation(&mut self) -> Vec<(String, String)> {
        let units = match self.deg_parse_units_valid() {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.deg_result);
            }
        };
        let thr = match valid_f64(&self.deg_threshold, "DegUi.threshold") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.deg_result);
            }
        };
        self.deg_parsed = units.clone();
        match degradation::degradation_analysis(
            &units,
            thr,
            &self.deg_model,
            &self.deg_dist,
            self.deg_increasing,
        ) {
            Ok(r) => {
                let mut rows: Vec<(String, String)> = Vec::new();
                for (i, pt) in r.pseudo_times.iter().enumerate() {
                    rows.push((
                        format!("Unit {} pseudo-failure time", i + 1),
                        if pt.is_finite() {
                            format!(
                                "{:.1} ({})",
                                pt,
                                if r.failed[i] { "failed" } else { "censored" }
                            )
                        } else {
                            "never crosses".into()
                        },
                    ));
                }
                if let Some(f) = &r.fit {
                    let info = fit_status_info(f);
                    self.deg_fit_status = info.status;
                    self.deg_fit_status_note = info.note;
                    self.deg_fit_warnings = info.warnings;
                    rows.push(("Life fit".into(), f.summary().replace('\n', " | ")));
                } else {
                    self.deg_fit_status = "success".into();
                    self.deg_fit_status_note = String::new();
                    self.deg_fit_warnings = Vec::new();
                }
                for r2 in rows.iter_mut() {
                    if r2.1.contains("NaN") || r2.1.contains("inf") {
                        r2.1 = "unavailable: non-finite result".into();
                    }
                }
                self.deg_result = rows_text(&rows);
                self.deg_data = Some(r);
                self.note("Degradation analysis complete.");
                self.mark_computed();
                rows
            }
            Err(e) => {
                self.deg_result = format!("Error: {}", e);
                rows_of(&self.deg_result)
            }
        }
    }

    pub fn compute_alt(&mut self) -> Vec<(String, String)> {
        let mut stress: Vec<Vec<f64>> = Vec::new();
        let mut times: Vec<f64> = Vec::new();
        let mut failed: Vec<bool> = Vec::new();
        if self.alt_data.trim().is_empty() {
            self.set_validation_error(ValidationError::new(
                "AltUi.data",
                "",
                "empty required field",
            ));
            return rows_of(&self.alt_result);
        }
        for line in self.alt_data.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let cells: Vec<&str> = line.split(',').map(|c| c.trim()).collect();
            if cells.iter().any(|c| c.is_empty()) {
                self.set_validation_error(ValidationError::new(
                    "AltUi.data",
                    line,
                    "missing cell in ALT data row",
                ));
                return rows_of(&self.alt_result);
            }
            let mut vals: Vec<f64> = Vec::new();
            for c in &cells {
                match valid_f64(c, "AltUi.data") {
                    Ok(v) => vals.push(v),
                    Err(mut e) => {
                        e.token = line.to_string();
                        self.set_validation_error(e);
                        return rows_of(&self.alt_result);
                    }
                }
            }
            if vals.len() < 2 {
                self.set_validation_error(ValidationError::new(
                    "AltUi.data",
                    line,
                    "ALT data row needs at least stress and time",
                ));
                return rows_of(&self.alt_result);
            }
            if vals[1] < 0.0 {
                self.set_validation_error(ValidationError::new(
                    "AltUi.data",
                    line,
                    "negative time is not allowed",
                ));
                return rows_of(&self.alt_result);
            }
            if vals.len() >= 2 {
                if self.alt_model == "eyring" && vals.len() >= 3 {
                    stress.push(vec![vals[0], vals[1]]);
                    times.push(vals[2]);
                    failed.push(if vals.len() >= 4 {
                        vals[3] != 0.0
                    } else {
                        true
                    });
                } else {
                    stress.push(vec![vals[0]]);
                    times.push(vals[1]);
                    failed.push(if vals.len() >= 3 {
                        vals[2] != 0.0
                    } else {
                        true
                    });
                }
            }
        }
        let use_s = match valid_parse_numbers(&self.alt_use_stress, "AltUi.use-stress") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.alt_result);
            }
        };
        let t = match valid_f64_nonneg(&self.alt_t, "AltUi.t") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.alt_result);
            }
        };
        let mut rows: Vec<(String, String)> = Vec::new();
        self.alt_series = Vec::new();
        match alt_mod::fit_alt(&self.alt_model, &stress, &times) {
            Ok(f) => {
                rows.push((
                    "Least squares coefficients".into(),
                    f.coef
                        .iter()
                        .map(|(k, v)| format!("{}={:.4}", k, v))
                        .collect::<Vec<String>>()
                        .join(", "),
                ));
                rows.push(("Least squares sigma".into(), format!("{:.3}", f.sigma)));
                rows.push((
                    "Median life at use stress".into(),
                    format!("{:.1}", f.median_life(&use_s)),
                ));
                rows.push((
                    format!("Least squares R({:.0})", t),
                    format!("{:.4}", f.reliability(t, &use_s)),
                ));
                if let Some(ea) = f.activation_energy_ev() {
                    rows.push(("Activation energy".into(), format!("{:.3} eV", ea)));
                }
                let med = f.median_life(&use_s);
                if med.is_finite() && med > 0.0 {
                    self.alt_series = (0..=60)
                        .map(|k| {
                            let tt = med * 2.5 * k as f64 / 60.0;
                            (tt, f.reliability(tt, &use_s))
                        })
                        .collect();
                }
            }
            Err(e) => rows.push(("Least squares error".into(), e)),
        }
        match alt_mod::fit_alt_mle(&self.alt_model, &stress, &times, Some(&failed)) {
            Ok(f) => {
                let (lo, hi) = f.reliability_bounds(t, &use_s, 0.05);
                rows.push((
                    "MLE coefficients".into(),
                    f.coef
                        .iter()
                        .map(|(k, v)| format!("{}={:.4}", k, v))
                        .collect::<Vec<String>>()
                        .join(", "),
                ));
                rows.push(("MLE sigma".into(), format!("{:.3}", f.sigma)));
                rows.push(("MLE loglik".into(), format!("{:.2}", f.loglik)));
                rows.push((
                    format!("MLE R({:.0}) at use stress", t),
                    format!(
                        "{:.4} (bounds {:.4} to {:.4})",
                        f.reliability(t, &use_s),
                        lo,
                        hi
                    ),
                ));
                if let Some(ea) = f.activation_energy_ev() {
                    rows.push(("MLE activation energy".into(), format!("{:.3} eV", ea)));
                }
            }
            Err(e) => rows.push(("MLE error".into(), e)),
        }
        // ALT status: success if either fit produced rows without error
        if rows.iter().any(|(l, _)| l.contains("error")) && rows.len() <= 2 {
            self.alt_fit_status = "failed".into();
            self.alt_fit_status_note = rows.first().map(|(_, v)| v.clone()).unwrap_or_default();
        } else {
            self.alt_fit_status = "success".into();
            self.alt_fit_status_note = String::new();
        }
        self.alt_fit_warnings = Vec::new();
        for r in rows.iter_mut() {
            if r.1.contains("NaN") || r.1 == "inf" || r.1.contains(": inf") {
                r.1 = "unavailable: non-finite result".into();
            }
        }
        self.alt_result = rows_text(&rows);
        self.note("ALT fit complete.");
        self.mark_computed();
        rows
    }

    pub fn compute_growth(&mut self) -> Vec<(String, String)> {
        let times = match valid_parse_numbers_nonneg(&self.growth_times, "GrowthUi.times") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.growth_result);
            }
        };
        let ca = growth::fit_crow_amsaa(&times, None);
        let mut rows: Vec<(String, String)> = vec![
            ("Crow-AMSAA beta".into(), format!("{:.3}", ca.beta)),
            ("Crow-AMSAA lambda".into(), format!("{:.5}", ca.lam)),
            ("Failures n".into(), format!("{}", ca.n_fail)),
            (
                format!("Instantaneous MTBF at T={:.0}", ca.t_end),
                format!("{:.1}", ca.instantaneous_mtbf(ca.t_end)),
            ),
            (
                "Cumulative MTBF".into(),
                format!("{:.1}", ca.cumulative_mtbf(ca.t_end)),
            ),
        ];
        match growth::fit_duane(&times) {
            Ok(du) => {
                rows.push(("Duane alpha".into(), format!("{:.3}", du.alpha)));
                rows.push((
                    "Duane instantaneous MTBF at T".into(),
                    format!("{:.1}", du.instantaneous_mtbf_at(ca.t_end)),
                ));
            }
            Err(e) => rows.push(("Duane error".into(), e)),
        }
        self.growth_result = rows_text(&rows);
        self.growth_fit = Some(ca);
        self.note("Growth fit complete.");
        self.mark_computed();
        rows
    }

    pub fn compute_competing(&mut self) -> Vec<(String, String)> {
        let mut times: Vec<f64> = Vec::new();
        let mut mode_strings: Vec<Option<String>> = Vec::new();
        if self.comp_data.trim().is_empty() {
            self.set_validation_error(ValidationError::new(
                "CompUi.data",
                "",
                "empty required field",
            ));
            return rows_of(&self.comp_result);
        }
        for line in self.comp_data.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match line.split_once(',') {
                Some((a, b)) => match valid_f64_nonneg(a.trim(), "CompUi.data") {
                    Ok(t) => {
                        times.push(t);
                        let m = b.trim().to_string();
                        mode_strings.push(if m.is_empty() { None } else { Some(m) });
                    }
                    Err(mut e) => {
                        e.token = a.trim().to_string();
                        self.set_validation_error(e);
                        return rows_of(&self.comp_result);
                    }
                },
                None => {
                    self.set_validation_error(ValidationError::new(
                        "CompUi.data",
                        line,
                        "expected time,cause",
                    ));
                    return rows_of(&self.comp_result);
                }
            }
        }
        let comp_t_val = match valid_f64_nonneg(&self.comp_t, "CompUi.t") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.comp_result);
            }
        };
        let events: Vec<Option<&str>> = mode_strings.iter().map(|o| o.as_deref()).collect();
        match competing::fit_competing_risks(&times, &events, &self.comp_dist) {
            Ok(cr) => {
                let t = comp_t_val;
                let mut modes: Vec<String> = Vec::new();
                for m in mode_strings.iter().flatten() {
                    if !modes.contains(m) {
                        modes.push(m.clone());
                    }
                }
                modes.sort();
                let mut rows: Vec<(String, String)> = Vec::new();
                for (name, fit) in &cr.modes {
                    rows.push((
                        format!("Mode {} fit", name),
                        fit.summary().replace('\n', " | "),
                    ));
                }
                rows.push((
                    format!("Overall reliability at {:.0}", t),
                    format!("{:.4}", competing::overall_reliability(&cr, t)),
                ));
                for m in &modes {
                    rows.push((
                        format!("CIF {} at {:.0}", m, t),
                        format!("{:.4}", competing::cumulative_incidence(&cr, m, t)),
                    ));
                }
                for (name, p) in competing::failure_probabilities(&cr, t) {
                    rows.push((
                        format!("Failure probability {} at {:.0}", name, t),
                        format!("{:.4}", p),
                    ));
                }
                if let Some((_, f0)) = cr.modes.first() {
                    let info = fit_status_info(f0);
                    self.comp_fit_status = info.status;
                    self.comp_fit_status_note = info.note;
                    self.comp_fit_warnings = info.warnings;
                } else {
                    self.comp_fit_status = "success".into();
                    self.comp_fit_status_note = String::new();
                    self.comp_fit_warnings = Vec::new();
                }
                for r in rows.iter_mut() {
                    if r.1.contains("NaN") || r.1.contains("inf") {
                        r.1 = "unavailable: non-finite result".into();
                    }
                }
                self.comp_result = rows_text(&rows);
                self.note("Competing risks fit complete.");
                self.mark_computed();
                rows
            }
            Err(e) => {
                self.comp_result = format!("Error: {}", e);
                rows_of(&self.comp_result)
            }
        }
    }
}

impl Work {
    pub fn compute_rbd_availability(&mut self) -> Vec<(String, String)> {
        let horizon = match valid_f64_nonneg(&self.rbd_horizon, "RbdUi.horizon") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.rbd_avail);
            }
        };
        let _ = match valid_f64_nonneg(&self.rbd_t, "RbdUi.t") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.rbd_avail);
            }
        };
        let n_sim = match valid_usize(&self.rbd_sims, "RbdUi.sims") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.rbd_avail);
            }
        };
        let res = system::simulate_availability(&self.rbd_spec, horizon, n_sim, 3, 20);
        let rows: Vec<(String, String)> = vec![
            (
                "Mean availability".into(),
                format!("{:.4}", res.mean_availability),
            ),
            (
                "Expected downtime".into(),
                format!("{:.1}", res.expected_downtime),
            ),
            ("Mean failures".into(), format!("{:.2}", res.mean_failures)),
            ("Mean repairs".into(), format!("{:.2}", res.mean_repairs)),
            ("Simulations".into(), format!("{}", n_sim)),
            (
                "Monte Carlo standard error".into(),
                format!("{:.4}", res.std_error),
            ),
        ];
        self.rbd_avail = rows_text(&rows);
        self.note("Availability simulation complete.");
        self.mark_computed();
        rows
    }

    pub fn compute_rbd_importance(&mut self) {
        let t = match valid_f64_nonneg(&self.rbd_t, "RbdUi.t") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return;
            }
        };
        match importance::importance_measures(&self.rbd_spec, t) {
            Ok(v) => {
                self.rbd_importance = v;
                self.rbd_importance_note = format!(
                    "Importance at t={:.0} for {} blocks.",
                    t,
                    self.rbd_importance.len()
                );
                self.note("Importance measures computed.");
                self.mark_computed();
            }
            Err(e) => {
                self.rbd_importance.clear();
                self.rbd_importance_note = format!("Error: {}", e);
            }
        }
    }

    pub fn compute_fault_tree(&mut self) -> Vec<(String, String)> {
        match serde_json::from_str::<FtNode>(&self.ft_json) {
            Ok(node) => {
                let eval = match faulttree::evaluate(&node) {
                    Ok(v) => v,
                    Err(msg) => {
                        self.set_validation_error(ValidationError::new("FtUi.json", "", &msg));
                        return rows_of(&self.ft_result);
                    }
                };
                let (top_prob, cuts) = (eval.top_probability, eval.cut_sets);
                let mut rows: Vec<(String, String)> = vec![
                    ("Top probability".into(), format!("{:.6}", top_prob)),
                    ("Minimal cut sets".into(), format!("{}", cuts.len())),
                ];
                for (i, c) in cuts.iter().enumerate() {
                    let mut names: Vec<String> = c.iter().cloned().collect();
                    names.sort();
                    rows.push((format!("Cut set {}", i + 1), names.join(", ")));
                }
                self.ft_result = rows_text(&rows);
                self.note("Fault tree evaluated.");
                self.mark_computed();
                rows
            }
            Err(e) => {
                self.ft_result = format!("JSON error: {}", e);
                rows_of(&self.ft_result)
            }
        }
    }

    pub fn fmea_criticality(&self, idx: usize) -> Option<f64> {
        let c = self.fmea_crit.get(idx)?;
        let a = opt_num(&c[0])?;
        let b = opt_num(&c[1])?;
        let l = opt_num(&c[2])?;
        let t = opt_num(&c[3])?;
        Some(fmea::criticality_number(a, b, l, t))
    }

    pub fn fmea_add_row(&mut self) {
        self.fmea_rows.push(FmeaRow::new(
            &self.fm_item.clone(),
            &self.fm_mode.clone(),
            self.fm_sev.trim().parse::<u32>().unwrap_or(1),
            self.fm_occ.trim().parse::<u32>().unwrap_or(1),
            self.fm_det.trim().parse::<u32>().unwrap_or(1),
        ));
        self.fmea_crit.push([
            self.fm_alpha.clone(),
            self.fm_beta.clone(),
            self.fm_lambda.clone(),
            self.fm_hours.clone(),
        ]);
        self.fmea_status = "Row added.".into();
        self.note(self.fmea_status.clone());
    }

    pub fn fmea_delete_row(&mut self, idx: usize) {
        if idx < self.fmea_rows.len() {
            self.fmea_rows.remove(idx);
            if idx < self.fmea_crit.len() {
                self.fmea_crit.remove(idx);
            }
            self.fmea_status = "Row deleted.".into();
            self.note(self.fmea_status.clone());
        }
    }

    pub fn fmea_export_worksheet(&mut self) {
        let ws = FmeaWorksheet {
            fmea_type: FmeaType::from_str(&self.fmea_type).unwrap_or(FmeaType::Design),
            rows: self.fmea_rows.clone(),
        };
        self.fmea_csv = ws.to_csv();
        self.fmea_status = "Worksheet exported to the CSV text area.".into();
        self.note(self.fmea_status.clone());
    }

    pub fn fmea_export_rows(&mut self) {
        self.fmea_csv = fmea::rows_to_csv(&self.fmea_rows);
        self.fmea_status = "Rows exported to the CSV text area.".into();
        self.note(self.fmea_status.clone());
    }

    pub fn fmea_import_csv(&mut self) {
        match FmeaWorksheet::from_csv(&self.fmea_csv) {
            Ok(ws) => {
                self.fmea_type = ws.fmea_type.as_str().to_string();
                self.fmea_rows = ws.rows;
                self.fmea_crit = vec![
                    [String::new(), String::new(), String::new(), String::new()];
                    self.fmea_rows.len()
                ];
                self.fmea_status = "CSV imported.".into();
                self.note(self.fmea_status.clone());
            }
            Err(e) => self.fmea_status = format!("Error: {}", e),
        }
    }

    pub fn fmea_clear_rows(&mut self) {
        self.fmea_rows.clear();
        self.fmea_crit.clear();
        self.fmea_status = "Rows cleared.".into();
        self.note(self.fmea_status.clone());
    }

    pub fn fmea_export_control_plan(&mut self) {
        let csv = fmea::control_plan_csv(&self.fmea_rows);
        match std::fs::write(&self.fmea_export_path, csv) {
            Ok(()) => {
                self.fmea_status = format!("Control plan written to {}", self.fmea_export_path)
            }
            Err(e) => self.fmea_status = format!("Export error: {}", e),
        }
        self.note(self.fmea_status.clone());
    }

    pub fn fmea_export_dvpr(&mut self) {
        let csv = fmea::dvpr_to_csv(&self.dvpr_entries);
        match std::fs::write(&self.fmea_export_path, csv) {
            Ok(()) => self.fmea_status = format!("DVP&R written to {}", self.fmea_export_path),
            Err(e) => self.fmea_status = format!("Export error: {}", e),
        }
        self.note(self.fmea_status.clone());
    }

    pub fn fmea_add_dvpr(&mut self) {
        self.dvpr_entries.push(DvprEntry {
            id: self.dv_id.clone(),
            failure_mode_ref: self.dv_ref.clone(),
            test: self.dv_test.clone(),
            sample_size: self.dv_sample.trim().parse::<u32>().unwrap_or(0),
            acceptance_criteria: self.dv_criteria.clone(),
        });
        self.fmea_status = "DVP&R entry added.".into();
        self.note(self.fmea_status.clone());
    }

    pub fn compute_warranty(&mut self) -> Vec<(String, String)> {
        // Validate sales as integer counts
        if self.war_sales.trim().is_empty() {
            self.set_validation_error(ValidationError::new(
                "WarUi.sales",
                "",
                "empty required field",
            ));
            return rows_of(&self.war_result);
        }
        let mut sales: Vec<u32> = Vec::new();
        for tok in self
            .war_sales
            .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
            .filter(|s| !s.trim().is_empty())
        {
            match valid_usize(tok.trim(), "WarUi.sales") {
                Ok(v) => {
                    if v > u32::MAX as usize {
                        self.set_validation_error(ValidationError::new(
                            "WarUi.sales",
                            tok,
                            "count too large",
                        ));
                        return rows_of(&self.war_result);
                    }
                    sales.push(v as u32);
                }
                Err(e) => {
                    self.set_validation_error(e);
                    return rows_of(&self.war_result);
                }
            }
        }
        let mut records_all: Vec<WarrantyRecord> = Vec::new();
        for line in self.war_returns.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let toks: Vec<&str> = line
                .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
                .filter(|s| !s.trim().is_empty())
                .collect();
            if toks.len() != 2 {
                self.set_validation_error(ValidationError::new(
                    "WarUi.returns",
                    line,
                    "expected sale period, return period",
                ));
                return rows_of(&self.war_result);
            }
            let sp = match valid_usize(toks[0], "WarUi.returns") {
                Ok(v) => v,
                Err(e) => {
                    self.set_validation_error(e);
                    return rows_of(&self.war_result);
                }
            };
            let rp = match valid_usize(toks[1], "WarUi.returns") {
                Ok(v) => v,
                Err(e) => {
                    self.set_validation_error(e);
                    return rows_of(&self.war_result);
                }
            };
            records_all.push(WarrantyRecord {
                sale_period: sp,
                return_period: rp,
            });
        }
        let current = match valid_usize(&self.war_current, "WarUi.current") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.war_result);
            }
        };
        let horizon = match valid_usize(&self.war_horizon, "WarUi.horizon") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.war_result);
            }
        };
        let excluded = records_all
            .iter()
            .filter(|r| r.return_period > current)
            .count();
        let records: Vec<WarrantyRecord> = records_all
            .iter()
            .cloned()
            .filter(|r| r.return_period <= current)
            .collect();
        self.war_excluded_future = excluded;
        self.war_notes = if excluded > 0 {
            vec![format!(
                "Excluded {} future return(s) with return period after current period {}",
                excluded, current
            )]
        } else {
            Vec::new()
        };
        let max_age = records
            .iter()
            .filter(|r| r.return_period >= r.sale_period)
            .map(|r| r.return_period - r.sale_period)
            .max()
            .unwrap_or(0);
        let table = warranty::nevada_table(sales.len(), &records, max_age);
        self.war_sale_labels = (0..sales.len())
            .map(|i| format!("Sale period {}", i))
            .collect();
        self.war_table = table.clone();
        let mut rows: Vec<(String, String)> = vec![
            (
                "Returns by age".into(),
                warranty::returns_by_age(&table)
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
            ),
            (
                "Total returns".into(),
                format!("{}", warranty::total_returns(&table)),
            ),
            ("Excluded future returns".into(), format!("{}", excluded)),
        ];
        for n in &self.war_notes.clone() {
            rows.push(("Note".into(), n.clone()));
        }
        match warranty::forecast_returns(&sales, &records_all, current, horizon) {
            Ok(f) => {
                self.war_excluded_future = f.excluded_future_returns;
                self.war_notes = f.notes.clone();
                for n in &f.notes {
                    rows.push(("Forecast note".into(), n.clone()));
                }
                rows.push(("Forecast Weibull beta".into(), format!("{:.3}", f.beta)));
                rows.push(("Forecast Weibull eta".into(), format!("{:.2}", f.eta)));
                rows.push((
                    "Expected returns over horizon".into(),
                    format!("{:.2}", f.expected_returns),
                ));
                rows.push((
                    "Units in service".into(),
                    format!("{:.0}", f.units_in_service),
                ));
            }
            Err(e) => rows.push(("Forecast error".into(), e)),
        }
        self.war_result = rows_text(&rows);
        self.note("Warranty analysis complete.");
        self.mark_computed();
        rows
    }
}

impl Work {
    pub fn compute_pred_life(&mut self) -> Vec<(String, String)> {
        let params = match valid_parse_params(&self.pred_life_params, "PredUi.life-params") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.pred_life_result);
            }
        };
        // also support whitespace separated pairs via fallback if empty and text non-empty handled by valid parser (comma/semicolon split covers)
        let t = match valid_f64_nonneg(&self.pred_life_t, "PredUi.life-t") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.pred_life_result);
            }
        };
        let r = dist_mod::reliability(&self.pred_life_dist, &params, t);
        self.pred_life_result = format!(
            "Life model {}: R({:.0}) = {}",
            dist_mod::dist_label(&self.pred_life_dist),
            t,
            if r.is_finite() {
                format!("{:.4}", r)
            } else {
                "unavailable: non-finite result".into()
            }
        );
        self.headline = self.pred_life_result.clone();
        self.note("Life model reliability computed.");
        self.mark_computed();
        rows_of(&self.pred_life_result)
    }

    pub fn pred_parse_parts(&self, text: &str) -> Vec<PartEntry> {
        let mut parts: Vec<PartEntry> = Vec::new();
        for line in text.lines() {
            let cells: Vec<&str> = line.split(',').map(|c| c.trim()).collect();
            if cells.len() >= 3 {
                if let Ok(rate) = cells[1].parse::<f64>() {
                    let mut p =
                        PartEntry::new(cells[0], rate, cells[2].parse::<u32>().unwrap_or(1));
                    if cells.len() >= 4 {
                        p.stress_factor = cells[3].parse::<f64>().unwrap_or(1.0);
                    }
                    if cells.len() >= 5 {
                        p.quality_factor = cells[4].parse::<f64>().unwrap_or(1.0);
                    }
                    parts.push(p);
                }
            }
        }
        parts
    }

    pub fn compute_pred_parts(&mut self) -> Vec<(String, String)> {
        let parts = self.pred_parse_parts(&self.pred_parts.clone());
        let t = match valid_f64_nonneg(&self.pred_t, "PredUi.t") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.pred_result);
            }
        };
        let rows: Vec<(String, String)> = vec![
            (
                "System failure rate per million hours".into(),
                format!(
                    "{:.3}",
                    prediction::system_failure_rate_per_million_hours(&parts)
                ),
            ),
            (
                "System MTBF (hours)".into(),
                format!("{:.0}", prediction::system_mtbf_hours(&parts)),
            ),
            (
                format!("Reliability at {:.0} h", t),
                format!("{:.4}", prediction::system_reliability(&parts, t)),
            ),
        ];
        self.pred_result = rows_text(&rows);
        self.note("Prediction complete.");
        self.mark_computed();
        rows
    }

    pub fn pred_lib_load(&mut self) {
        match std::fs::read_to_string(&self.pred_lib_path) {
            Ok(text) => match prediction::library_from_csv(&text) {
                Ok(parts) => {
                    self.pred_lib_note =
                        format!("Loaded {} parts from {}", parts.len(), self.pred_lib_path);
                    self.pred_lib_parts = parts;
                }
                Err(e) => self.pred_lib_note = format!("Library error: {}", e),
            },
            Err(e) => self.pred_lib_note = format!("Load error: {}", e),
        }
        self.note(self.pred_lib_note.clone());
    }

    pub fn pred_lib_rows(&self) -> Vec<(String, String)> {
        let t = fnum(&self.pred_t, 1000.0);
        vec![
            (
                "System failure rate per million hours".into(),
                format!(
                    "{:.3}",
                    prediction::system_failure_rate_per_million_hours(&self.pred_lib_parts)
                ),
            ),
            (
                "System MTBF (hours)".into(),
                format!("{:.0}", prediction::system_mtbf_hours(&self.pred_lib_parts)),
            ),
            (
                format!("Reliability at {:.0} h", t),
                format!(
                    "{:.4}",
                    prediction::system_reliability(&self.pred_lib_parts, t)
                ),
            ),
        ]
    }

    pub fn pred217_load(&mut self) {
        let path = mil217_dir().join(&self.pred217_file);
        match std::fs::read_to_string(&path) {
            Ok(text) => match standards217::load_category(&text) {
                Ok(v) => {
                    self.pred217_json = Some(v);
                    self.pred217_result = String::new();
                    self.note(format!(
                        "Loaded MIL-HDBK-217F category {}",
                        self.pred217_file
                    ));
                }
                Err(e) => self.pred217_result = format!("Category JSON error: {}", e),
            },
            Err(e) => self.pred217_result = format!("Category load error: {}", e),
        }
    }

    pub fn pred217_is_misc(&self) -> bool {
        self.pred217_json
            .as_ref()
            .and_then(|cat| cat.get("lambda_b"))
            .and_then(|lb| lb.get("kind"))
            .and_then(|v| v.as_str())
            == Some("fixed_rates")
    }

    pub fn pred217_is_pth(&self) -> bool {
        self.pred217_file == "interconnection_assemblies_pth.json"
    }

    /// Field list for a 217F category: (is_named, key, label, options).
    pub fn pred217_fields(cat: &Value) -> Vec<(bool, String, String, Vec<String>)> {
        let mut fields: Vec<(bool, String, String, Vec<String>)> = Vec::new();
        let lb = cat.get("lambda_b").cloned().unwrap_or(json!({}));
        for (key, map_name) in [("style", "by_style"), ("type", "by_type")] {
            if let Some(m) = lb.get(map_name).and_then(|v| v.as_object()) {
                if !m.is_empty() {
                    let mut opts: Vec<String> = m.keys().cloned().collect();
                    opts.sort();
                    fields.push((
                        true,
                        key.to_string(),
                        format!("{} ({})", key, map_name),
                        opts,
                    ));
                }
            }
        }
        if let Some(axes) = lb.get("axes").and_then(|v| v.as_array()) {
            for a in axes {
                if let Some(name) = a.get("name").and_then(|v| v.as_str()) {
                    fields.push((
                        false,
                        name.to_string(),
                        format!("lambda_b variable {}", name),
                        Vec::new(),
                    ));
                }
            }
        }
        if let Some(tables) = cat.get("pi_tables").and_then(|v| v.as_object()) {
            let mut names: Vec<String> = tables.keys().cloned().collect();
            names.sort();
            for fname in names {
                let table = &tables[&fname];
                let kind = table.get("kind").and_then(|v| v.as_str()).unwrap_or("");
                let input = table.get("input").and_then(|v| v.as_str()).unwrap_or("");
                match kind {
                    "named" | "named_grid" => {
                        let mut opts: Vec<String> = table
                            .get("values")
                            .and_then(|v| v.as_object())
                            .map(|m| m.keys().cloned().collect())
                            .unwrap_or_default();
                        opts.sort();
                        if !input.is_empty() && !opts.is_empty() {
                            fields.push((
                                true,
                                input.to_string(),
                                format!("{} input {}", fname, input),
                                opts,
                            ));
                        }
                    }
                    "series" | "columns" | "bands" => {
                        if !input.is_empty() {
                            fields.push((
                                false,
                                input.to_string(),
                                format!("{} input {}", fname, input),
                                Vec::new(),
                            ));
                        }
                    }
                    _ => {
                        fields.push((
                            false,
                            fname.clone(),
                            format!("{} value (formula factor)", fname),
                            Vec::new(),
                        ));
                    }
                }
            }
        }
        let mut seen: Vec<(bool, String)> = Vec::new();
        fields.retain(|(n, k, _, _)| {
            let key = (*n, k.clone());
            if seen.contains(&key) {
                false
            } else {
                seen.push(key);
                true
            }
        });
        fields
    }

    pub fn pred217_part_options(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .pred217_json
            .as_ref()
            .and_then(|cat| cat.get("lambda_b"))
            .and_then(|lb| lb.get("rates"))
            .and_then(|v| v.as_object())
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        names.sort();
        names
    }

    pub fn compute_pred217(&mut self) {
        let cat = match self.pred217_json.clone() {
            Some(c) => c,
            None => {
                self.pred217_load();
                match self.pred217_json.clone() {
                    Some(c) => c,
                    None => return,
                }
            }
        };
        if self.pred217_is_misc() {
            let rates = cat
                .get("lambda_b")
                .and_then(|lb| lb.get("rates"))
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();
            let part = self.pred217_named.get("part").cloned().unwrap_or_default();
            let env = self
                .pred217_named
                .get("environment")
                .cloned()
                .unwrap_or_else(|| "GB".into());
            let base = rates
                .get(part.as_str())
                .and_then(|v| v.as_f64())
                .unwrap_or(f64::NAN);
            let pi_e = standards217::pi_table_value(&cat, "pi_E", &env).unwrap_or(f64::NAN);
            self.pred217_result = format!(
                "Base rate per million hours: {:.4}\npi_E ({}): {:.4}\nFailure rate per million hours: {:.4}",
                base, env, pi_e, base * pi_e
            );
            return;
        }
        if self.pred217_is_pth() {
            let rate = standards217::pth_failure_rate(
                &cat,
                &self
                    .pred217_named
                    .get("pth_technology")
                    .cloned()
                    .unwrap_or_default(),
                fnum(
                    self.pred217_num
                        .get("pth_n1")
                        .map(|s| s.as_str())
                        .unwrap_or(""),
                    0.0,
                ),
                fnum(
                    self.pred217_num
                        .get("pth_n2")
                        .map(|s| s.as_str())
                        .unwrap_or(""),
                    0.0,
                ),
                fnum(
                    self.pred217_num
                        .get("pth_planes")
                        .map(|s| s.as_str())
                        .unwrap_or(""),
                    0.0,
                ),
                &self
                    .pred217_named
                    .get("pth_quality")
                    .cloned()
                    .unwrap_or_default(),
                &self
                    .pred217_named
                    .get("pth_environment")
                    .cloned()
                    .unwrap_or_default(),
            );
            self.pred217_result = format!("PTH failure rate per million hours: {:.5}", rate);
            return;
        }
        let mut inputs: standards217::Inputs = std::collections::HashMap::new();
        for (k, v) in &self.pred217_named {
            if k != "part" {
                inputs.insert(k.clone(), FactorInput::named(v));
            }
        }
        for (k, v) in &self.pred217_num {
            if let Some(x) = opt_num(v) {
                inputs.insert(k.clone(), FactorInput::num(x));
            }
        }
        let lb = standards217::lambda_b_for(&cat, &inputs);
        let rate = standards217::part_failure_rate(&cat, &inputs);
        self.pred217_result = format!(
            "lambda_b per million hours: {}\nPart failure rate per million hours: {:.5}\nMTBF (hours): {}",
            lb.map(|v| format!("{:.5}", v)).unwrap_or_else(|| "not resolved".into()),
            rate,
            if rate > 0.0 { format!("{:.0}", 1e6 / rate) } else { "not resolved".into() }
        );
        self.headline = format!("217F rate {:.5} per million hours", rate);
        self.note("MIL-HDBK-217F calculation complete.");
    }

    pub fn compute_alloc(&mut self) -> Vec<(String, String)> {
        let target = match valid_f64(&self.alloc_target, "PredUi.alloc-target") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.alloc_result);
            }
        };
        let n_alloc = match valid_usize(&self.alloc_n, "PredUi.alloc-n") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.alloc_result);
            }
        };
        let eq = prediction::allocate_equal(target, n_alloc);
        let w = match valid_parse_numbers(&self.alloc_weights, "PredUi.alloc-weights") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.alloc_result);
            }
        };
        let wt = prediction::allocate_weighted(target, &w);
        let rows: Vec<(String, String)> = vec![
            (
                "Equal allocation".into(),
                eq.iter()
                    .map(|v| format!("{:.4}", v))
                    .collect::<Vec<String>>()
                    .join(", "),
            ),
            (
                "Weighted allocation".into(),
                wt.iter()
                    .map(|v| format!("{:.4}", v))
                    .collect::<Vec<String>>()
                    .join(", "),
            ),
        ];
        self.alloc_result = rows_text(&rows);
        self.mark_computed();
        rows
    }

    pub fn compute_derating(&mut self) -> Vec<(String, String)> {
        let op = match valid_f64(&self.der_operating, "PredUi.der-operating") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.der_result);
            }
        };
        let rated = match valid_f64(&self.der_rated, "PredUi.der-rated") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.der_result);
            }
        };
        let mx = match valid_f64(&self.der_max, "PredUi.der-max") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.der_result);
            }
        };
        let r = prediction::derating_check(&self.der_name.clone(), op, rated, mx);
        let rows: Vec<(String, String)> = vec![
            ("Part".into(), r.name.clone()),
            ("Derating ratio".into(), format!("{:.3}", r.ratio)),
            (
                "Result".into(),
                if r.pass { "pass".into() } else { "fail".into() },
            ),
        ];
        self.der_result = rows_text(&rows);
        self.mark_computed();
        rows
    }

    pub fn compute_clean_room(&mut self) -> Vec<(String, String)> {
        for (val, fld) in [
            (self.sr_ea.clone(), "PredUi.sr-ea"),
            (self.sr_temp.clone(), "PredUi.sr-temp"),
            (self.sr_ref_temp.clone(), "PredUi.sr-ref-temp"),
            (self.sr_generic.clone(), "PredUi.sr-generic"),
            (self.sr_ci_exposure.clone(), "PredUi.sr-ci-exposure"),
            (self.sr_ci_conf.clone(), "PredUi.sr-ci-conf"),
        ] {
            if let Err(e) = valid_f64(&val, fld) {
                self.set_validation_error(e);
                return rows_of(&self.pred_clean_result);
            }
        }
        if let Err(e) = valid_usize(&self.sr_ci_failures, "PredUi.sr-ci-failures") {
            self.set_validation_error(e);
            return rows_of(&self.pred_clean_result);
        }
        let ea = fnum(&self.sr_ea, prediction::SR332_DEFAULT_ACTIVATION_ENERGY_EV);
        let pi_t = prediction::sr332_temperature_factor(
            ea,
            fnum(&self.sr_temp, 25.0),
            fnum(&self.sr_ref_temp, 25.0),
        );
        let device = prediction::sr332_method1(
            fnum(&self.sr_generic, 0.0),
            fnum(&self.sr_pi_q, 1.0),
            fnum(&self.sr_pi_s, 1.0),
            pi_t,
        );
        let mut parts: Vec<(u32, f64)> = Vec::new();
        for line in self.sr_qty_rate.lines() {
            let vals = parse_numbers(line);
            if vals.len() >= 2 {
                parts.push((vals[0] as u32, vals[1]));
            }
        }
        let equip = prediction::sr332_equipment_rate(&parts, fnum(&self.sr_pi_e, 1.0));
        let m2 = prediction::sr332_method2_extrapolate(
            fnum(&self.sr_obs_rate, 0.0),
            fnum(&self.sr_pit_obs, 1.0),
            fnum(&self.sr_pit_tgt, 1.0),
            fnum(&self.sr_pis_obs, 1.0),
            fnum(&self.sr_pis_tgt, 1.0),
        );
        let (lo, hi) = prediction::failure_rate_ci(
            self.sr_ci_failures.trim().parse::<u64>().unwrap_or(0),
            fnum(&self.sr_ci_exposure, 1.0),
            fnum(&self.sr_ci_conf, 0.90),
        );
        let m3 = prediction::sr332_method3_combine(
            fnum(&self.sr_m3_pred, 0.0),
            fnum(&self.sr_m3_pw, 1.0),
            fnum(&self.sr_m3_obs, 0.0),
            fnum(&self.sr_m3_ow, 1.0),
        );
        let rows: Vec<(String, String)> = vec![
            ("pi_t (temperature factor)".into(), format!("{:.4}", pi_t)),
            ("Method I device rate".into(), format!("{:.5}", device)),
            (
                "Equipment rate (pi_e applied once)".into(),
                format!("{:.5}", equip),
            ),
            ("Method II extrapolated rate".into(), format!("{:.5}", m2)),
            (
                "Failure rate CI per hour".into(),
                format!("{:.3e} to {:.3e}", lo, hi),
            ),
            ("Method III combined rate".into(), format!("{:.5}", m3)),
        ];
        self.pred_clean_result = rows_text(&rows);
        self.note("Clean-room calculations complete.");
        self.mark_computed();
        rows
    }

    pub fn compute_mission(&mut self) -> Vec<(String, String)> {
        for (val, fld) in [
            (self.mp_base.clone(), "PredUi.mp-base"),
            (self.mp_ea.clone(), "PredUi.mp-ea"),
            (self.mp_ref.clone(), "PredUi.mp-ref"),
            (self.mp_cyc_factor.clone(), "PredUi.mp-cyc"),
        ] {
            if let Err(e) = valid_f64(&val, fld) {
                self.set_validation_error(e);
                return rows_of(&self.pred_clean_result);
            }
        }
        let mut phases: Vec<MissionPhase> = Vec::new();
        for line in self.mp_phases.lines() {
            let cells: Vec<&str> = line.split(',').map(|c| c.trim()).collect();
            if cells.len() >= 4 {
                let mut ph = MissionPhase::new(
                    cells[0],
                    cells[1].parse::<f64>().unwrap_or(0.0),
                    cells[2].parse::<f64>().unwrap_or(25.0),
                    cells[3].parse::<f64>().unwrap_or(0.0),
                );
                if cells.len() >= 5 {
                    ph.dormant = cells[4] == "1";
                }
                if cells.len() >= 6 {
                    ph.dormant_factor = cells[5].parse::<f64>().unwrap_or(1.0);
                }
                phases.push(ph);
            }
        }
        let rate = prediction::iec62380_style_mission_profile_rate(
            fnum(&self.mp_base, 0.0),
            &phases,
            fnum(&self.mp_ea, 0.35),
            fnum(&self.mp_ref, 25.0),
            fnum(&self.mp_cyc_factor, 0.0),
        );
        let rows: Vec<(String, String)> = vec![(
            "62380-style mission profile rate".into(),
            format!("{:.5}", rate),
        )];
        self.pred_clean_result = rows_text(&rows);
        self.note("Mission profile rate computed.");
        rows
    }

    pub fn compute_doe(&mut self) -> Vec<(String, String)> {
        let k = match valid_usize(&self.doe_k, "DoeUi.k") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.doe_result);
            }
        };
        let y = match valid_parse_numbers(&self.doe_y, "DoeUi.y") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.doe_result);
            }
        };
        match doe::analyze_factorial(k, &y) {
            Ok(effects) => {
                let mut rows: Vec<(String, String)> = vec![(
                    "Design runs".into(),
                    format!("{}", doe::factorial_design(k).len()),
                )];
                for e in &effects {
                    rows.push((
                        format!("Term {}", e.name),
                        format!(
                            "effect {:.3}, sum of squares {:.3}",
                            e.effect, e.sum_squares
                        ),
                    ));
                }
                self.doe_result = rows_text(&rows);
                self.note("DOE analysis complete.");
                self.mark_computed();
                rows
            }
            Err(e) => {
                self.doe_result = format!("Error: {}", e);
                rows_of(&self.doe_result)
            }
        }
    }

    pub fn compute_flowchart(&mut self) -> Vec<(String, String)> {
        if let Err(e) = valid_usize(&self.flow_sims, "FlowUi.sims") {
            self.set_validation_error(e);
            return rows_of(&self.flow_result);
        }
        if let Err(e) = valid_int(&self.flow_seed, "FlowUi.seed") {
            self.set_validation_error(e);
            return rows_of(&self.flow_result);
        }
        match serde_json::from_str::<Value>(&self.flow_json) {
            Ok(spec) => match flowchart::simulate_flow(
                &spec,
                inum(&self.flow_sims, 5000),
                self.flow_seed.trim().parse::<u64>().unwrap_or(1),
            ) {
                Ok(r) => {
                    let rows: Vec<(String, String)> = vec![
                        (
                            "Mean time".into(),
                            format!("{:.3} (sd {:.3})", r.mean_time, r.sd_time),
                        ),
                        ("P10".into(), format!("{:.3}", r.p10)),
                        ("P50".into(), format!("{:.3}", r.p50)),
                        ("P90".into(), format!("{:.3}", r.p90)),
                        ("Min".into(), format!("{:.3}", r.min_time)),
                        ("Max".into(), format!("{:.3}", r.max_time)),
                        ("Simulations".into(), format!("{}", r.n_sim)),
                        ("Mean cost".into(), format!("{:.3}", r.mean_cost)),
                    ];
                    self.flow_result = rows_text(&rows);
                    self.note("Flowchart simulation complete.");
                    self.mark_computed();
                    rows
                }
                Err(e) => {
                    self.flow_result = format!("Error: {}", e);
                    rows_of(&self.flow_result)
                }
            },
            Err(e) => {
                self.flow_result = format!("JSON error: {}", e);
                rows_of(&self.flow_result)
            }
        }
    }

    pub fn markov_preset(&mut self) {
        self.mk_states = vec!["Both up".into(), "One down".into(), "System down".into()];
        self.mk_transitions = vec![
            ["Both up".into(), "One down".into(), "0.02".into()],
            ["One down".into(), "Both up".into(), "0.1".into()],
            ["One down".into(), "System down".into(), "0.01".into()],
        ];
        self.mk_absorbing = "System down".into();
        self.mk_start = "Both up".into();
        self.note("Markov preset loaded.");
    }

    pub fn compute_markov(&mut self) -> Vec<(String, String)> {
        if let Err(e) = valid_f64_nonneg(&self.mk_time, "MarkovUi.time") {
            self.set_validation_error(e);
            return rows_of(&self.mk_result);
        }
        for tr in &self.mk_transitions {
            if let Err(mut e) = valid_f64(tr[2].trim(), "MarkovUi.tr-rate") {
                e.token = tr[2].clone();
                self.set_validation_error(e);
                return rows_of(&self.mk_result);
            }
        }
        let names = self.mk_states.clone();
        let idx = |n: &str| names.iter().position(|s| s == n);
        let mut trs: Vec<(usize, usize, f64)> = Vec::new();
        let mut bad: Option<String> = None;
        for t in &self.mk_transitions {
            match (idx(&t[0]), idx(&t[1]), t[2].trim().parse::<f64>()) {
                (Some(a), Some(b), Ok(r)) => trs.push((a, b, r)),
                _ => {
                    bad = Some(format!("Transition names a state that is not in the state list, or has a bad rate: {} to {}", t[0], t[1]));
                    break;
                }
            }
        }
        if let Some(e) = bad {
            self.mk_result = format!("Error: {}", e);
            return rows_of(&self.mk_result);
        }
        match MarkovModel::new(names.clone(), &trs) {
            Ok(model) => {
                let mut rows: Vec<(String, String)> = Vec::new();
                match model.steady_state() {
                    Ok(pi) => {
                        for (n, p) in names.iter().zip(&pi) {
                            rows.push((format!("Steady state P({})", n), format!("{:.5}", p)));
                        }
                    }
                    Err(e) => rows.push(("Steady state error".into(), e)),
                }
                let absorbing: Vec<bool> = names.iter().map(|n| *n == self.mk_absorbing).collect();
                if let Some(start) = idx(&self.mk_start) {
                    match model.mean_time_to_absorption(&absorbing, start) {
                        Ok(v) => rows.push((
                            format!("MTTF from {} to {}", self.mk_start, self.mk_absorbing),
                            format!("{:.2} hours", v),
                        )),
                        Err(e) => rows.push(("MTTF error".into(), e)),
                    }
                    match model.expected_time_in_state(&absorbing, start) {
                        Ok(v) => {
                            for (n, x) in names.iter().zip(&v) {
                                rows.push((
                                    format!("Expected time in {}", n),
                                    format!("{:.2} hours", x),
                                ));
                            }
                        }
                        Err(e) => rows.push(("Expected time error".into(), e)),
                    }
                    let mut p0 = vec![0.0; names.len()];
                    p0[start] = 1.0;
                    let t = fnum(&self.mk_time, 100.0);
                    match model.transient(&p0, &[t]) {
                        Ok(rows_t) => {
                            if let Some(row) = rows_t.first() {
                                for (n, p) in names.iter().zip(row) {
                                    rows.push((
                                        format!("P({}) at t={:.0}", n, t),
                                        format!("{:.5}", p),
                                    ));
                                }
                            }
                        }
                        Err(e) => rows.push(("Transient error".into(), e)),
                    }
                    // Transient curves for the chart: 41 points to the
                    // requested transient time.
                    let grid: Vec<f64> = (0..=40).map(|k| t * k as f64 / 40.0).collect();
                    if let Ok(all) = model.transient(&p0, &grid) {
                        self.mk_series = names
                            .iter()
                            .enumerate()
                            .map(|(j, n)| {
                                (
                                    n.clone(),
                                    grid.iter()
                                        .zip(&all)
                                        .map(|(tt, row)| (*tt, row[j]))
                                        .collect::<Vec<(f64, f64)>>(),
                                )
                            })
                            .collect();
                    }
                }
                self.mk_result = rows_text(&rows);
                self.note("Markov computation complete.");
                self.mark_computed();
                rows
            }
            Err(e) => {
                self.mk_result = format!("Error: {}", e);
                rows_of(&self.mk_result)
            }
        }
    }

    pub fn compute_qcp(&mut self) -> Vec<(String, String)> {
        let mut params = dist_mod::Params::new();
        for (k, v) in &self.qcp_params.clone() {
            match valid_f64(v, "QcpUi.params") {
                Ok(x) => {
                    params.insert(k.clone(), x);
                }
                Err(mut e) => {
                    e.token = format!("{}={}", k, v);
                    e.field = "QcpUi.params".into();
                    self.set_validation_error(e);
                    return rows_of(&self.qcp_result);
                }
            }
        }
        let t = match valid_f64_nonneg(&self.qcp_t, "QcpUi.t") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.qcp_result);
            }
        };
        let bx = match valid_f64(&self.qcp_bx, "QcpUi.bx") {
            Ok(v) => v,
            Err(e) => {
                self.set_validation_error(e);
                return rows_of(&self.qcp_result);
            }
        };
        match quickcalc::quick_calc(&self.qcp_dist, &params, t, bx) {
            Ok(r) => {
                let rows: Vec<(String, String)> = vec![
                    (
                        format!("Reliability R({:.0})", t),
                        format!("{:.5}", r.reliability),
                    ),
                    (
                        format!("Unreliability F({:.0})", t),
                        format!("{:.5}", r.unreliability),
                    ),
                    (format!("Hazard at {:.0}", t), format!("{:.6}", r.hazard)),
                    (format!("Pdf at {:.0}", t), format!("{:.6}", r.pdf)),
                    ("Mean life".into(), format!("{:.2}", r.mean_life)),
                    (
                        format!("B{:.0} life", bx * 100.0),
                        format!("{:.2}", r.bx_life),
                    ),
                ];
                self.qcp_result = rows_text(&rows);
                self.headline = format!("QCP R({:.0}) = {:.5}", t, r.reliability);
                self.note("QCP calculation complete.");
                self.mark_computed();
                rows
            }
            Err(e) => {
                self.qcp_result = format!("Error: {}", e);
                rows_of(&self.qcp_result)
            }
        }
    }

    pub fn fracas_summary_rows(&self) -> Vec<(String, String)> {
        let by_status = fracas::count_by_status(&self.fracas_rows);
        let by_mode = fracas::count_by_mode(&self.fracas_rows);
        let mut rows: Vec<(String, String)> = by_status
            .iter()
            .map(|(s, c)| (format!("Status {}", s), format!("{}", c)))
            .collect();
        for (m, c) in &by_mode {
            rows.push((format!("Mode {}", m), format!("{}", c)));
        }
        match fracas::mtbf_hours(fnum(&self.fracas_total_hours, 0.0), &self.fracas_rows) {
            Some(v) => rows.push(("MTBF (hours)".into(), format!("{:.1}", v))),
            None => rows.push(("MTBF (hours)".into(), "no incidents".into())),
        }
        rows
    }

    pub fn fracas_add_incident(&mut self) {
        // Next free FR-N id: one past the largest suffix in use, so
        // ids stay unique after removals.
        let n = self
            .fracas_rows
            .iter()
            .filter_map(|r| {
                r.id.strip_prefix("FR-")
                    .and_then(|s| s.parse::<usize>().ok())
            })
            .max()
            .unwrap_or(0)
            + 1;
        self.fracas_rows.push(Incident {
            id: format!("FR-{}", n),
            part: String::new(),
            failure_mode: String::new(),
            hours: 0.0,
            date: String::new(),
            status: IncidentStatus::Open,
            corrective_action: String::new(),
        });
        self.fracas_sel = Some(self.fracas_rows.len() - 1);
    }

    pub fn fracas_export_csv(&mut self) {
        match std::fs::write(
            &self.fracas_path,
            fracas::incidents_to_csv(&self.fracas_rows),
        ) {
            Ok(()) => {
                self.fracas_status = format!(
                    "Exported {} incidents to {}",
                    self.fracas_rows.len(),
                    self.fracas_path
                )
            }
            Err(e) => self.fracas_status = format!("Export error: {}", e),
        }
        self.note(self.fracas_status.clone());
    }

    pub fn fracas_import_csv(&mut self) {
        match std::fs::read_to_string(&self.fracas_path) {
            Ok(text) => match fracas::incidents_from_csv(&text) {
                Ok(v) => {
                    self.fracas_status = format!("Imported {} incidents.", v.len());
                    self.fracas_rows = v;
                }
                Err(e) => self.fracas_status = format!("Import error: {}", e),
            },
            Err(e) => self.fracas_status = format!("Load error: {}", e),
        }
        self.note(self.fracas_status.clone());
    }

    pub fn rcm_add_row(&mut self) {
        self.rcm_rows.push(RcmRowUi {
            asset: String::new(),
            function: String::new(),
            mode: String::new(),
            consequence: "Operational".into(),
            monitorable: false,
            age_related: false,
            restorable: false,
            rate: String::new(),
            alpha: String::new(),
            beta: String::new(),
            hours: String::new(),
        });
        self.rcm_sel = Some(self.rcm_rows.len() - 1);
    }
}
