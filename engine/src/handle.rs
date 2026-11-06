//! JSON dispatcher: the host-independent pull, dispatch, push
//! cycle ported from the Slint build's gui/src/main.rs.
//!
//! A host owns one `Work`, sends requests of the shape
//! `{"action": string, "fields": {name: string, ...}}`, and gets
//! the full UI state back as JSON. See API.md for the contract.
//!
//! The pull step mirrors main.rs: only fields present in the map
//! are applied, so a host may send a full form snapshot or just
//! the fields that changed. Enum fields accept either the value
//! itself (for example "weibull2") or an index into the same list
//! the Slint UI used.

use crate::series;
use crate::work::*;
use reliakit::fracas::IncidentStatus;
use serde_json::{json, Map, Value};

pub const MCMC_DISTS: [&str; 2] = ["weibull2", "lognormal"];
pub const DEG_MODELS: [&str; 3] = ["linear", "exponential", "power"];
pub const ALT_MODELS: [&str; 3] = ["arrhenius", "ipl", "eyring"];
pub const TOPOLOGIES: [&str; 4] = ["series", "parallel", "k-of-n", "series with standby group"];
pub const NB_MODES: [&str; 4] = ["active", "hot standby", "cold standby", "warm"];
pub const FMEA_TYPES: [&str; 3] = ["Design", "Process", "System"];
pub const PRED_SOURCES: [&str; 3] = ["Generic user data", "MIL-HDBK-217F library", "Local library file"];
pub const ENVIRONMENTS: [&str; 7] = ["GB", "GF", "GM", "NS", "NU", "AIC", "CL"];
pub const CONSEQUENCES: [&str; 5] = ["Hidden", "Safety", "Environmental", "Operational", "NonOperational"];
pub const FRACAS_STATUSES: [&str; 4] = ["Open", "Analysis", "CorrectiveAction", "Closed"];
pub const PTH_NAMED: [(&str, &str); 3] = [
    ("pth_technology", "Printed Wiring Assembly/Printed Circuit Boards with PTHs"),
    ("pth_quality", "MIL-SPEC or IPC Level 3"),
    ("pth_environment", "GB"),
];
pub const PTH_NUM: [(&str, &str); 3] = [
    ("pth_n1", "100"),
    ("pth_n2", "20"),
    ("pth_planes", "4"),
];

type Fields = Map<String, Value>;

// ---------------- field access ----------------

fn raw<'a>(f: &'a Fields, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|k| f.get(*k)).filter(|v| !v.is_null())
}

/// Any JSON value as the string the work engine stores. Numbers
/// and booleans use their plain JSON spelling.
fn fs(f: &Fields, keys: &[&str]) -> Option<String> {
    match raw(f, keys)? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        other => Some(other.to_string()),
    }
}

fn fi(f: &Fields, keys: &[&str]) -> Option<i64> {
    match raw(f, keys)? {
        Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|x| x as i64)),
        Value::String(s) => s.trim().parse::<i64>().ok().or_else(|| s.trim().parse::<f64>().ok().map(|x| x as i64)),
        _ => None,
    }
}

fn fb(f: &Fields, keys: &[&str]) -> Option<bool> {
    match raw(f, keys)? {
        Value::Bool(b) => Some(*b),
        Value::String(s) => match s.trim() {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        },
        Value::Number(n) => n.as_i64().map(|i| i != 0),
        _ => None,
    }
}

