//! ReliaKit desktop application (egui / eframe).
//!
//! Pages call the same library functions as the `reliakit` CLI.
//! Headless mode: if argv contains --headless or --selftest, no
//! window is opened; the built-in Example 3 diagram is evaluated and
//! a short check line is printed instead.

#![recursion_limit = "256"]

use eframe::egui;
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use egui_extras::{Column, TableBuilder};
use egui_plot::{Line, Plot, PlotPoints, Points};
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
use reliakit::rcm::{self, Consequence, RcmMode};
use reliakit::standards217::{self, FactorInput};
use reliakit::system;
use reliakit::warranty::{self, WarrantyRecord};
use reliakit::wizard::{self, FitRank};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;

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
    // Project save and open round trip, at the state level through
    // the pure serialization functions the app uses.
    let sample_items = vec![
        ProjectItem { id: 1, kind: ItemKind::LifeData, name: "L1".into(), state: json!({"life_times": "10, 20"}) },
        ProjectItem { id: 2, kind: ItemKind::Rbd, name: "R1".into(), state: json!({"rbd_spec": spec.clone()}) },
    ];
    let pv = project_to_value("Check project", &sample_items, Some(0));
    match project_from_value(&pv) {
        Some((name, items, sel)) if name == "Check project" && items.len() == 2 && items[0].name == "L1" && items[0].state["life_times"] == json!("10, 20") && items[1].kind == ItemKind::Rbd && sel == Some(0) => {
            println!("Project round trip check OK");
        }
        _ => {
            eprintln!("project round trip check failed");
            return 1;
        }
    }
    // Flow (a): assign a fit to the first RBD block.
    let mut aparams = dist_mod::Params::new();
    aparams.insert("mu".to_string(), 8.0);
    aparams.insert("sigma".to_string(), 0.5);
    let mut rbd_state = json!({"rbd_spec": spec.clone()});
    match apply_fit_to_rbd_state(&mut rbd_state, 0, "lognormal", &aparams) {
        Ok(block_name) => {
            let new_spec = rbd_state["rbd_spec"].clone();
            let paths = RelKitApp::leaf_paths(&new_spec);
            let node = RelKitApp::node_at(&new_spec, &paths[0]).unwrap_or(json!({}));
            if node.get("dist").and_then(|v| v.as_str()) == Some("lognormal") {
                println!("Flow (a) assign fit check OK (block {})", block_name);
            } else {
                eprintln!("flow (a) check failed: block dist not updated");
                return 1;
            }
        }
        Err(e) => {
            eprintln!("flow (a) check failed: {}", e);
            return 1;
        }
    }
    // Flow (e): FRACAS incidents become life data failure times.
    let fracas_state = json!({"fracas_rows": [{"hours": 1200.0}, {"hours": 3400.0}]});
    let (life_text, life_n) = fracas_life_text(&fracas_state);
    if life_text == "1200, 3400" && life_n == 2 {
        println!("Flow (e) FRACAS export check OK");
    } else {
        eprintln!("flow (e) check failed: {} ({})", life_text, life_n);
        return 1;
    }
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
            .with_inner_size([1500.0, 1050.0])
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

fn params_to_text(v: &Value) -> String {
    match v.as_object() {
        Some(obj) => obj
            .iter()
            .filter_map(|(k, val)| val.as_f64().map(|x| format!("{}={}", k, x)))
            .collect::<Vec<String>>()
            .join(", "),
        None => String::new(),
    }
}

fn fnum(s: &str, default: f64) -> f64 {
    s.trim().parse::<f64>().unwrap_or(default)
}

fn inum(s: &str, default: usize) -> usize {
    s.trim().parse::<usize>().unwrap_or(default)
}

fn opt_num(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        t.parse::<f64>().ok()
    }
}

// ---------------- directories and bundled data ----------------

fn datasets_dir() -> PathBuf {
    let p = PathBuf::from("datasets");
    if p.is_dir() {
        return p;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("datasets")
}

fn mil217_dir() -> PathBuf {
    let p = PathBuf::from("libraries").join("mil217");
    if p.is_dir() {
        return p;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("libraries").join("mil217")
}

/// The 16 bundled dataset files documented in datasets/README.md.
const BUNDLED_CSV: [&str; 16] = [
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

const MIL217_FILES: [&str; 18] = [
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
const ALL_DISTS: [&str; 14] = [
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
fn csv_table(text: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let mut lines = text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'));
    let header: Vec<String> = match lines.next() {
        Some(h) => h.split(',').map(|c| c.trim().to_string()).collect(),
        None => return (Vec::new(), Vec::new()),
    };
    let rows: Vec<Vec<String>> = lines
        .map(|l| l.split(',').map(|c| c.trim().to_string()).collect())
        .collect();
    (header, rows)
}

fn col_idx(header: &[String], name: &str) -> Option<usize> {
    header.iter().position(|h| h.eq_ignore_ascii_case(name))
}


// ---------------- design system (see docs/DESIGN.md) ----------------
// HIG based: Clarity, Deference, Depth. One tint accent, neutral dark
// surfaces, semantic colors in the spirit of Apple system colors.
pub const COL_BG: Color32 = Color32::from_rgb(23, 24, 26);
pub const COL_SIDEBAR: Color32 = Color32::from_rgb(19, 20, 22);
pub const COL_CARD: Color32 = Color32::from_rgb(33, 35, 38);
pub const COL_BORDER: Color32 = Color32::from_rgb(58, 60, 64);
pub const COL_TEXT: Color32 = Color32::from_rgb(242, 242, 247);
pub const COL_MUTED: Color32 = Color32::from_rgb(167, 167, 173);
pub const COL_TINT: Color32 = Color32::from_rgb(10, 132, 255);
pub const COL_GREEN: Color32 = Color32::from_rgb(48, 209, 88);
pub const COL_RED: Color32 = Color32::from_rgb(255, 69, 58);
pub const COL_ORANGE: Color32 = Color32::from_rgb(255, 159, 10);
pub const COL_YELLOW: Color32 = Color32::from_rgb(255, 214, 10);
pub const COL_SERIES2: Color32 = Color32::from_rgb(100, 210, 255);
pub const COL_SERIES3: Color32 = Color32::from_rgb(191, 90, 242);
pub const COL_SERIES4: Color32 = Color32::from_rgb(255, 100, 130);
pub const SERIES_COLORS: [Color32; 5] = [COL_TINT, COL_SERIES2, COL_SERIES3, COL_SERIES4, COL_GREEN];

fn apply_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = COL_SIDEBAR;
    v.window_fill = COL_BG;
    v.extreme_bg_color = Color32::from_rgb(15, 15, 17);
    v.faint_bg_color = COL_CARD;
    v.override_text_color = Some(COL_TEXT);
    v.selection.bg_fill = COL_TINT;
    v.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    v.widgets.inactive.bg_fill = COL_CARD;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, COL_BORDER);
    v.widgets.hovered.bg_fill = Color32::from_rgb(48, 50, 55);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, COL_MUTED);
    v.widgets.active.bg_fill = COL_TINT;
    ctx.set_visuals(v);
    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(egui::TextStyle::Heading, FontId::proportional(24.0));
    style.text_styles.insert(egui::TextStyle::Body, FontId::proportional(14.0));
    style.text_styles.insert(egui::TextStyle::Button, FontId::proportional(14.0));
    style.text_styles.insert(egui::TextStyle::Small, FontId::proportional(12.0));
    style.text_styles.insert(egui::TextStyle::Monospace, FontId::monospace(13.0));
    style.spacing.button_padding = Vec2::new(10.0, 7.0);
    style.spacing.interact_size = Vec2::new(40.0, 28.0);
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    ctx.set_style(style);
}

/// Draw one original icon on an 18 px grid, 1.6 px stroke.
/// All icons are plain lines, rectangles, and circles.
fn draw_icon_painter(painter: &egui::Painter, r: Rect, name: &str, col: Color32) {
    let st = Stroke::new(1.6_f32, col);
    let (x, y, w, h) = (r.left(), r.top(), r.width(), r.height());
    let pt = |dx: f32, dy: f32| Pos2::new(x + w * dx, y + h * dy);
    let line = |painter: &egui::Painter, pts: Vec<Pos2>| {
        painter.add(egui::Shape::line(pts, st));
    };
    match name {
        "life" => line(painter, vec![pt(0.08, 0.15), pt(0.35, 0.2), pt(0.6, 0.45), pt(0.92, 0.85)]),
        "limited" => {
            for cx in [0.22, 0.5, 0.78] {
                painter.circle_stroke(pt(cx, 0.4), 2.2, st);
            }
            line(painter, vec![pt(0.1, 0.78), pt(0.9, 0.78)]);
        }
        "mcmc" => {
            painter.circle_stroke(pt(0.28, 0.5), 4.5, st);
            painter.circle_stroke(pt(0.55, 0.5), 4.5, st);
            painter.circle_stroke(pt(0.82, 0.5), 4.5, st);
        }
        "degradation" => {
            line(painter, vec![pt(0.08, 0.2), pt(0.92, 0.72)]);
            line(painter, vec![pt(0.08, 0.6), pt(0.35, 0.6), pt(0.5, 0.6), pt(0.65, 0.6), pt(0.92, 0.6)]);
        }
        "alt" => {
            line(painter, vec![pt(0.5, 0.08), pt(0.9, 0.85), pt(0.1, 0.85), pt(0.5, 0.08)]);
            line(painter, vec![pt(0.55, 0.35), pt(0.42, 0.6), pt(0.56, 0.6), pt(0.47, 0.8)]);
        }
        "growth" => line(painter, vec![pt(0.08, 0.88), pt(0.4, 0.6), pt(0.65, 0.35), pt(0.92, 0.12)]),
        "competing" => {
            line(painter, vec![pt(0.08, 0.5), pt(0.5, 0.5)]);
            line(painter, vec![pt(0.5, 0.5), pt(0.92, 0.18)]);
            line(painter, vec![pt(0.5, 0.5), pt(0.92, 0.82)]);
        }
        "rbd" => {
            painter.rect_stroke(Rect::from_min_size(pt(0.05, 0.32), Vec2::new(w * 0.32, h * 0.36)), 1.0, st);
            painter.rect_stroke(Rect::from_min_size(pt(0.63, 0.32), Vec2::new(w * 0.32, h * 0.36)), 1.0, st);
            line(painter, vec![pt(0.37, 0.5), pt(0.63, 0.5)]);
        }
        "faulttree" => {
            painter.circle_stroke(pt(0.5, 0.18), 3.0, st);
            line(painter, vec![pt(0.5, 0.3), pt(0.5, 0.45), pt(0.22, 0.45), pt(0.22, 0.68)]);
            line(painter, vec![pt(0.5, 0.45), pt(0.78, 0.45), pt(0.78, 0.68)]);
            painter.circle_stroke(pt(0.22, 0.8), 3.0, st);
            painter.circle_stroke(pt(0.78, 0.8), 3.0, st);
        }
        "fmea" => {
            painter.rect_stroke(Rect::from_min_size(pt(0.08, 0.12), Vec2::new(w * 0.84, h * 0.76)), 1.0, st);
            line(painter, vec![pt(0.36, 0.12), pt(0.36, 0.88)]);
            line(painter, vec![pt(0.64, 0.12), pt(0.64, 0.88)]);
            line(painter, vec![pt(0.08, 0.5), pt(0.92, 0.5)]);
        }
        "warranty" => {
            line(painter, vec![pt(0.85, 0.3), pt(0.5, 0.3), pt(0.3, 0.55), pt(0.45, 0.82)]);
            line(painter, vec![pt(0.85, 0.3), pt(0.7, 0.14)]);
            line(painter, vec![pt(0.85, 0.3), pt(0.68, 0.44)]);
        }
        "prediction" => {
            let pts: Vec<Pos2> = (0..=12).map(|i| {
                let a = std::f32::consts::PI * (0.85 + 1.3 * i as f32 / 12.0);
                Pos2::new(x + w * 0.5 + (w * 0.4) * a.cos(), y + h * 0.55 + (h * 0.4) * a.sin())
            }).collect();
            line(painter, pts);
            line(painter, vec![pt(0.5, 0.55), pt(0.72, 0.3)]);
        }
        "doe" => {
            for (cx, cy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
                painter.circle_filled(pt(cx, cy), 2.4, col);
            }
            painter.rect_stroke(Rect::from_min_size(pt(0.12, 0.12), Vec2::new(w * 0.76, h * 0.76)), 1.0, st);
        }
        "flowchart" => {
            painter.rect_stroke(Rect::from_min_size(pt(0.08, 0.1), Vec2::new(w * 0.4, h * 0.3)), 1.0, st);
            line(painter, vec![pt(0.28, 0.4), pt(0.28, 0.55), pt(0.55, 0.55)]);
            line(painter, vec![pt(0.72, 0.38), pt(0.9, 0.55), pt(0.72, 0.72), pt(0.54, 0.55), pt(0.72, 0.38)]);
        }
        "markov" => {
            painter.circle_stroke(pt(0.28, 0.5), 4.5, st);
            painter.circle_stroke(pt(0.74, 0.5), 4.5, st);
            line(painter, vec![pt(0.4, 0.3), pt(0.62, 0.3)]);
            line(painter, vec![pt(0.62, 0.7), pt(0.4, 0.7)]);
        }
        "qcp" => {
            painter.rect_stroke(Rect::from_min_size(pt(0.2, 0.08), Vec2::new(w * 0.6, h * 0.84)), 1.0, st);
            line(painter, vec![pt(0.3, 0.3), pt(0.7, 0.3)]);
            for (cx, cy) in [(0.36, 0.52), (0.55, 0.52), (0.36, 0.72), (0.55, 0.72)] {
                painter.circle_filled(pt(cx, cy), 1.8, col);
            }
        }
        "rcm" => {
            painter.circle_stroke(pt(0.5, 0.5), 4.0, st);
            for i in 0..6 {
                let a = i as f32 * std::f32::consts::PI / 3.0;
                line(painter, vec![
                    Pos2::new(x + w * 0.5 + (w * 0.22) * a.cos(), y + h * 0.5 + (h * 0.22) * a.sin()),
                    Pos2::new(x + w * 0.5 + (w * 0.4) * a.cos(), y + h * 0.5 + (h * 0.4) * a.sin()),
                ]);
            }
        }
        "fracas" => {
            line(painter, vec![pt(0.25, 0.08), pt(0.25, 0.92)]);
            line(painter, vec![pt(0.25, 0.12), pt(0.85, 0.22), pt(0.25, 0.38)]);
        }
        "plus" => {
            line(painter, vec![pt(0.5, 0.15), pt(0.5, 0.85)]);
            line(painter, vec![pt(0.15, 0.5), pt(0.85, 0.5)]);
        }
        "save" => {
            line(painter, vec![pt(0.5, 0.12), pt(0.5, 0.62)]);
            line(painter, vec![pt(0.3, 0.45), pt(0.5, 0.65), pt(0.7, 0.45)]);
            line(painter, vec![pt(0.12, 0.82), pt(0.88, 0.82)]);
        }
        "open" => {
            line(painter, vec![pt(0.5, 0.62), pt(0.5, 0.12)]);
            line(painter, vec![pt(0.3, 0.3), pt(0.5, 0.1), pt(0.7, 0.3)]);
            line(painter, vec![pt(0.12, 0.82), pt(0.88, 0.82)]);
        }
        "delete" => {
            painter.rect_stroke(Rect::from_min_size(pt(0.14, 0.14), Vec2::new(w * 0.72, h * 0.72)), 1.0, st);
            line(painter, vec![pt(0.32, 0.32), pt(0.68, 0.68)]);
            line(painter, vec![pt(0.68, 0.32), pt(0.32, 0.68)]);
        }
        "duplicate" => {
            painter.rect_stroke(Rect::from_min_size(pt(0.3, 0.3), Vec2::new(w * 0.55, h * 0.55)), 1.0, st);
            painter.rect_stroke(Rect::from_min_size(pt(0.14, 0.14), Vec2::new(w * 0.55, h * 0.55)), 1.0, st);
        }
        "run" => line(painter, vec![pt(0.3, 0.15), pt(0.8, 0.5), pt(0.3, 0.85), pt(0.3, 0.15)]),
        "export" => {
            painter.rect_stroke(Rect::from_min_size(pt(0.1, 0.25), Vec2::new(w * 0.55, h * 0.55)), 1.0, st);
            line(painter, vec![pt(0.5, 0.5), pt(0.88, 0.5)]);
            line(painter, vec![pt(0.7, 0.32), pt(0.9, 0.5), pt(0.7, 0.68)]);
        }
        "link" => {
            painter.circle_stroke(pt(0.38, 0.5), 5.0, st);
            painter.circle_stroke(pt(0.64, 0.5), 5.0, st);
        }
        "star" => {
            let mut pts: Vec<Pos2> = Vec::new();
            for i in 0..10 {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
                let rad = if i % 2 == 0 { 0.42 } else { 0.18 };
                pts.push(Pos2::new(x + w * 0.5 + w * rad * a.cos(), y + h * 0.5 + h * rad * a.sin()));
            }
            pts.push(pts[0]);
            line(painter, pts);
        }
        "home" => {
            line(painter, vec![pt(0.12, 0.5), pt(0.5, 0.14), pt(0.88, 0.5)]);
            line(painter, vec![pt(0.26, 0.42), pt(0.26, 0.85), pt(0.74, 0.85), pt(0.74, 0.42)]);
        }
        _ => {
            painter.circle_stroke(pt(0.5, 0.5), 6.0, st);
        }
    }
}

fn icon_widget(ui: &mut egui::Ui, name: &str) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(18.0, 18.0), Sense::hover());
    let col = ui.visuals().text_color();
    draw_icon_painter(&ui.painter_at(rect), rect, name, col);
}

fn draw_logo(painter: &egui::Painter, rect: Rect) {
    painter.rect_stroke(rect, 6.0, Stroke::new(2.0_f32, COL_TINT));
    let (x, y, w, h) = (rect.left(), rect.top(), rect.width(), rect.height());
    let pts: Vec<Pos2> = vec![
        Pos2::new(x + w * 0.16, y + h * 0.24),
        Pos2::new(x + w * 0.4, y + h * 0.28),
        Pos2::new(x + w * 0.58, y + h * 0.5),
        Pos2::new(x + w * 0.84, y + h * 0.78),
    ];
    painter.add(egui::Shape::line(pts.clone(), Stroke::new(2.0_f32, COL_TEXT)));
    painter.circle_filled(pts[0], 3.0, COL_TINT);
}

fn draw_hero(ui: &mut egui::Ui) {
    let width = ui.available_width().min(980.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 176.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, COL_CARD);
    painter.rect_stroke(rect, 6.0, Stroke::new(1.0_f32, COL_BORDER));
    // Reliability curves: tint primary, muted secondary.
    for (scale, col, sw) in [(1.0f32, COL_TINT, 2.5f32), (0.72f32, COL_MUTED, 1.5f32)] {
        let pts: Vec<Pos2> = (0..=40)
            .map(|i| {
                let t = i as f64 / 40.0;
                let rel = (-2.6 * t * scale as f64).exp();
                Pos2::new(
                    rect.left() + 28.0 + (rect.width() * 0.52) * t as f32,
                    rect.top() + 22.0 + ((1.0 - rel) as f32) * 108.0,
                )
            })
            .collect();
        painter.add(egui::Shape::line(pts, Stroke::new(sw, col)));
    }
    // Block diagram motif, lower right.
    let by = rect.top() + 96.0;
    let bw = 74.0;
    let bh = 38.0;
    let xs = [rect.right() - 292.0, rect.right() - 196.0, rect.right() - 100.0];
    for bx in xs {
        painter.rect_stroke(Rect::from_min_size(Pos2::new(bx, by), Vec2::new(bw, bh)), 4.0, Stroke::new(1.5_f32, COL_MUTED));
    }
    for wpair in xs.windows(2) {
        painter.line_segment(
            [Pos2::new(wpair[0] + bw, by + bh / 2.0), Pos2::new(wpair[1], by + bh / 2.0)],
            Stroke::new(1.5_f32, COL_TINT),
        );
    }
    // Test unit dots along the bottom.
    for i in 0..14 {
        painter.circle_filled(Pos2::new(rect.left() + 30.0 + i as f32 * 22.0, rect.bottom() - 16.0), 2.5, COL_MUTED);
    }
    painter.text(rect.min + Vec2::new(28.0, 150.0), Align2::LEFT_CENTER, "Reliability over time, and the system it belongs to.", FontId::proportional(12.0), COL_MUTED);
}

// ---------------- project item model ----------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ItemKind {
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

const KIND_DEFS: [(ItemKind, &str, &str); 18] = [
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
    fn label(self) -> &'static str {
        KIND_DEFS.iter().find(|(k, _, _)| *k == self).map(|(_, l, _)| *l).unwrap_or("")
    }
    fn icon(self) -> &'static str {
        KIND_DEFS.iter().find(|(k, _, _)| *k == self).map(|(_, _, i)| *i).unwrap_or("life")
    }
    fn from_label(s: &str) -> Option<ItemKind> {
        KIND_DEFS.iter().find(|(_, l, _)| *l == s).map(|(k, _, _)| *k)
    }
    fn from_key(s: &str) -> Option<ItemKind> {
        // Keys are the labels in lowercase without spaces, plus short forms.
        let key = s.to_lowercase().replace(' ', "").replace('-', "");
        KIND_DEFS.iter().find(|(k, l, _)| {
            l.to_lowercase().replace(' ', "").replace('-', "") == key
                || format!("{:?}", k).to_lowercase() == key
        }).map(|(k, _, _)| *k)
    }
}

