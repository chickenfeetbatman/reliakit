//! ReliaKit desktop application (egui / eframe).
//!
//! Pages call the same library functions as the `reliakit` CLI.
//! Headless mode: if argv contains --headless or --selftest, no
//! window is opened; the built-in Example 3 diagram is evaluated and
//! a short check line is printed instead.

use eframe::egui;
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use reliakit::alt as alt_mod;
use reliakit::bayes;
use reliakit::competing;
use reliakit::datasets;
use reliakit::degradation;
use reliakit::distributions as dist_mod;
use reliakit::doe;
use reliakit::faulttree::{self, FtNode};
use reliakit::flowchart;
use reliakit::fmea::{self, FmeaRow};
use reliakit::growth;
use reliakit::mcmc;
use reliakit::prediction::{self, PartEntry};
use reliakit::system;
use reliakit::warranty::{self, WarrantyRecord};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const EXAMPLE3_JSON: &str = r#"{"type":"series","children":[{"type":"block","name":"power supply","dist":"weibull2","params":{"beta":1.4,"eta":8000},"mtbf":6000,"mttr":24},{"type":"standby_group","name":"pumps","n_units":2,"dist":"exponential","params":{"theta":5000},"mtbf":5000,"mttr":8,"dormancy":0.0,"switch_reliability":0.98,"swap_time":0.5},{"type":"parallel","children":[{"type":"block","name":"controller A","dist":"exponential","params":{"theta":12000},"mtbf":12000,"mttr":4},{"type":"block","name":"controller B","dist":"exponential","params":{"theta":12000},"mtbf":12000,"mttr":4}]}]}"#;

fn headless_check() -> i32 {
    let spec: Value = match serde_json::from_str(EXAMPLE3_JSON) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("example 3 parse error: {}", e);
            return 1;
        }
    };
    let spec = match system::rbd_from_json(spec) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("example 3 validation error: {}", e);
            return 1;
        }
    };
    match system::evaluate_rbd(&spec, Some(1000.0)) {
        Ok(r) => println!("Example 3 RBD reliability at t=1000: {:.6}", r),
        Err(e) => {
            eprintln!("evaluate_rbd error: {}", e);
            return 1;
        }
    }
    let avail = system::simulate_availability(&spec, 20000.0, 40, 3, 50);
    println!("Example 3 mean availability (horizon 20000, 40 sims, seed 3): {:.6}", avail.mean_availability);
    println!("Example 3 expected downtime: {:.3}", avail.expected_downtime);
    println!("reliakit-app headless check OK");
    0
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--headless" || a == "--selftest") {
        std::process::exit(headless_check());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1240.0, 820.0])
            .with_title("ReliaKit"),
        ..Default::default()
    };
    eframe::run_native(
        "ReliaKit",
        options,
        Box::new(|_cc| Ok(Box::new(RelKitApp::new()))),
    )
}

// ---------------- shared parsing helpers ----------------

fn parse_numbers(text: &str) -> Vec<f64> {
    text.split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|s| !s.trim().is_empty())
        .filter_map(|s| s.trim().parse::<f64>().ok())
        .collect()
}