fn ff(f: &Fields, keys: &[&str]) -> Option<f64> {
    match raw(f, keys)? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn idx_of(list: &[&str], v: &str) -> i64 {
    list.iter().position(|x| *x == v).map(|i| i as i64).unwrap_or(0)
}

fn at(list: &[&str], idx: i64) -> String {
    list.get(idx.max(0) as usize).copied().unwrap_or(list[0]).to_string()
}

/// Enum field: the value itself when a value key is present,
/// otherwise the entry at the index key. None when neither is.
fn enum_val(f: &Fields, value_keys: &[&str], index_keys: &[&str], list: &[&str]) -> Option<String> {
    if let Some(v) = fs(f, value_keys) {
        if !v.is_empty() {
            return Some(v);
        }
    }
    fi(f, index_keys).map(|i| at(list, i))
}

fn set_s(slot: &mut String, f: &Fields, keys: &[&str]) {
    if let Some(v) = fs(f, keys) {
        *slot = v;
    }
}
fn set_b(slot: &mut bool, f: &Fields, keys: &[&str]) {
    if let Some(v) = fb(f, keys) {
        *slot = v;
    }
}
fn set_e(slot: &mut String, f: &Fields, value_keys: &[&str], index_keys: &[&str], list: &[&str]) {
    if let Some(v) = enum_val(f, value_keys, index_keys, list) {
        *slot = v;
    }
}

fn current_kind(w: &Work) -> Option<ItemKind> {
    w.selected.and_then(|i| w.items.get(i)).map(|it| it.kind)
}

fn current_title(w: &Work) -> String {
    match w.selected.and_then(|i| w.items.get(i)) {
        Some(it) => format!("{} - {}", it.name, it.kind.label()),
        None => "Start page".to_string(),
    }
}

// ---------------- pull (port of pull_shell and pull_current) ----------------

fn pull_shell(w: &mut Work, f: &Fields) {
    set_s(&mut w.project_name, f, &["AppState.project-name", "project-name", "project_name", "projectName"]);
    set_s(&mut w.project_path, f, &["AppState.project-path", "project-path", "project_path", "projectPath"]);
    if let Some(v) = fs(f, &["new_kind", "new-kind", "kind"]) {
        if !v.is_empty() {
            w.new_kind = v;
        }
    } else if let Some(i) = fi(f, &["AppState.kind-index", "kind-index", "kind_index", "kindIndex"]) {
        if let Some((_, label, _)) = KIND_DEFS.get(i.max(0) as usize) {
            w.new_kind = label.to_string();
        }
    }
    set_s(&mut w.new_name, f, &["AppState.new-name", "new-name", "new_name", "newName"]);
    set_s(&mut w.rename_name, f, &["AppState.rename-name", "rename-name", "rename_name", "renameName"]);
    if let Some(v) = fs(f, &["csv_pick", "csv-pick", "pick"]) {
        if BUNDLED_CSV.contains(&v.as_str()) {
            w.csv_pick = v;
        }
    } else if let Some(i) = fi(f, &["CsvUi.pick-index", "pick-index", "pick_index", "pickIndex"]) {
        w.csv_pick = at(&BUNDLED_CSV, i);
    }
    set_s(&mut w.csv_path, f, &["CsvUi.path", "csv_path", "csvPath"]);
}

fn pull_current(w: &mut Work, f: &Fields) {
    pull_shell(w, f);
    match current_kind(w) {
        Some(ItemKind::LifeData) => {
            set_s(&mut w.life_times, f, &["LifeUi.times", "life_times", "times"]);
            set_e(&mut w.life_dist, f, &["life_dist", "dist"], &["LifeUi.dist-index", "dist-index", "dist_index", "distIndex"], &ALL_DISTS);
            set_b(&mut w.life_rank, f, &["LifeUi.rank", "life_rank", "rank"]);
            set_s(&mut w.life_t, f, &["LifeUi.t", "life_t", "t"]);
            set_b(&mut w.life_show_prob, f, &["LifeUi.show-prob", "show-prob", "life_show_prob", "show_prob", "showProb"]);
            let rbd_ids: Vec<u64> = w.items.iter().filter(|it| it.kind == ItemKind::Rbd).map(|it| it.id).collect();
            if let Some(name) = fs(f, &["flow_target", "target_name"]) {
                if let Some(it) = w.items.iter().find(|it| it.kind == ItemKind::Rbd && it.name == name) {
                    w.flow_target_id = it.id;
                }
            } else if let Some(i) = fi(f, &["LifeUi.target-index", "target-index", "target_index", "targetIndex"]) {
                if let Some(id) = rbd_ids.get(i.max(0) as usize) {
                    w.flow_target_id = *id;
                }
            }
            if let Some(i) = fi(f, &["LifeUi.block-index", "block-index", "block_index", "blockIndex", "flow_block_idx"]) {
                w.flow_block_idx = i.max(0) as usize;
            }
        }
        Some(ItemKind::LimitedData) => {
            set_s(&mut w.limited_groups, f, &["LimitedUi.groups", "limited_groups", "groups"]);
        }
        Some(ItemKind::Mcmc) => {
            set_s(&mut w.mcmc_times, f, &["McmcUi.times", "mcmc_times", "times"]);
            set_e(&mut w.mcmc_dist, f, &["mcmc_dist", "dist"], &["McmcUi.dist-index", "dist-index", "dist_index", "distIndex"], &MCMC_DISTS);
            set_s(&mut w.mcmc_draws, f, &["McmcUi.draws", "mcmc_draws", "draws"]);
            set_s(&mut w.mcmc_warmup, f, &["McmcUi.warmup", "mcmc_warmup", "warmup"]);
            set_s(&mut w.mcmc_chains, f, &["McmcUi.chains", "mcmc_chains", "chains"]);
            set_s(&mut w.mcmc_seed, f, &["McmcUi.seed", "mcmc_seed", "seed"]);
            set_s(&mut w.mcmc_t, f, &["McmcUi.t", "mcmc_t", "t"]);
        }
        Some(ItemKind::Degradation) => {
            set_s(&mut w.deg_units, f, &["DegUi.units", "deg_units", "units"]);
            set_s(&mut w.deg_threshold, f, &["DegUi.threshold", "deg_threshold", "threshold"]);
            set_e(&mut w.deg_model, f, &["deg_model", "model"], &["DegUi.model-index", "model-index", "model_index", "modelIndex"], &DEG_MODELS);
            set_e(&mut w.deg_dist, f, &["deg_dist", "dist"], &["DegUi.dist-index", "dist-index", "dist_index", "distIndex"], &ALL_DISTS);
            set_b(&mut w.deg_increasing, f, &["DegUi.increasing", "deg_increasing", "increasing"]);
        }
        Some(ItemKind::Alt) => {
            set_s(&mut w.alt_data, f, &["AltUi.data", "alt_data", "data"]);
            set_e(&mut w.alt_model, f, &["alt_model", "model"], &["AltUi.model-index", "model-index", "model_index", "modelIndex"], &ALT_MODELS);
            set_s(&mut w.alt_use_stress, f, &["AltUi.use-stress", "use-stress", "alt_use_stress", "use_stress", "useStress"]);
            set_s(&mut w.alt_t, f, &["AltUi.t", "alt_t", "t"]);
        }
        Some(ItemKind::Growth) => {
            set_s(&mut w.growth_times, f, &["GrowthUi.times", "growth_times", "times"]);
        }
        Some(ItemKind::Competing) => {
            set_s(&mut w.comp_data, f, &["CompUi.data", "comp_data", "data"]);
            set_e(&mut w.comp_dist, f, &["comp_dist", "dist"], &["CompUi.dist-index", "dist-index", "dist_index", "distIndex"], &ALL_DISTS);
            set_s(&mut w.comp_t, f, &["CompUi.t", "comp_t", "t"]);
        }
        Some(ItemKind::Rbd) => {
            set_s(&mut w.rbd_t, f, &["RbdUi.t", "rbd_t", "t"]);
            set_s(&mut w.rbd_horizon, f, &["RbdUi.horizon", "rbd_horizon", "horizon"]);
            set_e(&mut w.rbd_topology, f, &["rbd_topology", "topology"], &["RbdUi.topology-index", "topology-index", "topology_index", "topologyIndex"], &TOPOLOGIES);
            set_s(&mut w.rbd_k, f, &["RbdUi.k", "rbd_k", "k"]);
            set_s(&mut w.ed_name, f, &["RbdUi.ed-name", "ed-name", "ed_name"]);
            set_e(&mut w.ed_dist, f, &["ed_dist"], &["RbdUi.ed-dist-index", "ed-dist-index", "ed_dist_index", "edDistIndex"], &ALL_DISTS);
            set_s(&mut w.ed_params, f, &["RbdUi.ed-params", "ed-params", "ed_params"]);
            set_s(&mut w.ed_mtbf, f, &["RbdUi.ed-mtbf", "ed-mtbf", "ed_mtbf"]);
            set_s(&mut w.ed_mttr, f, &["RbdUi.ed-mttr", "ed-mttr", "ed_mttr"]);
            set_s(&mut w.ed_units, f, &["RbdUi.ed-units", "ed-units", "ed_units"]);
            set_s(&mut w.ed_dormancy, f, &["RbdUi.ed-dormancy", "ed-dormancy", "ed_dormancy"]);
            set_s(&mut w.ed_switch, f, &["RbdUi.ed-switch", "ed-switch", "ed_switch"]);
            set_s(&mut w.nb_name, f, &["RbdUi.nb-name", "nb-name", "nb_name"]);
            set_e(&mut w.nb_dist, f, &["nb_dist"], &["RbdUi.nb-dist-index", "nb-dist-index", "nb_dist_index", "nbDistIndex"], &ALL_DISTS);
            set_s(&mut w.nb_params, f, &["RbdUi.nb-params", "nb-params", "nb_params"]);
            set_s(&mut w.nb_mtbf, f, &["RbdUi.nb-mtbf", "nb-mtbf", "nb_mtbf"]);
            set_s(&mut w.nb_mttr, f, &["RbdUi.nb-mttr", "nb-mttr", "nb_mttr"]);
            set_e(&mut w.nb_mode, f, &["nb_mode", "mode"], &["RbdUi.nb-mode-index", "nb-mode-index", "nb_mode_index", "nbModeIndex"], &NB_MODES);
            set_s(&mut w.nb_dormancy, f, &["RbdUi.nb-dormancy", "nb-dormancy", "nb_dormancy"]);
            set_s(&mut w.rbd_json, f, &["RbdUi.json", "rbd_json", "json"]);
            set_s(&mut w.rbd_path, f, &["RbdUi.path", "rbd_path", "path"]);
        }
        Some(ItemKind::FaultTree) => {
            set_s(&mut w.ft_json, f, &["FtUi.json", "ft_json", "json"]);
        }
        Some(ItemKind::Fmea) => {
            set_e(&mut w.fmea_type, f, &["fmea_type", "type"], &["FmeaUi.type-index", "type-index", "type_index", "typeIndex"], &FMEA_TYPES);
            set_s(&mut w.fm_item, f, &["FmeaUi.fm-item", "fm-item", "fm_item"]);
            set_s(&mut w.fm_mode, f, &["FmeaUi.fm-mode", "fm-mode", "fm_mode"]);
            set_s(&mut w.fm_sev, f, &["FmeaUi.fm-sev", "fm-sev", "fm_sev"]);
            set_s(&mut w.fm_occ, f, &["FmeaUi.fm-occ", "fm-occ", "fm_occ"]);
            set_s(&mut w.fm_det, f, &["FmeaUi.fm-det", "fm-det", "fm_det"]);
            set_s(&mut w.fm_alpha, f, &["FmeaUi.fm-alpha", "fm-alpha", "fm_alpha"]);
            set_s(&mut w.fm_beta, f, &["FmeaUi.fm-beta", "fm-beta", "fm_beta"]);
            set_s(&mut w.fm_lambda, f, &["FmeaUi.fm-lambda", "fm-lambda", "fm_lambda"]);
            set_s(&mut w.fm_hours, f, &["FmeaUi.fm-hours", "fm-hours", "fm_hours"]);
            set_s(&mut w.dv_id, f, &["FmeaUi.dv-id", "dv-id", "dv_id"]);
            set_s(&mut w.dv_ref, f, &["FmeaUi.dv-ref", "dv-ref", "dv_ref"]);
            set_s(&mut w.dv_test, f, &["FmeaUi.dv-test", "dv-test", "dv_test"]);
            set_s(&mut w.dv_sample, f, &["FmeaUi.dv-sample", "dv-sample", "dv_sample"]);
            set_s(&mut w.dv_criteria, f, &["FmeaUi.dv-criteria", "dv-criteria", "dv_criteria"]);
            set_s(&mut w.fmea_csv, f, &["FmeaUi.csv", "fmea_csv", "csv"]);
            set_s(&mut w.fmea_export_path, f, &["FmeaUi.export-path", "export-path", "fmea_export_path", "export_path"]);
            set_s(&mut w.fmea_rcm_row, f, &["FmeaUi.rcm-row", "rcm-row", "fmea_rcm_row", "rcm_row"]);
        }
        Some(ItemKind::Warranty) => {
            set_s(&mut w.war_sales, f, &["WarUi.sales", "war_sales", "sales"]);
            set_s(&mut w.war_returns, f, &["WarUi.returns", "war_returns", "returns"]);
            set_s(&mut w.war_current, f, &["WarUi.current", "war_current", "current"]);
            set_s(&mut w.war_horizon, f, &["WarUi.horizon", "war_horizon", "horizon"]);
        }
        Some(ItemKind::Prediction) => {
            set_e(&mut w.pred_source, f, &["pred_source", "source"], &["PredUi.source-index", "source-index", "source_index", "sourceIndex"], &PRED_SOURCES);
            set_e(&mut w.pred_life_dist, f, &["pred_life_dist"], &["PredUi.life-dist-index", "life-dist-index", "life_dist_index", "lifeDistIndex"], &ALL_DISTS);
            set_s(&mut w.pred_life_params, f, &["PredUi.life-params", "life-params", "pred_life_params", "life_params"]);
            set_s(&mut w.pred_life_t, f, &["PredUi.life-t", "life-t", "pred_life_t", "life_t"]);
            set_s(&mut w.pred_parts, f, &["PredUi.parts", "pred_parts", "parts"]);
            set_s(&mut w.pred_t, f, &["PredUi.t", "pred_t", "t"]);
            set_s(&mut w.pred_lib_path, f, &["PredUi.lib-path", "lib-path", "pred_lib_path", "lib_path"]);
            set_e(&mut w.pred217_file, f, &["pred217_file", "p217_file", "pred_p217_file", "pred.p217_file"], &["PredUi.p217-file-index", "p217-file-index", "p217_file_index", "p217FileIndex"], &MIL217_FILES);
            if w.pred217_is_misc() {
                if let Some(p) = fs(f, &["p217_part", "part", "pred_p217_part", "pred.p217_part"]) {
                    w.pred217_named.insert("part".to_string(), p);
                } else if let Some(i) = fi(f, &["PredUi.p217-part-index", "p217-part-index", "p217_part_index", "p217PartIndex", "p217_part_index", "pred_p217_part_index"]) {
                    let opts = w.pred217_part_options();
                    if let Some(p) = opts.get(i.max(0) as usize) {
                        w.pred217_named.insert("part".to_string(), p.clone());
                    }
                }
                if let Some(e) = enum_val(f, &["p217_env", "environment", "pred_p217_env", "pred.p217_env"], &["PredUi.p217-env-index", "p217-env-index", "p217_env_index", "p217EnvIndex", "p217_env_index", "pred_p217_env_index"], &ENVIRONMENTS) {
                    w.pred217_named.insert("environment".to_string(), e);
                }
            }
            set_s(&mut w.alloc_target, f, &["PredUi.alloc-target", "alloc-target", "alloc_target"]);
            set_s(&mut w.alloc_n, f, &["PredUi.alloc-n", "alloc-n", "alloc_n"]);
            set_s(&mut w.alloc_weights, f, &["PredUi.alloc-weights", "alloc-weights", "alloc_weights"]);
            set_s(&mut w.der_name, f, &["PredUi.der-name", "der-name", "der_name"]);
            set_s(&mut w.der_operating, f, &["PredUi.der-operating", "der-operating", "der_operating"]);
            set_s(&mut w.der_rated, f, &["PredUi.der-rated", "der-rated", "der_rated"]);
            set_s(&mut w.der_max, f, &["PredUi.der-max", "der-max", "der_max"]);
            set_s(&mut w.sr_generic, f, &["PredUi.sr-generic", "sr-generic", "sr_generic"]);
            set_s(&mut w.sr_pi_q, f, &["PredUi.sr-pi-q", "sr-pi-q", "sr_pi_q"]);
            set_s(&mut w.sr_pi_s, f, &["PredUi.sr-pi-s", "sr-pi-s", "sr_pi_s"]);
            set_s(&mut w.sr_ea, f, &["PredUi.sr-ea", "sr-ea", "sr_ea"]);
            set_s(&mut w.sr_temp, f, &["PredUi.sr-temp", "sr-temp", "sr_temp"]);
            set_s(&mut w.sr_ref_temp, f, &["PredUi.sr-ref-temp", "sr-ref-temp", "sr_ref_temp"]);
            set_s(&mut w.sr_pi_e, f, &["PredUi.sr-pi-e", "sr-pi-e", "sr_pi_e"]);
            set_s(&mut w.sr_qty_rate, f, &["PredUi.sr-qty-rate", "sr-qty-rate", "sr_qty_rate"]);
            set_s(&mut w.sr_obs_rate, f, &["PredUi.sr-obs-rate", "sr-obs-rate", "sr_obs_rate"]);
            set_s(&mut w.sr_pit_obs, f, &["PredUi.sr-pit-obs", "sr-pit-obs", "sr_pit_obs"]);
            set_s(&mut w.sr_pit_tgt, f, &["PredUi.sr-pit-tgt", "sr-pit-tgt", "sr_pit_tgt"]);
            set_s(&mut w.sr_pis_obs, f, &["PredUi.sr-pis-obs", "sr-pis-obs", "sr_pis_obs"]);
            set_s(&mut w.sr_pis_tgt, f, &["PredUi.sr-pis-tgt", "sr-pis-tgt", "sr_pis_tgt"]);
            set_s(&mut w.sr_ci_failures, f, &["PredUi.sr-ci-failures", "sr-ci-failures", "sr_ci_failures"]);
            set_s(&mut w.sr_ci_exposure, f, &["PredUi.sr-ci-exposure", "sr-ci-exposure", "sr_ci_exposure"]);
            set_s(&mut w.sr_ci_conf, f, &["PredUi.sr-ci-conf", "sr-ci-conf", "sr_ci_conf"]);
            set_s(&mut w.sr_m3_pred, f, &["PredUi.sr-m3-pred", "sr-m3-pred", "sr_m3_pred"]);
            set_s(&mut w.sr_m3_pw, f, &["PredUi.sr-m3-pw", "sr-m3-pw", "sr_m3_pw"]);
            set_s(&mut w.sr_m3_obs, f, &["PredUi.sr-m3-obs", "sr-m3-obs", "sr_m3_obs"]);
            set_s(&mut w.sr_m3_ow, f, &["PredUi.sr-m3-ow", "sr-m3-ow", "sr_m3_ow"]);
            set_s(&mut w.mp_base, f, &["PredUi.mp-base", "mp-base", "mp_base"]);
            set_s(&mut w.mp_ea, f, &["PredUi.mp-ea", "mp-ea", "mp_ea"]);
            set_s(&mut w.mp_ref, f, &["PredUi.mp-ref", "mp-ref", "mp_ref"]);
            set_s(&mut w.mp_cyc_factor, f, &["PredUi.mp-cyc", "mp-cyc", "mp_cyc", "mp_cyc_factor"]);
            set_s(&mut w.mp_phases, f, &["PredUi.mp-phases", "mp-phases", "mp_phases"]);
        }
        Some(ItemKind::Doe) => {
            set_s(&mut w.doe_k, f, &["DoeUi.k", "doe_k", "k"]);
            set_s(&mut w.doe_y, f, &["DoeUi.y", "doe_y", "y"]);
        }
        Some(ItemKind::Flowchart) => {
            set_s(&mut w.flow_json, f, &["FlowUi.json", "flow_json", "json"]);
            set_s(&mut w.flow_sims, f, &["FlowUi.sims", "flow_sims", "sims"]);
            set_s(&mut w.flow_seed, f, &["FlowUi.seed", "flow_seed", "seed"]);
        }
        Some(ItemKind::Markov) => {
            set_s(&mut w.mk_new_state, f, &["MarkovUi.new-state", "new-state", "mk_new_state", "new_state"]);
            if let Some(s) = fs(f, &["mk_start", "start"]) {
                if w.mk_states.contains(&s) {
                    w.mk_start = s;
                }
            } else if let Some(i) = fi(f, &["MarkovUi.start-index", "start-index", "start_index", "startIndex"]) {
                if let Some(s) = w.mk_states.get(i.max(0) as usize).cloned() {
                    w.mk_start = s;
                }
            }
            if let Some(s) = fs(f, &["mk_absorbing", "absorbing"]) {
                if w.mk_states.contains(&s) {
                    w.mk_absorbing = s;
                }
            } else if let Some(i) = fi(f, &["MarkovUi.absorbing-index", "absorbing-index", "absorbing_index", "absorbingIndex"]) {
                if let Some(s) = w.mk_states.get(i.max(0) as usize).cloned() {
                    w.mk_absorbing = s;
                }
            }
            set_s(&mut w.mk_time, f, &["MarkovUi.time", "mk_time", "time"]);
            if let Some(i) = w.mk_sel {
                let from = fs(f, &["MarkovUi.tr-from", "tr-from", "tr_from"]);
                let to = fs(f, &["MarkovUi.tr-to", "tr-to", "tr_to"]);
                let rate = fs(f, &["MarkovUi.tr-rate", "tr-rate", "tr_rate"]);
                if from.is_some() || to.is_some() || rate.is_some() {
                    if let Some(t) = w.mk_transitions.get_mut(i) {
                        if let Some(v) = from { t[0] = v; }
                        if let Some(v) = to { t[1] = v; }
                        if let Some(v) = rate { t[2] = v; }
                    }
                }
            }
        }
        Some(ItemKind::Qcp) => {
            set_e(&mut w.qcp_dist, f, &["qcp_dist", "dist"], &["QcpUi.dist-index", "dist-index", "dist_index", "distIndex"], &ALL_DISTS);
            set_s(&mut w.qcp_t, f, &["QcpUi.t", "qcp_t", "t"]);
            set_s(&mut w.qcp_bx, f, &["QcpUi.bx", "qcp_bx", "bx"]);
        }
        Some(ItemKind::Rcm) => {
            if let Some(r) = w.rcm_sel.and_then(|i| w.rcm_rows.get_mut(i)) {
                set_s(&mut r.asset, f, &["RcmUi.ed-asset", "ed-asset", "ed_asset", "asset"]);
                set_s(&mut r.function, f, &["RcmUi.ed-function", "ed-function", "ed_function", "function"]);
                set_s(&mut r.mode, f, &["RcmUi.ed-mode", "ed-mode", "ed_mode", "mode"]);
                if let Some(v) = enum_val(f, &["consequence"], &["RcmUi.ed-consequence-index", "ed-consequence-index", "ed_consequence_index", "edConsequenceIndex"], &CONSEQUENCES) {
                    r.consequence = v;
                }
                set_b(&mut r.monitorable, f, &["RcmUi.ed-monitorable", "ed-monitorable", "ed_monitorable", "monitorable"]);
                set_b(&mut r.age_related, f, &["RcmUi.ed-age", "ed-age", "ed_age", "age_related"]);
                set_b(&mut r.restorable, f, &["RcmUi.ed-restorable", "ed-restorable", "ed_restorable", "restorable"]);
                set_s(&mut r.rate, f, &["RcmUi.ed-rate", "ed-rate", "ed_rate", "rate"]);
                set_s(&mut r.alpha, f, &["RcmUi.ed-alpha", "ed-alpha", "ed_alpha", "alpha"]);
                set_s(&mut r.beta, f, &["RcmUi.ed-beta", "ed-beta", "ed_beta", "beta"]);
                set_s(&mut r.hours, f, &["RcmUi.ed-hours", "ed-hours", "ed_hours", "hours"]);
            }
        }
        Some(ItemKind::Fracas) => {
            set_s(&mut w.fracas_total_hours, f, &["FracasUi.total-hours", "total-hours", "fracas_total_hours", "total_hours"]);
            set_s(&mut w.fracas_path, f, &["FracasUi.path", "fracas_path", "path"]);
            if let Some(inc) = w.fracas_sel.and_then(|i| w.fracas_rows.get_mut(i)) {
                set_s(&mut inc.id, f, &["FracasUi.ed-id", "ed-id", "ed_id"]);
                set_s(&mut inc.part, f, &["FracasUi.ed-part", "ed-part", "ed_part", "part"]);
                set_s(&mut inc.failure_mode, f, &["FracasUi.ed-mode", "ed-mode", "ed_mode", "mode"]);
                if let Some(h) = ff(f, &["FracasUi.ed-hours", "ed-hours", "ed_hours", "hours"]) {
                    inc.hours = h;
                }
                set_s(&mut inc.date, f, &["FracasUi.ed-date", "ed-date", "ed_date", "date"]);
                if let Some(s) = fs(f, &["status_name", "ed_status"]) {
                    inc.status = match s.as_str() {
                        "Analysis" => IncidentStatus::Analysis,
                        "CorrectiveAction" | "Corrective Action" => IncidentStatus::CorrectiveAction,
                        "Closed" => IncidentStatus::Closed,
                        _ => IncidentStatus::Open,
                    };
                } else if let Some(i) = fi(f, &["FracasUi.ed-status-index", "ed-status-index", "ed_status_index", "edStatusIndex"]) {
                    inc.status = match i {
                        1 => IncidentStatus::Analysis,
                        2 => IncidentStatus::CorrectiveAction,
                        3 => IncidentStatus::Closed,
                        _ => IncidentStatus::Open,
                    };
                }
                set_s(&mut inc.corrective_action, f, &["FracasUi.ed-action", "ed-action", "ed_action", "action_text"]);
            }
        }
        None => {}
    }
}

// ---------------- normalize (port from main.rs) ----------------

fn pred217_specs(w: &Work) -> Vec<(bool, String, String, Vec<String>)> {
    if w.pred217_is_pth() {
        let mut v: Vec<(bool, String, String, Vec<String>)> = Vec::new();
        for (key, _) in PTH_NAMED {
            let label = match key {
                "pth_technology" => "Technology (by-type key)",
                "pth_quality" => "Quality",
                _ => "Environment",
            };
            v.push((false, key.to_string(), label.to_string(), Vec::new()));
        }
        for (key, _) in PTH_NUM {
            let label = match key {
                "pth_n1" => "N1 automated PTH count",
                "pth_n2" => "N2 hand soldered PTH count",
                _ => "Circuit planes",
            };
            v.push((false, key.to_string(), label.to_string(), Vec::new()));
        }
        return v;
    }
    match &w.pred217_json {
        Some(cat) => Work::pred217_fields(cat),
        None => Vec::new(),
    }
}

fn pred217_value(w: &Work, named: bool, key: &str) -> String {
    if let Some((_, def)) = PTH_NAMED.iter().find(|(k, _)| *k == key) {
        return w.pred217_named.get(key).cloned().unwrap_or_else(|| def.to_string());
    }
    if let Some((_, def)) = PTH_NUM.iter().find(|(k, _)| *k == key) {
        return w.pred217_num.get(key).cloned().unwrap_or_else(|| def.to_string());
    }
    if named {
        w.pred217_named.get(key).cloned().unwrap_or_default()
    } else {
        w.pred217_num.get(key).cloned().unwrap_or_default()
    }
}

fn pred217_seed_defaults(w: &mut Work) {
    if w.pred217_json.is_none() {
        return;
    }
    if w.pred217_is_pth() {
        for (k, def) in PTH_NAMED {
            w.pred217_named.entry(k.to_string()).or_insert_with(|| def.to_string());
        }
        for (k, def) in PTH_NUM {
            w.pred217_num.entry(k.to_string()).or_insert_with(|| def.to_string());
        }
    }
    if w.pred217_is_misc() {
        if !w.pred217_named.contains_key("part") {
            if let Some(first) = w.pred217_part_options().first().cloned() {
                w.pred217_named.insert("part".to_string(), first);
            }
        }
        w.pred217_named.entry("environment".to_string()).or_insert_with(|| "GB".to_string());
    }
}

fn normalize(w: &mut Work) {
    if !w.items.iter().any(|it| it.id == w.flow_target_id) {
        w.flow_target_id = w
            .items
            .iter()
            .find(|it| it.kind == ItemKind::Rbd)
            .map(|it| it.id)
            .unwrap_or(0);
    }
    pred217_seed_defaults(w);
}

// ---------------- data helpers with embedded fallbacks ----------------

/// Bundled CSV into the current item. Uses the embedded copy so
/// hosts without a filesystem behave the same as the desktop.
fn csv_bundled_action(w: &mut Work) {
    if let Some(text) = crate::bundled_csv_text(&w.csv_pick) {
        let pick = w.csv_pick.clone();
        let text = text.to_string();
        w.apply_csv_to_current(&text);
        w.note(format!("Loaded bundled dataset {}", pick));
    } else {
        w.csv_load_bundled();
    }
}

/// Dataset card: open the file as a new item, with the embedded
/// copy as fallback when the datasets directory is not readable
/// (the browser demo).
fn load_dataset_action(w: &mut Work, file: &str) {
    let before = w.items.len();
    w.load_dataset_item(file);
    if w.items.len() > before {
        return;
    }
    if let Some(text) = crate::bundled_csv_text(file) {
        let kind = dataset_kind(file);
        let idx = w.create_item_silent(kind, format!("{} ({})", file, kind.label()), json!({}));
        w.select_index(Some(idx));
        let text = text.to_string();
        w.apply_csv_to_current(&text);
        w.note(format!("Loaded dataset {} as a new {} item.", file, kind.label()));
    }
}

/// MIL-HDBK-217F category load with the embedded library as
/// fallback, then the same default seeding main.rs applied.
fn pred217_load_action(w: &mut Work) {
    w.pred217_load();
    if w.pred217_json.is_none() {
        if let Some(text) = crate::mil217_text(&w.pred217_file) {
            if let Ok(v) = reliakit::standards217::load_category(text) {
                w.pred217_json = Some(v);
                w.pred217_result = String::new();
                let file = w.pred217_file.clone();
                w.note(format!("Loaded MIL-HDBK-217F category {}", file));
            }
        }
    }
    pred217_seed_defaults(w);
}

/// Open a project from JSON text (the project_open_data action and
/// the legacy branch of open_project, without the filesystem).
fn open_project_value(w: &mut Work, v: &Value) {
    if let Some((name, items, sel)) = project_from_value(v) {
        w.project_name = name;
        w.next_id = items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
        w.items = items;
        w.selected = None;
        if let Some(i) = sel {
            w.select_index(Some(i));
        }
        w.note("Project opened from data.".to_string());
        return;
    }
    if v.get("life_times").is_some() || v.get("rbd").is_some() || v.get("fmea_rows").is_some() {
        w.items = Vec::new();
        w.selected = None;
        w.next_id = 1;
        if v.get("life_times").is_some() {
            let st = json!({
                "life_times": v.get("life_times").cloned().unwrap_or(json!("")),
                "life_dist": v.get("life_dist").cloned().unwrap_or(json!("weibull2")),
                "life_t": v.get("life_t").cloned().unwrap_or(json!("1000")),
            });
            w.create_item_silent(ItemKind::LifeData, "Life Data (from old project)".into(), st);
        }
        if let Some(rbd) = v.get("rbd") {
            let st = json!({"rbd_spec": rbd.clone(), "rbd_t": "1000"});
            w.create_item_silent(ItemKind::Rbd, "RBD (from old project)".into(), st);
        }
        if v.get("fmea_rows").is_some() {
            let st = json!({
                "fmea_type": v.get("fmea_type").cloned().unwrap_or(json!("Design")),
                "fmea_rows": v.get("fmea_rows").cloned().unwrap_or(json!([])),
                "dvpr": v.get("dvpr").cloned().unwrap_or(json!([])),
            });
            w.create_item_silent(ItemKind::Fmea, "FMEA (from old project)".into(), st);
        }
        let n = w.items.len();
        w.note(format!("Old format project opened from data as {} items.", n));
        return;
    }
    w.note("Project data is not a ReliaKit project (v2 or legacy).".to_string());
}

fn project_open_data(w: &mut Work, f: &Fields) {
    match fs(f, &["text", "data", "json", "project", "content"]) {
        Some(text) => match serde_json::from_str::<Value>(&text) {
            Ok(v) => open_project_value(w, &v),
            Err(e) => w.note(format!("Project JSON error: {}", e)),
        },
        None => w.note("project_open_data needs the project JSON text in fields.text.".to_string()),
    }
}

// ---------------- dispatch (port of dispatch_act, dispatch_row,
// dispatch_field, and the Actions callbacks in main.rs) ----------------

fn markov_remove_state(w: &mut Work, i: usize) {
    if let Some(name) = w.mk_states.get(i).cloned() {
        w.mk_states.remove(i);
        w.mk_transitions.retain(|t| t[0] != name && t[1] != name);
        if w.mk_start == name {
            w.mk_start = w.mk_states.first().cloned().unwrap_or_default();
        }
        if w.mk_absorbing == name {
            w.mk_absorbing = w.mk_states.last().cloned().unwrap_or_default();
        }
        w.mk_sel = None;
        w.note(format!("State {} removed.", name));
    }
}

/// Returns true when the name was a screen action.
fn dispatch_act(w: &mut Work, name: &str) -> bool {
    match name {
        "life-fit" => { w.compute_life_fit(); Vec::<(String, String)>::new() }
        "life-wizard" => { w.compute_wizard(); Vec::new() }
        "flow-rbd" => { w.flow_assign_fit_to_rbd(); Vec::new() }
        "flow-qcp" => { w.flow_send_fit_to_qcp(); Vec::new() }
        "flow-pred" => { w.flow_send_fit_to_prediction(); Vec::new() }
        "flow-target" => Vec::new(),
        "limited-run" => { w.compute_limited(); Vec::new() }
        "mcmc-run" => { w.compute_mcmc(); Vec::new() }
        "deg-run" => { w.compute_degradation(); Vec::new() }
        "alt-run" => { w.compute_alt(); Vec::new() }
        "growth-run" => { w.compute_growth(); Vec::new() }
        "comp-run" => { w.compute_competing(); Vec::new() }
        "rbd-avail" => { w.compute_rbd_availability(); Vec::new() }
        "rbd-importance" => {
            w.compute_rbd_importance();
            Vec::new()
        }
        "rbd-topology" => {
            w.rbd_apply_topology();
            Vec::new()
        }
        "rbd-apply-props" => {
            w.rbd_apply_editor();
            Vec::new()
        }
        "rbd-duplicate" => {
            w.rbd_duplicate_block();
            Vec::new()
        }
        "rbd-delete" => {
            w.rbd_delete_block();
            Vec::new()
        }
        "rbd-undo" => {
            w.undo_rbd();
            Vec::new()
        }
        "rbd-add-form" => {
            w.rbd_add_block_form();
            Vec::new()
        }
        "rbd-load-json" => {
            w.rbd_load_json_text();
            Vec::new()
        }
        "rbd-save-file" => {
            w.rbd_save_file();
            Vec::new()
        }
        "rbd-load-file" => {
            w.rbd_load_file();
            Vec::new()
        }
        "ft-run" => { w.compute_fault_tree(); Vec::new() }
        "fmea-add-row" => {
            w.fmea_add_row();
            Vec::new()
        }
        "fmea-to-rcm" => {
            w.flow_rcm_row_from_field();
            Vec::new()
        }
        "fmea-add-dvpr" => {
            w.fmea_add_dvpr();
            Vec::new()
        }
        "fmea-export-ws" => {
            w.fmea_export_worksheet();
            Vec::new()
        }
        "fmea-export-rows" => {
            w.fmea_export_rows();
            Vec::new()
        }
        "fmea-import" => {
            w.fmea_import_csv();
            Vec::new()
        }
        "fmea-clear" => {
            w.fmea_clear_rows();
            Vec::new()
        }
        "fmea-export-cp" => {
            w.fmea_export_control_plan();
            Vec::new()
        }
        "fmea-export-dvpr" => {
            w.fmea_export_dvpr();
            Vec::new()
        }
        "war-run" => { w.compute_warranty(); Vec::new() }
        "pred-life" => { w.compute_pred_life(); Vec::new() }
        "pred-run" => { w.compute_pred_parts(); Vec::new() }
        "pred-lib-load" => {
            w.pred_lib_load();
            Vec::new()
        }
        "pred217-load" => {
            pred217_load_action(w);
            Vec::new()
        }
        "pred217-compute" => {
            w.compute_pred217();
            Vec::new()
        }
        "pred-alloc" => { w.compute_alloc(); Vec::new() }
        "pred-derate" => { w.compute_derating(); Vec::new() }
        "pred-clean" => { w.compute_clean_room(); Vec::new() }
        "pred-mission" => { w.compute_mission(); Vec::new() }
        "doe-run" => { w.compute_doe(); Vec::new() }
        "flow-run" => { w.compute_flowchart(); Vec::new() }
        "mk-preset" => {
            w.markov_preset();
            Vec::new()
        }
        "mk-run" => { w.compute_markov(); Vec::new() }
        "mk-add-state" => {
            let name = w.mk_new_state.trim().to_string();
            if !name.is_empty() && !w.mk_states.contains(&name) {
                w.mk_states.push(name.clone());
                if w.mk_states.len() == 1 {
                    w.mk_start = name.clone();
                    w.mk_absorbing = name.clone();
                }
                w.mk_new_state.clear();
                w.note(format!("State {} added.", name));
            }
            Vec::new()
        }
        "mk-add-trans" => {
            if w.mk_states.len() >= 2 {
                w.mk_transitions.push([
                    w.mk_states[0].clone(),
                    w.mk_states[1].clone(),
                    "0.01".to_string(),
                ]);
                w.mk_sel = Some(w.mk_transitions.len() - 1);
                w.note("Transition added. Edit it in the inspector.".to_string());
            } else {
                w.note("Add at least two states before adding a transition.".to_string());
            }
            Vec::new()
        }
        "mk-apply-trans" => {
            w.note("Transition updated.".to_string());
            Vec::new()
        }
        "mk-del-trans" => {
            if let Some(i) = w.mk_sel.take() {
                if i < w.mk_transitions.len() {
                    w.mk_transitions.remove(i);
                }
                w.note("Transition removed.".to_string());
            }
            Vec::new()
        }
        "qcp-dist" => {
            w.qcp_reset_params();
            Vec::new()
        }
        "qcp-run" => { w.compute_qcp(); Vec::new() }
        "rcm-add" => {
            w.rcm_add_row();
            Vec::new()
        }
        "rcm-refresh" => {
            w.note("RCM row updated.".to_string());
            Vec::new()
        }
        "rcm-remove" => {
            if let Some(i) = w.rcm_sel.take() {
                if i < w.rcm_rows.len() {
                    w.rcm_rows.remove(i);
                    w.note("RCM row removed.".to_string());
                }
            }
            Vec::new()
        }
        "fracas-add" => {
            w.fracas_add_incident();
            Vec::new()
        }
        "fracas-refresh" => {
            w.fracas_status = "Incident updated.".to_string();
            w.note("Incident updated.".to_string());
            Vec::new()
        }
        "fracas-remove" => {
            if let Some(i) = w.fracas_sel.take() {
                if i < w.fracas_rows.len() {
                    w.fracas_rows.remove(i);
                    w.fracas_status = "Incident removed.".to_string();
                    w.note("Incident removed.".to_string());
                }
            }
            Vec::new()
        }
        "fracas-export-csv" => {
            w.fracas_export_csv();
            Vec::new()
        }
        "fracas-import-csv" => {
            w.fracas_import_csv();
            Vec::new()
        }
        "flow-fracas-life" => {
            w.flow_fracas_to_life();
            Vec::new()
        }
        _ => return false,
    };
    true
}

/// Returns true when the name was a row action.
fn dispatch_row(w: &mut Work, name: &str, i: usize) -> bool {
    match name {
        "wizard-use" => w.wizard_use(i),
        "fmea-rcm-row" => w.flow_fmea_to_rcm(i),
        "fmea-del" => w.fmea_delete_row(i),
        "rcm-select" => w.rcm_sel = Some(i),
        "fracas-select" => w.fracas_sel = Some(i),
        "mk-select" => w.mk_sel = Some(i),
        "mk-del-state" => markov_remove_state(w, i),
        _ => return false,
    }
    true
}

fn dispatch_field(w: &mut Work, id: &str, value: &str) {
    if let Some(key) = id.strip_prefix("qcp:") {
        w.qcp_params.insert(key.to_string(), value.to_string());
        return;
    }
    if let Some(key) = id.strip_prefix("p217:") {
        if PTH_NAMED.iter().any(|(k, _)| *k == key) {
            w.pred217_named.insert(key.to_string(), value.to_string());
        } else if PTH_NUM.iter().any(|(k, _)| *k == key) {
            w.pred217_num.insert(key.to_string(), value.to_string());
        } else {
            let named = pred217_specs(w)
                .iter()
                .find(|(_, k, _, _)| k == key)
                .map(|(n, _, _, _)| *n)
                .unwrap_or(false);
            if named {
                w.pred217_named.insert(key.to_string(), value.to_string());
            } else {
                w.pred217_num.insert(key.to_string(), value.to_string());
            }
        }
    }
}

fn dispatch(w: &mut Work, action: &str, f: &Fields) {
    match action {
        "" | "state" | "init" | "get_state" | "noop" => {}
        "go_start" | "go-start" => w.select_index(None),
        "select_item" | "select-item" => {
            if let Some(i) = fi(f, &["index", "item_index", "selected_index", "value"]) {
                w.select_index(Some(i.max(0) as usize));
            }
        }
        "add_item" | "add-item" => w.add_item_from_panel(),
        "remove_item" | "remove-item" => w.remove_selected_item(),
        "rename_item" | "rename-item" => w.rename_selected_item(),
        "new_project" | "new-project" => w.new_project(),
        "save_project" | "save-project" => w.save_project(),
        "open_project" | "open-project" => w.open_project(),
        "project_open_data" | "project-open-data" => project_open_data(w, f),
        "card_action" | "card-action" => {
            if let Some(id) = fs(f, &["id", "action_id", "action-id", "card", "value"]) {
                if let Some(n) = id.strip_prefix("example:") {
                    if let Ok(n) = n.parse::<usize>() {
                        w.load_example_item(n);
                    }
                } else if let Some(file) = id.strip_prefix("dataset:") {
                    load_dataset_action(w, file);
                }
            }
        }
        "csv_bundled" | "csv-bundled" | "csv_load_bundled" => csv_bundled_action(w),
        "csv_file" | "csv-file" | "csv_load_file" => w.csv_load_file(),
        "csv_load_text" | "csv-load-text" => {
            if let Some(text) = fs(f, &["text", "data", "csv", "content"]) {
                w.apply_csv_to_current(&text);
            } else {
                w.note("csv_load_text needs the CSV text in fields.text.".to_string());
            }
        }
        "pred_lib_load_text" | "pred-lib-load-text" => {
            if let Some(text) = fs(f, &["text", "file_text", "data", "content", "csv_text"]) {
                match reliakit::prediction::library_from_csv(&text) {
                    Ok(parts) => {
                        w.pred_lib_note = format!("Loaded {} parts from the chosen file", parts.len());
                        w.pred_lib_parts = parts;
                    }
                    Err(e) => w.pred_lib_note = format!("Library error: {}", e),
                }
                let note = w.pred_lib_note.clone();
                w.note(note);
            }
        }
        "fracas_import_text" | "fracas-import-text" => {
            if let Some(text) = fs(f, &["text", "file_text", "data", "content", "csv_text"]) {
                match reliakit::fracas::incidents_from_csv(&text) {
                    Ok(v) => {
                        w.fracas_status = format!("Imported {} incidents.", v.len());
                        w.fracas_rows = v;
                    }
                    Err(e) => w.fracas_status = format!("Import error: {}", e),
                }
                let st = w.fracas_status.clone();
                w.note(st);
            }
        }
        "rbd-load-json" => {
            if let Some(text) = fs(f, &["file_text", "text"]) {
                w.rbd_json = text;
            }
            dispatch_act(w, "rbd-load-json");
        }
        "act" => {
            if let Some(name) = fs(f, &["name", "action_name", "value", "id"]) {
                dispatch_act(w, &name);
            }
        }
        "row_act" | "row-act" => {
            let idx = fi(f, &["index", "row", "value"]).unwrap_or(0).max(0) as usize;
            if let Some(name) = fs(f, &["name", "action_name", "id"]) {
                dispatch_row(w, &name, idx);
            }
        }
        "field_changed" | "field-changed" => {
            if let Some(id) = fs(f, &["id", "field", "field_id"]) {
                let value = fs(f, &["value", "field_value"]).unwrap_or_default();
                dispatch_field(w, &id, &value);
            }
        }
        "rbd_arm" | "rbd-arm" => {
            if let Some(name) = fs(f, &["name", "value", "id"]) {
                w.rbd_armed = if w.rbd_armed.as_deref() == Some(name.as_str()) {
                    None
                } else {
                    Some(name)
                };
            }
        }
        "rbd_place" | "rbd-place" => {
            let x = ff(f, &["x"]).unwrap_or(0.0) as f32;
            let y = ff(f, &["y"]).unwrap_or(0.0) as f32;
            w.rbd_place_armed(x, y);
        }
        "rbd_press" | "rbd-press" => {
            if let Some(i) = fi(f, &["index", "idx", "value"]) {
                w.load_rbd_editor(i.max(0) as usize);
            }
        }
        "rbd_drag" | "rbd-drag" => {
            if let Some(i) = fi(f, &["index", "idx", "value"]) {
                let idx = i.max(0) as usize;
                let x = ff(f, &["x"]).unwrap_or(0.0) as f32;
                let y = ff(f, &["y"]).unwrap_or(0.0) as f32;
                // With grab offsets the pointer position is given;
                // without them x and y are the block top-left.
                let gx = ff(f, &["grab_x", "grabX"]).unwrap_or(0.0) as f32;
                let gy = ff(f, &["grab_y", "grabY"]).unwrap_or(0.0) as f32;
                w.rbd_move_block(idx, x - gx, y - gy);
            }
        }
        "rbd_release" | "rbd-release" => {}
        other => {
            if !dispatch_act(w, other) {
                let idx = fi(f, &["index", "row", "value"]).unwrap_or(0).max(0) as usize;
                dispatch_row(w, other, idx);
            }
        }
    }
}

// ---------------- push (port of the push_* functions) ----------------

fn pairs_json(rows: &[(String, String)]) -> Value {
    Value::Array(
        rows.iter()
            .map(|(a, b)| json!({"label": a, "value": b}))
            .collect(),
    )
}

fn rows_json(text: &str) -> Value {
    pairs_json(&rows_of(text))
}

fn cells_json(rows: &[Vec<String>]) -> Value {
    Value::Array(
        rows.iter()
            .map(|r| json!({"cells": r}))
            .collect(),
    )
}

fn strings_json(v: &[String]) -> Value {
    Value::Array(v.iter().map(|s| json!(s)).collect())
}

fn str_json(v: &[&str]) -> Value {
    Value::Array(v.iter().map(|s| json!(s)).collect())
}

const BLOCK_W: f32 = 128.0;
const BLOCK_H: f32 = 56.0;

fn block_geom_json(w: &Work, idx: usize) -> Option<Value> {
    let paths = Work::leaf_paths(&w.rbd_spec);
    let path = paths.get(idx)?;
    let node = Work::node_at(&w.rbd_spec, path)?;
    let (x, y) = Work::leaf_pos(&node, idx);
    let ty = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("block").to_string();
    let sub = if ty == "standby_group" {
        let n = node.get("n_units").and_then(|v| v.as_u64()).unwrap_or(2);
        let d = node.get("dormancy").and_then(|v| v.as_f64()).unwrap_or(0.0);
        format!("standby x{} - dormancy {:.2}", n, d)
    } else {
        let dist = node.get("dist").and_then(|v| v.as_str()).unwrap_or("");
        reliakit::distributions::dist_label(dist).to_string()
    };
    Some(json!({
        "idx": idx,
        "x": x,
        "y": y,
        "w": BLOCK_W,
        "h": BLOCK_H,
        "name": name,
        "sub": sub,
        "selected": w.rbd_sel == Some(idx),
    }))
}

fn rbd_geoms_json(w: &Work) -> Value {
    let n = Work::leaf_paths(&w.rbd_spec).len();
    Value::Array((0..n).filter_map(|i| block_geom_json(w, i)).collect())
}

/// Connector bars between blocks, ported from segments_vec.
fn segments_json(w: &Work) -> Value {
    const T: f32 = 2.0;
    let mut out: Vec<Value> = Vec::new();
    let mut bar = |x: f32, y: f32, wdt: f32, hgt: f32, out: &mut Vec<Value>| {
        if wdt > 0.0 && hgt > 0.0 {
            out.push(json!({"x": x, "y": y, "w": wdt, "h": hgt}));
        }
    };
    let paths = Work::leaf_paths(&w.rbd_spec);
    for pair in paths.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        let ia = paths.iter().position(|p| p == a).unwrap_or(0);
        let ib = ia + 1;
        let _ = b;
        let (ga, gb) = match (block_geom_json(w, ia), block_geom_json(w, ib)) {
            (Some(x), Some(y)) => (x, y),
            _ => continue,
        };
        let gx = |v: &Value| v.get("x").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
        let gy = |v: &Value| v.get("y").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
        let (x1, y1) = (gx(&ga) + BLOCK_W, gy(&ga) + BLOCK_H / 2.0);
        let (x2, y2) = (gx(&gb), gy(&gb) + BLOCK_H / 2.0);
        if y2 > y1 + BLOCK_H {
            let sx = gx(&ga) + BLOCK_W / 2.0;
            let sy = gy(&ga) + BLOCK_H;
            let tx = gx(&gb) + BLOCK_W / 2.0;
            let ty = gy(&gb);
            let midy = (sy + ty) / 2.0;
            bar(sx - T / 2.0, sy, T, midy - sy, &mut out);
            let (hx, hw) = if sx < tx { (sx, tx - sx) } else { (tx, sx - tx) };
            bar(hx, midy - T / 2.0, hw, T, &mut out);
            bar(tx - T / 2.0, midy, T, ty - midy, &mut out);
        } else if (y1 - y2).abs() < 1.0 && x2 >= x1 {
            bar(x1, y1 - T / 2.0, x2 - x1, T, &mut out);
        } else {
            let mid = if x2 >= x1 { (x1 + x2) / 2.0 } else { x1 + 16.0 };
            bar(x1, y1 - T / 2.0, mid - x1, T, &mut out);
            let (top, bot) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
            bar(mid - T / 2.0, top, T, bot - top, &mut out);
            let (hx, hw) = if x2 >= mid { (mid, x2 - mid) } else { (x2, mid - x2) };
            bar(hx, y2 - T / 2.0, hw, T, &mut out);
        }
    }
    Value::Array(out)
}