#[derive(Clone)]
struct ProjectItem {
    id: u64,
    kind: ItemKind,
    name: String,
    state: Value,
}

/// Which bundled dataset file loads into which item kind, following
/// the categories in datasets/README.md.
fn dataset_kind(file: &str) -> ItemKind {
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

/// Pure helper for flow (a): set one leaf of an RBD state to a fitted
/// distribution. Returns the block name on success.
fn apply_fit_to_rbd_state(state: &mut Value, block_idx: usize, dist: &str, params: &dist_mod::Params) -> Result<String, String> {
    let spec = state.get("rbd_spec").cloned().filter(|x| !x.is_null())
        .unwrap_or_else(|| serde_json::from_str(EXAMPLE3_JSON).unwrap_or(json!({})));
    let paths = RelKitApp::leaf_paths(&spec);
    let path = paths.get(block_idx).cloned().ok_or_else(|| "Block index out of range.".to_string())?;
    let mut node = RelKitApp::node_at(&spec, &path).ok_or_else(|| "Block not found.".to_string())?;
    let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("block").to_string();
    node["dist"] = json!(dist);
    node["params"] = params_to_json(params);
    let new_spec = RelKitApp::replace_at(&spec, &path, node);
    state["rbd_spec"] = new_spec.clone();
    state["rbd_json"] = json!(serde_json::to_string_pretty(&new_spec).unwrap_or_default());
    Ok(name)
}

/// Pure helper for flow (e): incident hours from a FRACAS item state,
/// all treated as failure times, as a life data text string.
fn fracas_life_text(state: &Value) -> (String, usize) {
    let mut times: Vec<String> = Vec::new();
    if let Some(rows) = state.get("fracas_rows").and_then(|v| v.as_array()) {
        for r in rows {
            if let Some(h) = r.get("hours").and_then(|v| v.as_f64()) {
                times.push(format!("{:.0}", h));
            }
        }
    }
    let n = times.len();
    (times.join(", "), n)
}

/// Pure project serialization: one JSON value for the whole project.
fn project_to_value(project_name: &str, items: &[ProjectItem], selected: Option<usize>) -> Value {
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
fn project_from_value(v: &Value) -> Option<(String, Vec<ProjectItem>, Option<usize>)> {
    if v.get("format").and_then(|x| x.as_str()) != Some("reliakit-project-2") {
        return None;
    }
    let name = v.get("project_name").and_then(|x| x.as_str()).unwrap_or("Untitled project").to_string();
    let mut items: Vec<ProjectItem> = Vec::new();
    for (i, iv) in v.get("items").and_then(|x| x.as_array()).map(|a| a.as_slice()).unwrap_or(&[]).iter().enumerate() {
        let kind = iv.get("kind").and_then(|x| x.as_str()).and_then(ItemKind::from_key)?;
        items.push(ProjectItem {
            id: iv.get("id").and_then(|x| x.as_u64()).unwrap_or((i + 1) as u64),
            kind,
            name: iv.get("name").and_then(|x| x.as_str()).unwrap_or(kind.label()).to_string(),
            state: iv.get("state").cloned().unwrap_or(json!({})),
        });
    }
    let sel = v.get("selected_index").and_then(|x| x.as_u64()).map(|i| i as usize).filter(|i| *i < items.len());
    Some((name, items, sel))
}

// ---------------- pages ----------------

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
    Markov,
    Qcp,
    Rcm,
    Fracas,
    Examples,
    Start,
}

const PAGES: [(Page, &str); 19] = [
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
    (Page::Markov, "Markov"),
    (Page::Qcp, "QCP"),
    (Page::Rcm, "RCM"),
    (Page::Fracas, "FRACAS"),
    (Page::Examples, "Examples"),
];

fn page_title(p: Page) -> &'static str {
    if p == Page::Start {
        return "Start";
    }
    PAGES.iter().find(|(pg, _)| *pg == p).map(|(_, t)| *t).unwrap_or("")
}

// ---------------- small UI state structs ----------------

#[derive(Clone)]
struct RcmRowUi {
    asset: String,
    function: String,
    mode: String,
    consequence: String,
    monitorable: bool,
    age_related: bool,
    restorable: bool,
    rate: String,
    alpha: String,
    beta: String,
    hours: String,
}