/// Life data text: one value per token; a trailing 'c' or '+' marks a
/// censored (suspended) unit.
fn parse_life(text: &str) -> (Vec<f64>, Vec<bool>) {
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

fn format_life(times: &[f64], failed: &[bool]) -> String {
    times
        .iter()
        .zip(failed)
        .map(|(t, f)| if *f { format!("{:.0}", t) } else { format!("{:.0}c", t) })
        .collect::<Vec<String>>()
        .join(", ")
}

fn parse_params_text(text: &str) -> dist_mod::Params {
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

fn params_to_json(p: &dist_mod::Params) -> Value {
    let mut m = serde_json::Map::new();
    for (k, v) in p {
        m.insert(k.clone(), json!(v));
    }
    Value::Object(m)
}

fn fnum(s: &str, default: f64) -> f64 {
    s.trim().parse::<f64>().unwrap_or(default)
}

fn inum(s: &str, default: usize) -> usize {
    s.trim().parse::<usize>().unwrap_or(default)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    LifeData,
    LimitedData,
    Mcmc,
    Degradation,
    Alt,
    Growth,
    Competing,
    SystemRbd,
    FaultTree,
    Fmea,
    Warranty,
    Prediction,
    Doe,
    Flowchart,
    Examples,
}

const PAGES: [(Page, &str); 15] = [
    (Page::LifeData, "Life Data"),
    (Page::LimitedData, "Limited Data"),
    (Page::Mcmc, "MCMC"),
    (Page::Degradation, "Degradation"),
    (Page::Alt, "ALT"),
    (Page::Growth, "Growth"),
    (Page::Competing, "Competing Risks"),
    (Page::SystemRbd, "System RBD"),
    (Page::FaultTree, "Fault Tree"),
    (Page::Fmea, "FMEA"),
    (Page::Warranty, "Warranty"),
    (Page::Prediction, "Prediction"),
    (Page::Doe, "DOE"),
    (Page::Flowchart, "Flowchart"),
    (Page::Examples, "Examples"),
];

const DISTS: [&str; 7] = ["weibull2", "weibull3", "lognormal", "normal", "exponential", "gamma", "gumbel"];

struct RelKitApp {
    page: Page,
    // Life Data
    life_times: String,
    life_dist: String,
    life_rank: bool,
    life_t: String,
    life_result: String,
    life_fit: Option<dist_mod::FitResult>,
    // Limited Data
    limited_groups: String,
    limited_result: String,
    // MCMC
    mcmc_times: String,
    mcmc_dist: String,
    mcmc_draws: String,
    mcmc_warmup: String,
    mcmc_chains: String,
    mcmc_seed: String,
    mcmc_t: String,
    mcmc_result: String,
    // Degradation
    deg_units: String,
    deg_threshold: String,
    deg_model: String,
    deg_dist: String,
    deg_increasing: bool,
    deg_result: String,
    // ALT
    alt_data: String,
    alt_model: String,
    alt_use_stress: String,
    alt_t: String,
    alt_result: String,
    // Growth
    growth_times: String,
    growth_result: String,
    // Competing
    comp_data: String,
    comp_dist: String,
    comp_t: String,
    comp_result: String,
    // System RBD
    rbd_json: String,
    rbd_spec: Value,
    rbd_positions: Vec<(String, Vec2)>,
    rbd_t: String,
    rbd_horizon: String,
    rbd_topology: String,
    rbd_k: String,
    rbd_avail: String,
    rbd_path: String,
    rbd_status: String,
    nb_name: String,
    nb_dist: String,
    nb_params: String,
    nb_mtbf: String,
    nb_mttr: String,
    nb_mode: String,
    nb_dormancy: String,
    // Fault tree
    ft_json: String,
    ft_result: String,
    // FMEA
    fmea_rows: Vec<FmeaRow>,
    fmea_csv: String,
    fm_item: String,
    fm_mode: String,
    fm_sev: String,
    fm_occ: String,
    fm_det: String,
    fmea_status: String,
    // Warranty
    war_sales: String,
    war_returns: String,
    war_current: String,
    war_horizon: String,
    war_result: String,
    // Prediction
    pred_parts: String,
    pred_t: String,
    pred_result: String,
    // DOE
    doe_k: String,
    doe_y: String,
    doe_result: String,
    // Flowchart
    flow_json: String,
    flow_sims: String,
    flow_seed: String,
    flow_result: String,
}

impl RelKitApp {
    fn new() -> Self {
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
                    t.iter().map(|v| format!("{:.0}", v)).collect::<Vec<String>>().join(","),
                    y.iter().map(|v| format!("{:.2}", v)).collect::<Vec<String>>().join(",")
                )
            })
            .collect::<Vec<String>>()
            .join("\n");
        let (alt_stress, alt_times) = datasets::alt_dataset(2026);
        let alt_text = alt_stress
            .iter()
            .zip(&alt_times)
            .map(|(s, t)| format!("{:.0},{:.1}", s[0], t))
            .collect::<Vec<String>>()
            .join("\n");
        let growth_text = datasets::growth_dataset(2026)
            .iter()
            .map(|v| format!("{:.1}", v))
            .collect::<Vec<String>>()
            .join(", ");
        let spec: Value = serde_json::from_str(EXAMPLE3_JSON).unwrap_or(json!({"type": "series", "children": []}));
        let mut app = RelKitApp {
            page: Page::LifeData,
            life_times: life_text.clone(),
            life_dist: "weibull2".into(),
            life_rank: false,
            life_t: "1000".into(),
            life_result: String::new(),
            life_fit: None,
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
            alt_data: alt_text,
            alt_model: "arrhenius".into(),
            alt_use_stress: "55".into(),
            alt_t: "1000".into(),
            alt_result: String::new(),
            growth_times: growth_text,
            growth_result: String::new(),
            comp_data: "120,wear\n340,electrical\n560,wear\n800,\n950,electrical\n1300,wear\n1700,\n2200,electrical".into(),
            comp_dist: "weibull2".into(),
            comp_t: "1000".into(),
            comp_result: String::new(),
            rbd_json: serde_json::to_string_pretty(&spec).unwrap_or_default(),
            rbd_spec: spec,
            rbd_positions: Vec::new(),
            rbd_t: "1000".into(),
            rbd_horizon: "20000".into(),
            rbd_topology: "series".into(),
            rbd_k: "2".into(),
            rbd_avail: String::new(),
            rbd_path: "rbd.json".into(),
            rbd_status: String::new(),
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
            fmea_csv: String::new(),
            fm_item: String::new(),
            fm_mode: String::new(),
            fm_sev: "5".into(),
            fm_occ: "5".into(),
            fm_det: "5".into(),
            fmea_status: String::new(),
            war_sales: "120, 150, 140, 160".into(),
            war_returns: "0,1\n0,2\n1,2\n1,3\n2,3\n0,3\n2,4\n1,4".into(),
            war_current: "3".into(),
            war_horizon: "6".into(),
            war_result: String::new(),
            pred_parts: "Resistor, 2.5, 10, 1.0, 1.0\nCapacitor, 1.2, 6, 1.1, 1.0\nConnector, 0.8, 4, 1.0, 1.2".into(),
            pred_t: "1000".into(),
            pred_result: String::new(),
            doe_k: "3".into(),
            doe_y: "10, 12, 11, 15, 13, 16, 14, 18".into(),
            doe_result: String::new(),
            flow_json: "{\n  \"nodes\": [\n    {\"id\": \"s\", \"type\": \"start\", \"next\": \"a\"},\n    {\"id\": \"a\", \"type\": \"task\", \"name\": \"inspect\", \"time\": {\"dist\": \"fixed\", \"value\": 2.0}, \"cost\": 3.0, \"next\": \"b\"},\n    {\"id\": \"b\", \"type\": \"task\", \"name\": \"assemble\", \"time\": {\"dist\": \"fixed\", \"value\": 5.0}, \"next\": \"e\"},\n    {\"id\": \"e\", \"type\": \"end\"}\n  ]\n}".into(),
            flow_sims: "5000".into(),
            flow_seed: "1".into(),
            flow_result: String::new(),
        };
        app.load_fmea_example();
        app.sync_rbd_positions();
        app
    }

    fn load_fmea_example(&mut self) {
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
    }

    // ---------------- RBD helpers ----------------

    fn rbd_leaves(spec: &Value, out: &mut Vec<(String, String)>) {
        let ty = spec.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match ty {
            "block" | "standby_group" => {
                let name = spec
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("block")
                    .to_string();
                let kind = if ty == "standby_group" { "standby" } else { "block" }.to_string();
                out.push((name, kind));
            }
            _ => {
                if let Some(ch) = spec.get("children").and_then(|v| v.as_array()) {
                    for c in ch {
                        Self::rbd_leaves(c, out);
                    }
                }
            }
        }
    }

    fn sync_rbd_positions(&mut self) {
        let mut leaves = Vec::new();
        Self::rbd_leaves(&self.rbd_spec, &mut leaves);
        let mut next: Vec<(String, Vec2)> = Vec::new();
        for (i, (name, _kind)) in leaves.iter().enumerate() {
            let key = format!("{}|{}", i, name);
            let pos = self
                .rbd_positions
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, p)| *p)
                .unwrap_or(Vec2::new(30.0 + (i as f32) * 150.0, 90.0));
            next.push((key, pos));
        }
        self.rbd_positions = next;
    }

    fn set_rbd_spec(&mut self, spec: Value) {
        self.rbd_spec = spec;
        self.rbd_json = serde_json::to_string_pretty(&self.rbd_spec).unwrap_or_default();
        self.sync_rbd_positions();
        self.rbd_avail = String::new();
    }

    fn rbd_reliability(&self) -> Option<f64> {
        let t = self.rbd_t.trim().parse::<f64>().ok();
        system::evaluate_rbd(&self.rbd_spec, t).ok()
    }
}