fn start_cards_json() -> (Value, Value) {
    let examples: Vec<Value> = [
        (1, "EXAMPLE 1", "Bearing life data", "Weibull fit on the bundled bearing dataset.", "life"),
        (2, "EXAMPLE 2", "Limited data lots", "Small-sample lots for hierarchical shrinkage.", "limited"),
        (3, "EXAMPLE 3", "RBD with cold standby", "Series system, standby pumps, parallel controllers.", "rbd"),
        (4, "EXAMPLE 4", "FMEA sample", "Three sample worksheet rows with FMECA criticality.", "fmea"),
        (5, "EXAMPLE 5", "ALT dataset", "Arrhenius data for least squares and censored MLE.", "alt"),
        (6, "EXAMPLE 6", "Growth dataset", "Crow-AMSAA reliability growth data.", "growth"),
        (7, "EXAMPLE 7", "Degradation units", "Wear paths against a failure threshold.", "degradation"),
        (8, "EXAMPLE 8", "Markov preset", "Two-unit repairable system. MTTF is 650 hours.", "markov"),
    ]
    .iter()
    .map(|(n, tag, title, sub, icon)| {
        json!({"title": title, "sub": sub, "tag": tag, "icon": icon, "action-id": format!("example:{}", n)})
    })
    .collect();
    let datasets: Vec<Value> = BUNDLED_CSV
        .iter()
        .map(|file| {
            let kind = dataset_kind(file);
            json!({
                "title": file,
                "sub": format!("Loads into a {} item.", kind.label()),
                "tag": kind.label().to_uppercase(),
                "icon": kind.icon(),
                "action-id": format!("dataset:{}", file),
            })
        })
        .collect();
    let chunk = |v: Vec<Value>| -> Value {
        Value::Array(
            v.chunks(4)
                .map(|c| Value::Array(c.to_vec()))
                .collect(),
        )
    };
    (chunk(examples), chunk(datasets))
}