impl RcmRowUi {
    fn to_json(&self) -> Value {
        json!({
            "asset": self.asset, "function": self.function, "mode": self.mode,
            "consequence": self.consequence, "monitorable": self.monitorable,
            "age_related": self.age_related, "restorable": self.restorable,
            "rate": self.rate, "alpha": self.alpha, "beta": self.beta, "hours": self.hours,
        })
    }
    fn from_json(v: &Value) -> Self {
        let g = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        RcmRowUi {
            asset: g("asset"), function: g("function"), mode: g("mode"),
            consequence: if g("consequence").is_empty() { "Operational".into() } else { g("consequence") },
            monitorable: v.get("monitorable").and_then(|x| x.as_bool()).unwrap_or(false),
            age_related: v.get("age_related").and_then(|x| x.as_bool()).unwrap_or(false),
            restorable: v.get("restorable").and_then(|x| x.as_bool()).unwrap_or(false),
            rate: g("rate"), alpha: g("alpha"), beta: g("beta"), hours: g("hours"),
        }
    }
    fn to_mode(&self) -> RcmMode {
        RcmMode {
            asset: self.asset.clone(),
            function: self.function.clone(),
            failure_mode: self.mode.clone(),
            consequence: Consequence::from_str(&self.consequence).unwrap_or(Consequence::Operational),
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

struct RelKitApp {
    page: Page,
    status: String,
    headline: String,
    project_path: String,
    // CSV import (shared controls)
    csv_pick: String,
    csv_path: String,
    // Life Data
    life_times: String,
    life_dist: String,
    life_rank: bool,
    life_t: String,
    life_result: String,
    life_fit: Option<dist_mod::FitResult>,
    life_show_prob: bool,
    wizard_ranks: Vec<FitRank>,
    wizard_note: String,
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
    deg_data: Option<degradation::DegradationResult>,
    deg_parsed: Vec<(Vec<f64>, Vec<f64>)>,
    // ALT
    alt_data: String,
    alt_model: String,
    alt_use_stress: String,
    alt_t: String,
    alt_result: String,
    // Growth
    growth_times: String,
    growth_result: String,
    growth_fit: Option<growth::CrowAmsaaFit>,
    // Competing
    comp_data: String,
    comp_dist: String,
    comp_t: String,
    comp_result: String,
    // System RBD
    rbd_json: String,
    rbd_spec: Value,
    rbd_t: String,
    rbd_horizon: String,
    rbd_topology: String,
    rbd_k: String,
    rbd_avail: String,
    rbd_path: String,
    rbd_status: String,
    rbd_armed: Option<String>,
    rbd_sel: Option<usize>,
    rbd_importance: Vec<BlockImportance>,
    rbd_importance_note: String,
    ed_name: String,
    ed_dist: String,
    ed_params: String,
    ed_mtbf: String,
    ed_mttr: String,
    ed_units: String,
    ed_k: String,
    ed_dormancy: String,
    ed_switch: String,
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
    fmea_type: String,
    fmea_crit: Vec<[String; 4]>,
    fmea_csv: String,
    fmea_export_path: String,
    fm_item: String,
    fm_mode: String,
    fm_sev: String,
    fm_occ: String,
    fm_det: String,
    fm_alpha: String,
    fm_beta: String,
    fm_lambda: String,
    fm_hours: String,
    fmea_status: String,
    dvpr_entries: Vec<DvprEntry>,
    dv_id: String,
    dv_ref: String,
    dv_test: String,
    dv_sample: String,
    dv_criteria: String,
    // Warranty
    war_sales: String,
    war_returns: String,
    war_current: String,
    war_horizon: String,
    war_result: String,
    war_table: Vec<Vec<u32>>,
    war_sale_labels: Vec<String>,
    // Prediction
    pred_parts: String,
    pred_t: String,
    pred_result: String,
    pred_source: String,
    pred_lib_path: String,
    pred_lib_parts: Vec<PartEntry>,
    pred_lib_note: String,
    pred217_file: String,
    pred217_json: Option<Value>,
    pred217_named: BTreeMap<String, String>,
    pred217_num: BTreeMap<String, String>,
    pred217_result: String,
    alloc_target: String,
    alloc_n: String,
    alloc_weights: String,
    alloc_result: String,
    der_name: String,
    der_operating: String,
    der_rated: String,
    der_max: String,
    der_result: String,
    sr_generic: String,
    sr_pi_q: String,
    sr_pi_s: String,
    sr_ea: String,
    sr_temp: String,
    sr_ref_temp: String,
    sr_pi_e: String,
    sr_qty_rate: String,
    sr_obs_rate: String,
    sr_pit_obs: String,
    sr_pit_tgt: String,
    sr_pis_obs: String,
    sr_pis_tgt: String,
    sr_ci_failures: String,
    sr_ci_exposure: String,
    sr_ci_conf: String,
    sr_m3_pred: String,
    sr_m3_pw: String,
    sr_m3_obs: String,
    sr_m3_ow: String,
    mp_base: String,
    mp_ea: String,
    mp_ref: String,
    mp_cyc_factor: String,
    mp_phases: String,
    pred_clean_result: String,
    // DOE
    doe_k: String,
    doe_y: String,
    doe_result: String,
    // Flowchart
    flow_json: String,
    flow_sims: String,
    flow_seed: String,
    flow_result: String,
    // Markov
    mk_states: Vec<String>,
    mk_new_state: String,
    mk_transitions: Vec<[String; 3]>,
    mk_absorbing: String,
    mk_start: String,
    mk_time: String,
    mk_result: String,
    // QCP
    qcp_dist: String,
    qcp_params: BTreeMap<String, String>,
    qcp_t: String,
    qcp_bx: String,
    qcp_result: String,
    // RCM
    rcm_rows: Vec<RcmRowUi>,
    // FRACAS
    fracas_rows: Vec<Incident>,
    fracas_total_hours: String,
    fracas_path: String,
    fracas_status: String,
    // Project workspace
    project_name: String,
    items: Vec<ProjectItem>,
    selected: Option<usize>,
    next_id: u64,
    new_kind: String,
    new_name: String,
    rename_name: String,
    flow_target_id: u64,
    flow_block_idx: usize,
    fmea_rcm_row: String,
    pred_life_dist: String,
    pred_life_params: String,
    pred_life_t: String,
    pred_life_result: String,
    rbd_undo: Vec<Value>,
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
            qcp_dist: "weibull2".into(),
            qcp_params: BTreeMap::new(),
            qcp_t: "1000".into(),
            qcp_bx: "0.10".into(),
            qcp_result: String::new(),
            rcm_rows: vec![RcmRowUi {
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
            }],
            fracas_rows: vec![
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
            ],
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
            fmea_rcm_row: "0".into(),
            pred_life_dist: "weibull2".into(),
            pred_life_params: "beta=1.5, eta=5000".into(),
            pred_life_t: "1000".into(),
            pred_life_result: String::new(),
            rbd_undo: Vec::new(),
        };
        app.load_fmea_example();
        app.ensure_rbd_positions();
        app.qcp_reset_params();
        app.init_project();
        app
    }

    fn note(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
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
        self.fmea_crit = vec![
            [String::new(), String::new(), String::new(), String::new()],
            [String::new(), String::new(), String::new(), String::new()],
            [String::new(), String::new(), String::new(), String::new()],
        ];
    }

    // ---------------- project file ----------------

    fn save_project(&mut self) {
        self.capture_selected();
        let proj = project_to_value(&self.project_name, &self.items, self.selected);
        match std::fs::write(&self.project_path, serde_json::to_string_pretty(&proj).unwrap_or_default()) {
            Ok(()) => self.note(format!("Project saved to {}", self.project_path)),
            Err(e) => self.note(format!("Project save error: {}", e)),
        }
    }

    fn open_project(&mut self) {
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
            } else {
                self.page = Page::Start;
            }
            self.note(format!("Project opened from {}", self.project_path));
            return;
        }
        // Legacy format (reliakit-project-1): load the three covered
        // parts as items. This is the nice to have path for old files.
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
                self.create_item_silent(ItemKind::LifeData, "Life Data (from old project)".into(), st);
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
            self.note(format!("Old format project opened from {} as {} items.", self.project_path, self.items.len()));
            return;
        }
        self.note("Project file is not a ReliaKit project (v2 or legacy).");
    }

    fn new_project(&mut self) {
        self.items = Vec::new();
        self.selected = None;
        self.next_id = 1;
        self.project_name = "Untitled project".into();
        self.page = Page::Start;
        self.headline = String::new();
        self.note("New project. Add an item, or load an example or dataset from the start page.");
    }

    // ---------------- project workspace ----------------

    fn gs(v: &Value, k: &str, d: &str) -> String {
        v.get(k).and_then(|x| x.as_str()).map(|s| s.to_string()).unwrap_or_else(|| d.to_string())
    }
    fn gb(v: &Value, k: &str, d: bool) -> bool {
        v.get(k).and_then(|x| x.as_bool()).unwrap_or(d)
    }

    fn page_of_kind(kind: ItemKind) -> Page {
        match kind {
            ItemKind::LifeData => Page::LifeData,
            ItemKind::LimitedData => Page::LimitedData,
            ItemKind::Mcmc => Page::Mcmc,
            ItemKind::Degradation => Page::Degradation,
            ItemKind::Alt => Page::Alt,
            ItemKind::Growth => Page::Growth,
            ItemKind::Competing => Page::Competing,
            ItemKind::Rbd => Page::SystemRbd,
            ItemKind::FaultTree => Page::FaultTree,
            ItemKind::Fmea => Page::Fmea,
            ItemKind::Warranty => Page::Warranty,
            ItemKind::Prediction => Page::Prediction,
            ItemKind::Doe => Page::Doe,
            ItemKind::Flowchart => Page::Flowchart,
            ItemKind::Markov => Page::Markov,
            ItemKind::Qcp => Page::Qcp,
            ItemKind::Rcm => Page::Rcm,
            ItemKind::Fracas => Page::Fracas,
        }
    }

    fn init_project(&mut self) {
        if !self.items.is_empty() {
            return;
        }
        let st = self.capture_state(ItemKind::LifeData);
        self.items.push(ProjectItem { id: 1, kind: ItemKind::LifeData, name: "Life Data 1".into(), state: st });
        self.next_id = 2;
        self.selected = Some(0);
        self.rename_name = "Life Data 1".into();
        self.page = Page::LifeData;
    }

    fn capture_selected(&mut self) {
        if let Some(i) = self.selected {
            if i < self.items.len() {
                let kind = self.items[i].kind;
                let st = self.capture_state(kind);
                self.items[i].state = st;
            }
        }
    }

    fn select_index(&mut self, idx: Option<usize>) {
        self.capture_selected();
        self.selected = idx;
        match idx {
            Some(i) => {
                if let Some(it) = self.items.get(i).cloned() {
                    self.apply_state(it.kind, &it.state.clone());
                    self.rename_name = it.name.clone();
                    self.page = Self::page_of_kind(it.kind);
                }
            }
            None => self.page = Page::Start,
        }
    }

    fn create_item_silent(&mut self, kind: ItemKind, name: String, state: Value) -> usize {
        self.capture_selected();
        let id = self.next_id;
        self.next_id += 1;
        self.items.push(ProjectItem { id, kind, name, state });
        self.items.len() - 1
    }

    fn find_or_create_item(&mut self, kind: ItemKind, name: &str) -> usize {
        if let Some(i) = self.items.iter().position(|it| it.kind == kind) {
            return i;
        }
        self.create_item_silent(kind, name.to_string(), json!({}))
    }

    fn default_item_name(&self, kind: ItemKind) -> String {
        let n = self.items.iter().filter(|it| it.kind == kind).count() + 1;
        format!("{} {}", kind.label(), n)
    }

    // ---------------- item state capture and restore ----------------

    fn capture_state(&self, kind: ItemKind) -> Value {
        match kind {
            ItemKind::LifeData => json!({
                "life_times": self.life_times, "life_dist": self.life_dist,
                "life_rank": self.life_rank, "life_t": self.life_t,
                "life_result": self.life_result,
                "life_fit": serde_json::to_value(&self.life_fit).unwrap_or(json!(null)),
                "life_show_prob": self.life_show_prob, "wizard_note": self.wizard_note,
                "wizard_ranks": self.wizard_ranks.iter().map(|r| json!({
                    "dist": r.dist, "loglik": r.loglik, "aic": r.aic, "bic": r.bic,
                    "ad": r.ad_statistic, "success": r.success, "n_params": r.n_params,
                })).collect::<Vec<Value>>(),
            }),
            ItemKind::LimitedData => json!({"limited_groups": self.limited_groups, "limited_result": self.limited_result}),
            ItemKind::Mcmc => json!({
                "mcmc_times": self.mcmc_times, "mcmc_dist": self.mcmc_dist,
                "mcmc_draws": self.mcmc_draws, "mcmc_warmup": self.mcmc_warmup,
                "mcmc_chains": self.mcmc_chains, "mcmc_seed": self.mcmc_seed,
                "mcmc_t": self.mcmc_t, "mcmc_result": self.mcmc_result,
            }),
            ItemKind::Degradation => json!({
                "deg_units": self.deg_units, "deg_threshold": self.deg_threshold,
                "deg_model": self.deg_model, "deg_dist": self.deg_dist,
                "deg_increasing": self.deg_increasing, "deg_result": self.deg_result,
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
                "alt_use_stress": self.alt_use_stress, "alt_t": self.alt_t, "alt_result": self.alt_result,
            }),
            ItemKind::Growth => json!({
                "growth_times": self.growth_times, "growth_result": self.growth_result,
                "growth_fit": serde_json::to_value(&self.growth_fit).unwrap_or(json!(null)),
            }),
            ItemKind::Competing => json!({
                "comp_data": self.comp_data, "comp_dist": self.comp_dist,
                "comp_t": self.comp_t, "comp_result": self.comp_result,
            }),
            ItemKind::Rbd => json!({
                "rbd_spec": self.rbd_spec, "rbd_t": self.rbd_t, "rbd_horizon": self.rbd_horizon,
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
                "war_result": self.war_result,
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
            ItemKind::Doe => json!({"doe_k": self.doe_k, "doe_y": self.doe_y, "doe_result": self.doe_result}),
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

    fn apply_state(&mut self, kind: ItemKind, v: &Value) {
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
                self.life_fit = v.get("life_fit").and_then(|x| serde_json::from_value(x.clone()).ok());
                self.life_show_prob = Self::gb(v, "life_show_prob", false);
                self.wizard_note = gs("wizard_note", "");
                self.wizard_ranks = v.get("wizard_ranks").and_then(|x| x.as_array()).map(|arr| arr.iter().map(|r| FitRank {
                    dist: Self::gs(r, "dist", ""), loglik: r.get("loglik").and_then(|x| x.as_f64()).unwrap_or(0.0),
                    aic: r.get("aic").and_then(|x| x.as_f64()).unwrap_or(0.0),
                    bic: r.get("bic").and_then(|x| x.as_f64()).unwrap_or(0.0),
                    ad_statistic: r.get("ad").and_then(|x| x.as_f64()),
                    n_params: r.get("n_params").and_then(|x| x.as_u64()).unwrap_or(0) as usize,
                    success: Self::gb(r, "success", false),
                }).collect()).unwrap_or_default();
            }
            ItemKind::LimitedData => {
                if v.get("limited_groups").and_then(|x| x.as_str()).is_some() {
                    self.limited_groups = gs("limited_groups", "");
                } else {
                    let groups = datasets::small_sample_groups(2026);
                    self.limited_groups = groups.iter().map(|(t, f)| format_life(t, f)).collect::<Vec<String>>().join("\n");
                }
                self.limited_result = gs("limited_result", "");
            }
            ItemKind::Mcmc => {
                self.mcmc_times = if v.get("mcmc_times").and_then(|x| x.as_str()).is_some() { gs("mcmc_times", "") } else { self.life_times.clone() };
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
                    self.deg_units = deg.iter().map(|(t, y)| format!("{} | {}", t.iter().map(|v2| format!("{:.0}", v2)).collect::<Vec<String>>().join(","), y.iter().map(|v2| format!("{:.2}", v2)).collect::<Vec<String>>().join(","))).collect::<Vec<String>>().join("\n");
                }
                self.deg_threshold = gs("deg_threshold", "70");
                self.deg_model = gs("deg_model", "linear");
                self.deg_dist = gs("deg_dist", "weibull2");
                self.deg_increasing = Self::gb(v, "deg_increasing", false);
                self.deg_result = gs("deg_result", "");
                self.deg_parsed = v.get("deg_parsed").and_then(|x| serde_json::from_value(x.clone()).ok()).unwrap_or_default();
                self.deg_data = v.get("deg_data").and_then(|d| {
                    if d.is_null() { return None; }
                    let paths: Vec<(f64, f64)> = d.get("paths").and_then(|x| serde_json::from_value::<Vec<[f64; 2]>>(x.clone()).ok())
                        .map(|a| a.iter().map(|p| (p[0], p[1])).collect()).unwrap_or_default();
                    let pseudo: Vec<f64> = d.get("pseudo_times").and_then(|x| serde_json::from_value(x.clone()).ok())?;
                    let failed: Vec<bool> = d.get("failed").and_then(|x| serde_json::from_value(x.clone()).ok())?;
                    let fit: Option<dist_mod::FitResult> = d.get("fit").and_then(|x| serde_json::from_value(x.clone()).ok()).flatten();
                    Some(degradation::DegradationResult { paths, pseudo_times: pseudo, failed, fit, model: Self::gs(d, "model", "linear") })
                });
            }
            ItemKind::Alt => {
                if v.get("alt_data").and_then(|x| x.as_str()).is_some() {
                    self.alt_data = gs("alt_data", "");
                } else {
                    let (alt_stress, alt_times) = datasets::alt_dataset(2026);
                    self.alt_data = alt_stress.iter().zip(&alt_times).map(|(s2, t)| format!("{:.0},{:.1}", s2[0], t)).collect::<Vec<String>>().join("\n");
                }
                self.alt_model = gs("alt_model", "arrhenius");
                self.alt_use_stress = gs("alt_use_stress", "55");
                self.alt_t = gs("alt_t", "1000");
                self.alt_result = gs("alt_result", "");
            }
            ItemKind::Growth => {
                if v.get("growth_times").and_then(|x| x.as_str()).is_some() {
                    self.growth_times = gs("growth_times", "");
                } else {
                    self.growth_times = datasets::growth_dataset(2026).iter().map(|v2| format!("{:.1}", v2)).collect::<Vec<String>>().join(", ");
                }
                self.growth_result = gs("growth_result", "");
                self.growth_fit = v.get("growth_fit").and_then(|x| serde_json::from_value(x.clone()).ok());
            }
            ItemKind::Competing => {
                self.comp_data = gs("comp_data", "120,wear\n340,electrical\n560,wear\n800,\n950,electrical\n1300,wear\n1700,\n2200,electrical");
                self.comp_dist = gs("comp_dist", "weibull2");
                self.comp_t = gs("comp_t", "1000");
                self.comp_result = gs("comp_result", "");
            }
            ItemKind::Rbd => {
                self.rbd_spec = v.get("rbd_spec").cloned().filter(|x| !x.is_null())
                    .unwrap_or_else(|| serde_json::from_str(EXAMPLE3_JSON).unwrap_or(json!({"type": "series", "children": []})));
                self.rbd_t = gs("rbd_t", "1000");
                self.rbd_horizon = gs("rbd_horizon", "20000");
                self.rbd_topology = gs("rbd_topology", "series");
                self.rbd_k = gs("rbd_k", "2");
                self.rbd_avail = gs("rbd_avail", "");
                self.rbd_path = gs("rbd_path", "rbd.json");
                self.rbd_status = gs("rbd_status", "");
                self.rbd_sel = v.get("rbd_sel").and_then(|x| x.as_u64()).map(|i| i as usize);
                self.rbd_importance = v.get("rbd_importance").and_then(|x| x.as_array()).map(|arr| arr.iter().map(|b| BlockImportance {
                    name: Self::gs(b, "name", ""), birnbaum: b.get("birnbaum").and_then(|x| x.as_f64()).unwrap_or(0.0),
                    fussell_vesely: b.get("fussell_vesely").and_then(|x| x.as_f64()).unwrap_or(0.0),
                    criticality: b.get("criticality").and_then(|x| x.as_f64()).unwrap_or(0.0),
                }).collect()).unwrap_or_default();
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
                let had_rows = v.get("fmea_rows").and_then(|x| x.as_array()).map(|a| !a.is_empty()).unwrap_or(false);
                if had_rows {
                    self.fmea_rows = serde_json::from_value(v.get("fmea_rows").cloned().unwrap_or(json!([]))).unwrap_or_default();
                    self.fmea_crit = v.get("fmea_crit").and_then(|x| serde_json::from_value(x.clone()).ok())
                        .unwrap_or_else(|| vec![[String::new(), String::new(), String::new(), String::new()]; self.fmea_rows.len()]);
                    self.dvpr_entries = v.get("dvpr").and_then(|x| x.as_array()).map(|arr| arr.iter().map(|e| DvprEntry {
                        id: Self::gs(e, "id", ""), failure_mode_ref: Self::gs(e, "failure_mode_ref", ""),
                        test: Self::gs(e, "test", ""), sample_size: e.get("sample_size").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
                        acceptance_criteria: Self::gs(e, "acceptance_criteria", ""),
                    }).collect()).unwrap_or_default();
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
                self.war_table = v.get("war_table").and_then(|x| serde_json::from_value(x.clone()).ok()).unwrap_or_default();
                self.war_sale_labels = v.get("war_sale_labels").and_then(|x| serde_json::from_value(x.clone()).ok()).unwrap_or_default();
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
                self.pred217_named = v.get("pred217_named").and_then(|x| serde_json::from_value(x.clone()).ok()).unwrap_or_default();
                self.pred217_num = v.get("pred217_num").and_then(|x| serde_json::from_value(x.clone()).ok()).unwrap_or_default();
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
                self.mp_phases = gs("mp_phases", "Operating, 8000, 55, 100, 0, 1.0\nDormant, 760, 25, 0, 1, 0.1");
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
                self.mk_states = v.get("mk_states").and_then(|x| serde_json::from_value(x.clone()).ok())
                    .unwrap_or_else(|| vec!["Both up".into(), "One down".into(), "System down".into()]);
                self.mk_transitions = v.get("mk_transitions").and_then(|x| serde_json::from_value(x.clone()).ok())
                    .unwrap_or_else(|| vec![
                        ["Both up".into(), "One down".into(), "0.02".into()],
                        ["One down".into(), "Both up".into(), "0.1".into()],
                        ["One down".into(), "System down".into(), "0.01".into()],
                    ]);
                self.mk_new_state = String::new();
                self.mk_absorbing = gs("mk_absorbing", "System down");
                self.mk_start = gs("mk_start", "Both up");
                self.mk_time = gs("mk_time", "100");
                self.mk_result = gs("mk_result", "");
            }
            ItemKind::Qcp => {
                self.qcp_dist = gs("qcp_dist", "weibull2");
                let had = v.get("qcp_params").and_then(|x| x.as_object()).map(|m| !m.is_empty()).unwrap_or(false);
                if had {
                    self.qcp_params = serde_json::from_value(v.get("qcp_params").cloned().unwrap_or(json!({}))).unwrap_or_default();
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
                        asset: "Pump".into(), function: "Move fluid".into(), mode: "Seal leaks".into(),
                        consequence: "Operational".into(), monitorable: true, age_related: false,
                        restorable: false, rate: "0.00001".into(), alpha: "0.3".into(),
                        beta: "1.0".into(), hours: "8760".into(),
                    }];
                }
            }
            ItemKind::Fracas => {
                if let Some(arr) = v.get("fracas_rows").and_then(|x| x.as_array()) {
                    self.fracas_rows = arr.iter().map(|r| Incident {
                        id: Self::gs(r, "id", ""), part: Self::gs(r, "part", ""),
                        failure_mode: Self::gs(r, "failure_mode", ""),
                        hours: r.get("hours").and_then(|x| x.as_f64()).unwrap_or(0.0),
                        date: Self::gs(r, "date", ""),
                        status: IncidentStatus::from_str(&Self::gs(r, "status", "Open")).unwrap_or(IncidentStatus::Open),
                        corrective_action: Self::gs(r, "corrective_action", ""),
                    }).collect();
                } else {
                    self.fracas_rows = vec![
                        Incident { id: "FR-1".into(), part: "Pump seal".into(), failure_mode: "Leak".into(), hours: 1200.0, date: "2026-01-15".into(), status: IncidentStatus::Closed, corrective_action: "Replaced seal material".into() },
                        Incident { id: "FR-2".into(), part: "Controller board".into(), failure_mode: "Overheat".into(), hours: 3400.0, date: "2026-03-02".into(), status: IncidentStatus::Analysis, corrective_action: String::new() },
                    ];
                }
                self.fracas_total_hours = gs("fracas_total_hours", "10000");
                self.fracas_path = gs("fracas_path", "fracas.csv");
                self.fracas_status = gs("fracas_status", "");
            }
        }
    }

    // ---------------- data flows between items ----------------

    fn flow_send_fit_to_qcp(&mut self) {
        let fit = match self.life_fit.clone() {
            Some(f) => f,
            None => { self.note("Run a fit first, then send it to QCP."); return; }
        };
        let idx = self.find_or_create_item(ItemKind::Qcp, "QCP 1");
        let mut params = serde_json::Map::new();
        for (k, val) in &fit.params {
            params.insert(k.clone(), json!(val.to_string()));
        }
        let mut st = self.items[idx].state.clone();
        if !st.is_object() { st = json!({}); }
        st["qcp_dist"] = json!(fit.dist);
        st["qcp_params"] = Value::Object(params);
        st["qcp_t"] = json!(self.life_t);
        let name = self.items[idx].name.clone();
        self.items[idx].state = st;
        self.headline = format!("Fit sent to QCP item {}", name);
        self.note(format!("Sent {} fit to QCP item {}. Open it to run the calculation.", fit.dist, name));
    }

    fn flow_send_fit_to_prediction(&mut self) {
        let fit = match self.life_fit.clone() {
            Some(f) => f,
            None => { self.note("Run a fit first, then send it to Prediction."); return; }
        };
        let idx = self.find_or_create_item(ItemKind::Prediction, "Prediction 1");
        let params_text = fit.params.iter().map(|(k, val)| format!("{}={}", k, val)).collect::<Vec<String>>().join(", ");
        let mut st = self.items[idx].state.clone();
        if !st.is_object() { st = json!({}); }
        st["pred_life_dist"] = json!(fit.dist);
        st["pred_life_params"] = json!(params_text);
        st["pred_life_t"] = json!(self.life_t);
        let name = self.items[idx].name.clone();
        self.items[idx].state = st;
        self.headline = format!("Fit sent to Prediction item {}", name);
        self.note(format!("Sent {} fit to Prediction item {}. Its life model section shows R(t) for the fitted model.", fit.dist, name));
    }

    fn flow_assign_fit_to_rbd(&mut self) {
        let fit = match self.life_fit.clone() {
            Some(f) => f,
            None => { self.note("Run a fit first, then assign it to an RBD block."); return; }
        };
        let idx = match self.items.iter().position(|it| it.id == self.flow_target_id) {
            Some(i) => i,
            None => { self.note("Pick a target RBD item first."); return; }
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

    fn flow_fmea_to_rcm(&mut self, row_idx: usize) {
        let row = match self.fmea_rows.get(row_idx).cloned() {
            Some(r) => r,
            None => { self.note("Pick a valid FMEA row first."); return; }
        };
        let idx = self.find_or_create_item(ItemKind::Rcm, "RCM 1");
        let new_row = json!({
            "asset": row.item, "function": row.function, "mode": row.failure_mode,
            "consequence": "Operational", "monitorable": false, "age_related": false,
            "restorable": false, "rate": "", "alpha": "", "beta": "", "hours": "",
        });
        let mut st = self.items[idx].state.clone();
        if !st.is_object() { st = json!({}); }
        let mut rows: Vec<Value> = st.get("rcm_rows").and_then(|x| x.as_array()).cloned().unwrap_or_default();
        rows.push(new_row);
        st["rcm_rows"] = json!(rows);
        let name = self.items[idx].name.clone();
        self.items[idx].state = st;
        self.note(format!("Created an RCM row in {} from FMEA row {} ({} / {}).", name, row_idx + 1, row.item, row.failure_mode));
    }

    fn flow_fracas_to_life(&mut self) {
        // Build the life text from the current FRACAS rows directly.
        let times: Vec<String> = self.fracas_rows.iter().map(|inc| format!("{:.0}", inc.hours)).collect();
        let n = times.len();
        if n == 0 {
            self.note("No incidents to export. Add an incident first.");
            return;
        }
        let st = json!({
            "life_times": times.join(", "), "life_dist": "weibull2", "life_t": "1000",
            "life_result": "Exported from FRACAS. All incidents are treated as failures; the FRACAS log has no censoring notion.",
        });
        let idx = self.create_item_silent(ItemKind::LifeData, "FRACAS export (all incidents as failures)".into(), st);
        self.select_index(Some(idx));
        self.note(format!("Created a Life Data item from {} FRACAS incidents. All incidents count as failures.", n));
    }

    fn delete_selected_rbd_block(&mut self) {
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

    fn undo_rbd(&mut self) {
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

    // ---------------- start page loading ----------------

    fn load_example_item(&mut self, n: usize) {
        match n {
            1 => {
                let idx = self.create_item_silent(ItemKind::LifeData, self.default_item_name(ItemKind::LifeData), json!({}));
                self.select_index(Some(idx));
                let (t, f) = datasets::bearing_life(2026);
                self.life_times = format_life(&t, &f);
                self.mcmc_times = self.life_times.clone();
                self.life_fit = None;
                self.life_result = "Loaded Example 1 (bearing life data).".into();
                self.note("Example 1 loaded as a new Life Data item.");
            }
            2 => {
                let idx = self.create_item_silent(ItemKind::LimitedData, self.default_item_name(ItemKind::LimitedData), json!({}));
                self.select_index(Some(idx));
                let groups = datasets::small_sample_groups(2026);
                self.limited_groups = groups.iter().map(|(t, f)| format_life(t, f)).collect::<Vec<String>>().join("\n");
                self.limited_result = "Loaded Example 2 (limited data lots).".into();
                self.note("Example 2 loaded as a new Limited Data item.");
            }
            3 => {
                let idx = self.create_item_silent(ItemKind::Rbd, self.default_item_name(ItemKind::Rbd), json!({}));
                self.select_index(Some(idx));
                let spec: Value = serde_json::from_str(EXAMPLE3_JSON).unwrap_or(json!({}));
                self.set_rbd_spec(spec);
                self.rbd_t = "1000".into();
                self.rbd_status = "Loaded Example 3.".into();
                self.note("Example 3 loaded as a new RBD item.");
            }
            4 => {
                let idx = self.create_item_silent(ItemKind::Fmea, self.default_item_name(ItemKind::Fmea), json!({}));
                self.select_index(Some(idx));
                self.load_fmea_example();
                self.fmea_status = "Loaded Example 4 (3 sample rows).".into();
                self.note("Example 4 loaded as a new FMEA item.");
            }
            5 => {
                let idx = self.create_item_silent(ItemKind::Alt, self.default_item_name(ItemKind::Alt), json!({}));
                self.select_index(Some(idx));
                let (stress, times) = datasets::alt_dataset(2026);
                self.alt_data = stress.iter().zip(&times).map(|(s2, t)| format!("{:.0},{:.1}", s2[0], t)).collect::<Vec<String>>().join("\n");
                self.alt_result = "Loaded ALT example.".into();
                self.note("Example 5 loaded as a new ALT item.");
            }
            6 => {
                let idx = self.create_item_silent(ItemKind::Growth, self.default_item_name(ItemKind::Growth), json!({}));
                self.select_index(Some(idx));
                self.growth_times = datasets::growth_dataset(2026).iter().map(|v| format!("{:.1}", v)).collect::<Vec<String>>().join(", ");
                self.growth_result = "Loaded growth example.".into();
                self.note("Example 6 loaded as a new Growth item.");
            }
            7 => {
                let idx = self.create_item_silent(ItemKind::Degradation, self.default_item_name(ItemKind::Degradation), json!({}));
                self.select_index(Some(idx));
                let deg = datasets::degradation_units(2026);
                self.deg_units = deg.iter().map(|(t, y)| format!("{} | {}", t.iter().map(|v| format!("{:.0}", v)).collect::<Vec<String>>().join(","), y.iter().map(|v| format!("{:.2}", v)).collect::<Vec<String>>().join(","))).collect::<Vec<String>>().join("\n");
                self.deg_result = "Loaded degradation example.".into();
                self.note("Example 7 loaded as a new Degradation item.");
            }
            _ => {
                let idx = self.create_item_silent(ItemKind::Markov, self.default_item_name(ItemKind::Markov), json!({}));
                self.select_index(Some(idx));
                self.mk_states = vec!["Both up".into(), "One down".into(), "System down".into()];
                self.mk_transitions = vec![
                    ["Both up".into(), "One down".into(), "0.02".into()],
                    ["One down".into(), "Both up".into(), "0.1".into()],
                    ["One down".into(), "System down".into(), "0.01".into()],
                ];
                self.mk_absorbing = "System down".into();
                self.mk_start = "Both up".into();
                self.mk_result = "Loaded Markov preset. MTTF from Both up should be 650 hours.".into();
                self.note("Example 8 loaded as a new Markov item.");
            }
        }
    }

    fn load_dataset_item(&mut self, file: &str) {
        let path = datasets_dir().join(file);
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => { self.note(format!("Dataset load error: {}", e)); return; }
        };
        let kind = dataset_kind(file);
        let idx = self.create_item_silent(kind, format!("{} ({})", file, kind.label()), json!({}));
        self.select_index(Some(idx));
        self.apply_csv_to_current(&text);
        self.note(format!("Loaded dataset {} as a new {} item.", file, kind.label()));
    }

    /// Apply CSV text to the currently selected item, using the same
    /// column rules as that item view's own CSV loader.
    fn apply_csv_to_current(&mut self, text: &str) {
        let kind = match self.selected.and_then(|i| self.items.get(i)).map(|it| it.kind) {
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
                        if let (Some(tv), Some(fv)) = (r.get(t).and_then(|s2| s2.parse::<f64>().ok()), r.get(f).and_then(|s2| s2.parse::<f64>().ok())) {
                            let count = ni.and_then(|n| r.get(n)).and_then(|s2| s2.parse::<usize>().ok()).unwrap_or(1);
                            let is_failed = if is_failed_col { fv != 0.0 } else { fv == 0.0 };
                            for _ in 0..count { times.push(tv); failed.push(is_failed); }
                        }
                    }
                    if !times.is_empty() {
                        self.life_times = format_life(&times, &failed);
                        self.mcmc_times = self.life_times.clone();
                    }
                }
            }
            ItemKind::Alt => {
                if let (Some(si), Some(ti)) = (col_idx(&header, "stress"), col_idx(&header, "time")) {
                    let fi = col_idx(&header, "failed");
                    let s2i = col_idx(&header, "stress2");
                    let mut lines: Vec<String> = Vec::new();
                    for r in &rows {
                        let s2 = r.get(si).cloned().unwrap_or_default();
                        let t = r.get(ti).cloned().unwrap_or_default();
                        let f = fi.and_then(|i| r.get(i)).cloned().unwrap_or_else(|| "1".into());
                        match s2i {
                            Some(i2) => lines.push(format!("{},{},{},{}", s2, r.get(i2).cloned().unwrap_or_default(), t, f)),
                            None => lines.push(format!("{},{},{}", s2, t, f)),
                        }
                    }
                    if !lines.is_empty() { self.alt_data = lines.join("\n"); }
                }
            }
            ItemKind::Degradation => {
                if let (Some(ui_i), Some(ti), Some(vi)) = (col_idx(&header, "unit"), col_idx(&header, "time"), col_idx(&header, "value")) {
                    let mut units: Vec<(Vec<f64>, Vec<f64>)> = Vec::new();
                    let mut order: Vec<String> = Vec::new();
                    for r in &rows {
                        let unit = r.get(ui_i).cloned().unwrap_or_default();
                        if !order.contains(&unit) { order.push(unit.clone()); units.push((Vec::new(), Vec::new())); }
                        let k = order.iter().position(|u| *u == unit).unwrap_or(0);
                        if let (Some(t), Some(vv)) = (r.get(ti).and_then(|s2| s2.parse::<f64>().ok()), r.get(vi).and_then(|s2| s2.parse::<f64>().ok())) {
                            units[k].0.push(t);
                            units[k].1.push(vv);
                        }
                    }
                    if !units.is_empty() {
                        self.deg_units = units.iter().map(|(t, y)| format!("{} | {}", t.iter().map(|v2| format!("{:.4}", v2)).collect::<Vec<String>>().join(","), y.iter().map(|v2| format!("{:.4}", v2)).collect::<Vec<String>>().join(","))).collect::<Vec<String>>().join("\n");
                    }
                }
            }
            ItemKind::Growth => {
                if let Some(ti) = col_idx(&header, "time") {
                    let times: Vec<String> = rows.iter().filter_map(|r| r.get(ti).cloned()).collect();
                    if !times.is_empty() { self.growth_times = times.join(", "); }
                }
            }
            ItemKind::Warranty => {
                if let (Some(sp), Some(sa), Some(lg), Some(rt)) = (col_idx(&header, "ship_period"), col_idx(&header, "sales"), col_idx(&header, "lag_months"), col_idx(&header, "returns")) {
                    let mut sales_map: BTreeMap<usize, u32> = BTreeMap::new();
                    let mut records: Vec<String> = Vec::new();
                    for r in &rows {
                        let ship = r.get(sp).and_then(|s2| s2.parse::<usize>().ok()).unwrap_or(1);
                        let sale = r.get(sa).and_then(|s2| s2.parse::<u32>().ok()).unwrap_or(0);
                        let lag = r.get(lg).and_then(|s2| s2.parse::<usize>().ok()).unwrap_or(0);
                        let ret = r.get(rt).and_then(|s2| s2.parse::<u32>().ok()).unwrap_or(0);
                        sales_map.insert(ship, sale);
                        for _ in 0..ret { records.push(format!("{},{}", ship - 1, ship - 1 + lag)); }
                    }
                    let max_ship = sales_map.keys().max().copied().unwrap_or(0);
                    self.war_sales = (1..=max_ship).map(|k| sales_map.get(&k).copied().unwrap_or(0).to_string()).collect::<Vec<String>>().join(", ");
                    self.war_returns = records.join("\n");
                }
            }
            _ => {}
        }
    }

    // ---------------- CSV import controls ----------------

    /// Bundled dataset picker plus a file path field. Returns the CSV
    /// text when a load button was clicked this frame.
    fn csv_import_controls(&mut self, ui: &mut egui::Ui, id: &str) -> Option<String> {
        let mut out: Option<String> = None;
        ui.horizontal(|ui| {
            ui.label("Bundled dataset:");
            egui::ComboBox::from_id_salt(format!("csv_pick_{}", id))
                .selected_text(self.csv_pick.clone())
                .width(300.0)
                .show_ui(ui, |ui| {
                    for f in BUNDLED_CSV {
                        ui.selectable_value(&mut self.csv_pick, f.to_string(), f);
                    }
                });
            if ui.button("Load bundled CSV").clicked() {
                let path = datasets_dir().join(&self.csv_pick);
                match std::fs::read_to_string(&path) {
                    Ok(t) => {
                        self.note(format!("Loaded bundled dataset {}", self.csv_pick));
                        out = Some(t);
                    }
                    Err(e) => self.note(format!("Dataset load error: {}", e)),
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("CSV file path:");
            ui.add(egui::TextEdit::singleline(&mut self.csv_path).desired_width(320.0));
            if ui.button("Load CSV file").clicked() {
                match std::fs::read_to_string(&self.csv_path) {
                    Ok(t) => {
                        self.note(format!("Loaded CSV file {}", self.csv_path));
                        out = Some(t);
                    }
                    Err(e) => self.note(format!("CSV load error: {}", e)),
                }
            }
        });
        out
    }

    // ---------------- RBD helpers ----------------

    /// Paths (child index lists from the root) of every leaf, in walk order.
    fn leaf_paths(spec: &Value) -> Vec<Vec<usize>> {
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

    fn node_at(spec: &Value, path: &[usize]) -> Option<Value> {
        let mut cur = spec;
        for &i in path {
            cur = cur.get("children")?.get(i)?;
        }
        Some(cur.clone())
    }

    fn replace_at(spec: &Value, path: &[usize], new_node: Value) -> Value {
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

    fn remove_at(spec: &Value, path: &[usize]) -> Value {
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

    fn leaf_pos(node: &Value, idx: usize) -> Vec2 {
        if let Some(a) = node.get("pos").and_then(|v| v.as_array()) {
            if a.len() == 2 {
                return Vec2::new(
                    a[0].as_f64().unwrap_or(30.0) as f32,
                    a[1].as_f64().unwrap_or(90.0) as f32,
                );
            }
        }
        Vec2::new(30.0 + (idx as f32) * 150.0, 90.0)
    }

    /// Write default positions into leaves that do not carry one yet,
    /// so positions survive in the diagram JSON itself.
    fn ensure_rbd_positions(&mut self) {
        let paths = Self::leaf_paths(&self.rbd_spec);
        let mut spec = self.rbd_spec.clone();
        for (i, p) in paths.iter().enumerate() {
            if let Some(mut node) = Self::node_at(&spec, p) {
                if node.get("pos").is_none() {
                    let pos = Self::leaf_pos(&node, i);
                    node["pos"] = json!([pos.x, pos.y]);
                    spec = Self::replace_at(&spec, p, node);
                }
            }
        }
        self.rbd_spec = spec;
        self.rbd_json = serde_json::to_string_pretty(&self.rbd_spec).unwrap_or_default();
    }

    fn set_rbd_spec(&mut self, spec: Value) {
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

    fn rbd_reliability(&self) -> Option<f64> {
        let t = self.rbd_t.trim().parse::<f64>().ok();
        system::evaluate_rbd(&self.rbd_spec, t).ok()
    }

    fn load_rbd_editor(&mut self, idx: usize) {
        let paths = Self::leaf_paths(&self.rbd_spec);
        if let Some(p) = paths.get(idx) {
            if let Some(node) = Self::node_at(&self.rbd_spec, p) {
                self.ed_name = node.get("name").and_then(|v| v.as_str()).unwrap_or("block").to_string();
                self.ed_dist = node.get("dist").and_then(|v| v.as_str()).unwrap_or("weibull2").to_string();
                self.ed_params = params_to_text(&node.get("params").cloned().unwrap_or(json!({})));
                self.ed_mtbf = node.get("mtbf").and_then(|v| v.as_f64()).map(|x| x.to_string()).unwrap_or_default();
                self.ed_mttr = node.get("mttr").and_then(|v| v.as_f64()).map(|x| x.to_string()).unwrap_or_default();
                self.ed_units = node.get("n_units").and_then(|v| v.as_u64()).unwrap_or(2).to_string();
                self.ed_k = node.get("k").and_then(|v| v.as_u64()).unwrap_or(1).to_string();
                self.ed_dormancy = node.get("dormancy").and_then(|v| v.as_f64()).unwrap_or(0.0).to_string();
                self.ed_switch = node.get("switch_reliability").and_then(|v| v.as_f64()).unwrap_or(1.0).to_string();
                self.rbd_sel = Some(idx);
            }
        }
    }

    // ---------------- QCP helpers ----------------

    fn qcp_default(dist: &str, name: &str) -> &'static str {
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

    fn qcp_reset_params(&mut self) {
        self.qcp_params.clear();
        if let Some(spec) = dist_mod::dist_spec(&self.qcp_dist) {
            for (name, _kind) in spec {
                self.qcp_params
                    .insert(name.to_string(), Self::qcp_default(&self.qcp_dist, name).to_string());
            }
        }
    }
}

impl eframe::App for RelKitApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        apply_theme(ctx);
        // Keyboard shortcuts (HIG): Ctrl+S save, Ctrl+O open,
        // Ctrl+Z undo in the RBD editor, Delete removes a selected
        // RBD block, or otherwise the selected project item. Delete
        // and undo are skipped while a text field has focus.
        let focused = ctx.memory(|m| m.focused().is_some());
        let (mut do_save, mut do_open, mut do_delete, mut do_undo) = (false, false, false, false);
        ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) { do_save = true; }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::O) { do_open = true; }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) { do_undo = true; }
            if i.key_pressed(egui::Key::Delete) { do_delete = true; }
        });
        let current_kind = self.selected.and_then(|i| self.items.get(i)).map(|it| it.kind);
        if do_save { self.save_project(); }
        if do_open { self.open_project(); }
        if do_undo && !focused && current_kind == Some(ItemKind::Rbd) { self.undo_rbd(); }
        if do_delete && !focused {
            if current_kind == Some(ItemKind::Rbd) && self.rbd_sel.is_some() {
                self.delete_selected_rbd_block();
            } else if let Some(i) = self.selected {
                let removed = self.items.remove(i);
                self.selected = None;
                self.page = Page::Start;
                self.note(format!("Removed item {}.", removed.name));
            }
        }
        let current_title = match self.selected.and_then(|i| self.items.get(i)) {
            Some(it) => format!("{}: {}", page_title(self.page), it.name),
            None => page_title(Page::Start).to_string(),
        };
        // Unified toolbar layer (Depth): project actions left,
        // current item title center, version right.
        egui::TopBottomPanel::top("topbar")
            .frame(egui::Frame::none().fill(COL_CARD).inner_margin(egui::Margin::symmetric(12.0, 6.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (lrect, _) = ui.allocate_exact_size(Vec2::new(28.0, 28.0), Sense::hover());
                    draw_logo(&ui.painter_at(lrect), lrect);
                    ui.heading("ReliaKit");
                    ui.separator();
                    icon_widget(ui, "save");
                    if ui.button("Save project").clicked() { self.save_project(); }
                    icon_widget(ui, "open");
                    if ui.button("Open project").clicked() { self.open_project(); }
                    ui.separator();
                    ui.strong(current_title.clone());
                    ui.label(format!("Project: {}", self.project_name));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label("v0.2.0");
                    });
                });
            });
        egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Status: {}", self.status));
                if !self.headline.is_empty() {
                    ui.separator();
                    ui.strong(self.headline.clone());
                }
            });
        });
        // Source list sidebar (HIG source list): grouped sections,
        // icon plus label rows, tint selection, resizable.
        egui::SidePanel::left("nav")
            .resizable(true)
            .default_width(280.0)
            .width_range(220.0..=380.0)
            .frame(egui::Frame::none().fill(COL_SIDEBAR).inner_margin(egui::Margin::symmetric(10.0, 8.0)))
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    icon_widget(ui, "home");
                    let sel = self.selected.is_none();
                    if ui.add_sized([ui.available_width(), 28.0], egui::Button::new("Start page").selected(sel)).clicked() {
                        self.select_index(None);
                    }
                });
                ui.add_space(8.0);
                ui.strong("Project");
                ui.add(egui::TextEdit::singleline(&mut self.project_name).desired_width(f32::INFINITY));
                ui.add_space(8.0);
                ui.strong(format!("Items ({})", self.items.len()));
                let items_snapshot: Vec<(u64, ItemKind, String)> = self.items.iter().map(|it| (it.id, it.kind, it.name.clone())).collect();
                let mut clicked: Option<usize> = None;
                egui::ScrollArea::vertical().max_height(430.0).show(ui, |ui| {
                    if items_snapshot.is_empty() {
                        ui.label("No items yet. Add one below, or load an example or dataset from the start page.");
                    }
                    for (i, (_id, kind, name)) in items_snapshot.iter().enumerate() {
                        ui.horizontal(|ui| {
                            icon_widget(ui, kind.icon());
                            let sel = self.selected == Some(i);
                            let label = format!("{}: {}", kind.label(), name);
                            if ui.add_sized([ui.available_width(), 28.0], egui::Button::new(label).selected(sel)).clicked() {
                                clicked = Some(i);
                            }
                        });
                    }
                });
                if let Some(i) = clicked {
                    self.select_index(Some(i));
                }
                ui.separator();
                ui.strong("Add item");
                egui::ComboBox::from_id_salt("new_kind")
                    .selected_text(self.new_kind.clone())
                    .width(240.0)
                    .show_ui(ui, |ui| {
                        for (_k, label, _icon) in KIND_DEFS {
                            ui.selectable_value(&mut self.new_kind, label.to_string(), label);
                        }
                    });
                ui.label("Name:");
                ui.add(egui::TextEdit::singleline(&mut self.new_name).desired_width(240.0));
                ui.horizontal(|ui| {
                    icon_widget(ui, "plus");
                    if ui.button("Add item").clicked() {
                        if let Some(kind) = ItemKind::from_label(&self.new_kind.clone()) {
                            let name = if self.new_name.trim().is_empty() {
                                self.default_item_name(kind)
                            } else {
                                self.new_name.trim().to_string()
                            };
                            let idx = self.create_item_silent(kind, name, json!({}));
                            self.new_name.clear();
                            self.select_index(Some(idx));
                            self.note(format!("Added {} item.", kind.label()));
                        }
                    }
                });
                if self.selected.is_some() {
                    ui.separator();
                    ui.strong("Selected item");
                    ui.add(egui::TextEdit::singleline(&mut self.rename_name).desired_width(240.0));
                    ui.horizontal(|ui| {
                        if ui.button("Rename item").clicked() {
                            if let Some(i) = self.selected {
                                let new_name = self.rename_name.trim().to_string();
                                if !new_name.is_empty() {
                                    self.items[i].name = new_name;
                                    self.note("Item renamed.");
                                }
                            }
                        }
                        icon_widget(ui, "delete");
                        if ui.button("Remove item").clicked() {
                            if let Some(i) = self.selected {
                                let removed = self.items.remove(i);
                                self.selected = None;
                                self.page = Page::Start;
                                self.note(format!("Removed item {}.", removed.name));
                            }
                        }
                    });
                }
                ui.separator();
                ui.strong("Project file");
                ui.add(egui::TextEdit::singleline(&mut self.project_path).desired_width(240.0));
                ui.horizontal(|ui| {
                    icon_widget(ui, "save");
                    if ui.button("Save").clicked() { self.save_project(); }
                    icon_widget(ui, "open");
                    if ui.button("Open").clicked() { self.open_project(); }
                });
                if ui.button("New project").clicked() { self.new_project(); }
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(COL_BG).inner_margin(egui::Margin::symmetric(16.0, 8.0)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(4.0);
                    match current_kind {
                        None => self.page_start(ui),
                        Some(ItemKind::LifeData) => self.page_life(ui),
                        Some(ItemKind::LimitedData) => self.page_limited(ui),
                        Some(ItemKind::Mcmc) => self.page_mcmc(ui),
                        Some(ItemKind::Degradation) => self.page_degradation(ui),
                        Some(ItemKind::Alt) => self.page_alt(ui),
                        Some(ItemKind::Growth) => self.page_growth(ui),
                        Some(ItemKind::Competing) => self.page_competing(ui),
                        Some(ItemKind::Rbd) => self.page_rbd(ui),
                        Some(ItemKind::FaultTree) => self.page_fault_tree(ui),
                        Some(ItemKind::Fmea) => self.page_fmea(ui),
                        Some(ItemKind::Warranty) => self.page_warranty(ui),
                        Some(ItemKind::Prediction) => self.page_prediction(ui),
                        Some(ItemKind::Doe) => self.page_doe(ui),
                        Some(ItemKind::Flowchart) => self.page_flowchart(ui),
                        Some(ItemKind::Markov) => self.page_markov(ui),
                        Some(ItemKind::Qcp) => self.page_qcp(ui),
                        Some(ItemKind::Rcm) => self.page_rcm(ui),
                        Some(ItemKind::Fracas) => self.page_fracas(ui),
                    }
                    ui.add_space(24.0);
                });
            });
    }
}