impl eframe::App for RelKitApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("nav")
            .resizable(false)
            .default_width(170.0)
            .show(ctx, |ui| {
                ui.heading("ReliaKit");
                ui.separator();
                for (p, label) in PAGES {
                    if ui.selectable_label(self.page == p, label).clicked() {
                        self.page = p;
                    }
                }
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.page {
                    Page::LifeData => self.page_life(ui),
                    Page::LimitedData => self.page_limited(ui),
                    Page::Mcmc => self.page_mcmc(ui),
                    Page::Degradation => self.page_degradation(ui),
                    Page::Alt => self.page_alt(ui),
                    Page::Growth => self.page_growth(ui),
                    Page::Competing => self.page_competing(ui),
                    Page::SystemRbd => self.page_rbd(ui),
                    Page::FaultTree => self.page_fault_tree(ui),
                    Page::Fmea => self.page_fmea(ui),
                    Page::Warranty => self.page_warranty(ui),
                    Page::Prediction => self.page_prediction(ui),
                    Page::Doe => self.page_doe(ui),
                    Page::Flowchart => self.page_flowchart(ui),
                    Page::Examples => self.page_examples(ui),
                }
            });
        });
    }
}

fn result_box(ui: &mut egui::Ui, text: &str) {
    if !text.is_empty() {
        ui.separator();
        ui.label(egui::RichText::new(text).monospace().size(12.0));
    }
}

fn dist_combo(ui: &mut egui::Ui, id: &str, current: &mut String) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(current.clone())
        .show_ui(ui, |ui| {
            for d in DISTS {
                ui.selectable_value(current, d.to_string(), d);
            }
        });
}