fn chart_json(w: &Work) -> Value {
    let (title, list) = series::current_chart(w);
    json!({
        "title": title,
        "has-chart": !list.is_empty(),
        "series": serde_json::to_value(&list).unwrap_or(json!([])),
    })
}

/// The full UI state, mirroring gui/ui/state.slint: one object per
/// global, plus the status line, the current chart, and a project
/// snapshot the browser host can offer as a download.
pub fn build_state(w: &Work) -> Value {
    let screen = match current_kind(w) {
        Some(k) => k.key().to_string(),
        None => "start".to_string(),
    };
    let (example_rows, dataset_rows) = start_cards_json();
    let chart = chart_json(w);
    let series_arr = chart.get("series").cloned().unwrap_or(json!([]));
    let has_chart = chart.get("has-chart").cloned().unwrap_or(json!(false));
    let chart_title = chart.get("title").cloned().unwrap_or(json!(""));

    let items: Vec<Value> = w
        .items
        .iter()
        .map(|it| {
            json!({
                "id": it.id,
                "kind": it.kind.key(),
                "kind-label": it.kind.label(),
                "name": it.name,
                "icon": it.kind.icon(),
            })
        })
        .collect();

    // Life Data
    let life_headline = match &w.life_fit {
        Some(fit) => {
            let t = fnum(&w.life_t, 1000.0);
            format!("R({:.0}) = {:.4}", t, fit.reliability(t))
        }
        None => String::new(),
    };
    let wizard_rows: Vec<Value> = w
        .wizard_ranks
        .iter()
        .map(|r| {
            json!({
                "dist": r.dist,
                "label": reliakit::distributions::dist_label(&r.dist),
                "loglik": format!("{:.2}", r.loglik),
                "aic": format!("{:.2}", r.aic),
                "bic": format!("{:.2}", r.bic),
                "ad": r.ad_statistic.map(|v| format!("{:.3}", v)).unwrap_or_else(|| "-".to_string()),
                "usable": r.success,
            })
        })
        .collect();
    let rbd_targets: Vec<String> = w.items.iter().filter(|it| it.kind == ItemKind::Rbd).map(|it| it.name.clone()).collect();
    let target_pos = w
        .items
        .iter()
        .filter(|it| it.kind == ItemKind::Rbd)
        .position(|it| it.id == w.flow_target_id)
        .map(|i| i as i64)
        .unwrap_or(0);
    let life_chart_title = if w.life_show_prob { "Probability plot" } else { "Reliability curve" };

    // RBD inspector
    let (sel_title, ed_is_standby) = match w.rbd_sel {
        Some(idx) => {
            let paths = Work::leaf_paths(&w.rbd_spec);
            let node = paths.get(idx).and_then(|p| Work::node_at(&w.rbd_spec, p));
            let title = node
                .as_ref()
                .map(|n| {
                    let name = n.get("name").and_then(|v| v.as_str()).unwrap_or("block");
                    let ty = n.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    format!("{} ({})", name, ty)
                })
                .unwrap_or_default();
            let standby = node
                .as_ref()
                .and_then(|n| n.get("type").and_then(|v| v.as_str()).map(|t| t == "standby_group"))
                .unwrap_or(false);
            (title, standby)
        }
        None => (String::new(), false),
    };
    let imp_rows: Vec<Vec<String>> = w
        .rbd_importance
        .iter()
        .map(|b| {
            vec![
                b.name.clone(),
                format!("{:.4}", b.birnbaum),
                format!("{:.4}", b.fussell_vesely),
                format!("{:.4}", b.criticality),
            ]
        })
        .collect();

    // FMEA
    let fmea_rows: Vec<Vec<String>> = w
        .fmea_rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            vec![
                r.item.clone(),
                r.failure_mode.clone(),
                r.severity.to_string(),
                r.occurrence.to_string(),
                r.detection.to_string(),
                r.rpn().to_string(),
                r.revised_rpn().map(|v| v.to_string()).unwrap_or_default(),
                w.fmea_criticality(i).map(|v| format!("{:.4}", v)).unwrap_or_default(),
            ]
        })
        .collect();
    let dvpr_rows: Vec<Vec<String>> = w
        .dvpr_entries
        .iter()
        .map(|d| {
            vec![
                d.id.clone(),
                d.failure_mode_ref.clone(),
                d.test.clone(),
                d.sample_size.to_string(),
                d.acceptance_criteria.clone(),
            ]
        })
        .collect();

    // Warranty
    let ages = w.war_table.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut war_header = vec!["Sale period".to_string()];
    for a in 0..ages {
        war_header.push(format!("Age {}", a));
    }
    let war_rows: Vec<Vec<String>> = w
        .war_table
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let mut cells = vec![w
                .war_sale_labels
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("Sale {}", i))];
            cells.extend(row.iter().map(|v| v.to_string()));
            cells
        })
        .collect();

    // Prediction
    let lib_rows: Vec<Vec<String>> = w
        .pred_lib_parts
        .iter()
        .map(|p| {
            vec![
                p.name.clone(),
                p.quantity.to_string(),
                format!(
                    "{:.3} per Mh",
                    p.base_rate_per_million_hours * p.quantity as f64 * p.stress_factor * p.quality_factor
                ),
            ]
        })
        .collect();
    let category_text = match &w.pred217_json {
        Some(cat) => {
            let name = cat.get("category").and_then(|v| v.as_str()).unwrap_or(&w.pred217_file);
            let section = cat.get("section").and_then(|v| v.as_str()).unwrap_or("");
            if section.is_empty() {
                name.to_string()
            } else {
                format!("{} (Section {})", name, section)
            }
        }
        None => {
            if w.pred217_result.is_empty() {
                "No category loaded yet.".to_string()
            } else {
                w.pred217_result.clone()
            }
        }
    };
    let p217_fields: Vec<Value> = pred217_specs(w)
        .iter()
        .map(|(named, key, label, options)| {
            let value = pred217_value(w, *named, key);
            json!({
                "id": "p217",
                "key": key,
                "label": label,
                "named": named,
                "options": options,
                "value": value,
                "sel": options.iter().position(|o| *o == value).map(|i| i as i64).unwrap_or(0),
            })
        })
        .collect();
    let part_options = w.pred217_part_options();
    let part = w.pred217_named.get("part").cloned().unwrap_or_default();
    let part_index = part_options.iter().position(|p| *p == part).map(|i| i as i64).unwrap_or(0);
    let env = w.pred217_named.get("environment").cloned().unwrap_or_else(|| "GB".to_string());

    // Markov
    let trs: Vec<Vec<String>> = w.mk_transitions.iter().map(|t| vec![t[0].clone(), t[1].clone(), t[2].clone()]).collect();
    let (tr_from, tr_to, tr_rate) = match w.mk_sel.and_then(|i| w.mk_transitions.get(i)) {
        Some(t) => (t[0].clone(), t[1].clone(), t[2].clone()),
        None => (String::new(), String::new(), String::new()),
    };

    // QCP
    let qcp_specs = reliakit::distributions::dist_spec(&w.qcp_dist).unwrap_or_default();
    let qcp_params_json: Vec<Value> = qcp_specs
        .iter()
        .map(|(name, _kind)| {
            json!({
                "id": "qcp",
                "key": name,
                "label": name,
                "named": false,
                "options": [],
                "value": w.qcp_params.get(*name).cloned().unwrap_or_else(|| Work::qcp_default(&w.qcp_dist, name).to_string()),
                "sel": 0,
            })
        })
        .collect();
    let qcp_headline = rows_of(&w.qcp_result)
        .into_iter()
        .find(|(l, _)| l.starts_with("Reliability"))
        .map(|(_, v)| v)
        .unwrap_or_default();

    // RCM
    let rcm_rows_json: Vec<Vec<String>> = w
        .rcm_rows
        .iter()
        .map(|r| {
            let mode = r.to_mode();
            vec![
                r.asset.clone(),
                r.function.clone(),
                r.mode.clone(),
                r.consequence.clone(),
                reliakit::rcm::select_task(&mode).as_str().to_string(),
                mode.criticality().map(|v| format!("{:.4}", v)).unwrap_or_default(),
            ]
        })
        .collect();
    let rcm_sel_row = w.rcm_sel.and_then(|i| w.rcm_rows.get(i));
    let (rcm_task, rcm_crit) = match rcm_sel_row {
        Some(r) => {
            let mode = r.to_mode();
            (
                reliakit::rcm::select_task(&mode).as_str().to_string(),
                mode.criticality().map(|v| format!("{:.4}", v)).unwrap_or_default(),
            )
        }
        None => (String::new(), String::new()),
    };

    // FRACAS
    let fracas_rows_json: Vec<Vec<String>> = w
        .fracas_rows
        .iter()
        .map(|inc| {
            vec![
                inc.id.clone(),
                inc.part.clone(),
                inc.failure_mode.clone(),
                format!("{:.0}", inc.hours),
                inc.date.clone(),
                inc.status.as_str().to_string(),
                inc.corrective_action.clone(),
            ]
        })
        .collect();
    let fracas_sel_inc = w.fracas_sel.and_then(|i| w.fracas_rows.get(i));

    json!({
        "status": w.status,
        "headline": w.headline,
        "screen": screen,
        "chart": chart,
        "series": series_arr,
        "project": project_to_value(&w.project_name, &w.items, w.selected),
        "project_json": serde_json::to_string_pretty(&project_to_value(&w.project_name, &w.items, w.selected)).unwrap_or_default(),
        "AppState": {
            "screen": screen,
            "project-name": w.project_name,
            "project-path": w.project_path,
            "status": w.status,
            "headline": w.headline,
            "items": items,
            "selected-index": w.selected.map(|i| i as i64).unwrap_or(-1),
            "current-title": current_title(w),
            "kind-index": KIND_DEFS.iter().position(|(_, l, _)| *l == w.new_kind).map(|i| i as i64).unwrap_or(0),
            "new-name": w.new_name,
            "rename-name": w.rename_name,
            "example-rows": example_rows,
            "dataset-rows": dataset_rows,
            "about-open": false,
            "item-count": w.items.len(),
            "kinds": KIND_DEFS.iter().map(|(_, l, k)| json!({"label": l, "key": k})).collect::<Vec<Value>>(),
        },
        "Actions": {},
        "CsvUi": {
            "files": str_json(&BUNDLED_CSV),
            "pick-index": idx_of(&BUNDLED_CSV, &w.csv_pick),
            "path": w.csv_path,
        },
        "LifeUi": {
            "times": w.life_times,
            "dist-index": idx_of(&ALL_DISTS, &w.life_dist),
            "dist": w.life_dist,
            "dist-options": str_json(&ALL_DISTS),
            "rank": w.life_rank,
            "t": w.life_t,
            "show-prob": w.life_show_prob,
            "results": rows_json(&w.life_result),
            "has-chart": if current_kind(w) == Some(ItemKind::LifeData) { has_chart.clone() } else { json!(false) },
            "chart-title": if current_kind(w) == Some(ItemKind::LifeData) { chart_title.clone() } else { json!(life_chart_title) },
            "series": if current_kind(w) == Some(ItemKind::LifeData) { series_arr.clone() } else { json!([]) },
            "headline-value": life_headline,
            "wizard-rows": wizard_rows,
            "wizard-note": w.wizard_note,
            "has-fit": w.life_fit.is_some(),
            "rbd-targets": rbd_targets,
            "target-index": target_pos,
            "block-names": w.flow_target_block_names(),
            "block-index": w.flow_block_idx,
        },
        "LimitedUi": {
            "groups": w.limited_groups,
            "results": rows_json(&w.limited_result),
        },
        "McmcUi": {
            "times": w.mcmc_times,
            "dist-index": idx_of(&MCMC_DISTS, &w.mcmc_dist),
            "dist": w.mcmc_dist,
            "dist-options": str_json(&MCMC_DISTS),
            "draws": w.mcmc_draws,
            "warmup": w.mcmc_warmup,
            "chains": w.mcmc_chains,
            "seed": w.mcmc_seed,
            "t": w.mcmc_t,
            "results": rows_json(&w.mcmc_result),
            "headline-value": if w.mcmc_result.is_empty() { String::new() } else { w.headline.clone() },
        },
        "DegUi": {
            "units": w.deg_units,
            "threshold": w.deg_threshold,
            "model-index": idx_of(&DEG_MODELS, &w.deg_model),
            "model": w.deg_model,
            "model-options": str_json(&DEG_MODELS),
            "dist-index": idx_of(&ALL_DISTS, &w.deg_dist),
            "dist": w.deg_dist,
            "dist-options": str_json(&ALL_DISTS),
            "increasing": w.deg_increasing,
            "results": rows_json(&w.deg_result),
            "has-chart": if current_kind(w) == Some(ItemKind::Degradation) { has_chart.clone() } else { json!(false) },
            "series": if current_kind(w) == Some(ItemKind::Degradation) { series_arr.clone() } else { json!([]) },
        },
        "AltUi": {
            "data": w.alt_data,
            "model-index": idx_of(&ALT_MODELS, &w.alt_model),
            "model": w.alt_model,
            "model-options": str_json(&ALT_MODELS),
            "use-stress": w.alt_use_stress,
            "t": w.alt_t,
            "results": rows_json(&w.alt_result),
            "has-chart": if current_kind(w) == Some(ItemKind::Alt) { has_chart.clone() } else { json!(false) },
            "series": if current_kind(w) == Some(ItemKind::Alt) { series_arr.clone() } else { json!([]) },
        },
        "GrowthUi": {
            "times": w.growth_times,
            "results": rows_json(&w.growth_result),
            "has-chart": if current_kind(w) == Some(ItemKind::Growth) { has_chart.clone() } else { json!(false) },
            "series": if current_kind(w) == Some(ItemKind::Growth) { series_arr.clone() } else { json!([]) },
        },
        "CompUi": {
            "data": w.comp_data,
            "dist-index": idx_of(&ALL_DISTS, &w.comp_dist),
            "dist": w.comp_dist,
            "dist-options": str_json(&ALL_DISTS),
            "t": w.comp_t,
            "results": rows_json(&w.comp_result),
        },
        "RbdUi": {
            "t": w.rbd_t,
            "horizon": w.rbd_horizon,
            "topology-index": idx_of(&TOPOLOGIES, &w.rbd_topology),
            "topology": w.rbd_topology,
            "topology-options": str_json(&TOPOLOGIES),
            "k": w.rbd_k,
            "rel-label": format!("Reliability at t = {}", w.rbd_t),
            "rel-value": w.rbd_reliability().map(|r| format!("{:.4}", r)).unwrap_or_default(),
            "avail-results": rows_json(&w.rbd_avail),
            "armed": w.rbd_armed.clone().unwrap_or_default(),
            "blocks": rbd_geoms_json(w),
            "segments": segments_json(w),
            "spec": w.rbd_spec,
            "has-selection": w.rbd_sel.is_some(),
            "sel-title": sel_title,
            "ed-name": w.ed_name,
            "ed-dist-index": idx_of(&ALL_DISTS, &w.ed_dist),
            "ed-dist": w.ed_dist,
            "ed-params": w.ed_params,
            "ed-mtbf": w.ed_mtbf,
            "ed-mttr": w.ed_mttr,
            "ed-units": w.ed_units,
            "ed-dormancy": w.ed_dormancy,
            "ed-switch": w.ed_switch,
            "ed-is-standby": ed_is_standby,
            "nb-name": w.nb_name,
            "nb-dist-index": idx_of(&ALL_DISTS, &w.nb_dist),
            "nb-dist": w.nb_dist,
            "nb-params": w.nb_params,
            "nb-mtbf": w.nb_mtbf,
            "nb-mttr": w.nb_mttr,
            "nb-mode-index": idx_of(&NB_MODES, &w.nb_mode),
            "nb-mode": w.nb_mode,
            "nb-mode-options": str_json(&NB_MODES),
            "nb-dormancy": w.nb_dormancy,
            "json": w.rbd_json,
            "path": w.rbd_path,
            "status": w.rbd_status,
            "importance-rows": cells_json(&imp_rows),
            "importance-note": w.rbd_importance_note,
        },
        "FtUi": {
            "json": w.ft_json,
            "results": rows_json(&w.ft_result),
        },
        "FmeaUi": {
            "type-index": idx_of(&FMEA_TYPES, &w.fmea_type),
            "type": w.fmea_type,
            "type-options": str_json(&FMEA_TYPES),
            "fm-item": w.fm_item,
            "fm-mode": w.fm_mode,
            "fm-sev": w.fm_sev,
            "fm-occ": w.fm_occ,
            "fm-det": w.fm_det,
            "fm-alpha": w.fm_alpha,
            "fm-beta": w.fm_beta,
            "fm-lambda": w.fm_lambda,
            "fm-hours": w.fm_hours,
            "rows": cells_json(&fmea_rows),
            "dvpr-rows": cells_json(&dvpr_rows),
            "dv-id": w.dv_id,
            "dv-ref": w.dv_ref,
            "dv-test": w.dv_test,
            "dv-sample": w.dv_sample,
            "dv-criteria": w.dv_criteria,
            "csv": w.fmea_csv,
            "export-path": w.fmea_export_path,
            "rcm-row": w.fmea_rcm_row,
            "status": w.fmea_status,
        },
        "WarUi": {
            "sales": w.war_sales,
            "returns": w.war_returns,
            "current": w.war_current,
            "horizon": w.war_horizon,
            "results": rows_json(&w.war_result),
            "table-header": war_header,
            "table-rows": cells_json(&war_rows),
        },
        "PredUi": {
            "source-index": idx_of(&PRED_SOURCES, &w.pred_source),
            "source": w.pred_source,
            "source-options": str_json(&PRED_SOURCES),
            "life-dist-index": idx_of(&ALL_DISTS, &w.pred_life_dist),
            "life-dist": w.pred_life_dist,
            "life-params": w.pred_life_params,
            "life-t": w.pred_life_t,
            "life-result": w.pred_life_result,
            "parts": w.pred_parts,
            "t": w.pred_t,
            "parts-results": rows_json(&w.pred_result),
            "lib-path": w.pred_lib_path,
            "lib-note": w.pred_lib_note,
            "lib-rows": cells_json(&lib_rows),
            "lib-results": pairs_json(&w.pred_lib_rows()),
            "p217-file-index": idx_of(&MIL217_FILES, &w.pred217_file),
            "p217-file": w.pred217_file,
            "p217-files": str_json(&MIL217_FILES),
            "p217-category": category_text,
            "p217-fields": p217_fields,
            "p217-misc": w.pred217_is_misc(),
            "p217-pth": w.pred217_is_pth(),
            "p217-part-options": part_options,
            "p217-part-index": part_index,
            "p217-env-index": idx_of(&ENVIRONMENTS, &env),
            "p217-env": env,
            "p217-env-options": str_json(&ENVIRONMENTS),
            "p217-results": rows_json(&w.pred217_result),
            "alloc-target": w.alloc_target,
            "alloc-n": w.alloc_n,
            "alloc-weights": w.alloc_weights,
            "alloc-results": rows_json(&w.alloc_result),
            "der-name": w.der_name,
            "der-operating": w.der_operating,
            "der-rated": w.der_rated,
            "der-max": w.der_max,
            "der-results": rows_json(&w.der_result),
            "sr-generic": w.sr_generic,
            "sr-pi-q": w.sr_pi_q,
            "sr-pi-s": w.sr_pi_s,
            "sr-ea": w.sr_ea,
            "sr-temp": w.sr_temp,
            "sr-ref-temp": w.sr_ref_temp,
            "sr-pi-e": w.sr_pi_e,
            "sr-qty-rate": w.sr_qty_rate,
            "sr-obs-rate": w.sr_obs_rate,
            "sr-pit-obs": w.sr_pit_obs,
            "sr-pit-tgt": w.sr_pit_tgt,
            "sr-pis-obs": w.sr_pis_obs,
            "sr-pis-tgt": w.sr_pis_tgt,
            "sr-ci-failures": w.sr_ci_failures,
            "sr-ci-exposure": w.sr_ci_exposure,
            "sr-ci-conf": w.sr_ci_conf,
            "sr-m3-pred": w.sr_m3_pred,
            "sr-m3-pw": w.sr_m3_pw,
            "sr-m3-obs": w.sr_m3_obs,
            "sr-m3-ow": w.sr_m3_ow,
            "mp-base": w.mp_base,
            "mp-ea": w.mp_ea,
            "mp-ref": w.mp_ref,
            "mp-cyc": w.mp_cyc_factor,
            "mp-phases": w.mp_phases,
            "clean-results": rows_json(&w.pred_clean_result),
        },
        "DoeUi": {
            "k": w.doe_k,
            "y": w.doe_y,
            "results": rows_json(&w.doe_result),
        },
        "FlowUi": {
            "json": w.flow_json,
            "sims": w.flow_sims,
            "seed": w.flow_seed,
            "results": rows_json(&w.flow_result),
        },
        "MarkovUi": {
            "states": strings_json(&w.mk_states),
            "new-state": w.mk_new_state,
            "transitions": cells_json(&trs),
            "sel-transition": w.mk_sel.map(|i| i as i64).unwrap_or(-1),
            "tr-from": tr_from,
            "tr-to": tr_to,
            "tr-rate": tr_rate,
            "start-index": w.mk_states.iter().position(|s| *s == w.mk_start).map(|i| i as i64).unwrap_or(0),
            "absorbing-index": w.mk_states.iter().position(|s| *s == w.mk_absorbing).map(|i| i as i64).unwrap_or(0),
            "time": w.mk_time,
            "results": rows_json(&w.mk_result),
            "has-chart": if current_kind(w) == Some(ItemKind::Markov) { has_chart.clone() } else { json!(false) },
            "series": if current_kind(w) == Some(ItemKind::Markov) { series_arr.clone() } else { json!([]) },
        },
        "QcpUi": {
            "dist-index": idx_of(&ALL_DISTS, &w.qcp_dist),
            "dist": w.qcp_dist,
            "dist-options": str_json(&ALL_DISTS),
            "params": qcp_params_json,
            "t": w.qcp_t,
            "bx": w.qcp_bx,
            "results": rows_json(&w.qcp_result),
            "headline-value": qcp_headline,
        },
        "RcmUi": {
            "rows": cells_json(&rcm_rows_json),
            "sel": w.rcm_sel.map(|i| i as i64).unwrap_or(-1),
            "ed-asset": rcm_sel_row.map(|r| r.asset.clone()).unwrap_or_default(),
            "ed-function": rcm_sel_row.map(|r| r.function.clone()).unwrap_or_default(),
            "ed-mode": rcm_sel_row.map(|r| r.mode.clone()).unwrap_or_default(),
            "ed-consequence-index": rcm_sel_row.map(|r| idx_of(&CONSEQUENCES, &r.consequence)).unwrap_or(3),
            "ed-consequence": rcm_sel_row.map(|r| r.consequence.clone()).unwrap_or_default(),
            "ed-consequence-options": str_json(&CONSEQUENCES),
            "ed-monitorable": rcm_sel_row.map(|r| r.monitorable).unwrap_or(false),
            "ed-age": rcm_sel_row.map(|r| r.age_related).unwrap_or(false),
            "ed-restorable": rcm_sel_row.map(|r| r.restorable).unwrap_or(false),
            "ed-rate": rcm_sel_row.map(|r| r.rate.clone()).unwrap_or_default(),
            "ed-alpha": rcm_sel_row.map(|r| r.alpha.clone()).unwrap_or_default(),
            "ed-beta": rcm_sel_row.map(|r| r.beta.clone()).unwrap_or_default(),
            "ed-hours": rcm_sel_row.map(|r| r.hours.clone()).unwrap_or_default(),
            "ed-task": rcm_task,
            "ed-crit": rcm_crit,
        },
        "FracasUi": {
            "rows": cells_json(&fracas_rows_json),
            "sel": w.fracas_sel.map(|i| i as i64).unwrap_or(-1),
            "ed-id": fracas_sel_inc.map(|i| i.id.clone()).unwrap_or_default(),
            "ed-part": fracas_sel_inc.map(|i| i.part.clone()).unwrap_or_default(),
            "ed-mode": fracas_sel_inc.map(|i| i.failure_mode.clone()).unwrap_or_default(),
            "ed-hours": fracas_sel_inc.map(|i| format!("{:.0}", i.hours)).unwrap_or_default(),
            "ed-date": fracas_sel_inc.map(|i| i.date.clone()).unwrap_or_default(),
            "ed-status-index": fracas_sel_inc.map(|i| match i.status {
                IncidentStatus::Open => 0,
                IncidentStatus::Analysis => 1,
                IncidentStatus::CorrectiveAction => 2,
                IncidentStatus::Closed => 3,
            }).unwrap_or(0),
            "ed-status": fracas_sel_inc.map(|i| i.status.as_str().to_string()).unwrap_or_default(),
            "ed-status-options": str_json(&FRACAS_STATUSES),
            "ed-action": fracas_sel_inc.map(|i| i.corrective_action.clone()).unwrap_or_default(),
            "total-hours": w.fracas_total_hours,
            "summary": pairs_json(&w.fracas_summary_rows()),
            "path": w.fracas_path,
            "status": w.fracas_status,
        },
    })
}