/// Render "label: value" lines as a labeled grid. Lines without a
/// colon are shown as a single spanning label.
fn result_box(ui: &mut egui::Ui, text: &str) {
    if text.is_empty() {
        return;
    }
    ui.add_space(8.0);
    egui::Frame::none()
        .fill(COL_CARD)
        .stroke(Stroke::new(1.0_f32, COL_BORDER))
        .rounding(6.0)
        .inner_margin(16.0)
        .show(ui, |ui| {
            egui::Grid::new(format!("grid_{:p}", text.as_ptr()))
                .num_columns(2)
                .spacing([16.0, 3.0])
                .striped(true)
                .show(ui, |ui| {
                    for line in text.lines() {
                        if let Some((a, b)) = line.split_once(':') {
                            ui.label(a.trim());
                            ui.label(b.trim());
                        } else {
                            ui.label(line);
                            ui.label("");
                        }
                        ui.end_row();
                    }
                });
        });
}

fn grid_pairs(ui: &mut egui::Ui, id: &str, rows: &[(String, String)]) {
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([16.0, 3.0])
        .striped(true)
        .show(ui, |ui| {
            for (a, b) in rows {
                ui.label(a);
                ui.label(b);
                ui.end_row();
            }
        });
}

fn dist_combo(ui: &mut egui::Ui, id: &str, current: &mut String) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(format!("{} ({})", current.as_str(), dist_mod::dist_label(current)))
        .show_ui(ui, |ui| {
            for d in ALL_DISTS {
                ui.selectable_value(current, d.to_string(), format!("{} ({})", d, dist_mod::dist_label(d)));
            }
        });
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(12.0);
    ui.label(egui::RichText::new(title).size(15.0).strong());
    ui.add_space(4.0);
}