impl RelKitApp {
    fn page_life(&mut self, ui: &mut egui::Ui) {
        ui.heading("Life Data");
        ui.label("Times (comma separated; suffix c or + = censored):");
        ui.add(egui::TextEdit::multiline(&mut self.life_times).desired_rows(4).desired_width(f32::INFINITY));
        ui.horizontal(|ui| {
            ui.label("Distribution:");
            dist_combo(ui, "life_dist", &mut self.life_dist);
            ui.checkbox(&mut self.life_rank, "Rank regression");
            ui.label("Mission t:");
            ui.add(egui::TextEdit::singleline(&mut self.life_t).desired_width(80.0));
        });
        if ui.button("Run fit").clicked() {
            let (times, failed) = parse_life(&self.life_times);
            let res = if self.life_rank {
                dist_mod::fit_rank_regression(&self.life_dist, &times, Some(&failed))
            } else {
                dist_mod::fit_mle(&self.life_dist, &times, Some(&failed))
            };
            match res {
                Ok(fit) => {
                    let t = fnum(&self.life_t, 1000.0);
                    let (lo, hi) = fit.reliability_bounds(t, 0.05);
                    self.life_result = format!(
                        "{}\nR({:.0}) = {:.4}  (95% bounds {:.4} to {:.4})\nLower bound (one-sided 95%): {:.4}\nB10 = {:.1}, mean life = {:.1}",
                        fit.summary(),
                        t,
                        fit.reliability(t),
                        lo,
                        hi,
                        fit.reliability_lower_bound(t, 0.05),
                        fit.b_life(0.10),
                        fit.mean_life()
                    );
                    self.life_fit = Some(fit);
                }
                Err(e) => {
                    self.life_result = format!("error: {}", e);
                    self.life_fit = None;
                }
            }
        }
        result_box(ui, &self.life_result.clone());
        if let Some(fit) = self.life_fit.clone() {
            ui.separator();
            ui.label("Reliability curve:");
            let size = Vec2::new(520.0, 220.0);
            let (resp, painter) = ui.allocate_painter(size, Sense::hover());
            let rect = resp.rect;
            painter.rect_filled(rect, 0.0, Color32::from_rgb(24, 24, 28));
            painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::GRAY));
            let tmax = fit.times.iter().cloned().fold(1.0f64, f64::max) * 1.15;
            let to_xy = |t: f64, r: f64| -> Pos2 {
                Pos2::new(
                    rect.left() + 30.0 + (rect.width() - 40.0) * (t / tmax) as f32,
                    rect.bottom() - 20.0 - (rect.height() - 30.0) * r as f32,
                )
            };
            // axes ticks
            for i in 0..=4 {
                let r = i as f64 / 4.0;
                let a = to_xy(0.0, r);
                let b = to_xy(tmax, r);
                painter.line_segment([a, b], Stroke::new(0.5, Color32::from_rgb(70, 70, 80)));
                painter.text(Pos2::new(rect.left() + 15.0, a.y), Align2::CENTER_CENTER, format!("{:.2}", r), FontId::proportional(10.0), Color32::LIGHT_GRAY);
            }
            let mut pts: Vec<Pos2> = Vec::new();
            for i in 0..=80 {
                let t = tmax * i as f64 / 80.0;
                pts.push(to_xy(t, fit.reliability(t)));
            }
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], Stroke::new(2.0, Color32::from_rgb(90, 200, 120)));
            }
            painter.text(Pos2::new(rect.center().x, rect.bottom() - 8.0), Align2::CENTER_CENTER, format!("time (0 to {:.0})", tmax), FontId::proportional(10.0), Color32::LIGHT_GRAY);
        }
    }

    fn page_limited(&mut self, ui: &mut egui::Ui) {
        ui.heading("Limited Data (hierarchical shrinkage)");
        ui.label("One lot per line, times comma separated (c = censored):");
        ui.add(egui::TextEdit::multiline(&mut self.limited_groups).desired_rows(5).desired_width(f32::INFINITY));
        if ui.button("Run hierarchical fit").clicked() {
            let groups: Vec<(Vec<f64>, Vec<bool>)> = self
                .limited_groups
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| parse_life(l))
                .collect();
            match bayes::hierarchical_weibull(&groups) {
                Ok(h) => {
                    let mut s = format!(
                        "Pooled beta = {:.3}\nmu log eta = {:.3}, tau2 = {:.4}\n",
                        h.pooled_beta, h.mu_log_eta, h.tau2
                    );
                    for (i, f) in h.fits.iter().enumerate() {
                        s += &format!(
                            "Lot {}: shrunk eta = {:.1} (MLE eta = {:.1}), shrink B = {:.2}\n",
                            i + 1,
                            f.params["eta"],
                            h.group_fits[i].params["eta"],
                            h.b[i]
                        );
                    }
                    if let Some(first) = h.group_fits.first() {
                        if let Ok(bc) = bayes::bias_corrected_weibull(first) {
                            s += &format!(
                                "\nBias correction, lot 1: beta {:.3} -> {:.3}, eta {:.1} -> {:.1} (factor {:.3})\n",
                                first.params["beta"],
                                bc.params["beta"],
                                first.params["eta"],
                                bc.params["eta"],
                                bayes::weibull_shape_bias_factor(first.n_fail)
                            );
                        }
                        let calib: Vec<f64> = groups.iter().flat_map(|(t, _)| t.iter().cloned()).collect();
                        s += &format!(
                            "Conformal mission time (alpha 0.10): {:.1}\nConformal R lower bound at t=1000 (alpha 0.05): {:.3}",
                            bayes::conformal_mission_time(first, &calib, 0.10),
                            bayes::conformal_reliability_lower(&calib, 1000.0, 0.05)
                        );
                    }
                    self.limited_result = s;
                }
                Err(e) => self.limited_result = format!("error: {}", e),
            }
        }
        result_box(ui, &self.limited_result.clone());
    }

    fn page_mcmc(&mut self, ui: &mut egui::Ui) {
        ui.heading("Bayesian MCMC");
        ui.label("Times (c = censored):");
        ui.add(egui::TextEdit::multiline(&mut self.mcmc_times).desired_rows(3).desired_width(f32::INFINITY));
        ui.horizontal(|ui| {
            ui.label("Model:");
            egui::ComboBox::from_id_salt("mcmc_dist")
                .selected_text(self.mcmc_dist.clone())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.mcmc_dist, "weibull2".to_string(), "weibull2");
                    ui.selectable_value(&mut self.mcmc_dist, "lognormal".to_string(), "lognormal");
                });
            ui.label("Draws:");
            ui.add(egui::TextEdit::singleline(&mut self.mcmc_draws).desired_width(55.0));
            ui.label("Warmup:");
            ui.add(egui::TextEdit::singleline(&mut self.mcmc_warmup).desired_width(55.0));
            ui.label("Chains:");
            ui.add(egui::TextEdit::singleline(&mut self.mcmc_chains).desired_width(35.0));
            ui.label("Seed:");
            ui.add(egui::TextEdit::singleline(&mut self.mcmc_seed).desired_width(55.0));
            ui.label("t:");
            ui.add(egui::TextEdit::singleline(&mut self.mcmc_t).desired_width(70.0));
        });
        if ui.button("Run MCMC").clicked() {
            let (times, failed) = parse_life(&self.mcmc_times);
            let res = mcmc::fit_bayes(
                &self.mcmc_dist,
                &times,
                Some(&failed),
                inum(&self.mcmc_draws, 300),
                inum(&self.mcmc_warmup, 300),
                inum(&self.mcmc_chains, 2),
                self.mcmc_seed.trim().parse::<u64>().unwrap_or(42),
            );
            match res {
                Ok(r) => {
                    let mut s = String::new();
                    for (name, ps) in r.summary() {
                        s += &format!("{}: mean {:.3}, sd {:.3}, q05 {:.3}, median {:.3}, q95 {:.3}\n", name, ps.mean, ps.sd, ps.q05, ps.q50, ps.q95);
                    }
                    s += &format!("R-hat: {:?}\n", r.rhat().iter().map(|v| format!("{:.3}", v)).collect::<Vec<String>>());
                    s += &format!("ESS: {:?}\n", r.ess_values().iter().map(|v| format!("{:.0}", v)).collect::<Vec<String>>());
                    let t = fnum(&self.mcmc_t, 1000.0);
                    let ci = r.reliability_credible(t, 0.90, 0);
                    s += &format!("R({:.0}) credible: mean {:.3}, 90% CI {:.3} to {:.3}\n", t, ci.mean, ci.lower, ci.upper);
                    s += &format!("Posterior predictive p-value: {:.3}", r.posterior_predictive_pvalue(7));
                    self.mcmc_result = s;
                }
                Err(e) => self.mcmc_result = format!("error: {}", e),
            }
        }
        result_box(ui, &self.mcmc_result.clone());
    }

    fn page_degradation(&mut self, ui: &mut egui::Ui) {
        ui.heading("Degradation");
        ui.label("One unit per line: times | measurements");
        ui.add(egui::TextEdit::multiline(&mut self.deg_units).desired_rows(5).desired_width(f32::INFINITY));
        ui.horizontal(|ui| {
            ui.label("Threshold:");
            ui.add(egui::TextEdit::singleline(&mut self.deg_threshold).desired_width(60.0));
            ui.label("Path model:");
            egui::ComboBox::from_id_salt("deg_model")
                .selected_text(self.deg_model.clone())
                .show_ui(ui, |ui| {
                    for m in ["linear", "exponential", "power"] {
                        ui.selectable_value(&mut self.deg_model, m.to_string(), m);
                    }
                });
            ui.label("Life dist:");
            dist_combo(ui, "deg_dist", &mut self.deg_dist);
            ui.checkbox(&mut self.deg_increasing, "Increasing failure direction");
        });
        if ui.button("Run degradation analysis").clicked() {
            let mut units: Vec<(Vec<f64>, Vec<f64>)> = Vec::new();
            for line in self.deg_units.lines() {
                if let Some((a, b)) = line.split_once('|') {
                    units.push((parse_numbers(a), parse_numbers(b)));
                }
            }
            match degradation::degradation_analysis(
                &units,
                fnum(&self.deg_threshold, 70.0),
                &self.deg_model,
                &self.deg_dist,
                self.deg_increasing,
            ) {
                Ok(r) => {
                    let mut s = format!("Pseudo-failure times: {:?}\n", r.pseudo_times.iter().map(|v| format!("{:.0}", v)).collect::<Vec<String>>());
                    s += &format!("Failed flags: {:?}\n", r.failed);
                    if let Some(f) = &r.fit {
                        s += &format!("Life fit:\n{}", f.summary());
                    }
                    self.deg_result = s;
                }
                Err(e) => self.deg_result = format!("error: {}", e),
            }
        }
        result_box(ui, &self.deg_result.clone());
    }

    fn page_alt(&mut self, ui: &mut egui::Ui) {
        ui.heading("Accelerated Life Test");
        ui.label("One unit per line: stress,time (eyring: stress,stress2,time). Optional trailing ,1/,0 failed flag defaults to failed.");
        ui.add(egui::TextEdit::multiline(&mut self.alt_data).desired_rows(5).desired_width(f32::INFINITY));
        ui.horizontal(|ui| {
            ui.label("Model:");
            egui::ComboBox::from_id_salt("alt_model")
                .selected_text(self.alt_model.clone())
                .show_ui(ui, |ui| {
                    for m in ["arrhenius", "ipl", "eyring"] {
                        ui.selectable_value(&mut self.alt_model, m.to_string(), m);
                    }
                });
            ui.label("Use stress:");
            ui.add(egui::TextEdit::singleline(&mut self.alt_use_stress).desired_width(90.0));
            ui.label("t:");
            ui.add(egui::TextEdit::singleline(&mut self.alt_t).desired_width(70.0));
        });
        if ui.button("Run ALT fit").clicked() {
            let mut stress: Vec<Vec<f64>> = Vec::new();
            let mut times: Vec<f64> = Vec::new();
            let mut failed: Vec<bool> = Vec::new();
            for line in self.alt_data.lines() {
                let vals = parse_numbers(line);
                if vals.len() >= 2 {
                    if self.alt_model == "eyring" && vals.len() >= 3 {
                        stress.push(vec![vals[0], vals[1]]);
                        times.push(vals[2]);
                    } else {
                        stress.push(vec![vals[0]]);
                        times.push(vals[1]);
                    }
                    failed.push(true);
                }
            }
            let use_s = parse_numbers(&self.alt_use_stress);
            let t = fnum(&self.alt_t, 1000.0);
            let mut s = String::new();
            match alt_mod::fit_alt(&self.alt_model, &stress, &times) {
                Ok(f) => {
                    s += &format!(
                        "Least squares: coef {:?}, sigma {:.3}\nMedian life at use stress: {:.1}, R({:.0}) = {:.4}\n",
                        f.coef,
                        f.sigma,
                        f.median_life(&use_s),
                        t,
                        f.reliability(t, &use_s)
                    );
                    if let Some(ea) = f.activation_energy_ev() {
                        s += &format!("Activation energy: {:.3} eV\n", ea);
                    }
                }
                Err(e) => s += &format!("least squares error: {}\n", e),
            }
            match alt_mod::fit_alt_mle(&self.alt_model, &stress, &times, Some(&failed)) {
                Ok(f) => {
                    let (lo, hi) = f.reliability_bounds(t, &use_s, 0.05);
                    s += &format!(
                        "MLE: coef {:?}, sigma {:.3}, loglik {:.2}\nR({:.0}) at use stress = {:.4} (bounds {:.4} to {:.4})",
                        f.coef,
                        f.sigma,
                        f.loglik,
                        t,
                        f.reliability(t, &use_s),
                        lo,
                        hi
                    );
                    if let Some(ea) = f.activation_energy_ev() {
                        s += &format!("\nMLE activation energy: {:.3} eV", ea);
                    }
                }
                Err(e) => s += &format!("MLE error: {}", e),
            }
            self.alt_result = s;
        }
        result_box(ui, &self.alt_result.clone());
    }

    fn page_growth(&mut self, ui: &mut egui::Ui) {
        ui.heading("Reliability Growth");
        ui.label("Failure times (comma separated):");
        ui.add(egui::TextEdit::multiline(&mut self.growth_times).desired_rows(3).desired_width(f32::INFINITY));
        if ui.button("Run growth fit").clicked() {
            let times = parse_numbers(&self.growth_times);
            let ca = growth::fit_crow_amsaa(&times, None);
            let mut s = format!(
                "Crow-AMSAA: beta {:.3}, lambda {:.5}, n {}\nInstantaneous MTBF at T={:.0}: {:.1}\nCumulative MTBF: {:.1}\n",
                ca.beta,
                ca.lam,
                ca.n_fail,
                ca.t_end,
                ca.instantaneous_mtbf(ca.t_end),
                ca.cumulative_mtbf(ca.t_end)
            );
            match growth::fit_duane(&times) {
                Ok(du) => {
                    s += &format!(
                        "Duane: alpha {:.3}, instantaneous MTBF at T: {:.1}",
                        du.alpha,
                        du.instantaneous_mtbf_at(ca.t_end)
                    );
                }
                Err(e) => s += &format!("Duane error: {}", e),
            }
            self.growth_result = s;
        }
        result_box(ui, &self.growth_result.clone());
    }

    fn page_competing(&mut self, ui: &mut egui::Ui) {
        ui.heading("Competing Risks");
        ui.label("One unit per line: time,mode (blank mode = censored):");
        ui.add(egui::TextEdit::multiline(&mut self.comp_data).desired_rows(5).desired_width(f32::INFINITY));
        ui.horizontal(|ui| {
            ui.label("Distribution:");
            dist_combo(ui, "comp_dist", &mut self.comp_dist);
            ui.label("Mission t:");
            ui.add(egui::TextEdit::singleline(&mut self.comp_t).desired_width(80.0));
        });
        if ui.button("Run competing risks fit").clicked() {
            let mut times: Vec<f64> = Vec::new();
            let mut mode_strings: Vec<Option<String>> = Vec::new();
            for line in self.comp_data.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Some((a, b)) = line.split_once(',') {
                    if let Ok(t) = a.trim().parse::<f64>() {
                        times.push(t);
                        let m = b.trim().to_string();
                        mode_strings.push(if m.is_empty() { None } else { Some(m) });
                    }
                }
            }
            let events: Vec<Option<&str>> = mode_strings.iter().map(|o| o.as_deref()).collect();
            match competing::fit_competing_risks(&times, &events, &self.comp_dist) {
                Ok(cr) => {
                    let t = fnum(&self.comp_t, 1000.0);
                    let mut modes: Vec<String> = Vec::new();
                    for m in mode_strings.iter().flatten() {
                        if !modes.contains(m) {
                            modes.push(m.clone());
                        }
                    }
                    modes.sort();
                    let mut s = String::from("Per-mode fits:\n");
                    for (name, fit) in &cr.modes {
                        s += &format!("[{}]\n{}\n", name, fit.summary());
                    }
                    s += &format!("Overall reliability at {:.0}: {:.4}\n", t, competing::overall_reliability(&cr, t));
                    for m in &modes {
                        s += &format!("CIF[{}] at {:.0}: {:.4}\n", m, t, competing::cumulative_incidence(&cr, m, t));
                    }
                    s += &format!("Failure probabilities at {:.0}: {}", t, serde_json::to_string(&competing::failure_probabilities(&cr, t)).unwrap_or_default());
                    self.comp_result = s;
                }
                Err(e) => self.comp_result = format!("error: {}", e),
            }
        }
        result_box(ui, &self.comp_result.clone());
    }

    fn page_rbd(&mut self, ui: &mut egui::Ui) {
        ui.heading("System RBD");
        ui.label("Drag blocks on the canvas. Blocks connect in series order, left to right.");
        // Canvas with draggable blocks
        let canvas_size = Vec2::new(760.0, 240.0);
        let (canvas_resp, _) = ui.allocate_painter(canvas_size, Sense::hover());
        let rect = canvas_resp.rect;
        let mut leaves = Vec::new();
        Self::rbd_leaves(&self.rbd_spec, &mut leaves);
        let block_size = Vec2::new(128.0, 52.0);
        // interactions first
        let mut new_positions = self.rbd_positions.clone();
        for (i, (key, pos)) in new_positions.iter_mut().enumerate() {
            let _ = key;
            let brect = Rect::from_min_size(rect.min + *pos, block_size);
            let resp = ui.interact(brect, egui::Id::new(format!("rbd_block_{}", i)), Sense::drag());
            if resp.dragged() {
                *pos += resp.drag_delta();
                pos.x = pos.x.clamp(0.0, (rect.width() - block_size.x).max(0.0));
                pos.y = pos.y.clamp(0.0, (rect.height() - block_size.y).max(0.0));
            }
        }
        if new_positions != self.rbd_positions {
            self.rbd_positions = new_positions.clone();
        }
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(24, 24, 28));
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::GRAY));
        // connecting lines in series order (sorted by x)
        let mut order: Vec<usize> = (0..leaves.len()).collect();
        order.sort_by(|&a, &b| {
            let pa = self.rbd_positions.get(a).map(|(_, p)| p.x).unwrap_or(0.0);
            let pb = self.rbd_positions.get(b).map(|(_, p)| p.x).unwrap_or(0.0);
            pa.partial_cmp(&pb).unwrap_or(std::cmp::Ordering::Equal)
        });
        for w in order.windows(2) {
            if let (Some((_, pa)), Some((_, pb))) = (self.rbd_positions.get(w[0]), self.rbd_positions.get(w[1])) {
                let a = Pos2::new(rect.left() + pa.x + block_size.x, rect.top() + pa.y + block_size.y / 2.0);
                let b = Pos2::new(rect.left() + pb.x, rect.top() + pb.y + block_size.y / 2.0);
                painter.line_segment([a, b], Stroke::new(1.5, Color32::LIGHT_BLUE));
            }
        }
        for (i, (name, kind)) in leaves.iter().enumerate() {
            if let Some((_, pos)) = self.rbd_positions.get(i) {
                let brect = Rect::from_min_size(rect.min + *pos, block_size);
                let fill = if kind == "standby" { Color32::from_rgb(70, 55, 20) } else { Color32::from_rgb(30, 60, 40) };
                painter.rect_filled(brect, 4.0, fill);
                painter.rect_stroke(brect, 4.0, Stroke::new(1.0, Color32::WHITE));
                painter.text(brect.center_top() + Vec2::new(0.0, 10.0), Align2::CENTER_CENTER, name, FontId::proportional(12.0), Color32::WHITE);
                painter.text(brect.center_top() + Vec2::new(0.0, 30.0), Align2::CENTER_CENTER, kind, FontId::proportional(10.0), Color32::LIGHT_GRAY);
            }
        }
        // Controls
        ui.horizontal(|ui| {
            ui.label("Mission t:");
            ui.add(egui::TextEdit::singleline(&mut self.rbd_t).desired_width(80.0));
            ui.label("Topology:");
            egui::ComboBox::from_id_salt("rbd_topology")
                .selected_text(self.rbd_topology.clone())
                .show_ui(ui, |ui| {
                    for t in ["series", "parallel", "k-of-n", "series with standby group"] {
                        ui.selectable_value(&mut self.rbd_topology, t.to_string(), t);
                    }
                });
            ui.label("k:");
            ui.add(egui::TextEdit::singleline(&mut self.rbd_k).desired_width(35.0));
            if ui.button("Apply topology").clicked() {
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
                }
            }
        });
        let rel = self.rbd_reliability();
        ui.label(match rel {
            Some(r) => format!("Reliability at t={}: {:.4}", self.rbd_t, r),
            None => "Reliability: (needs a valid diagram and mission time)".to_string(),
        });
        ui.horizontal(|ui| {
            ui.label("Availability horizon:");
            ui.add(egui::TextEdit::singleline(&mut self.rbd_horizon).desired_width(80.0));
            if ui.button("Run availability (40 sims)").clicked() {
                let horizon = fnum(&self.rbd_horizon, 20000.0);
                let res = system::simulate_availability(&self.rbd_spec, horizon, 40, 3, 20);
                self.rbd_avail = format!(
                    "Mean availability {:.4}, expected downtime {:.1}, mean failures {:.2}, mean repairs {:.2}",
                    res.mean_availability, res.expected_downtime, res.mean_failures, res.mean_repairs
                );
            }
        });
        if !self.rbd_avail.is_empty() {
            ui.label(self.rbd_avail.clone());
        }
        ui.separator();
        ui.label("Add block:");
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.add(egui::TextEdit::singleline(&mut self.nb_name).desired_width(110.0));
            ui.label("Dist:");
            dist_combo(ui, "nb_dist", &mut self.nb_dist);
            ui.label("Params:");
            ui.add(egui::TextEdit::singleline(&mut self.nb_params).desired_width(170.0));
        });
        ui.horizontal(|ui| {
            ui.label("MTBF:");
            ui.add(egui::TextEdit::singleline(&mut self.nb_mtbf).desired_width(70.0));
            ui.label("MTTR:");
            ui.add(egui::TextEdit::singleline(&mut self.nb_mttr).desired_width(60.0));
            ui.label("Mode:");
            egui::ComboBox::from_id_salt("nb_mode")
                .selected_text(self.nb_mode.clone())
                .show_ui(ui, |ui| {
                    for m in ["active", "hot standby", "cold standby", "warm"] {
                        ui.selectable_value(&mut self.nb_mode, m.to_string(), m);
                    }
                });
            ui.label("Dormancy (warm):");
            ui.add(egui::TextEdit::singleline(&mut self.nb_dormancy).desired_width(50.0));
            if ui.button("Add block").clicked() {
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
                let mut spec = self.rbd_spec.clone();
                if spec.get("type").and_then(|v| v.as_str()) == Some("series") {
                    if let Some(ch) = spec.get_mut("children").and_then(|v| v.as_array_mut()) {
                        ch.push(node);
                    }
                } else {
                    spec = json!({"type": "series", "children": [spec, node]});
                }
                self.set_rbd_spec(spec);
                self.rbd_status = "Block added.".into();
            }
        });
        ui.separator();
        ui.label("Diagram JSON (same schema as system::rbd_from_json):");
        ui.add(egui::TextEdit::multiline(&mut self.rbd_json).desired_rows(8).desired_width(f32::INFINITY).code_editor());
        ui.horizontal(|ui| {
            if ui.button("Load from JSON text").clicked() {
                match serde_json::from_str::<Value>(&self.rbd_json) {
                    Ok(v) => match system::rbd_from_json(v) {
                        Ok(spec) => {
                            self.set_rbd_spec(spec);
                            self.rbd_status = "Diagram loaded.".into();
                        }
                        Err(e) => self.rbd_status = format!("error: {}", e),
                    },
                    Err(e) => self.rbd_status = format!("JSON error: {}", e),
                }
            }
            ui.label("File path:");
            ui.add(egui::TextEdit::singleline(&mut self.rbd_path).desired_width(160.0));
            if ui.button("Save").clicked() {
                match std::fs::write(&self.rbd_path, &self.rbd_json) {
                    Ok(()) => self.rbd_status = format!("Saved to {}", self.rbd_path),
                    Err(e) => self.rbd_status = format!("save error: {}", e),
                }
            }
            if ui.button("Load").clicked() {
                match std::fs::read_to_string(&self.rbd_path) {
                    Ok(text) => match serde_json::from_str::<Value>(&text) {
                        Ok(v) => match system::rbd_from_json(v) {
                            Ok(spec) => {
                                self.set_rbd_spec(spec);
                                self.rbd_status = format!("Loaded {}", self.rbd_path);
                            }
                            Err(e) => self.rbd_status = format!("error: {}", e),
                        },
                        Err(e) => self.rbd_status = format!("JSON error: {}", e),
                    },
                    Err(e) => self.rbd_status = format!("load error: {}", e),
                }
            }
        });
        if !self.rbd_status.is_empty() {
            ui.label(self.rbd_status.clone());
        }
    }

    fn page_fault_tree(&mut self, ui: &mut egui::Ui) {
        ui.heading("Fault Tree");
        ui.label("FtNode JSON (serde format):");
        ui.add(egui::TextEdit::multiline(&mut self.ft_json).desired_rows(7).desired_width(f32::INFINITY).code_editor());
        if ui.button("Evaluate").clicked() {
            match serde_json::from_str::<FtNode>(&self.ft_json) {
                Ok(node) => {
                    let cuts = faulttree::minimal_cut_sets(&node);
                    let mut s = format!("Top probability: {:.6}\nMinimal cut sets ({}):\n", faulttree::top_probability(&node), cuts.len());
                    for c in &cuts {
                        s += &format!("  {{{}}}\n", c.iter().cloned().collect::<Vec<String>>().join(", "));
                    }
                    self.ft_result = s;
                }
                Err(e) => self.ft_result = format!("JSON error: {}", e),
            }
        }
        result_box(ui, &self.ft_result.clone());
    }

    fn page_fmea(&mut self, ui: &mut egui::Ui) {
        ui.heading("FMEA Worksheet");
        ui.horizontal(|ui| {
            ui.label("Item:");
            ui.add(egui::TextEdit::singleline(&mut self.fm_item).desired_width(130.0));
            ui.label("Failure mode:");
            ui.add(egui::TextEdit::singleline(&mut self.fm_mode).desired_width(150.0));
            ui.label("Sev:");
            ui.add(egui::TextEdit::singleline(&mut self.fm_sev).desired_width(35.0));
            ui.label("Occ:");
            ui.add(egui::TextEdit::singleline(&mut self.fm_occ).desired_width(35.0));
            ui.label("Det:");
            ui.add(egui::TextEdit::singleline(&mut self.fm_det).desired_width(35.0));
            if ui.button("Add row").clicked() {
                self.fmea_rows.push(FmeaRow::new(
                    &self.fm_item.clone(),
                    &self.fm_mode.clone(),
                    self.fm_sev.trim().parse::<u32>().unwrap_or(1),
                    self.fm_occ.trim().parse::<u32>().unwrap_or(1),
                    self.fm_det.trim().parse::<u32>().unwrap_or(1),
                ));
                self.fmea_status = "Row added.".into();
            }
        });
        let mut sorted = self.fmea_rows.clone();
        fmea::sort_by_rpn_desc(&mut sorted);
        let mut s = String::from("Item | Failure mode | Sev | Occ | Det | RPN | Revised RPN\n");
        for r in &sorted {
            s += &format!(
                "{} | {} | {} | {} | {} | {} | {}\n",
                r.item,
                r.failure_mode,
                r.severity,
                r.occurrence,
                r.detection,
                r.rpn(),
                r.revised_rpn().map(|v| v.to_string()).unwrap_or_else(|| "-".to_string())
            );
        }
        result_box(ui, &s);
        ui.separator();
        ui.label("CSV (FmeaRow format):");
        ui.add(egui::TextEdit::multiline(&mut self.fmea_csv).desired_rows(5).desired_width(f32::INFINITY).code_editor());
        ui.horizontal(|ui| {
            if ui.button("Export rows to CSV").clicked() {
                self.fmea_csv = fmea::rows_to_csv(&self.fmea_rows);
            }
            if ui.button("Import CSV").clicked() {
                match fmea::rows_from_csv(&self.fmea_csv) {
                    Ok(rows) => {
                        self.fmea_rows = rows;
                        self.fmea_status = "CSV imported.".into();
                    }
                    Err(e) => self.fmea_status = format!("error: {}", e),
                }
            }
            if ui.button("Clear rows").clicked() {
                self.fmea_rows.clear();
            }
        });
        if !self.fmea_status.is_empty() {
            ui.label(self.fmea_status.clone());
        }
    }

    fn page_warranty(&mut self, ui: &mut egui::Ui) {
        ui.heading("Warranty (Nevada table)");
        ui.label("Sales per period (comma separated):");
        ui.add(egui::TextEdit::singleline(&mut self.war_sales).desired_width(f32::INFINITY));
        ui.label("Returns, one per line: sale_period,return_period");
        ui.add(egui::TextEdit::multiline(&mut self.war_returns).desired_rows(4).desired_width(f32::INFINITY));
        ui.horizontal(|ui| {
            ui.label("Current period:");
            ui.add(egui::TextEdit::singleline(&mut self.war_current).desired_width(50.0));
            ui.label("Horizon periods:");
            ui.add(egui::TextEdit::singleline(&mut self.war_horizon).desired_width(50.0));
        });
        if ui.button("Run warranty analysis").clicked() {
            let sales: Vec<u32> = parse_numbers(&self.war_sales).iter().map(|v| *v as u32).collect();
            let mut records: Vec<WarrantyRecord> = Vec::new();
            for line in self.war_returns.lines() {
                let vals = parse_numbers(line);
                if vals.len() >= 2 {
                    records.push(WarrantyRecord { sale_period: vals[0] as usize, return_period: vals[1] as usize });
                }
            }
            let max_age = records.iter().filter(|r| r.return_period >= r.sale_period).map(|r| r.return_period - r.sale_period).max().unwrap_or(0);
            let table = warranty::nevada_table(sales.len(), &records, max_age);
            let mut s = format!("Nevada table (rows = sale period, cols = age):\n{:?}\n", table);
            s += &format!("Returns by age: {:?}\nTotal returns: {}\n", warranty::returns_by_age(&table), warranty::total_returns(&table));
            match warranty::forecast_returns(&sales, &records, inum(&self.war_current, 0), inum(&self.war_horizon, 6)) {
                Ok(f) => {
                    s += &format!(
                        "Forecast: Weibull beta {:.3}, eta {:.2}\nExpected returns over horizon: {:.2}\nUnits in service: {:.0}",
                        f.beta, f.eta, f.expected_returns, f.units_in_service
                    );
                }
                Err(e) => s += &format!("Forecast error: {}", e),
            }
            self.war_result = s;
        }
        result_box(ui, &self.war_result.clone());
    }

    fn page_prediction(&mut self, ui: &mut egui::Ui) {
        ui.heading("Parts-Count Prediction");
        ui.label("One part per line: name, base rate per million hours, quantity, stress factor, quality factor");
        ui.add(egui::TextEdit::multiline(&mut self.pred_parts).desired_rows(4).desired_width(f32::INFINITY));
        ui.horizontal(|ui| {
            ui.label("Mission t (hours):");
            ui.add(egui::TextEdit::singleline(&mut self.pred_t).desired_width(80.0));
        });
        if ui.button("Run prediction").clicked() {
            let mut parts: Vec<PartEntry> = Vec::new();
            for line in self.pred_parts.lines() {
                let cells: Vec<&str> = line.split(',').map(|c| c.trim()).collect();
                if cells.len() >= 3 {
                    if let Ok(rate) = cells[1].parse::<f64>() {
                        let mut p = PartEntry::new(cells[0], rate, cells[2].parse::<u32>().unwrap_or(1));
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
            let t = fnum(&self.pred_t, 1000.0);
            self.pred_result = format!(
                "System failure rate: {:.3} per million hours\nSystem MTBF: {:.0} hours\nReliability at {:.0} h: {:.4}",
                prediction::system_failure_rate_per_million_hours(&parts),
                prediction::system_mtbf_hours(&parts),
                t,
                prediction::system_reliability(&parts, t)
            );
        }
        result_box(ui, &self.pred_result.clone());
    }

    fn page_doe(&mut self, ui: &mut egui::Ui) {
        ui.heading("Factorial DOE (2^k)");
        ui.horizontal(|ui| {
            ui.label("Factors k:");
            ui.add(egui::TextEdit::singleline(&mut self.doe_k).desired_width(40.0));
            ui.label("Responses y (2^k values, Yates order):");
            ui.add(egui::TextEdit::singleline(&mut self.doe_y).desired_width(320.0));
        });
        if ui.button("Analyze").clicked() {
            let k = inum(&self.doe_k, 3);
            let y = parse_numbers(&self.doe_y);
            match doe::analyze_factorial(k, &y) {
                Ok(effects) => {
                    let mut s = format!("Design ({} runs): {:?}\nTerm | Effect | Sum of squares\n", doe::factorial_design(k).len(), doe::factorial_design(k));
                    for e in &effects {
                        s += &format!("{} | {:.3} | {:.3}\n", e.name, e.effect, e.sum_squares);
                    }
                    self.doe_result = s;
                }
                Err(e) => self.doe_result = format!("error: {}", e),
            }
        }
        result_box(ui, &self.doe_result.clone());
    }

    fn page_flowchart(&mut self, ui: &mut egui::Ui) {
        ui.heading("Flowchart Simulation");
        ui.label("Process network JSON:");
        ui.add(egui::TextEdit::multiline(&mut self.flow_json).desired_rows(7).desired_width(f32::INFINITY).code_editor());
        ui.horizontal(|ui| {
            ui.label("Simulations:");
            ui.add(egui::TextEdit::singleline(&mut self.flow_sims).desired_width(70.0));
            ui.label("Seed:");
            ui.add(egui::TextEdit::singleline(&mut self.flow_seed).desired_width(60.0));
        });
        if ui.button("Run simulation").clicked() {
            match serde_json::from_str::<Value>(&self.flow_json) {
                Ok(spec) => match flowchart::simulate_flow(&spec, inum(&self.flow_sims, 5000), self.flow_seed.trim().parse::<u64>().unwrap_or(1)) {
                    Ok(r) => {
                        self.flow_result = format!(
                            "Mean time: {:.3} (sd {:.3})\nP10 {:.3}, P50 {:.3}, P90 {:.3}\nMin {:.3}, max {:.3}, simulations {}\nMean cost: {:.3}",
                            r.mean_time,
                            r.sd_time,
                            r.p10,
                            r.p50,
                            r.p90,
                            r.min_time,
                            r.max_time,
                            r.n_sim,
                            r.mean_cost
                        );
                    }
                    Err(e) => self.flow_result = format!("error: {}", e),
                },
                Err(e) => self.flow_result = format!("JSON error: {}", e),
            }
        }
        result_box(ui, &self.flow_result.clone());
    }

    fn page_examples(&mut self, ui: &mut egui::Ui) {
        ui.heading("Examples");
        ui.label("Load a built-in synthetic example into its page:");
        if ui.button("Example 1: bearing life data").clicked() {
            let (t, f) = datasets::bearing_life(2026);
            self.life_times = format_life(&t, &f);
            self.mcmc_times = self.life_times.clone();
            self.life_fit = None;
            self.life_result = "Loaded Example 1 into Life Data.".into();
            self.page = Page::LifeData;
        }
        if ui.button("Example 2: limited data lots").clicked() {
            let groups = datasets::small_sample_groups(2026);
            self.limited_groups = groups.iter().map(|(t, f)| format_life(t, f)).collect::<Vec<String>>().join("\n");
            self.limited_result = "Loaded Example 2 into Limited Data.".into();
            self.page = Page::LimitedData;
        }
        if ui.button("Example 3: RBD with standby").clicked() {
            let spec: Value = serde_json::from_str(EXAMPLE3_JSON).unwrap_or(json!({}));
            self.set_rbd_spec(spec);
            self.rbd_status = "Loaded Example 3.".into();
            self.page = Page::SystemRbd;
        }
        if ui.button("Example 4: FMEA worksheet").clicked() {
            self.load_fmea_example();
            self.fmea_status = "Loaded Example 4 (3 sample rows).".into();
            self.page = Page::Fmea;
        }
        if ui.button("Example 5: ALT dataset").clicked() {
            let (stress, times) = datasets::alt_dataset(2026);
            self.alt_data = stress.iter().zip(&times).map(|(s, t)| format!("{:.0},{:.1}", s[0], t)).collect::<Vec<String>>().join("\n");
            self.alt_result = "Loaded ALT example.".into();
            self.page = Page::Alt;
        }
        if ui.button("Example 6: growth dataset").clicked() {
            self.growth_times = datasets::growth_dataset(2026).iter().map(|v| format!("{:.1}", v)).collect::<Vec<String>>().join(", ");
            self.growth_result = "Loaded growth example.".into();
            self.page = Page::Growth;
        }
        if ui.button("Example 7: degradation dataset").clicked() {
            let deg = datasets::degradation_units(2026);
            self.deg_units = deg
                .iter()
                .map(|(t, y)| {
                    format!(
                        "{} | {}",
                        t.iter().map(|v| format!("{:.0}", v)).collect::<Vec<String>>().join(","),
                        y.iter().map(|v| format!("{:.2}", v)).collect::<Vec<String>>().join(",")
                    )
                })
                .collect::<Vec<String>>()
                .join("\n");
            self.deg_result = "Loaded degradation example.".into();
            self.page = Page::Degradation;
        }
    }
}