// ---------------- entry points ----------------

/// A Work in the state the desktop app starts in: the default
/// project with one Life Data item, showing the start page.
pub fn new_work() -> Work {
    let mut w = Work::new();
    w.init_project();
    w.select_index(None);
    w
}

/// Run one request against the work state and return the full UI
/// state. The request is `{"action": string, "fields": {...}}`;
/// top-level request keys other than action and fields are also
/// read as fields, with entries in `fields` winning.
pub fn handle(w: &mut Work, req: &Value) -> Value {
    let action = req
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("state")
        .to_string();
    let mut eff = Map::new();
    if let Some(obj) = req.as_object() {
        for (k, v) in obj {
            if k != "action" && k != "fields" {
                eff.insert(k.clone(), v.clone());
            }
        }
    }
    if let Some(f) = req.get("fields").and_then(|x| x.as_object()) {
        for (k, v) in f {
            eff.insert(k.clone(), v.clone());
        }
    }
    // Key spelling aliases: for a dotted or hyphenated key, also
    // register the underscore form and the bare suffix after the
    // first dot, so "pred.p217_file", "pred_p217_file", and
    // "p217_file" all reach the same pull entry. Existing keys win.
    let keys: Vec<String> = eff.keys().cloned().collect();
    for k in keys {
        let v = eff.get(&k).cloned().unwrap_or(Value::Null);
        let underscored = k.replace('.', "_").replace('-', "_");
        eff.entry(underscored).or_insert_with(|| v.clone());
        if let Some((_, suffix)) = k.split_once('.') {
            eff.entry(suffix.to_string()).or_insert_with(|| v.clone());
        }
    }
    pull_current(w, &eff);
    // Dynamic per-parameter fields travel as map entries of their
    // own (qcp:<name> and p217:<key>), as in dispatch_field.
    let dyn_fields: Vec<(String, String)> = eff
        .iter()
        .filter(|(k, _)| k.starts_with("qcp:") || k.starts_with("p217:"))
        .map(|(k, v)| {
            let s = match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            (k.clone(), s)
        })
        .collect();
    for (k, v) in &dyn_fields {
        dispatch_field(w, k, v);
    }
    normalize(w);
    dispatch(w, &action, &eff);
    w.capture_selected();
    let mut state = build_state(w);
    add_aliases(&mut state);
    state
}