impl RelKitApp {
    fn page_life(&mut self, ui: &mut egui::Ui) {
        ui.heading("Life Data");
        section(ui, "Data");
        ui.label("Times (comma separated; suffix c or + = censored):");
        ui.add(egui::TextEdit::multiline(&mut self.life_times).desired_rows(4).desired_width(f32::INFINITY));
        if let Some(text) = self.csv_import_controls(ui, "life") {
            let (header, rows) = csv_table(&text);
            let ti = col_idx(&header, "time");
            let fi = col_idx(&header, "failed");
            let ci = col_idx(&header, "censored");
            let ni = col_idx(&header, "count");
            match (ti, fi, ci) {
                (Some(t), Some(f), _) | (Some(t), _, Some(f)) => {
                    let is_failed_col = col_idx(&header, "failed").is_some();
                    let mut times: Vec<f64> = Vec::new();
                    let mut failed: Vec<bool> = Vec::new();
                    for r in &rows {
                        if let (Some(tv), Some(fv)) = (r.get(t).and_then(|s| s.parse::<f64>().ok()), r.get(f).and_then(|s| s.parse::<f64>().ok())) {
                            let count = ni.and_then(|n| r.get(n)).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
                            let is_failed = if is_failed_col { fv != 0.0 } else { fv == 0.0 };
                            for _ in 0..count {
                                times.push(tv);
                                failed.push(is_failed);
                            }
                        }
                    }
                    if !times.is_empty() {
                        self.life_times = format_life(&times, &failed);
                        self.mcmc_times = self.life_times.clone();
                        self.note(format!("Imported {} life data rows from CSV.", times.len()));
                    } else {
                        self.note("CSV import found no usable time rows.");
                    }
                }
                _ => self.note("Life CSV needs a time column plus failed or censored column."),
            }
        }
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
                    let mut rows: Vec<(String, String)> = vec![
                        ("Fit".into(), fit.summary().replace('\n', " | ")),
                        (format!("R({:.0})", t), format!("{:.4}", fit.reliability(t))),
                        ("95% bounds".into(), format!("{:.4} to {:.4}", lo, hi)),
                        ("Lower bound (one-sided 95%)".into(), format!("{:.4}", fit.reliability_lower_bound(t, 0.05))),
                        ("B10".into(), format!("{:.1}", fit.b_life(0.10))),
                        ("Mean life".into(), format!("{:.1}", fit.mean_life())),
                    ];
                    for (k, v) in &fit.params {
                        rows.push((format!("Parameter {}", k), format!("{:.6}", v)));
                    }
                    self.life_result = rows.iter().map(|(a, b)| format!("{}: {}", a, b)).collect::<Vec<String>>().join("\n");
                    self.headline = format!("R({:.0}) = {:.4}", t, fit.reliability(t));
                    self.note("Life data fit complete.");
                    self.life_fit = Some(fit);
                }
                Err(e) => {
                    self.life_result = format!("Error: {}", e);
                    self.life_fit = None;
                    self.note("Life data fit failed.");
                }
            }
        }
        result_box(ui, &self.life_result.clone());
        section(ui, "Send this fit to another item");
        if self.life_fit.is_some() {
            let rbd_items: Vec<(u64, String)> = self.items.iter().filter(|it| it.kind == ItemKind::Rbd).map(|it| (it.id, it.name.clone())).collect();
            if rbd_items.is_empty() {
                ui.label("No RBD item yet. Add one from the project panel, then assign the fit.");
                if ui.button("Create RBD item").clicked() {
                    let idx = self.create_item_silent(ItemKind::Rbd, self.default_item_name(ItemKind::Rbd), json!({}));
                    self.flow_target_id = self.items[idx].id;
                    self.note("Created an RBD item with the built-in diagram. Assign the fit to one of its blocks.");
                }
            } else {
                if self.flow_target_id == 0 || !rbd_items.iter().any(|(id, _)| *id == self.flow_target_id) {
                    self.flow_target_id = rbd_items[0].0;
                }
                ui.horizontal(|ui| {
                    ui.label("Target RBD item:");
                    let cur_name = rbd_items.iter().find(|(id, _)| *id == self.flow_target_id).map(|(_, n)| n.clone()).unwrap_or_default();
                    egui::ComboBox::from_id_salt("flow_rbd_target")
                        .selected_text(cur_name)
                        .show_ui(ui, |ui| {
                            for (id, name) in &rbd_items {
                                ui.selectable_value(&mut self.flow_target_id, *id, name.clone());
                            }
                        });
                });
                let target_state = self.items.iter().find(|it| it.id == self.flow_target_id).map(|it| it.state.clone()).unwrap_or(json!({}));
                let tspec = target_state.get("rbd_spec").cloned().filter(|x| !x.is_null())
                    .unwrap_or_else(|| serde_json::from_str(EXAMPLE3_JSON).unwrap_or(json!({})));
                let tpaths = Self::leaf_paths(&tspec);
                let block_names: Vec<String> = tpaths.iter().enumerate().map(|(i, path)| {
                    Self::node_at(&tspec, path).and_then(|n| n.get("name").and_then(|v| v.as_str()).map(|x| x.to_string()))
                        .unwrap_or_else(|| format!("block {}", i + 1))
                }).collect();
                if block_names.is_empty() {
                    ui.label("The target RBD item has no blocks yet. Open it and add a block, then assign.");
                } else {
                    if self.flow_block_idx >= block_names.len() {
                        self.flow_block_idx = 0;
                    }
                    ui.horizontal(|ui| {
                        ui.label("Block:");
                        egui::ComboBox::from_id_salt("flow_rbd_block")
                            .selected_text(block_names[self.flow_block_idx].clone())
                            .show_ui(ui, |ui| {
                                for (i, bn) in block_names.iter().enumerate() {
                                    ui.selectable_value(&mut self.flow_block_idx, i, bn.clone());
                                }
                            });
                        icon_widget(ui, "link");
                        if ui.button("Assign fit to RBD block").clicked() {
                            self.flow_assign_fit_to_rbd();
                        }
                    });
                }
            }
            ui.horizontal(|ui| {
                icon_widget(ui, "link");
                if ui.button("Send fit to QCP").clicked() {
                    self.flow_send_fit_to_qcp();
                }
                if ui.button("Send fit to Prediction").clicked() {
                    self.flow_send_fit_to_prediction();
                }
            });
        } else {
            ui.label("Run a fit first, then the send actions appear here.");
        }
        if let Some(fit) = self.life_fit.clone() {
            section(ui, "Plots");
            ui.checkbox(&mut self.life_show_prob, "Show probability plot instead of reliability curve");
            if self.life_show_prob {
                let (_times, f_emp, f_theo) = dist_mod::probability_plot_data(&fit);
                let pts: Vec<[f64; 2]> = f_emp.iter().zip(&f_theo).map(|(a, b)| [*a, *b]).collect();
                Plot::new("life_prob_plot")
                    .x_axis_label("Empirical F (median rank)")
                    .y_axis_label("Fitted F")
                    .show(ui, |plot_ui| {
                        plot_ui.line(Line::new(PlotPoints::from(vec![[0.0, 0.0], [1.0, 1.0]])).name("y = x").color(COL_MUTED));
                        plot_ui.points(Points::new(PlotPoints::from(pts)).name("Failures").color(COL_TINT));
                    });
            } else {
                let tmax = fit.times.iter().cloned().fold(1.0f64, f64::max) * 1.15;
                let pts: Vec<[f64; 2]> = (0..=100)
                    .map(|i| {
                        let t = tmax * i as f64 / 100.0;
                        [t, fit.reliability(t)]
                    })
                    .collect();
                Plot::new("life_rel_plot")
                    .x_axis_label("Time")
                    .y_axis_label("Reliability R(t)")
                    .show(ui, |plot_ui| {
                        plot_ui.line(Line::new(PlotPoints::from(pts)).name(dist_mod::dist_label(&fit.dist)).color(COL_TINT));
                    });
            }
        }
        section(ui, "Distribution wizard");
        ui.label("Rank candidate distributions by AIC on the current life data.");
        if ui.button("Rank distributions").clicked() {
            let (times, failed) = parse_life(&self.life_times);
            self.wizard_ranks = wizard::rank_distributions(&times, Some(&failed), wizard::RANKABLE_DISTS);
            self.wizard_note = format!("Ranked {} candidates.", self.wizard_ranks.len());
            self.note("Distribution wizard ranking complete.");
        }
        if !self.wizard_note.is_empty() {
            ui.label(self.wizard_note.clone());
        }
        if !self.wizard_ranks.is_empty() {
            let ranks = self.wizard_ranks.clone();
            egui::Grid::new("wizard_grid")
                .num_columns(6)
                .spacing([12.0, 3.0])
                .striped(true)
                .show(ui, |ui| {
                    for h in ["Distribution", "Loglik", "AIC", "BIC", "AD", "Action"] {
                        ui.strong(h);
                    }
                    ui.end_row();
                    for r in &ranks {
                        ui.label(dist_mod::dist_label(&r.dist));
                        ui.label(if r.success { format!("{:.2}", r.loglik) } else { "fit failed".into() });
                        ui.label(if r.success { format!("{:.2}", r.aic) } else { "-".into() });
                        ui.label(if r.success { format!("{:.2}", r.bic) } else { "-".into() });
                        ui.label(match r.ad_statistic {
                            Some(v) => format!("{:.3}", v),
                            None => "-".into(),
                        });
                        if r.success && ui.button(format!("Use {}", r.dist)).clicked() {
                            let data = self.life_times.clone();
                            let dist = r.dist.clone();
                            let idx = self.create_item_silent(ItemKind::LifeData, self.default_item_name(ItemKind::LifeData), json!({}));
                            self.select_index(Some(idx));
                            self.life_times = data;
                            self.life_dist = dist.clone();
                            let (times, failed) = parse_life(&self.life_times);
                            if let Ok(fit) = dist_mod::fit_mle(&dist, &times, Some(&failed)) {
                                self.headline = format!("Wizard fit: {}", dist_mod::dist_label(&fit.dist));
                                self.life_result = fit.summary().replace('\n', " | ");
                                self.life_fit = Some(fit);
                            }
                            self.note(format!("Wizard created a new Life Data item with {} preselected and the current data loaded.", dist));
                        } else {
                            ui.label("");
                        }
                        ui.end_row();
                    }
                });
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
                    let mut rows: Vec<(String, String)> = vec![
                        ("Pooled beta".into(), format!("{:.3}", h.pooled_beta)),
                        ("Mu log eta".into(), format!("{:.3}", h.mu_log_eta)),
                        ("Tau squared".into(), format!("{:.4}", h.tau2)),
                    ];
                    for (i, f) in h.fits.iter().enumerate() {
                        rows.push((
                            format!("Lot {} shrunk eta", i + 1),
                            format!("{:.1} (MLE eta {:.1}, shrink B {:.2})", f.params["eta"], h.group_fits[i].params["eta"], h.b[i]),
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
                        let calib: Vec<f64> = groups.iter().flat_map(|(t, _)| t.iter().cloned()).collect();
                        rows.push(("Conformal mission time (alpha 0.10)".into(), format!("{:.1}", bayes::conformal_mission_time(first, &calib, 0.10))));
                        rows.push(("Conformal R lower bound at t=1000 (alpha 0.05)".into(), format!("{:.3}", bayes::conformal_reliability_lower(&calib, 1000.0, 0.05))));
                    }
                    self.limited_result = rows.iter().map(|(a, b)| format!("{}: {}", a, b)).collect::<Vec<String>>().join("\n");
                    self.note("Hierarchical fit complete.");
                }
                Err(e) => self.limited_result = format!("Error: {}", e),
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
                    let mut rows: Vec<(String, String)> = Vec::new();
                    for (name, ps) in r.summary() {
                        rows.push((
                            format!("{} posterior", name),
                            format!("mean {:.3}, sd {:.3}, q05 {:.3}, median {:.3}, q95 {:.3}", ps.mean, ps.sd, ps.q05, ps.q50, ps.q95),
                        ));
                    }
                    // R-hat and ESS are plain numbers, one labeled row
                    // per parameter. Before this fix they printed as a
                    // quoted string array such as ["1.020", "1.012"].
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
                    let t = fnum(&self.mcmc_t, 1000.0);
                    let ci = r.reliability_credible(t, 0.90, 0);
                    rows.push((format!("R({:.0}) credible", t), format!("mean {:.3}, 90% CI {:.3} to {:.3}", ci.mean, ci.lower, ci.upper)));
                    rows.push(("Posterior predictive p-value".into(), format!("{:.3}", r.posterior_predictive_pvalue(7))));
                    self.mcmc_result = rows.iter().map(|(a, b)| format!("{}: {}", a, b)).collect::<Vec<String>>().join("\n");
                    self.headline = format!("R({:.0}) mean {:.3}", t, ci.mean);
                    self.note("MCMC run complete.");
                }
                Err(e) => self.mcmc_result = format!("Error: {}", e),
            }
        }
        result_box(ui, &self.mcmc_result.clone());
    }

    fn deg_parse_units(&self) -> Vec<(Vec<f64>, Vec<f64>)> {
        let mut units: Vec<(Vec<f64>, Vec<f64>)> = Vec::new();
        for line in self.deg_units.lines() {
            if let Some((a, b)) = line.split_once('|') {
                units.push((parse_numbers(a), parse_numbers(b)));
            }
        }
        units
    }

    fn page_degradation(&mut self, ui: &mut egui::Ui) {
        ui.heading("Degradation");
        ui.label("One unit per line: times | measurements");
        ui.add(egui::TextEdit::multiline(&mut self.deg_units).desired_rows(5).desired_width(f32::INFINITY));
        if let Some(text) = self.csv_import_controls(ui, "deg") {
            let (header, rows) = csv_table(&text);
            if let (Some(ui_i), Some(ti), Some(vi)) = (col_idx(&header, "unit"), col_idx(&header, "time"), col_idx(&header, "value")) {
                let mut units: Vec<(Vec<f64>, Vec<f64>)> = Vec::new();
                let mut order: Vec<String> = Vec::new();
                for r in &rows {
                    let unit = r.get(ui_i).cloned().unwrap_or_default();
                    if !order.contains(&unit) {
                        order.push(unit.clone());
                        units.push((Vec::new(), Vec::new()));
                    }
                    let k = order.iter().position(|u| *u == unit).unwrap_or(0);
                    if let (Some(t), Some(v)) = (r.get(ti).and_then(|s| s.parse::<f64>().ok()), r.get(vi).and_then(|s| s.parse::<f64>().ok())) {
                        units[k].0.push(t);
                        units[k].1.push(v);
                    }
                }
                if !units.is_empty() {
                    self.deg_units = units
                        .iter()
                        .map(|(t, y)| {
                            format!(
                                "{} | {}",
                                t.iter().map(|v| format!("{:.4}", v)).collect::<Vec<String>>().join(","),
                                y.iter().map(|v| format!("{:.4}", v)).collect::<Vec<String>>().join(",")
                            )
                        })
                        .collect::<Vec<String>>()
                        .join("\n");
                    self.note(format!("Imported degradation data for {} units.", units.len()));
                }
            } else {
                self.note("Degradation CSV needs unit, time, value columns.");
            }
        }
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
            let units = self.deg_parse_units();
            self.deg_parsed = units.clone();
            match degradation::degradation_analysis(
                &units,
                fnum(&self.deg_threshold, 70.0),
                &self.deg_model,
                &self.deg_dist,
                self.deg_increasing,
            ) {
                Ok(r) => {
                    let mut rows: Vec<(String, String)> = Vec::new();
                    for (i, pt) in r.pseudo_times.iter().enumerate() {
                        rows.push((
                            format!("Unit {} pseudo-failure time", i + 1),
                            if pt.is_finite() { format!("{:.1} ({})", pt, if r.failed[i] { "failed" } else { "censored" }) } else { "never crosses".into() },
                        ));
                    }
                    if let Some(f) = &r.fit {
                        rows.push(("Life fit".into(), f.summary().replace('\n', " | ")));
                    }
                    self.deg_result = rows.iter().map(|(a, b)| format!("{}: {}", a, b)).collect::<Vec<String>>().join("\n");
                    self.deg_data = Some(r);
                    self.note("Degradation analysis complete.");
                }
                Err(e) => self.deg_result = format!("Error: {}", e),
            }
        }
        result_box(ui, &self.deg_result.clone());
        if let Some(r) = &self.deg_data {
            section(ui, "Degradation paths");
            let units = self.deg_parsed.clone();
            let model = self.deg_model.clone();
            let paths = r.paths.clone();
            Plot::new("deg_plot")
                .x_axis_label("Time")
                .y_axis_label("Measurement")
                .show(ui, |plot_ui| {
                    for (i, (t, y)) in units.iter().enumerate() {
                        let raw: Vec<[f64; 2]> = t.iter().zip(y).map(|(a, b)| [*a, *b]).collect();
                        plot_ui.points(Points::new(PlotPoints::from(raw)).name(format!("Unit {} data", i + 1)).color(SERIES_COLORS[i % SERIES_COLORS.len()]));
                        if let Some((a, b)) = paths.get(i) {
                            let tmax = t.iter().cloned().fold(1.0f64, f64::max) * 1.2;
                            let line_pts: Vec<[f64; 2]> = (0..=40)
                                .map(|k| {
                                    let tt = tmax * k as f64 / 40.0;
                                    [tt, degradation::path_value(*a, *b, tt, &model)]
                                })
                                .collect();
                            plot_ui.line(Line::new(PlotPoints::from(line_pts)).name(format!("Unit {} path", i + 1)).color(SERIES_COLORS[i % SERIES_COLORS.len()]));
                        }
                    }
                });
        }
    }

    fn page_alt(&mut self, ui: &mut egui::Ui) {
        ui.heading("Accelerated Life Test");
        ui.label("One unit per line: stress,time (eyring: stress,stress2,time). Optional trailing failed flag 1 or 0 defaults to failed.");
        ui.add(egui::TextEdit::multiline(&mut self.alt_data).desired_rows(5).desired_width(f32::INFINITY));
        if let Some(text) = self.csv_import_controls(ui, "alt") {
            let (header, rows) = csv_table(&text);
            if let (Some(si), Some(ti)) = (col_idx(&header, "stress"), col_idx(&header, "time")) {
                let fi = col_idx(&header, "failed");
                let s2i = col_idx(&header, "stress2");
                let mut lines: Vec<String> = Vec::new();
                for r in &rows {
                    let s = r.get(si).cloned().unwrap_or_default();
                    let t = r.get(ti).cloned().unwrap_or_default();
                    let f = fi.and_then(|i| r.get(i)).cloned().unwrap_or_else(|| "1".into());
                    match s2i {
                        Some(i2) => lines.push(format!("{},{},{},{}", s, r.get(i2).cloned().unwrap_or_default(), t, f)),
                        None => lines.push(format!("{},{},{}", s, t, f)),
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
                        failed.push(if vals.len() >= 4 { vals[3] != 0.0 } else { true });
                    } else {
                        stress.push(vec![vals[0]]);
                        times.push(vals[1]);
                        failed.push(if vals.len() >= 3 { vals[2] != 0.0 } else { true });
                    }
                }
            }
            let use_s = parse_numbers(&self.alt_use_stress);
            let t = fnum(&self.alt_t, 1000.0);
            let mut rows: Vec<(String, String)> = Vec::new();
            match alt_mod::fit_alt(&self.alt_model, &stress, &times) {
                Ok(f) => {
                    rows.push(("Least squares coefficients".into(), f.coef.iter().map(|(k, v)| format!("{}={:.4}", k, v)).collect::<Vec<String>>().join(", ")));
                    rows.push(("Least squares sigma".into(), format!("{:.3}", f.sigma)));
                    rows.push(("Median life at use stress".into(), format!("{:.1}", f.median_life(&use_s))));
                    rows.push((format!("Least squares R({:.0})", t), format!("{:.4}", f.reliability(t, &use_s))));
                    if let Some(ea) = f.activation_energy_ev() {
                        rows.push(("Activation energy".into(), format!("{:.3} eV", ea)));
                    }
                }
                Err(e) => rows.push(("Least squares error".into(), e)),
            }
            match alt_mod::fit_alt_mle(&self.alt_model, &stress, &times, Some(&failed)) {
                Ok(f) => {
                    let (lo, hi) = f.reliability_bounds(t, &use_s, 0.05);
                    rows.push(("MLE coefficients".into(), f.coef.iter().map(|(k, v)| format!("{}={:.4}", k, v)).collect::<Vec<String>>().join(", ")));
                    rows.push(("MLE sigma".into(), format!("{:.3}", f.sigma)));
                    rows.push(("MLE loglik".into(), format!("{:.2}", f.loglik)));
                    rows.push((format!("MLE R({:.0}) at use stress", t), format!("{:.4} (bounds {:.4} to {:.4})", f.reliability(t, &use_s), lo, hi)));
                    if let Some(ea) = f.activation_energy_ev() {
                        rows.push(("MLE activation energy".into(), format!("{:.3} eV", ea)));
                    }
                }
                Err(e) => rows.push(("MLE error".into(), e)),
            }
            self.alt_result = rows.iter().map(|(a, b)| format!("{}: {}", a, b)).collect::<Vec<String>>().join("\n");
            self.note("ALT fit complete.");
        }
        result_box(ui, &self.alt_result.clone());
    }

    fn page_growth(&mut self, ui: &mut egui::Ui) {
        ui.heading("Reliability Growth");
        ui.label("Failure times (comma separated):");
        ui.add(egui::TextEdit::multiline(&mut self.growth_times).desired_rows(3).desired_width(f32::INFINITY));
        if let Some(text) = self.csv_import_controls(ui, "growth") {
            let (header, rows) = csv_table(&text);
            if let Some(ti) = col_idx(&header, "time") {
                let times: Vec<String> = rows.iter().filter_map(|r| r.get(ti).cloned()).collect();
                if !times.is_empty() {
                    self.growth_times = times.join(", ");
                    self.note(format!("Imported {} growth failure times.", times.len()));
                }
            } else {
                self.note("Growth CSV needs a time column (time,failures format).");
            }
        }
        if ui.button("Run growth fit").clicked() {
            let times = parse_numbers(&self.growth_times);
            let ca = growth::fit_crow_amsaa(&times, None);
            let mut rows: Vec<(String, String)> = vec![
                ("Crow-AMSAA beta".into(), format!("{:.3}", ca.beta)),
                ("Crow-AMSAA lambda".into(), format!("{:.5}", ca.lam)),
                ("Failures n".into(), format!("{}", ca.n_fail)),
                (format!("Instantaneous MTBF at T={:.0}", ca.t_end), format!("{:.1}", ca.instantaneous_mtbf(ca.t_end))),
                ("Cumulative MTBF".into(), format!("{:.1}", ca.cumulative_mtbf(ca.t_end))),
            ];
            match growth::fit_duane(&times) {
                Ok(du) => {
                    rows.push(("Duane alpha".into(), format!("{:.3}", du.alpha)));
                    rows.push(("Duane instantaneous MTBF at T".into(), format!("{:.1}", du.instantaneous_mtbf_at(ca.t_end))));
                }
                Err(e) => rows.push(("Duane error".into(), e)),
            }
            self.growth_result = rows.iter().map(|(a, b)| format!("{}: {}", a, b)).collect::<Vec<String>>().join("\n");
            self.growth_fit = Some(ca);
            self.note("Growth fit complete.");
        }
        result_box(ui, &self.growth_result.clone());
        if let Some(ca) = &self.growth_fit {
            section(ui, "Growth curve");
            let times = parse_numbers(&self.growth_times);
            let mut sorted = times.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let obs: Vec<[f64; 2]> = sorted.iter().enumerate().map(|(i, t)| [*t, (i + 1) as f64]).collect();
            let tmax = ca.t_end.max(1.0);
            let expected: Vec<[f64; 2]> = (0..=60)
                .map(|k| {
                    let t = tmax * k as f64 / 60.0;
                    [t, ca.lam * t.powf(ca.beta)]
                })
                .collect();
            Plot::new("growth_plot")
                .x_axis_label("Cumulative test time")
                .y_axis_label("Cumulative failures")
                .show(ui, |plot_ui| {
                    plot_ui.line(Line::new(PlotPoints::from(expected)).name("Crow-AMSAA expected").color(COL_TINT));
                    plot_ui.points(Points::new(PlotPoints::from(obs)).name("Observed failures").color(COL_SERIES2));
                });
        }
    }

    fn page_competing(&mut self, ui: &mut egui::Ui) {
        ui.heading("Competing Risks");
        ui.label("One unit per line: time,mode (blank mode = censored):");
        ui.add(egui::TextEdit::multiline(&mut self.comp_data).desired_rows(5).desired_width(f32::INFINITY));
        if let Some(text) = self.csv_import_controls(ui, "comp") {
            let (header, rows) = csv_table(&text);
            if let Some(ti) = col_idx(&header, "time") {
                let fi = col_idx(&header, "failed");
                let ci = col_idx(&header, "cause");
                let mut lines: Vec<String> = Vec::new();
                for r in &rows {
                    let t = r.get(ti).cloned().unwrap_or_default();
                    let failed = fi.and_then(|i| r.get(i)).map(|s| s != "0").unwrap_or(true);
                    let cause = ci.and_then(|i| r.get(i)).cloned().unwrap_or_default();
                    lines.push(format!("{},{}", t, if failed { cause } else { String::new() }));
                }
                if !lines.is_empty() {
                    self.comp_data = lines.join("\n");
                    self.note(format!("Imported {} competing risks rows.", lines.len()));
                }
            } else {
                self.note("Competing risks CSV needs time, failed, cause columns.");
            }
        }
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
                    let mut rows: Vec<(String, String)> = Vec::new();
                    for (name, fit) in &cr.modes {
                        rows.push((format!("Mode {} fit", name), fit.summary().replace('\n', " | ")));
                    }
                    rows.push((format!("Overall reliability at {:.0}", t), format!("{:.4}", competing::overall_reliability(&cr, t))));
                    for m in &modes {
                        rows.push((format!("CIF {} at {:.0}", m, t), format!("{:.4}", competing::cumulative_incidence(&cr, m, t))));
                    }
                    for (name, p) in competing::failure_probabilities(&cr, t) {
                        rows.push((format!("Failure probability {} at {:.0}", name, t), format!("{:.4}", p)));
                    }
                    self.comp_result = rows.iter().map(|(a, b)| format!("{}: {}", a, b)).collect::<Vec<String>>().join("\n");
                    self.note("Competing risks fit complete.");
                }
                Err(e) => self.comp_result = format!("Error: {}", e),
            }
        }
        result_box(ui, &self.comp_result.clone());
    }
}