/// Recursively rewrite object keys from kebab-case to snake_case.
fn snake_keys(v: &Value) -> Value {
    match v {
        Value::Object(m) => Value::Object(
            m.iter()
                .map(|(k, val)| (k.replace('-', "_"), snake_keys(val)))
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(snake_keys).collect()),
        other => other.clone(),
    }
}

/// Additive response aliases for hosts that read snake_case group
/// objects (app, csv, life, deg, pred, and friends) or flat
/// top-level keys instead of the Slint global names. The canonical
/// objects stay untouched; these are derived copies.
fn add_aliases(state: &mut Value) {
    let groups: [(&str, &str); 26] = [
        ("AppState", "app"),
        ("CsvUi", "csv"),
        ("LifeUi", "life"),
        ("LimitedUi", "limited"),
        ("McmcUi", "mcmc"),
        ("DegUi", "deg"),
        ("DegUi", "degradation"),
        ("AltUi", "alt"),
        ("GrowthUi", "growth"),
        ("CompUi", "comp"),
        ("CompUi", "competing"),
        ("RbdUi", "rbd"),
        ("FtUi", "ft"),
        ("FtUi", "faulttree"),
        ("FmeaUi", "fmea"),
        ("WarUi", "war"),
        ("WarUi", "warranty"),
        ("PredUi", "pred"),
        ("PredUi", "prediction"),
        ("DoeUi", "doe"),
        ("FlowUi", "flow"),
        ("FlowUi", "flowchart"),
        ("MarkovUi", "markov"),
        ("QcpUi", "qcp"),
        ("RcmUi", "rcm"),
        ("FracasUi", "fracas"),
    ];
    let Some(obj) = state.as_object_mut() else { return };
    for (global, alias) in groups {
        if let Some(g) = obj.get(global).cloned() {
            obj.insert(alias.to_string(), snake_keys(&g));
        }
    }
    if let Some(app) = obj.get("AppState").cloned() {
        let app = snake_keys(&app);
        for key in [
            "items",
            "selected_index",
            "project_name",
            "project_path",
            "current_title",
            "kind_index",
            "new_name",
            "rename_name",
            "item_count",
            "example_rows",
            "dataset_rows",
        ] {
            if let Some(v) = app.get(key).cloned() {
                obj.entry(key.to_string()).or_insert(v);
            }
        }
    }
    if let Some(csv) = obj.get("CsvUi").cloned() {
        if let Some(files) = csv.get("files").cloned() {
            obj.entry("csv_files".to_string()).or_insert(files);
        }
    }
}

/// String form of `handle`, for hosts that pass JSON text across
/// a boundary (the Tauri command and the WASM exports). A request
/// that is not valid JSON returns an error state object instead
/// of panicking.
pub fn handle_str(w: &mut Work, req: &str) -> String {
    match serde_json::from_str::<Value>(req) {
        Ok(v) => handle(w, &v).to_string(),
        Err(e) => json!({
            "status": format!("Request JSON error: {}", e),
            "headline": "",
            "screen": "start",
            "error": format!("{}", e),
        })
        .to_string(),
    }
}