impl RelKitApp {
    fn insert_into_root_series(&mut self, node: Value) {
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

    fn page_rbd(&mut self, ui: &mut egui::Ui) {
        ui.heading("System RBD");
        ui.label("Palette: click a palette item to arm placement, then click the canvas to place it. Click a placed block to select and edit it. Drag blocks to move them.");
        // Palette
        ui.horizontal(|ui| {
            ui.label("Palette:");
            for (kind, label) in [
                ("block", "Single block"),
                ("standby_group", "Standby group"),
                ("parallel", "Parallel group"),
                ("kofn", "k-out-of-n group"),
            ] {
                let armed = self.rbd_armed.as_deref() == Some(kind);
                if ui.selectable_label(armed, label).clicked() {
                    self.rbd_armed = if armed { None } else { Some(kind.to_string()) };
                }
            }
            if self.rbd_armed.is_some() {
                ui.label("Placement armed. Click the canvas to place.");
            }
        });
        // Canvas
        let canvas_size = Vec2::new(900.0, 260.0);
        let (canvas_resp, _) = ui.allocate_painter(canvas_size, Sense::click_and_drag());
        let rect = canvas_resp.rect;
        let paths = Self::leaf_paths(&self.rbd_spec);
        let mut leaves: Vec<(String, String, Vec2)> = Vec::new();
        for (i, p) in paths.iter().enumerate() {
            if let Some(node) = Self::node_at(&self.rbd_spec, p) {
                let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("block").to_string();
                let kind = if node.get("type").and_then(|v| v.as_str()) == Some("standby_group") { "standby".to_string() } else { "block".to_string() };
                leaves.push((name, kind, Self::leaf_pos(&node, i)));
            }
        }
        let block_size = Vec2::new(128.0, 52.0);
        // Drag and select interactions
        let mut drag_updates: Vec<(usize, Vec2)> = Vec::new();
        let mut clicked_leaf: Option<usize> = None;
        for (i, (_name, _kind, pos)) in leaves.iter().enumerate() {
            let brect = Rect::from_min_size(rect.min + *pos, block_size);
            let resp = ui.interact(brect, egui::Id::new(format!("rbd_block_{}", i)), Sense::drag());
            if resp.dragged() {
                let mut np = *pos + resp.drag_delta();
                np.x = np.x.clamp(0.0, (rect.width() - block_size.x).max(0.0));
                np.y = np.y.clamp(0.0, (rect.height() - block_size.y).max(0.0));
                drag_updates.push((i, np));
            }
            if resp.clicked() {
                clicked_leaf = Some(i);
            }
        }
        if !drag_updates.is_empty() {
            let mut spec = self.rbd_spec.clone();
            for (i, np) in drag_updates {
                if let Some(p) = paths.get(i) {
                    if let Some(mut node) = Self::node_at(&spec, p) {
                        node["pos"] = json!([np.x, np.y]);
                        spec = Self::replace_at(&spec, p, node);
                    }
                }
            }
            self.rbd_spec = spec;
            self.rbd_json = serde_json::to_string_pretty(&self.rbd_spec).unwrap_or_default();
        }
        if let Some(i) = clicked_leaf {
            self.rbd_armed = None;
            self.load_rbd_editor(i);
        }
        // Placement on canvas click
        if canvas_resp.clicked() && self.rbd_armed.is_some() {
            if let Some(click) = canvas_resp.interact_pointer_pos() {
                let local = click - rect.min;
                let over_block = leaves.iter().any(|(_, _, pos)| {
                    let br = Rect::from_min_size(pos.to_pos2(), block_size);
                    br.contains(local.to_pos2())
                });
                if !over_block {
                    let kind = self.rbd_armed.clone().unwrap_or_else(|| "block".into());
                    let n = leaves.len() + 1;
                    let base = Vec2::new((local.x - 20.0).max(0.0), (local.y - 20.0).max(0.0));
                    let mk_block = |name: &str, pos: Vec2| {
                        json!({"type": "block", "name": name, "dist": "weibull2",
                               "params": {"beta": 1.5, "eta": 5000}, "mtbf": 5000, "mttr": 10,
                               "pos": [pos.x, pos.y]})
                    };
                    match kind.as_str() {
                        "standby_group" => {
                            let node = json!({"type": "standby_group", "name": format!("standby {}", n), "n_units": 2,
                                              "dist": "exponential", "params": {"theta": 5000}, "mtbf": 5000, "mttr": 8,
                                              "dormancy": 0.0, "switch_reliability": 0.98, "swap_time": 0.5,
                                              "pos": [base.x, base.y]});
                            self.insert_into_root_series(node);
                        }
                        "parallel" => {
                            let node = json!({"type": "parallel", "children": [
                                mk_block(&format!("parallel {} A", n), base),
                                mk_block(&format!("parallel {} B", n), base + Vec2::new(150.0, 0.0)),
                            ]});
                            self.insert_into_root_series(node);
                        }
                        "kofn" => {
                            let node = json!({"type": "kofn", "k": 2, "children": [
                                mk_block(&format!("kofn {} A", n), base),
                                mk_block(&format!("kofn {} B", n), base + Vec2::new(150.0, 0.0)),
                                mk_block(&format!("kofn {} C", n), base + Vec2::new(300.0, 0.0)),
                            ]});
                            self.insert_into_root_series(node);
                        }
                        _ => {
                            let node = mk_block(&format!("block {}", n), base);
                            self.insert_into_root_series(node);
                        }
                    }
                    self.rbd_status = format!("Placed {} from palette.", kind);
                    self.note(self.rbd_status.clone());
                    self.rbd_armed = None;
                }
            }
        }
        // Paint canvas
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, COL_BG);
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, Color32::GRAY));
        let mut order: Vec<usize> = (0..leaves.len()).collect();
        order.sort_by(|&a, &b| leaves[a].2.x.partial_cmp(&leaves[b].2.x).unwrap_or(std::cmp::Ordering::Equal));
        for w in order.windows(2) {
            let pa = leaves[w[0]].2;
            let pb = leaves[w[1]].2;
            let a = Pos2::new(rect.left() + pa.x + block_size.x, rect.top() + pa.y + block_size.y / 2.0);
            let b = Pos2::new(rect.left() + pb.x, rect.top() + pb.y + block_size.y / 2.0);
            painter.line_segment([a, b], Stroke::new(1.5_f32, COL_SERIES2));
        }
        for (i, (name, kind, pos)) in leaves.iter().enumerate() {
            let brect = Rect::from_min_size(rect.min + *pos, block_size);
            let selected = self.rbd_sel == Some(i);
            let fill = COL_CARD;
            painter.rect_filled(brect, 4.0, fill);
            painter.rect_stroke(brect, 4.0, Stroke::new(if selected { 2.5_f32 } else { 1.0_f32 }, if selected { COL_TINT } else { COL_BORDER }));
            painter.text(brect.center_top() + Vec2::new(0.0, 10.0), Align2::CENTER_CENTER, name, FontId::proportional(12.0), Color32::WHITE);
            painter.text(brect.center_top() + Vec2::new(0.0, 30.0), Align2::CENTER_CENTER, kind, FontId::proportional(10.0), Color32::LIGHT_GRAY);
        }
        // Property editor
        if let Some(sel) = self.rbd_sel {
            if let Some(p) = Self::leaf_paths(&self.rbd_spec).get(sel).cloned() {
                if let Some(node) = Self::node_at(&self.rbd_spec, &p) {
                    let is_standby = node.get("type").and_then(|v| v.as_str()) == Some("standby_group");
                    section(ui, "Block inspector");
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.add(egui::TextEdit::singleline(&mut self.ed_name).desired_width(140.0));
                        ui.label("Distribution:");
                        dist_combo(ui, "ed_dist", &mut self.ed_dist);
                        ui.label("Params:");
                        ui.add(egui::TextEdit::singleline(&mut self.ed_params).desired_width(200.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("MTBF:");
                        ui.add(egui::TextEdit::singleline(&mut self.ed_mtbf).desired_width(70.0));
                        ui.label("MTTR:");
                        ui.add(egui::TextEdit::singleline(&mut self.ed_mttr).desired_width(60.0));
                        if is_standby {
                            ui.label("Units:");
                            ui.add(egui::TextEdit::singleline(&mut self.ed_units).desired_width(40.0));
                            ui.label("Dormancy:");
                            ui.add(egui::TextEdit::singleline(&mut self.ed_dormancy).desired_width(50.0));
                            ui.label("Switch reliability:");
                            ui.add(egui::TextEdit::singleline(&mut self.ed_switch).desired_width(50.0));
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Apply properties").clicked() {
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
                        if ui.button("Duplicate block").clicked() {
                            let mut copy = node.clone();
                            let old_name = copy.get("name").and_then(|v| v.as_str()).unwrap_or("block").to_string();
                            copy["name"] = json!(format!("{} copy", old_name));
                            let pos = Self::leaf_pos(&copy, sel);
                            copy["pos"] = json!([pos.x + 30.0, pos.y + 60.0]);
                            self.insert_into_root_series(copy);
                            self.rbd_status = "Block duplicated.".into();
                            self.note(self.rbd_status.clone());
                        }
                        if ui.button("Delete block").clicked() {
                            let spec = Self::remove_at(&self.rbd_spec, &p);
                            self.set_rbd_spec(spec);
                            self.rbd_status = "Block deleted.".into();
                            self.note(self.rbd_status.clone());
                        }
                    });
                }
            }
        }
        // Live reliability and availability
        section(ui, "Evaluation");
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
        grid_pairs(
            ui,
            "rbd_eval_grid",
            &[(
                format!("Reliability at t={}", self.rbd_t),
                match rel {
                    Some(r) => format!("{:.4}", r),
                    None => "needs a valid diagram and mission time".into(),
                },
            )],
        );
        if let Some(r) = rel {
            self.headline = format!("RBD R(t={}) = {:.4}", self.rbd_t, r);
        }
        ui.horizontal(|ui| {
            ui.label("Availability horizon:");
            ui.add(egui::TextEdit::singleline(&mut self.rbd_horizon).desired_width(80.0));
            if ui.button("Run availability (40 sims)").clicked() {
                let horizon = fnum(&self.rbd_horizon, 20000.0);
                let res = system::simulate_availability(&self.rbd_spec, horizon, 40, 3, 20);
                self.rbd_avail = format!(
                    "Mean availability: {:.4}\nExpected downtime: {:.1}\nMean failures: {:.2}\nMean repairs: {:.2}",
                    res.mean_availability, res.expected_downtime, res.mean_failures, res.mean_repairs
                );
                self.note("Availability simulation complete.");
            }
        });
        result_box(ui, &self.rbd_avail.clone());
        // Importance panel
        section(ui, "Reliability importance");
        ui.horizontal(|ui| {
            if ui.button("Compute importance measures").clicked() {
                let t = fnum(&self.rbd_t, 1000.0);
                match importance::importance_measures(&self.rbd_spec, t) {
                    Ok(v) => {
                        self.rbd_importance = v;
                        self.rbd_importance_note = format!("Importance at t={:.0} for {} blocks.", t, self.rbd_importance.len());
                        self.note("Importance measures computed.");
                    }
                    Err(e) => {
                        self.rbd_importance.clear();
                        self.rbd_importance_note = format!("Error: {}", e);
                    }
                }
            }
            ui.label(self.rbd_importance_note.clone());
        });
        if !self.rbd_importance.is_empty() {
            let rows = self.rbd_importance.clone();
            TableBuilder::new(ui)
                .striped(true)
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .header(18.0, |mut h| {
                    for t in ["Block", "Birnbaum", "Fussell-Vesely", "Criticality"] {
                        h.col(|ui| {
                            ui.strong(t);
                        });
                    }
                })
                .body(|mut body| {
                    for r in &rows {
                        body.row(18.0, |mut row| {
                            row.col(|ui| {
                                ui.label(r.name.clone());
                            });
                            row.col(|ui| {
                                ui.label(format!("{:.4}", r.birnbaum));
                            });
                            row.col(|ui| {
                                ui.label(format!("{:.4}", r.fussell_vesely));
                            });
                            row.col(|ui| {
                                ui.label(format!("{:.4}", r.criticality));
                            });
                        });
                    }
                });
        }
        ui.separator();
        section(ui, "Add block form");
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
                self.insert_into_root_series(node);
                self.rbd_status = "Block added.".into();
                self.note(self.rbd_status.clone());
            }
        });
        ui.separator();
        egui::CollapsingHeader::new("Diagram JSON").show(ui, |ui| {
            ui.label("Same schema as system::rbd_from_json. Leaf nodes may carry an additive pos field with canvas coordinates.");
            ui.add(egui::TextEdit::multiline(&mut self.rbd_json).desired_rows(8).desired_width(f32::INFINITY).code_editor());
            ui.horizontal(|ui| {
                if ui.button("Load from JSON text").clicked() {
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
                ui.label("File path:");
                ui.add(egui::TextEdit::singleline(&mut self.rbd_path).desired_width(160.0));
                if ui.button("Save").clicked() {
                    match std::fs::write(&self.rbd_path, &self.rbd_json) {
                        Ok(()) => self.rbd_status = format!("Saved to {}", self.rbd_path),
                        Err(e) => self.rbd_status = format!("Save error: {}", e),
                    }
                }
                if ui.button("Load").clicked() {
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
            });
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
                    let mut rows: Vec<(String, String)> = vec![
                        ("Top probability".into(), format!("{:.6}", faulttree::top_probability(&node))),
                        ("Minimal cut sets".into(), format!("{}", cuts.len())),
                    ];
                    for (i, c) in cuts.iter().enumerate() {
                        let mut names: Vec<String> = c.iter().cloned().collect();
                        names.sort();
                        rows.push((format!("Cut set {}", i + 1), names.join(", ")));
                    }
                    self.ft_result = rows.iter().map(|(a, b)| format!("{}: {}", a, b)).collect::<Vec<String>>().join("\n");
                    self.note("Fault tree evaluated.");
                }
                Err(e) => self.ft_result = format!("JSON error: {}", e),
            }
        }
        result_box(ui, &self.ft_result.clone());
    }

    fn fmea_criticality(&self, idx: usize) -> Option<f64> {
        let c = self.fmea_crit.get(idx)?;
        let a = opt_num(&c[0])?;
        let b = opt_num(&c[1])?;
        let l = opt_num(&c[2])?;
        let t = opt_num(&c[3])?;
        Some(fmea::criticality_number(a, b, l, t))
    }

    fn page_fmea(&mut self, ui: &mut egui::Ui) {
        ui.heading("FMEA Worksheet");
        ui.horizontal(|ui| {
            ui.label("Worksheet type:");
            egui::ComboBox::from_id_salt("fmea_type")
                .selected_text(self.fmea_type.clone())
                .show_ui(ui, |ui| {
                    for t in ["Design", "Process", "System"] {
                        ui.selectable_value(&mut self.fmea_type, t.to_string(), t);
                    }
                });
            ui.label("Type is stored with the worksheet and written into worksheet CSV exports.");
        });
        section(ui, "Add row");
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
        });
        ui.horizontal(|ui| {
            ui.label("Optional FMECA inputs: alpha:");
            ui.add(egui::TextEdit::singleline(&mut self.fm_alpha).desired_width(55.0));
            ui.label("beta:");
            ui.add(egui::TextEdit::singleline(&mut self.fm_beta).desired_width(55.0));
            ui.label("lambda per hour:");
            ui.add(egui::TextEdit::singleline(&mut self.fm_lambda).desired_width(80.0));
            ui.label("mission hours:");
            ui.add(egui::TextEdit::singleline(&mut self.fm_hours).desired_width(70.0));
            if ui.button("Add row").clicked() {
                self.fmea_rows.push(FmeaRow::new(
                    &self.fm_item.clone(),
                    &self.fm_mode.clone(),
                    self.fm_sev.trim().parse::<u32>().unwrap_or(1),
                    self.fm_occ.trim().parse::<u32>().unwrap_or(1),
                    self.fm_det.trim().parse::<u32>().unwrap_or(1),
                ));
                self.fmea_crit.push([self.fm_alpha.clone(), self.fm_beta.clone(), self.fm_lambda.clone(), self.fm_hours.clone()]);
                self.fmea_status = "Row added.".into();
                self.note(self.fmea_status.clone());
            }
        });
        section(ui, "Worksheet");
        let crits: Vec<Option<f64>> = (0..self.fmea_rows.len()).map(|i| self.fmea_criticality(i)).collect();
        let display_rows = self.fmea_rows.clone();
        TableBuilder::new(ui)
            .striped(true)
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .header(18.0, |mut h| {
                for t in ["Item", "Failure mode", "Sev", "Occ", "Det", "RPN", "Revised RPN", "Criticality"] {
                    h.col(|ui| {
                        ui.strong(t);
                    });
                }
            })
            .body(|mut body| {
                for (i, r) in display_rows.iter().enumerate() {
                    body.row(18.0, |mut row| {
                        row.col(|ui| {
                            ui.label(r.item.clone());
                        });
                        row.col(|ui| {
                            ui.label(r.failure_mode.clone());
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", r.severity));
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", r.occurrence));
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", r.detection));
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", r.rpn()));
                        });
                        row.col(|ui| {
                            ui.label(r.revised_rpn().map(|v| v.to_string()).unwrap_or_else(|| "-".into()));
                        });
                        row.col(|ui| {
                            ui.label(match crits.get(i).copied().flatten() {
                                Some(v) => format!("{:.6}", v),
                                None => "-".into(),
                            });
                        });
                    });
                }
            });
        ui.separator();
        section(ui, "Send a row to RCM");
        ui.horizontal(|ui| {
            ui.label("Row number (1 based):");
            ui.add(egui::TextEdit::singleline(&mut self.fmea_rcm_row).desired_width(45.0));
            icon_widget(ui, "link");
            if ui.button("Create RCM row from selected FMEA row").clicked() {
                let idx = inum(&self.fmea_rcm_row, 1).saturating_sub(1);
                self.flow_fmea_to_rcm(idx);
            }
        });
        if self.fmea_rows.is_empty() {
            ui.label("No FMEA rows yet. Add a row above, then send it to RCM.");
        }
        section(ui, "Worksheet CSV");
        ui.label("Worksheet CSV carries a first line with the FMEA type, then the rows CSV.");
        ui.add(egui::TextEdit::multiline(&mut self.fmea_csv).desired_rows(5).desired_width(f32::INFINITY).code_editor());
        ui.horizontal(|ui| {
            if ui.button("Export worksheet to CSV text").clicked() {
                let ws = FmeaWorksheet {
                    fmea_type: FmeaType::from_str(&self.fmea_type).unwrap_or(FmeaType::Design),
                    rows: self.fmea_rows.clone(),
                };
                self.fmea_csv = ws.to_csv();
            }
            if ui.button("Export rows only").clicked() {
                self.fmea_csv = fmea::rows_to_csv(&self.fmea_rows);
            }
            if ui.button("Import CSV").clicked() {
                match FmeaWorksheet::from_csv(&self.fmea_csv) {
                    Ok(ws) => {
                        self.fmea_type = ws.fmea_type.as_str().to_string();
                        self.fmea_rows = ws.rows;
                        self.fmea_crit = vec![[String::new(), String::new(), String::new(), String::new()]; self.fmea_rows.len()];
                        self.fmea_status = "CSV imported.".into();
                    }
                    Err(e) => self.fmea_status = format!("Error: {}", e),
                }
            }
            if ui.button("Clear rows").clicked() {
                self.fmea_rows.clear();
                self.fmea_crit.clear();
            }
        });
        ui.horizontal(|ui| {
            ui.label("Export file path:");
            ui.add(egui::TextEdit::singleline(&mut self.fmea_export_path).desired_width(220.0));
            if ui.button("Export control plan CSV").clicked() {
                let csv = fmea::control_plan_csv(&self.fmea_rows);
                match std::fs::write(&self.fmea_export_path, csv) {
                    Ok(()) => self.fmea_status = format!("Control plan written to {}", self.fmea_export_path),
                    Err(e) => self.fmea_status = format!("Export error: {}", e),
                }
            }
            if ui.button("Export DVP&R CSV").clicked() {
                let csv = fmea::dvpr_to_csv(&self.dvpr_entries);
                match std::fs::write(&self.fmea_export_path, csv) {
                    Ok(()) => self.fmea_status = format!("DVP&R written to {}", self.fmea_export_path),
                    Err(e) => self.fmea_status = format!("Export error: {}", e),
                }
            }
        });
        section(ui, "DVP&R entries");
        ui.label("Each entry links to a failure mode by its reference text (use the failure mode name).");
        ui.horizontal(|ui| {
            ui.label("Id:");
            ui.add(egui::TextEdit::singleline(&mut self.dv_id).desired_width(60.0));
            ui.label("Failure mode ref:");
            ui.add(egui::TextEdit::singleline(&mut self.dv_ref).desired_width(140.0));
            ui.label("Test:");
            ui.add(egui::TextEdit::singleline(&mut self.dv_test).desired_width(160.0));
            ui.label("Sample size:");
            ui.add(egui::TextEdit::singleline(&mut self.dv_sample).desired_width(50.0));
            ui.label("Acceptance criteria:");
            ui.add(egui::TextEdit::singleline(&mut self.dv_criteria).desired_width(160.0));
            if ui.button("Add DVP&R entry").clicked() {
                self.dvpr_entries.push(DvprEntry {
                    id: self.dv_id.clone(),
                    failure_mode_ref: self.dv_ref.clone(),
                    test: self.dv_test.clone(),
                    sample_size: self.dv_sample.trim().parse::<u32>().unwrap_or(0),
                    acceptance_criteria: self.dv_criteria.clone(),
                });
                self.fmea_status = "DVP&R entry added.".into();
            }
        });
        if !self.dvpr_entries.is_empty() {
            let entries = self.dvpr_entries.clone();
            TableBuilder::new(ui)
                .striped(true)
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .header(18.0, |mut h| {
                    for t in ["Id", "Failure mode ref", "Test", "Sample size", "Acceptance criteria"] {
                        h.col(|ui| {
                            ui.strong(t);
                        });
                    }
                })
                .body(|mut body| {
                    for e in &entries {
                        body.row(18.0, |mut row| {
                            row.col(|ui| {
                                ui.label(e.id.clone());
                            });
                            row.col(|ui| {
                                ui.label(e.failure_mode_ref.clone());
                            });
                            row.col(|ui| {
                                ui.label(e.test.clone());
                            });
                            row.col(|ui| {
                                ui.label(format!("{}", e.sample_size));
                            });
                            row.col(|ui| {
                                ui.label(e.acceptance_criteria.clone());
                            });
                        });
                    }
                });
        }
        if !self.fmea_status.is_empty() {
            ui.label(self.fmea_status.clone());
        }
    }

    fn page_warranty(&mut self, ui: &mut egui::Ui) {
        ui.heading("Warranty (Nevada table)");
        ui.label("Sales per period (comma separated):");
        ui.add(egui::TextEdit::singleline(&mut self.war_sales).desired_width(f32::INFINITY));
        ui.label("Returns, one per line: sale_period,return_period (periods are 0-based in this form)");
        ui.add(egui::TextEdit::multiline(&mut self.war_returns).desired_rows(4).desired_width(f32::INFINITY));
        if let Some(text) = self.csv_import_controls(ui, "warranty") {
            let (header, rows) = csv_table(&text);
            if let (Some(sp), Some(sa), Some(lg), Some(rt)) = (
                col_idx(&header, "ship_period"),
                col_idx(&header, "sales"),
                col_idx(&header, "lag_months"),
                col_idx(&header, "returns"),
            ) {
                // Nevada format: ship periods in the file are 1-based.
                let mut sales_map: BTreeMap<usize, u32> = BTreeMap::new();
                let mut records: Vec<String> = Vec::new();
                for r in &rows {
                    let ship = r.get(sp).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
                    let sale = r.get(sa).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                    let lag = r.get(lg).and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
                    let ret = r.get(rt).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                    sales_map.insert(ship, sale);
                    for _ in 0..ret {
                        records.push(format!("{},{}", ship - 1, ship - 1 + lag));
                    }
                }
                let max_ship = sales_map.keys().max().copied().unwrap_or(0);
                self.war_sales = (1..=max_ship).map(|k| sales_map.get(&k).copied().unwrap_or(0).to_string()).collect::<Vec<String>>().join(", ");
                self.war_returns = records.join("\n");
                self.note("Imported Nevada warranty CSV. Run the analysis to see the table.");
            } else if col_idx(&header, "time").is_some() {
                self.note("That file is time-to-failure warranty format. Load it on the Life Data screen, where the grouped time,failed,count format is handled.");
            } else {
                self.note("Warranty CSV needs ship_period, sales, lag_months, returns columns.");
            }
        }
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
            self.war_sale_labels = (0..sales.len()).map(|i| format!("Sale period {}", i)).collect();
            self.war_table = table.clone();
            let mut rows: Vec<(String, String)> = vec![
                ("Returns by age".into(), warranty::returns_by_age(&table).iter().map(|v| v.to_string()).collect::<Vec<String>>().join(", ")),
                ("Total returns".into(), format!("{}", warranty::total_returns(&table))),
            ];
            match warranty::forecast_returns(&sales, &records, inum(&self.war_current, 0), inum(&self.war_horizon, 6)) {
                Ok(f) => {
                    rows.push(("Forecast Weibull beta".into(), format!("{:.3}", f.beta)));
                    rows.push(("Forecast Weibull eta".into(), format!("{:.2}", f.eta)));
                    rows.push(("Expected returns over horizon".into(), format!("{:.2}", f.expected_returns)));
                    rows.push(("Units in service".into(), format!("{:.0}", f.units_in_service)));
                }
                Err(e) => rows.push(("Forecast error".into(), e)),
            }
            self.war_result = rows.iter().map(|(a, b)| format!("{}: {}", a, b)).collect::<Vec<String>>().join("\n");
            self.note("Warranty analysis complete.");
        }
        if !self.war_table.is_empty() {
            section(ui, "Nevada table (rows are sale periods, columns are age in periods)");
            let table = self.war_table.clone();
            let labels = self.war_sale_labels.clone();
            let ncols = table.iter().map(|r| r.len()).max().unwrap_or(0);
            let mut builder = TableBuilder::new(ui).striped(true).column(Column::auto());
            for _ in 0..ncols {
                builder = builder.column(Column::auto());
            }
            builder
                .header(18.0, |mut h| {
                    h.col(|ui| {
                        ui.strong("Sale period");
                    });
                    for a in 0..ncols {
                        h.col(|ui| {
                            ui.strong(format!("Age {}", a));
                        });
                    }
                })
                .body(|mut body| {
                    for (i, r) in table.iter().enumerate() {
                        body.row(18.0, |mut row| {
                            row.col(|ui| {
                                ui.label(labels.get(i).cloned().unwrap_or_else(|| format!("Sale period {}", i)));
                            });
                            for v in r {
                                row.col(|ui| {
                                    ui.label(format!("{}", v));
                                });
                            }
                        });
                    }
                });
        }
        result_box(ui, &self.war_result.clone());
    }
}

impl RelKitApp {
    fn pred_parse_parts(&self, text: &str) -> Vec<PartEntry> {
        let mut parts: Vec<PartEntry> = Vec::new();
        for line in text.lines() {
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
        parts
    }

    fn pred217_load(&mut self) {
        let path = mil217_dir().join(&self.pred217_file);
        match std::fs::read_to_string(&path) {
            Ok(text) => match standards217::load_category(&text) {
                Ok(v) => {
                    self.pred217_json = Some(v);
                    self.pred217_result = String::new();
                    self.note(format!("Loaded MIL-HDBK-217F category {}", self.pred217_file));
                }
                Err(e) => self.pred217_result = format!("Category JSON error: {}", e),
            },
            Err(e) => self.pred217_result = format!("Category load error: {}", e),
        }
    }

    /// Field list for a 217F category: (is_named, key, label, options).
    fn pred217_fields(cat: &Value) -> Vec<(bool, String, String, Vec<String>)> {
        let mut fields: Vec<(bool, String, String, Vec<String>)> = Vec::new();
        let lb = cat.get("lambda_b").cloned().unwrap_or(json!({}));
        for (key, map_name) in [("style", "by_style"), ("type", "by_type")] {
            if let Some(m) = lb.get(map_name).and_then(|v| v.as_object()) {
                if !m.is_empty() {
                    let mut opts: Vec<String> = m.keys().cloned().collect();
                    opts.sort();
                    fields.push((true, key.to_string(), format!("{} ({})", key, map_name), opts));
                }
            }
        }
        if let Some(axes) = lb.get("axes").and_then(|v| v.as_array()) {
            for a in axes {
                if let Some(name) = a.get("name").and_then(|v| v.as_str()) {
                    fields.push((false, name.to_string(), format!("lambda_b variable {}", name), Vec::new()));
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
                            fields.push((true, input.to_string(), format!("{} input {}", fname, input), opts));
                        }
                    }
                    "series" | "columns" | "bands" => {
                        if !input.is_empty() {
                            fields.push((false, input.to_string(), format!("{} input {}", fname, input), Vec::new()));
                        }
                    }
                    _ => {
                        fields.push((false, fname.clone(), format!("{} value (formula factor)", fname), Vec::new()));
                    }
                }
            }
        }
        // Deduplicate by (is_named, key), keeping the first entry.
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

    fn page_prediction(&mut self, ui: &mut egui::Ui) {
        ui.heading("Prediction");
        section(ui, "Life model (from a Life Data fit)");
        ui.label("A fitted distribution sent from a Life Data item, or set by hand here. R(t) uses the library reliability function for the distribution.");
        ui.horizontal(|ui| {
            ui.label("Distribution:");
            dist_combo(ui, "pred_life_dist", &mut self.pred_life_dist);
            ui.label("Params:");
            ui.add(egui::TextEdit::singleline(&mut self.pred_life_params).desired_width(220.0));
            ui.label("t:");
            ui.add(egui::TextEdit::singleline(&mut self.pred_life_t).desired_width(70.0));
            if ui.button("Compute R(t) for life model").clicked() {
                let params = parse_params_text(&self.pred_life_params);
                let t = fnum(&self.pred_life_t, 1000.0);
                let r = dist_mod::reliability(&self.pred_life_dist, &params, t);
                self.pred_life_result = format!("Life model {}: R({:.0}) = {:.4}", dist_mod::dist_label(&self.pred_life_dist), t, r);
                self.headline = self.pred_life_result.clone();
                self.note("Life model reliability computed.");
            }
        });
        if !self.pred_life_result.is_empty() {
            ui.label(self.pred_life_result.clone());
        } else {
            ui.label("No life model yet. Send a fit from a Life Data item, or set a distribution and parameters here.");
        }
        ui.horizontal(|ui| {
            ui.label("Standard:");
            egui::ComboBox::from_id_salt("pred_source")
                .selected_text(self.pred_source.clone())
                .show_ui(ui, |ui| {
                    for s in ["Generic user data", "MIL-HDBK-217F library", "Local library file"] {
                        ui.selectable_value(&mut self.pred_source, s.to_string(), s);
                    }
                });
        });
        if self.pred_source == "Generic user data" {
            section(ui, "Parts-count prediction (user supplied rates)");
            ui.label("One part per line: name, base rate per million hours, quantity, stress factor, quality factor");
            ui.add(egui::TextEdit::multiline(&mut self.pred_parts).desired_rows(4).desired_width(f32::INFINITY));
            ui.horizontal(|ui| {
                ui.label("Mission t (hours):");
                ui.add(egui::TextEdit::singleline(&mut self.pred_t).desired_width(80.0));
                if ui.button("Run prediction").clicked() {
                    let parts = self.pred_parse_parts(&self.pred_parts);
                    let t = fnum(&self.pred_t, 1000.0);
                    let rows: Vec<(String, String)> = vec![
                        ("System failure rate per million hours".into(), format!("{:.3}", prediction::system_failure_rate_per_million_hours(&parts))),
                        ("System MTBF (hours)".into(), format!("{:.0}", prediction::system_mtbf_hours(&parts))),
                        (format!("Reliability at {:.0} h", t), format!("{:.4}", prediction::system_reliability(&parts, t))),
                    ];
                    self.pred_result = rows.iter().map(|(a, b)| format!("{}: {}", a, b)).collect::<Vec<String>>().join("\n");
                    self.note("Prediction complete.");
                }
            });
            result_box(ui, &self.pred_result.clone());
        }
        if self.pred_source == "Local library file" {
            section(ui, "Local component library CSV");
            ui.label("Format: name,base_rate_per_million_hours,quantity,stress_factor,quality_factor. Comment lines starting with # are skipped.");
            ui.horizontal(|ui| {
                ui.label("Library CSV path:");
                ui.add(egui::TextEdit::singleline(&mut self.pred_lib_path).desired_width(320.0));
                if ui.button("Load library").clicked() {
                    match std::fs::read_to_string(&self.pred_lib_path) {
                        Ok(text) => match prediction::library_from_csv(&text) {
                            Ok(parts) => {
                                self.pred_lib_note = format!("Loaded {} parts from {}", parts.len(), self.pred_lib_path);
                                self.pred_lib_parts = parts;
                            }
                            Err(e) => self.pred_lib_note = format!("Library error: {}", e),
                        },
                        Err(e) => self.pred_lib_note = format!("Load error: {}", e),
                    }
                }
            });
            ui.label(self.pred_lib_note.clone());
            if !self.pred_lib_parts.is_empty() {
                let parts = self.pred_lib_parts.clone();
                egui::Grid::new("pred_lib_grid")
                    .num_columns(3)
                    .spacing([14.0, 3.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for h in ["Part", "Quantity", "Rate contribution per million hours"] {
                            ui.strong(h);
                        }
                        ui.end_row();
                        for p in &parts {
                            ui.label(p.name.clone());
                            ui.label(format!("{}", p.quantity));
                            ui.label(format!("{:.4}", prediction::part_failure_rate_per_million_hours(p)));
                            ui.end_row();
                        }
                    });
                let t = fnum(&self.pred_t, 1000.0);
                grid_pairs(
                    ui,
                    "pred_lib_sys",
                    &[
                        ("System failure rate per million hours".into(), format!("{:.3}", prediction::system_failure_rate_per_million_hours(&parts))),
                        ("System MTBF (hours)".into(), format!("{:.0}", prediction::system_mtbf_hours(&parts))),
                        (format!("Reliability at {:.0} h", t), format!("{:.4}", prediction::system_reliability(&parts, t))),
                    ],
                );
                ui.horizontal(|ui| {
                    ui.label("Mission t (hours):");
                    ui.add(egui::TextEdit::singleline(&mut self.pred_t).desired_width(80.0));
                });
            }
        }
        if self.pred_source == "MIL-HDBK-217F library" {
            section(ui, "MIL-HDBK-217F part stress");
            ui.horizontal(|ui| {
                ui.label("Category file:");
                egui::ComboBox::from_id_salt("pred217_file")
                    .selected_text(self.pred217_file.clone())
                    .show_ui(ui, |ui| {
                        for f in MIL217_FILES {
                            ui.selectable_value(&mut self.pred217_file, f.to_string(), f);
                        }
                    });
                if ui.button("Load category").clicked() {
                    self.pred217_load();
                }
            });
            if self.pred217_json.is_none() {
                self.pred217_load();
            }
            if let Some(cat) = self.pred217_json.clone() {
                let cat_name = cat.get("category").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let section_name = cat.get("section").and_then(|v| v.as_str()).unwrap_or("").to_string();
                ui.label(format!("Category: {} (section {})", cat_name, section_name));
                let is_pth = self.pred217_file == "interconnection_assemblies_pth.json";
                let is_misc = cat.get("lambda_b").and_then(|lb| lb.get("kind")).and_then(|v| v.as_str()) == Some("fixed_rates");
                if is_misc {
                    ui.label("This category holds fixed rates. Pick a part and an environment; the rate shown is the fixed rate times pi_E.");
                    let rates = cat.get("lambda_b").and_then(|lb| lb.get("rates")).and_then(|v| v.as_object()).cloned().unwrap_or_default();
                    let mut part_names: Vec<String> = rates.keys().cloned().collect();
                    part_names.sort();
                    let cur_part = self.pred217_named.get("part").cloned().unwrap_or_else(|| part_names.first().cloned().unwrap_or_default());
                    let mut sel_part = cur_part.clone();
                    egui::ComboBox::from_id_salt("misc_part")
                        .selected_text(cur_part)
                        .width(420.0)
                        .show_ui(ui, |ui| {
                            for p in &part_names {
                                ui.selectable_value(&mut sel_part, p.clone(), p.clone());
                            }
                        });
                    self.pred217_named.insert("part".into(), sel_part);
                    let envs = vec!["GB".to_string(), "GF".into(), "GM".into(), "NS".into(), "NU".into(), "AIC".into(), "CL".into()];
                    let cur_env = self.pred217_named.get("environment").cloned().unwrap_or_else(|| "GB".into());
                    let mut sel_env = cur_env.clone();
                    egui::ComboBox::from_id_salt("misc_env")
                        .selected_text(cur_env)
                        .show_ui(ui, |ui| {
                            for e in &envs {
                                ui.selectable_value(&mut sel_env, e.clone(), e.clone());
                            }
                        });
                    self.pred217_named.insert("environment".into(), sel_env.clone());
                    if ui.button("Compute fixed rate").clicked() {
                        let base = rates.get(self.pred217_named.get("part").cloned().unwrap_or_default().as_str()).and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
                        let pi_e = standards217::pi_table_value(&cat, "pi_E", &sel_env).unwrap_or(f64::NAN);
                        self.pred217_result = format!(
                            "Base rate per million hours: {:.4}\npi_E ({}): {:.4}\nFailure rate per million hours: {:.4}",
                            base, sel_env, pi_e, base * pi_e
                        );
                    }
                } else if is_pth {
                    ui.label("Section 16.1 uses its own printed equation, evaluated with standards217::pth_failure_rate.");
                    for (key, label, def) in [
                        ("pth_technology", "Technology (by-type key)", "Printed Wiring Assembly/Printed Circuit Boards with PTHs"),
                        ("pth_quality", "Quality", "MIL-SPEC or IPC Level 3"),
                        ("pth_environment", "Environment", "GB"),
                    ] {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}:", label));
                            let entry = self.pred217_named.entry(key.to_string()).or_insert_with(|| def.to_string());
                            ui.add(egui::TextEdit::singleline(entry).desired_width(420.0));
                        });
                    }
                    for (key, label, def) in [("pth_n1", "N1 automated PTH count", "100"), ("pth_n2", "N2 hand soldered PTH count", "20"), ("pth_planes", "Circuit planes", "4")] {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}:", label));
                            let entry = self.pred217_num.entry(key.to_string()).or_insert_with(|| def.to_string());
                            ui.add(egui::TextEdit::singleline(entry).desired_width(90.0));
                        });
                    }
                    if ui.button("Compute PTH failure rate").clicked() {
                        let rate = standards217::pth_failure_rate(
                            &cat,
                            &self.pred217_named.get("pth_technology").cloned().unwrap_or_default(),
                            fnum(self.pred217_num.get("pth_n1").map(|s| s.as_str()).unwrap_or(""), 0.0),
                            fnum(self.pred217_num.get("pth_n2").map(|s| s.as_str()).unwrap_or(""), 0.0),
                            fnum(self.pred217_num.get("pth_planes").map(|s| s.as_str()).unwrap_or(""), 0.0),
                            &self.pred217_named.get("pth_quality").cloned().unwrap_or_default(),
                            &self.pred217_named.get("pth_environment").cloned().unwrap_or_default(),
                        );
                        self.pred217_result = format!("PTH failure rate per million hours: {:.5}", rate);
                    }
                } else {
                    let fields = Self::pred217_fields(&cat);
                    for (is_named, key, label, options) in &fields {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}:", label));
                            if *is_named {
                                let cur = self.pred217_named.get(key).cloned().unwrap_or_else(|| options.first().cloned().unwrap_or_default());
                                let mut sel = cur.clone();
                                egui::ComboBox::from_id_salt(format!("f217_{}", key))
                                    .selected_text(cur)
                                    .width(300.0)
                                    .show_ui(ui, |ui| {
                                        for o in options {
                                            ui.selectable_value(&mut sel, o.clone(), o.clone());
                                        }
                                    });
                                self.pred217_named.insert(key.clone(), sel);
                            } else {
                                let entry = self.pred217_num.entry(key.clone()).or_default();
                                ui.add(egui::TextEdit::singleline(entry).desired_width(110.0));
                            }
                        });
                    }
                    if ui.button("Compute part failure rate").clicked() {
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
                }
                result_box(ui, &self.pred217_result.clone());
            }
        }
        // Allocation panel
        egui::CollapsingHeader::new("Reliability allocation").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("System target R:");
                ui.add(egui::TextEdit::singleline(&mut self.alloc_target).desired_width(60.0));
                ui.label("Subsystems n (equal):");
                ui.add(egui::TextEdit::singleline(&mut self.alloc_n).desired_width(40.0));
                ui.label("Weights (weighted):");
                ui.add(egui::TextEdit::singleline(&mut self.alloc_weights).desired_width(140.0));
                if ui.button("Allocate").clicked() {
                    let target = fnum(&self.alloc_target, 0.99);
                    let eq = prediction::allocate_equal(target, inum(&self.alloc_n, 3));
                    let w = parse_numbers(&self.alloc_weights);
                    let wt = prediction::allocate_weighted(target, &w);
                    self.alloc_result = format!(
                        "Equal allocation: {}\nWeighted allocation: {}",
                        eq.iter().map(|v| format!("{:.4}", v)).collect::<Vec<String>>().join(", "),
                        wt.iter().map(|v| format!("{:.4}", v)).collect::<Vec<String>>().join(", ")
                    );
                }
            });
            result_box(ui, &self.alloc_result.clone());
        });
        // Derating panel
        egui::CollapsingHeader::new("Derating check").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Part:");
                ui.add(egui::TextEdit::singleline(&mut self.der_name).desired_width(110.0));
                ui.label("Operating stress:");
                ui.add(egui::TextEdit::singleline(&mut self.der_operating).desired_width(60.0));
                ui.label("Rated stress:");
                ui.add(egui::TextEdit::singleline(&mut self.der_rated).desired_width(60.0));
                ui.label("Max ratio:");
                ui.add(egui::TextEdit::singleline(&mut self.der_max).desired_width(50.0));
                if ui.button("Check derating").clicked() {
                    let r = prediction::derating_check(
                        &self.der_name.clone(),
                        fnum(&self.der_operating, 0.0),
                        fnum(&self.der_rated, 1.0),
                        fnum(&self.der_max, 1.0),
                    );
                    self.der_result = format!(
                        "Part: {}\nDerating ratio: {:.3}\nResult: {}",
                        r.name,
                        r.ratio,
                        if r.pass { "pass" } else { "fail" }
                    );
                }
            });
            result_box(ui, &self.der_result.clone());
        });
        // Clean-room methods panel
        egui::CollapsingHeader::new("Clean-room methods (SR-332 style and 62380-style)").show(ui, |ui| {
            ui.label("Method shapes only. Every rate and factor below is a value you supply; no standard table data is built in.");
            section(ui, "SR-332 Method I device rate");
            ui.horizontal(|ui| {
                ui.label("Generic rate:");
                ui.add(egui::TextEdit::singleline(&mut self.sr_generic).desired_width(60.0));
                ui.label("pi_q:");
                ui.add(egui::TextEdit::singleline(&mut self.sr_pi_q).desired_width(50.0));
                ui.label("pi_s:");
                ui.add(egui::TextEdit::singleline(&mut self.sr_pi_s).desired_width(50.0));
                ui.label("Ea (eV):");
                ui.add(egui::TextEdit::singleline(&mut self.sr_ea).desired_width(50.0));
                ui.label("Use temp C:");
                ui.add(egui::TextEdit::singleline(&mut self.sr_temp).desired_width(50.0));
                ui.label("Ref temp C:");
                ui.add(egui::TextEdit::singleline(&mut self.sr_ref_temp).desired_width(50.0));
            });
            section(ui, "Equipment rate, Method II, failure rate CI, Method III");
            ui.horizontal(|ui| {
                ui.label("pi_e:");
                ui.add(egui::TextEdit::singleline(&mut self.sr_pi_e).desired_width(50.0));
                ui.label("Quantity,device rate lines:");
            });
            ui.add(egui::TextEdit::multiline(&mut self.sr_qty_rate).desired_rows(2).desired_width(300.0));
            ui.horizontal(|ui| {
                ui.label("Method II observed rate:");
                ui.add(egui::TextEdit::singleline(&mut self.sr_obs_rate).desired_width(55.0));
                ui.label("pi_t observed:");
                ui.add(egui::TextEdit::singleline(&mut self.sr_pit_obs).desired_width(50.0));
                ui.label("pi_t target:");
                ui.add(egui::TextEdit::singleline(&mut self.sr_pit_tgt).desired_width(50.0));
                ui.label("pi_s observed:");
                ui.add(egui::TextEdit::singleline(&mut self.sr_pis_obs).desired_width(50.0));
                ui.label("pi_s target:");
                ui.add(egui::TextEdit::singleline(&mut self.sr_pis_tgt).desired_width(50.0));
            });
            ui.horizontal(|ui| {
                ui.label("CI failures:");
                ui.add(egui::TextEdit::singleline(&mut self.sr_ci_failures).desired_width(45.0));
                ui.label("Exposure hours:");
                ui.add(egui::TextEdit::singleline(&mut self.sr_ci_exposure).desired_width(80.0));
                ui.label("Confidence:");
                ui.add(egui::TextEdit::singleline(&mut self.sr_ci_conf).desired_width(50.0));
                ui.label("Method III predicted rate:");
                ui.add(egui::TextEdit::singleline(&mut self.sr_m3_pred).desired_width(55.0));
                ui.label("weight:");
                ui.add(egui::TextEdit::singleline(&mut self.sr_m3_pw).desired_width(45.0));
                ui.label("observed rate:");
                ui.add(egui::TextEdit::singleline(&mut self.sr_m3_obs).desired_width(55.0));
                ui.label("weight:");
                ui.add(egui::TextEdit::singleline(&mut self.sr_m3_ow).desired_width(45.0));
            });
            if ui.button("Run clean-room calculations").clicked() {
                let ea = fnum(&self.sr_ea, prediction::SR332_DEFAULT_ACTIVATION_ENERGY_EV);
                let pi_t = prediction::sr332_temperature_factor(ea, fnum(&self.sr_temp, 25.0), fnum(&self.sr_ref_temp, 25.0));
                let device = prediction::sr332_method1(fnum(&self.sr_generic, 0.0), fnum(&self.sr_pi_q, 1.0), fnum(&self.sr_pi_s, 1.0), pi_t);
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
                self.pred_clean_result = format!(
                    "pi_t (temperature factor): {:.4}\nMethod I device rate: {:.5}\nEquipment rate (pi_e applied once): {:.5}\nMethod II extrapolated rate: {:.5}\nFailure rate CI per hour: {:.3e} to {:.3e}\nMethod III combined rate: {:.5}",
                    pi_t, device, equip, m2, lo, hi, m3
                );
                self.note("Clean-room calculations complete.");
            }
            result_box(ui, &self.pred_clean_result.clone());
            section(ui, "62380-style mission profile");
            ui.label("Labeled 62380-style: a mission profile framework with user supplied base rate and factors. It holds no IEC 62380 table data.");
            ui.horizontal(|ui| {
                ui.label("Base rate:");
                ui.add(egui::TextEdit::singleline(&mut self.mp_base).desired_width(55.0));
                ui.label("Ea (eV):");
                ui.add(egui::TextEdit::singleline(&mut self.mp_ea).desired_width(50.0));
                ui.label("Ref temp C:");
                ui.add(egui::TextEdit::singleline(&mut self.mp_ref).desired_width(50.0));
                ui.label("Cycling factor per cycle:");
                ui.add(egui::TextEdit::singleline(&mut self.mp_cyc_factor).desired_width(70.0));
            });
            ui.label("Phases, one per line: name, duration hours, temp C, cycles, dormant (1 or 0), dormant factor");
            ui.add(egui::TextEdit::multiline(&mut self.mp_phases).desired_rows(3).desired_width(f32::INFINITY));
            if ui.button("Compute 62380-style mission profile rate").clicked() {
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
                self.pred_clean_result = format!("62380-style mission profile rate: {:.5}", rate);
                self.note("Mission profile rate computed.");
            }
        });
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
                    let mut rows: Vec<(String, String)> = vec![("Design runs".into(), format!("{}", doe::factorial_design(k).len()))];
                    for e in &effects {
                        rows.push((format!("Term {}", e.name), format!("effect {:.3}, sum of squares {:.3}", e.effect, e.sum_squares)));
                    }
                    self.doe_result = rows.iter().map(|(a, b)| format!("{}: {}", a, b)).collect::<Vec<String>>().join("\n");
                    self.note("DOE analysis complete.");
                }
                Err(e) => self.doe_result = format!("Error: {}", e),
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
                        let rows: Vec<(String, String)> = vec![
                            ("Mean time".into(), format!("{:.3} (sd {:.3})", r.mean_time, r.sd_time)),
                            ("P10".into(), format!("{:.3}", r.p10)),
                            ("P50".into(), format!("{:.3}", r.p50)),
                            ("P90".into(), format!("{:.3}", r.p90)),
                            ("Min".into(), format!("{:.3}", r.min_time)),
                            ("Max".into(), format!("{:.3}", r.max_time)),
                            ("Simulations".into(), format!("{}", r.n_sim)),
                            ("Mean cost".into(), format!("{:.3}", r.mean_cost)),
                        ];
                        self.flow_result = rows.iter().map(|(a, b)| format!("{}: {}", a, b)).collect::<Vec<String>>().join("\n");
                        self.note("Flowchart simulation complete.");
                    }
                    Err(e) => self.flow_result = format!("Error: {}", e),
                },
                Err(e) => self.flow_result = format!("JSON error: {}", e),
            }
        }
        result_box(ui, &self.flow_result.clone());
    }

    fn page_markov(&mut self, ui: &mut egui::Ui) {
        ui.heading("Markov chain");
        ui.label("Preset: two-unit parallel repairable system, lambda 0.01 per hour, mu 0.1 per hour. The preset gives MTTF 650 hours from Both up. Add a repair transition out of System down to get a non-trivial steady state.");
        if ui.button("Load two-unit parallel repairable preset").clicked() {
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
        section(ui, "States");
        let mut remove_state: Option<usize> = None;
        for (i, s) in self.mk_states.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("State {}:", i));
                ui.label(s.clone());
                if ui.button("Remove").clicked() {
                    remove_state = Some(i);
                }
            });
        }
        if let Some(i) = remove_state {
            let name = self.mk_states.remove(i);
            self.mk_transitions.retain(|t| t[0] != name && t[1] != name);
        }
        ui.horizontal(|ui| {
            ui.label("New state:");
            ui.add(egui::TextEdit::singleline(&mut self.mk_new_state).desired_width(150.0));
            if ui.button("Add state").clicked() {
                let name = self.mk_new_state.trim().to_string();
                if !name.is_empty() && !self.mk_states.contains(&name) {
                    self.mk_states.push(name);
                    self.mk_new_state.clear();
                }
            }
        });
        section(ui, "Transitions");
        let transitions = &mut self.mk_transitions;
        let mut remove_tr: Option<usize> = None;
        TableBuilder::new(ui)
            .striped(true)
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .header(18.0, |mut h| {
                for t in ["From state", "To state", "Rate per hour", "Action"] {
                    h.col(|ui| {
                        ui.strong(t);
                    });
                }
            })
            .body(|mut body| {
                for (i, tr) in transitions.iter_mut().enumerate() {
                    body.row(22.0, |mut row| {
                        row.col(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut tr[0]).desired_width(130.0));
                        });
                        row.col(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut tr[1]).desired_width(130.0));
                        });
                        row.col(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut tr[2]).desired_width(80.0));
                        });
                        row.col(|ui| {
                            if ui.button("Remove").clicked() {
                                remove_tr = Some(i);
                            }
                        });
                    });
                }
            });
        if let Some(i) = remove_tr {
            self.mk_transitions.remove(i);
        }
        if ui.button("Add transition").clicked() {
            let a = self.mk_states.first().cloned().unwrap_or_default();
            let b = self.mk_states.get(1).cloned().unwrap_or_default();
            self.mk_transitions.push([a, b, "0.01".into()]);
        }
        section(ui, "Compute");
        ui.horizontal(|ui| {
            ui.label("Start state:");
            egui::ComboBox::from_id_salt("mk_start")
                .selected_text(self.mk_start.clone())
                .show_ui(ui, |ui| {
                    for s in self.mk_states.clone() {
                        ui.selectable_value(&mut self.mk_start, s.clone(), s);
                    }
                });
            ui.label("Absorbing state (for MTTF):");
            egui::ComboBox::from_id_salt("mk_abs")
                .selected_text(self.mk_absorbing.clone())
                .show_ui(ui, |ui| {
                    for s in self.mk_states.clone() {
                        ui.selectable_value(&mut self.mk_absorbing, s.clone(), s);
                    }
                });
            ui.label("Transient time:");
            ui.add(egui::TextEdit::singleline(&mut self.mk_time).desired_width(70.0));
        });
        if ui.button("Compute Markov results").clicked() {
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
            } else {
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
                                Ok(v) => rows.push((format!("MTTF from {} to {}", self.mk_start, self.mk_absorbing), format!("{:.2} hours", v))),
                                Err(e) => rows.push(("MTTF error".into(), e)),
                            }
                            match model.expected_time_in_state(&absorbing, start) {
                                Ok(v) => {
                                    for (n, x) in names.iter().zip(&v) {
                                        rows.push((format!("Expected time in {}", n), format!("{:.2} hours", x)));
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
                                            rows.push((format!("P({}) at t={:.0}", n, t), format!("{:.5}", p)));
                                        }
                                    }
                                }
                                Err(e) => rows.push(("Transient error".into(), e)),
                            }
                        }
                        self.mk_result = rows.iter().map(|(a, b)| format!("{}: {}", a, b)).collect::<Vec<String>>().join("\n");
                        self.note("Markov computation complete.");
                    }
                    Err(e) => self.mk_result = format!("Error: {}", e),
                }
            }
        }
        result_box(ui, &self.mk_result.clone());
    }

    fn page_qcp(&mut self, ui: &mut egui::Ui) {
        ui.heading("QCP (quick calculation pad)");
        ui.horizontal(|ui| {
            ui.label("Distribution:");
            let before = self.qcp_dist.clone();
            dist_combo(ui, "qcp_dist", &mut self.qcp_dist);
            if self.qcp_dist != before {
                self.qcp_reset_params();
            }
        });
        if let Some(spec) = dist_mod::dist_spec(&self.qcp_dist) {
            let names: Vec<String> = spec.iter().map(|(n, _)| n.to_string()).collect();
            for name in names {
                ui.horizontal(|ui| {
                    ui.label(format!("{}:", name));
                    let entry = self.qcp_params.entry(name.clone()).or_insert_with(|| Self::qcp_default(&self.qcp_dist, &name).to_string());
                    ui.add(egui::TextEdit::singleline(entry).desired_width(110.0));
                });
            }
        }
        ui.horizontal(|ui| {
            ui.label("Mission t:");
            ui.add(egui::TextEdit::singleline(&mut self.qcp_t).desired_width(80.0));
            ui.label("Bx fraction (0.10 = B10):");
            ui.add(egui::TextEdit::singleline(&mut self.qcp_bx).desired_width(60.0));
            if ui.button("Calculate").clicked() {
                let mut params = dist_mod::Params::new();
                for (k, v) in &self.qcp_params {
                    if let Some(x) = opt_num(v) {
                        params.insert(k.clone(), x);
                    }
                }
                let t = fnum(&self.qcp_t, 1000.0);
                let bx = fnum(&self.qcp_bx, 0.10);
                match quickcalc::quick_calc(&self.qcp_dist, &params, t, bx) {
                    Ok(r) => {
                        let rows: Vec<(String, String)> = vec![
                            (format!("Reliability R({:.0})", t), format!("{:.5}", r.reliability)),
                            (format!("Unreliability F({:.0})", t), format!("{:.5}", r.unreliability)),
                            (format!("Hazard at {:.0}", t), format!("{:.6}", r.hazard)),
                            (format!("Pdf at {:.0}", t), format!("{:.6}", r.pdf)),
                            ("Mean life".into(), format!("{:.2}", r.mean_life)),
                            (format!("B{:.0} life", bx * 100.0), format!("{:.2}", r.bx_life)),
                        ];
                        self.qcp_result = rows.iter().map(|(a, b)| format!("{}: {}", a, b)).collect::<Vec<String>>().join("\n");
                        self.headline = format!("QCP R({:.0}) = {:.5}", t, r.reliability);
                        self.note("QCP calculation complete.");
                    }
                    Err(e) => self.qcp_result = format!("Error: {}", e),
                }
            }
        });
        result_box(ui, &self.qcp_result.clone());
    }

    fn page_rcm(&mut self, ui: &mut egui::Ui) {
        ui.heading("RCM worksheet");
        ui.label("Task selection and criticality update live from the row inputs. Criticality needs failure rate, mode ratio, effect probability, and mission hours.");
        if ui.button("Add row").clicked() {
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
        }
        let consequences = ["Hidden", "Safety", "Environmental", "Operational", "NonOperational"];
        let mut remove: Option<usize> = None;
        egui::Grid::new("rcm_grid")
            .num_columns(9)
            .spacing([8.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                for h in ["Asset", "Function", "Failure mode", "Consequence", "Flags", "Rate, alpha, beta, hours", "Task", "Criticality", "Action"] {
                    ui.strong(h);
                }
                ui.end_row();
                for (i, row) in self.rcm_rows.iter_mut().enumerate() {
                    ui.add(egui::TextEdit::singleline(&mut row.asset).desired_width(90.0));
                    ui.add(egui::TextEdit::singleline(&mut row.function).desired_width(100.0));
                    ui.add(egui::TextEdit::singleline(&mut row.mode).desired_width(110.0));
                    egui::ComboBox::from_id_salt(format!("rcm_cons_{}", i))
                        .selected_text(row.consequence.clone())
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            for c in consequences {
                                ui.selectable_value(&mut row.consequence, c.to_string(), c);
                            }
                        });
                    ui.vertical(|ui| {
                        ui.checkbox(&mut row.monitorable, "Condition monitorable");
                        ui.checkbox(&mut row.age_related, "Age related");
                        ui.checkbox(&mut row.restorable, "Restorable");
                    });
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("rate:");
                            ui.add(egui::TextEdit::singleline(&mut row.rate).desired_width(70.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label("alpha:");
                            ui.add(egui::TextEdit::singleline(&mut row.alpha).desired_width(45.0));
                            ui.label("beta:");
                            ui.add(egui::TextEdit::singleline(&mut row.beta).desired_width(45.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label("hours:");
                            ui.add(egui::TextEdit::singleline(&mut row.hours).desired_width(60.0));
                        });
                    });
                    let mode = row.to_mode();
                    ui.label(rcm::select_task(&mode).as_str());
                    ui.label(match mode.criticality() {
                        Some(v) => format!("{:.6}", v),
                        None => "-".into(),
                    });
                    if ui.button("Remove").clicked() {
                        remove = Some(i);
                    }
                    ui.end_row();
                }
            });
        if let Some(i) = remove {
            self.rcm_rows.remove(i);
        }
    }

    fn page_fracas(&mut self, ui: &mut egui::Ui) {
        ui.heading("FRACAS incident log");
        section(ui, "Summary");
        let by_status = fracas::count_by_status(&self.fracas_rows);
        let by_mode = fracas::count_by_mode(&self.fracas_rows);
        let mut rows: Vec<(String, String)> = by_status.iter().map(|(s, c)| (format!("Status {}", s), format!("{}", c))).collect();
        for (m, c) in &by_mode {
            rows.push((format!("Mode {}", m), format!("{}", c)));
        }
        match fracas::mtbf_hours(fnum(&self.fracas_total_hours, 0.0), &self.fracas_rows) {
            Some(v) => rows.push(("MTBF (hours)".into(), format!("{:.1}", v))),
            None => rows.push(("MTBF (hours)".into(), "no incidents".into())),
        }
        grid_pairs(ui, "fracas_summary", &rows);
        ui.horizontal(|ui| {
            ui.label("Total operating hours (for MTBF):");
            ui.add(egui::TextEdit::singleline(&mut self.fracas_total_hours).desired_width(90.0));
            if ui.button("Add incident").clicked() {
                let n = self.fracas_rows.len() + 1;
                self.fracas_rows.push(Incident {
                    id: format!("FR-{}", n),
                    part: String::new(),
                    failure_mode: String::new(),
                    hours: 0.0,
                    date: String::new(),
                    status: IncidentStatus::Open,
                    corrective_action: String::new(),
                });
            }
        });
        section(ui, "Incidents");
        let incidents = &mut self.fracas_rows;
        let mut remove: Option<usize> = None;
        TableBuilder::new(ui)
            .striped(true)
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .header(18.0, |mut h| {
                for t in ["Id", "Part", "Failure mode", "Hours", "Date", "Status", "Corrective action", "Action"] {
                    h.col(|ui| {
                        ui.strong(t);
                    });
                }
            })
            .body(|mut body| {
                for (i, inc) in incidents.iter_mut().enumerate() {
                    body.row(22.0, |mut row| {
                        row.col(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut inc.id).desired_width(60.0));
                        });
                        row.col(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut inc.part).desired_width(110.0));
                        });
                        row.col(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut inc.failure_mode).desired_width(110.0));
                        });
                        row.col(|ui| {
                            let mut hs = format!("{}", inc.hours);
                            if ui.add(egui::TextEdit::singleline(&mut hs).desired_width(70.0)).changed() {
                                if let Ok(v) = hs.trim().parse::<f64>() {
                                    inc.hours = v;
                                }
                            }
                        });
                        row.col(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut inc.date).desired_width(90.0));
                        });
                        row.col(|ui| {
                            let mut sel = inc.status.as_str().to_string();
                            egui::ComboBox::from_id_salt(format!("fracas_status_{}", i))
                                .selected_text(sel.clone())
                                .width(120.0)
                                .show_ui(ui, |ui| {
                                    for s in IncidentStatus::workflow_order() {
                                        ui.selectable_value(&mut sel, s.as_str().to_string(), s.as_str());
                                    }
                                });
                            if let Ok(st) = IncidentStatus::from_str(&sel) {
                                inc.status = st;
                            }
                        });
                        row.col(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut inc.corrective_action).desired_width(170.0));
                        });
                        row.col(|ui| {
                            if ui.button("Remove").clicked() {
                                remove = Some(i);
                            }
                        });
                    });
                }
            });
        if let Some(i) = remove {
            self.fracas_rows.remove(i);
        }
        section(ui, "Send incidents to Life Data");
        ui.horizontal(|ui| {
            icon_widget(ui, "export");
            if ui.button("Export to Life Data item").clicked() {
                self.flow_fracas_to_life();
            }
            ui.label("Each incident hour value becomes a failure time. All incidents count as failures, because the FRACAS log has no censoring notion.");
        });
        section(ui, "CSV import and export");
        ui.horizontal(|ui| {
            ui.label("CSV file path:");
            ui.add(egui::TextEdit::singleline(&mut self.fracas_path).desired_width(240.0));
            if ui.button("Export CSV").clicked() {
                match std::fs::write(&self.fracas_path, fracas::incidents_to_csv(&self.fracas_rows)) {
                    Ok(()) => self.fracas_status = format!("Exported {} incidents to {}", self.fracas_rows.len(), self.fracas_path),
                    Err(e) => self.fracas_status = format!("Export error: {}", e),
                }
            }
            if ui.button("Import CSV").clicked() {
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
            }
        });
        if !self.fracas_status.is_empty() {
            ui.label(self.fracas_status.clone());
        }
    }

    fn page_start(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let (lrect, _) = ui.allocate_exact_size(Vec2::new(56.0, 56.0), Sense::hover());
            draw_logo(&ui.painter_at(lrect), lrect);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("ReliaKit").size(32.0).strong());
                ui.label("One project workspace. Each item in the project holds its own analysis, and items can send results to each other.");
            });
        });
        ui.add_space(16.0);
        draw_hero(ui);
        ui.add_space(8.0);
        section(ui, "Project");
        ui.horizontal(|ui| {
            icon_widget(ui, "plus");
            if ui.button("New project").clicked() {
                self.new_project();
            }
            ui.label("Project file path:");
            ui.add(egui::TextEdit::singleline(&mut self.project_path).desired_width(300.0));
            icon_widget(ui, "open");
            if ui.button("Open project").clicked() {
                self.open_project();
            }
        });
        if self.items.is_empty() {
            ui.label("This project has no items yet. Add an item from the project panel on the left, or load an example or a dataset below.");
        } else {
            ui.label(format!("This project has {} items. Click one in the project panel to open it, or add another below.", self.items.len()));
        }
        section(ui, "Examples");
        ui.label("Each example loads as a new item in this project.");
        let examples: [(usize, &str); 8] = [
            (1, "Example 1: bearing life data"),
            (2, "Example 2: limited data lots"),
            (3, "Example 3: RBD with standby"),
            (4, "Example 4: FMEA worksheet"),
            (5, "Example 5: ALT dataset"),
            (6, "Example 6: growth dataset"),
            (7, "Example 7: degradation dataset"),
            (8, "Example 8: Markov two-unit parallel repairable"),
        ];
        let mut pending_example: Option<usize> = None;
        for (n, label) in examples {
            ui.horizontal(|ui| {
                icon_widget(ui, "star");
                if ui.button(label).clicked() {
                    pending_example = Some(n);
                }
            });
        }
        if let Some(n) = pending_example {
            self.load_example_item(n);
        }
        section(ui, "Bundled datasets");
        ui.label("Each dataset loads as a new item of the kind it suits, following the categories in datasets/README.md. Life files go to Life Data, ALT files to ALT, degradation files to Degradation, growth files to Growth, and the Nevada warranty file to Warranty.");
        let mut pending_dataset: Option<String> = None;
        for f in BUNDLED_CSV {
            ui.horizontal(|ui| {
                icon_widget(ui, dataset_kind(f).icon());
                if ui.button(format!("{} ({})", f, dataset_kind(f).label())).clicked() {
                    pending_dataset = Some(f.to_string());
                }
            });
        }
        if let Some(f) = pending_dataset {
            self.load_dataset_item(&f);
        }
    }
}
