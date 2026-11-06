//! ReliaKit desktop application: a Slint interface over the reliakit
//! library. All state and computation live in `work.rs` (ported from
//! the v0.2.0 egui app); this file is the pull/push glue between the
//! Slint globals declared in ui/state.slint and that work copy, plus
//! callback dispatch. The project file format (reliakit-project-2)
//! is unchanged from v0.2.0.

#![recursion_limit = "256"]

mod charts;
mod work;

use slint::{Image, Model, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::rc::Rc;
use work::*;

slint::include_modules!();

type Pairs = Vec<(String, String)>;

const MCMC_DISTS: [&str; 2] = ["weibull2", "lognormal"];
const DEG_MODELS: [&str; 3] = ["linear", "exponential", "power"];
const ALT_MODELS: [&str; 3] = ["arrhenius", "ipl", "eyring"];
const TOPOLOGIES: [&str; 4] = ["series", "parallel", "k-of-n", "series with standby group"];
const NB_MODES: [&str; 4] = ["active", "hot standby", "cold standby", "warm"];
const FMEA_TYPES: [&str; 3] = ["Design", "Process", "System"];
const PRED_SOURCES: [&str; 3] = ["Generic user data", "MIL-HDBK-217F library", "Local library file"];
const ENVIRONMENTS: [&str; 7] = ["GB", "GF", "GM", "NS", "NU", "AIC", "CL"];
const CONSEQUENCES: [&str; 5] = ["Hidden", "Safety", "Environmental", "Operational", "NonOperational"];
const PTH_NAMED: [(&str, &str); 3] = [
    ("pth_technology", "Printed Wiring Assembly/Printed Circuit Boards with PTHs"),
    ("pth_quality", "MIL-SPEC or IPC Level 3"),
    ("pth_environment", "GB"),
];
const PTH_NUM: [(&str, &str); 3] = [
    ("pth_n1", "100"),
    ("pth_n2", "20"),
    ("pth_planes", "4"),
];

fn ss(s: &str) -> SharedString {
    SharedString::from(s)
}

fn mrc<T: Clone + 'static>(v: Vec<T>) -> ModelRc<T> {
    ModelRc::new(Rc::new(VecModel::from(v)))
}

fn pairs_model(rows: &Pairs) -> ModelRc<PairRow> {
    mrc(rows
        .iter()
        .map(|(a, b)| PairRow { label: ss(a), value: ss(b) })
        .collect())
}

fn rows_model(text: &str) -> ModelRc<PairRow> {
    pairs_model(&rows_of(text))
}

fn cells_model(rows: &[Vec<String>]) -> ModelRc<Cells> {
    mrc(rows
        .iter()
        .map(|r| Cells {
            cells: mrc(r.iter().map(|c| ss(c)).collect()),
        })
        .collect())
}

fn strings_model(v: &[String]) -> ModelRc<SharedString> {
    mrc(v.iter().map(|s| ss(s)).collect())
}

fn str_model(v: &[&str]) -> ModelRc<SharedString> {
    mrc(v.iter().map(|s| ss(s)).collect())
}

fn idx_of(list: &[&str], v: &str) -> i32 {
    list.iter().position(|x| *x == v).map(|i| i as i32).unwrap_or(0)
}

fn at(list: &[&str], idx: i32) -> String {
    list.get(idx.max(0) as usize).copied().unwrap_or(list[0]).to_string()
}

fn icon_image(name: &str) -> Image {
    let data: &[u8] = match name {
        "life" => include_bytes!("../ui/icons/life.svg"),
        "limited" => include_bytes!("../ui/icons/limited.svg"),
        "mcmc" => include_bytes!("../ui/icons/mcmc.svg"),
        "degradation" => include_bytes!("../ui/icons/degradation.svg"),
        "alt" => include_bytes!("../ui/icons/alt.svg"),
        "growth" => include_bytes!("../ui/icons/growth.svg"),
        "competing" => include_bytes!("../ui/icons/competing.svg"),
        "rbd" => include_bytes!("../ui/icons/rbd.svg"),
        "faulttree" => include_bytes!("../ui/icons/faulttree.svg"),
        "fmea" => include_bytes!("../ui/icons/fmea.svg"),
        "warranty" => include_bytes!("../ui/icons/warranty.svg"),
        "prediction" => include_bytes!("../ui/icons/prediction.svg"),
        "doe" => include_bytes!("../ui/icons/doe.svg"),
        "flowchart" => include_bytes!("../ui/icons/flowchart.svg"),
        "markov" => include_bytes!("../ui/icons/markov.svg"),
        "qcp" => include_bytes!("../ui/icons/qcp.svg"),
        "rcm" => include_bytes!("../ui/icons/rcm.svg"),
        "fracas" => include_bytes!("../ui/icons/fracas.svg"),
        "dataset" => include_bytes!("../ui/icons/dataset.svg"),
        _ => include_bytes!("../ui/icons/life.svg"),
    };
    Image::load_from_svg_data(data).unwrap_or_default()
}

// ---------------- app state ----------------

struct App {
    work: Work,
    blocks_model: Rc<VecModel<BlockGeom>>,
    drag_grab: Option<(f32, f32)>,
}

impl App {
    fn new() -> Self {
        App {
            work: Work::new(),
            blocks_model: Rc::new(VecModel::default()),
            drag_grab: None,
        }
    }
}

type AppRef = Rc<RefCell<App>>;

fn current_kind(w: &Work) -> Option<ItemKind> {
    w.selected.and_then(|i| w.items.get(i)).map(|it| it.kind)
}

fn current_title(w: &Work) -> String {
    match w.selected.and_then(|i| w.items.get(i)) {
        Some(it) => format!("{} - {}", it.name, it.kind.label()),
        None => "Start page".to_string(),
    }
}

// ---------------- pushes ----------------

fn push_shell(ui: &AppWindow, w: &Work) {
    let a = ui.global::<AppState>();
    let screen = match current_kind(w) {
        Some(k) => k.key(),
        None => "start",
    };
    a.set_screen(ss(screen));
    a.set_project_name(ss(&w.project_name));
    a.set_project_path(ss(&w.project_path));
    a.set_status(ss(&w.status));
    a.set_headline(ss(&w.headline));
    let items: Vec<ItemRow> = w
        .items
        .iter()
        .map(|it| ItemRow {
            id: it.id as i32,
            kind: ss(it.kind.key()),
            kind_label: ss(it.kind.label()),
            name: ss(&it.name),
            icon: icon_image(it.kind.icon()),
        })
        .collect();
    a.set_items(mrc(items));
    a.set_selected_index(w.selected.map(|i| i as i32).unwrap_or(-1));
    a.set_current_title(ss(&current_title(w)));
    a.set_item_count(w.items.len() as i32);
    a.set_rename_name(ss(&w.rename_name));
    let c = ui.global::<CsvUi>();
    c.set_pick_index(idx_of(&BUNDLED_CSV, &w.csv_pick));
    c.set_path(ss(&w.csv_path));
}

fn push_life(ui: &AppWindow, w: &Work) {
    let g = ui.global::<LifeUi>();
    g.set_times(ss(&w.life_times));
    g.set_dist_index(idx_of(&ALL_DISTS, &w.life_dist));
    g.set_rank(w.life_rank);
    g.set_t(ss(&w.life_t));
    g.set_show_prob(w.life_show_prob);
    g.set_results(rows_model(&w.life_result));
    g.set_has_fit(w.life_fit.is_some());
    match &w.life_fit {
        Some(fit) => {
            let t = fnum(&w.life_t, 1000.0);
            g.set_headline_value(ss(&format!("R({:.0}) = {:.4}", t, fit.reliability(t))));
            match charts::life_chart(fit, w.life_show_prob) {
                Some(img) => {
                    g.set_chart(img);
                    g.set_has_chart(true);
                }
                None => g.set_has_chart(false),
            }
            g.set_chart_title(ss(if w.life_show_prob {
                "Probability plot"
            } else {
                "Reliability curve"
            }));
        }
        None => {
            g.set_headline_value(ss(""));
            g.set_has_chart(false);
        }
    }
    let wiz: Vec<WizardRow> = w
        .wizard_ranks
        .iter()
        .map(|r| WizardRow {
            dist: ss(&r.dist),
            label: ss(reliakit::distributions::dist_label(&r.dist)),
            loglik: ss(&format!("{:.2}", r.loglik)),
            aic: ss(&format!("{:.2}", r.aic)),
            bic: ss(&format!("{:.2}", r.bic)),
            ad: ss(&r.ad_statistic.map(|v| format!("{:.3}", v)).unwrap_or_else(|| "-".into())),
            usable: r.success,
        })
        .collect();
    g.set_wizard_rows(mrc(wiz));
    g.set_wizard_note(ss(&w.wizard_note));
    // Send-fit panel: RBD targets and the target item's block names.
    let targets: Vec<String> = w
        .items
        .iter()
        .filter(|it| it.kind == ItemKind::Rbd)
        .map(|it| it.name.clone())
        .collect();
    let target_pos = w
        .items
        .iter()
        .filter(|it| it.kind == ItemKind::Rbd)
        .position(|it| it.id == w.flow_target_id)
        .map(|i| i as i32)
        .unwrap_or(0);
    g.set_rbd_targets(strings_model(&targets));
    g.set_target_index(target_pos);
    g.set_block_names(strings_model(&w.flow_target_block_names()));
    g.set_block_index(w.flow_block_idx as i32);
}

fn push_limited(ui: &AppWindow, w: &Work) {
    let g = ui.global::<LimitedUi>();
    g.set_groups(ss(&w.limited_groups));
    g.set_results(rows_model(&w.limited_result));
}

fn push_mcmc(ui: &AppWindow, w: &Work) {
    let g = ui.global::<McmcUi>();
    g.set_times(ss(&w.mcmc_times));
    g.set_dist_index(idx_of(&MCMC_DISTS, &w.mcmc_dist));
    g.set_draws(ss(&w.mcmc_draws));
    g.set_warmup(ss(&w.mcmc_warmup));
    g.set_chains(ss(&w.mcmc_chains));
    g.set_seed(ss(&w.mcmc_seed));
    g.set_t(ss(&w.mcmc_t));
    g.set_results(rows_model(&w.mcmc_result));
    g.set_headline_value(ss(if w.mcmc_result.is_empty() {
        ""
    } else {
        &w.headline
    }));
}

fn push_deg(ui: &AppWindow, w: &Work) {
    let g = ui.global::<DegUi>();
    g.set_units(ss(&w.deg_units));
    g.set_threshold(ss(&w.deg_threshold));
    g.set_model_index(idx_of(&DEG_MODELS, &w.deg_model));
    g.set_dist_index(idx_of(&ALL_DISTS, &w.deg_dist));
    g.set_increasing(w.deg_increasing);
    g.set_results(rows_model(&w.deg_result));
    let chart = match &w.deg_data {
        Some(res) => charts::degradation_chart(
            &w.deg_parse_units(),
            res,
            &w.deg_model,
            fnum(&w.deg_threshold, 70.0),
        ),
        None => None,
    };
    match chart {
        Some(img) => {
            g.set_chart(img);
            g.set_has_chart(true);
        }
        None => g.set_has_chart(false),
    }
}

fn push_alt(ui: &AppWindow, w: &Work) {
    let g = ui.global::<AltUi>();
    g.set_data(ss(&w.alt_data));
    g.set_model_index(idx_of(&ALT_MODELS, &w.alt_model));
    g.set_use_stress(ss(&w.alt_use_stress));
    g.set_t(ss(&w.alt_t));
    g.set_results(rows_model(&w.alt_result));
    match charts::alt_chart(&w.alt_series) {
        Some(img) => {
            g.set_chart(img);
            g.set_has_chart(true);
        }
        None => g.set_has_chart(false),
    }
}

fn push_growth(ui: &AppWindow, w: &Work) {
    let g = ui.global::<GrowthUi>();
    g.set_times(ss(&w.growth_times));
    g.set_results(rows_model(&w.growth_result));
    let chart = match &w.growth_fit {
        Some(fit) => charts::growth_chart(fit, &parse_numbers(&w.growth_times)),
        None => None,
    };
    match chart {
        Some(img) => {
            g.set_chart(img);
            g.set_has_chart(true);
        }
        None => g.set_has_chart(false),
    }
}

fn push_comp(ui: &AppWindow, w: &Work) {
    let g = ui.global::<CompUi>();
    g.set_data(ss(&w.comp_data));
    g.set_dist_index(idx_of(&ALL_DISTS, &w.comp_dist));
    g.set_t(ss(&w.comp_t));
    g.set_results(rows_model(&w.comp_result));
}

const BLOCK_W: f32 = 128.0;
const BLOCK_H: f32 = 56.0;

fn block_geom(w: &Work, idx: usize) -> Option<BlockGeom> {
    let paths = Work::leaf_paths(&w.rbd_spec);
    let path = paths.get(idx)?;
    let node = Work::node_at(&w.rbd_spec, path)?;
    let (x, y) = Work::leaf_pos(&node, idx);
    let ty = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let name = node
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("block")
        .to_string();
    let sub = if ty == "standby_group" {
        let n = node.get("n_units").and_then(|v| v.as_u64()).unwrap_or(2);
        let d = node.get("dormancy").and_then(|v| v.as_f64()).unwrap_or(0.0);
        format!("standby x{} - dormancy {:.2}", n, d)
    } else {
        let dist = node.get("dist").and_then(|v| v.as_str()).unwrap_or("");
        reliakit::distributions::dist_label(dist).to_string()
    };
    Some(BlockGeom {
        idx: idx as i32,
        x,
        y,
        w: BLOCK_W,
        h: BLOCK_H,
        name: ss(&name),
        sub: ss(&sub),
        selected: w.rbd_sel == Some(idx),
    })
}

fn rbd_geoms(w: &Work) -> Vec<BlockGeom> {
    (0..Work::leaf_paths(&w.rbd_spec).len())
        .filter_map(|i| block_geom(w, i))
        .collect()
}

/// Connector bars: leaves chained in reading order by their canvas
/// positions, drawn as thin rectangles forming elbow lines between
/// block edges. Bar thickness 2 px. Note: leaves inside a parallel
/// group are chained on the same line as their neighbours, the same
/// simplification the egui build used; the diagram JSON carries the
/// real topology.
fn segments_vec(w: &Work) -> Vec<Segment> {
    const T: f32 = 2.0;
    let mut out: Vec<Segment> = Vec::new();
    let mut bar = |x: f32, y: f32, wdt: f32, hgt: f32| {
        if wdt > 0.0 && hgt > 0.0 {
            out.push(Segment { x, y, w: wdt, h: hgt });
        }
    };
    let paths = Work::leaf_paths(&w.rbd_spec);
    for pair in paths.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        let ia = paths.iter().position(|p| p == a).unwrap_or(0);
        let ib = ia + 1;
        let (ga, gb) = match (block_geom(w, ia), block_geom(w, ib)) {
            (Some(x), Some(y)) => (x, y),
            _ => continue,
        };
        let (x1, y1) = (ga.x + BLOCK_W, ga.y + BLOCK_H / 2.0);
        let (x2, y2) = (gb.x, gb.y + BLOCK_H / 2.0);
        if y2 > y1 + BLOCK_H {
            // Target sits clearly below: route out of the source
            // bottom, across at mid height, and into the target top.
            let sx = ga.x + BLOCK_W / 2.0;
            let sy = ga.y + BLOCK_H;
            let tx = gb.x + BLOCK_W / 2.0;
            let ty = gb.y;
            let midy = (sy + ty) / 2.0;
            bar(sx - T / 2.0, sy, T, midy - sy);
            let (hx, hw) = if sx < tx { (sx, tx - sx) } else { (tx, sx - tx) };
            bar(hx, midy - T / 2.0, hw, T);
            bar(tx - T / 2.0, midy, T, ty - midy);
        } else if (y1 - y2).abs() < 1.0 && x2 >= x1 {
            bar(x1, y1 - T / 2.0, x2 - x1, T);
        } else {
            let mid = if x2 >= x1 { (x1 + x2) / 2.0 } else { x1 + 16.0 };
            bar(x1, y1 - T / 2.0, mid - x1, T);
            let (top, bot) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
            bar(mid - T / 2.0, top, T, bot - top);
            let (hx, hw) = if x2 >= mid { (mid, x2 - mid) } else { (x2, mid - x2) };
            bar(hx, y2 - T / 2.0, hw, T);
        }
    }
    out
}

fn push_rbd_inspector(ui: &AppWindow, w: &Work) {
    let g = ui.global::<RbdUi>();
    g.set_has_selection(w.rbd_sel.is_some());
    if let Some(idx) = w.rbd_sel {
        let paths = Work::leaf_paths(&w.rbd_spec);
        let title = paths
            .get(idx)
            .and_then(|p| Work::node_at(&w.rbd_spec, p))
            .map(|n| {
                let name = n.get("name").and_then(|v| v.as_str()).unwrap_or("block");
                let ty = n.get("type").and_then(|v| v.as_str()).unwrap_or("");
                format!("{} ({})", name, ty)
            })
            .unwrap_or_default();
        g.set_sel_title(ss(&title));
        let is_standby = paths
            .get(idx)
            .and_then(|p| Work::node_at(&w.rbd_spec, p))
            .and_then(|n| n.get("type").and_then(|v| v.as_str()).map(|t| t == "standby_group"))
            .unwrap_or(false);
        g.set_ed_is_standby(is_standby);
    }
    g.set_ed_name(ss(&w.ed_name));
    g.set_ed_dist_index(idx_of(&ALL_DISTS, &w.ed_dist));
    g.set_ed_params(ss(&w.ed_params));
    g.set_ed_mtbf(ss(&w.ed_mtbf));
    g.set_ed_mttr(ss(&w.ed_mttr));
    g.set_ed_units(ss(&w.ed_units));
    g.set_ed_dormancy(ss(&w.ed_dormancy));
    g.set_ed_switch(ss(&w.ed_switch));
}

fn push_rbd(ui: &AppWindow, app: &App, rebuild_blocks: bool) {
    let w = &app.work;
    let g = ui.global::<RbdUi>();
    g.set_t(ss(&w.rbd_t));
    g.set_horizon(ss(&w.rbd_horizon));
    g.set_topology_index(idx_of(&TOPOLOGIES, &w.rbd_topology));
    g.set_k(ss(&w.rbd_k));
    g.set_rel_label(ss(&format!("Reliability at t = {}", w.rbd_t)));
    g.set_rel_value(ss(&w
        .rbd_reliability()
        .map(|r| format!("{:.4}", r))
        .unwrap_or_default()));
    g.set_avail_results(rows_model(&w.rbd_avail));
    g.set_armed(ss(w.rbd_armed.as_deref().unwrap_or("")));
    if rebuild_blocks {
        app.blocks_model.set_vec(rbd_geoms(w));
    } else {
        // Light refresh (selection during press/drag): update rows in
        // place so live TouchArea delegates are not recreated.
        for (i, geom) in rbd_geoms(w).into_iter().enumerate() {
            let cur = app.blocks_model.row_data(i);
            let differs = match &cur {
                Some(c) => {
                    c.selected != geom.selected || c.x != geom.x || c.y != geom.y
                }
                None => true,
            };
            if differs {
                app.blocks_model.set_row_data(i, geom);
            }
        }
    }
    g.set_segments(slint::ModelRc::new(slint::VecModel::from(segments_vec(w))));
    push_rbd_inspector(ui, w);
    g.set_nb_name(ss(&w.nb_name));
    g.set_nb_dist_index(idx_of(&ALL_DISTS, &w.nb_dist));
    g.set_nb_params(ss(&w.nb_params));
    g.set_nb_mtbf(ss(&w.nb_mtbf));
    g.set_nb_mttr(ss(&w.nb_mttr));
    g.set_nb_mode_index(idx_of(&NB_MODES, &w.nb_mode));
    g.set_nb_dormancy(ss(&w.nb_dormancy));
    g.set_json(ss(&w.rbd_json));
    g.set_path(ss(&w.rbd_path));
    g.set_status(ss(&w.rbd_status));
    let imp: Vec<Vec<String>> = w
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
    g.set_importance_rows(cells_model(&imp));
    g.set_importance_note(ss(&w.rbd_importance_note));
}

fn push_ft(ui: &AppWindow, w: &Work) {
    let g = ui.global::<FtUi>();
    g.set_json(ss(&w.ft_json));
    g.set_results(rows_model(&w.ft_result));
}

fn push_fmea(ui: &AppWindow, w: &Work) {
    let g = ui.global::<FmeaUi>();
    g.set_type_index(idx_of(&FMEA_TYPES, &w.fmea_type));
    g.set_fm_item(ss(&w.fm_item));
    g.set_fm_mode(ss(&w.fm_mode));
    g.set_fm_sev(ss(&w.fm_sev));
    g.set_fm_occ(ss(&w.fm_occ));
    g.set_fm_det(ss(&w.fm_det));
    g.set_fm_alpha(ss(&w.fm_alpha));
    g.set_fm_beta(ss(&w.fm_beta));
    g.set_fm_lambda(ss(&w.fm_lambda));
    g.set_fm_hours(ss(&w.fm_hours));
    let rows: Vec<Vec<String>> = w
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
                w.fmea_criticality(i)
                    .map(|v| format!("{:.4}", v))
                    .unwrap_or_default(),
            ]
        })
        .collect();
    g.set_rows(cells_model(&rows));
    let dvpr: Vec<Vec<String>> = w
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
    g.set_dvpr_rows(cells_model(&dvpr));
    g.set_dv_id(ss(&w.dv_id));
    g.set_dv_ref(ss(&w.dv_ref));
    g.set_dv_test(ss(&w.dv_test));
    g.set_dv_sample(ss(&w.dv_sample));
    g.set_dv_criteria(ss(&w.dv_criteria));
    g.set_csv(ss(&w.fmea_csv));
    g.set_export_path(ss(&w.fmea_export_path));
    g.set_rcm_row(ss(&w.fmea_rcm_row));
    g.set_status(ss(&w.fmea_status));
}

fn push_war(ui: &AppWindow, w: &Work) {
    let g = ui.global::<WarUi>();
    g.set_sales(ss(&w.war_sales));
    g.set_returns(ss(&w.war_returns));
    g.set_current(ss(&w.war_current));
    g.set_horizon(ss(&w.war_horizon));
    g.set_results(rows_model(&w.war_result));
    let ages = w.war_table.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut header = vec!["Sale period".to_string()];
    for a in 0..ages {
        header.push(format!("Age {}", a));
    }
    g.set_table_header(strings_model(&header));
    let rows: Vec<Vec<String>> = w
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
    g.set_table_rows(cells_model(&rows));
}

/// Field definitions for the MIL-HDBK-217F panel: (named, key, label,
/// options). PTH (Section 16.1) has its own fixed field set, matching
/// the v0.2.0 app; other categories derive from the category JSON.
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
        return w
            .pred217_named
            .get(key)
            .cloned()
            .unwrap_or_else(|| def.to_string());
    }
    if let Some((_, def)) = PTH_NUM.iter().find(|(k, _)| *k == key) {
        return w
            .pred217_num
            .get(key)
            .cloned()
            .unwrap_or_else(|| def.to_string());
    }
    if named {
        w.pred217_named.get(key).cloned().unwrap_or_default()
    } else {
        w.pred217_num.get(key).cloned().unwrap_or_default()
    }
}

/// Seed the defaults the v0.2.0 app applied while rendering, so a
/// compute on a freshly loaded category resolves.
fn pred217_seed_defaults(w: &mut Work) {
    if w.pred217_json.is_none() {
        return;
    }
    if w.pred217_is_pth() {
        for (k, def) in PTH_NAMED {
            w.pred217_named
                .entry(k.to_string())
                .or_insert_with(|| def.to_string());
        }
        for (k, def) in PTH_NUM {
            w.pred217_num
                .entry(k.to_string())
                .or_insert_with(|| def.to_string());
        }
    }
    if w.pred217_is_misc() {
        if !w.pred217_named.contains_key("part") {
            if let Some(first) = w.pred217_part_options().first().cloned() {
                w.pred217_named.insert("part".to_string(), first);
            }
        }
        w.pred217_named
            .entry("environment".to_string())
            .or_insert_with(|| "GB".to_string());
    }
}

fn push_pred(ui: &AppWindow, w: &Work) {
    let g = ui.global::<PredUi>();
    g.set_source_index(idx_of(&PRED_SOURCES, &w.pred_source));
    g.set_life_dist_index(idx_of(&ALL_DISTS, &w.pred_life_dist));
    g.set_life_params(ss(&w.pred_life_params));
    g.set_life_t(ss(&w.pred_life_t));
    g.set_life_result(ss(&w.pred_life_result));
    g.set_parts(ss(&w.pred_parts));
    g.set_t(ss(&w.pred_t));
    g.set_parts_results(rows_model(&w.pred_result));
    g.set_lib_path(ss(&w.pred_lib_path));
    g.set_lib_note(ss(&w.pred_lib_note));
    let lib_rows: Vec<Vec<String>> = w
        .pred_lib_parts
        .iter()
        .map(|p| {
            vec![
                p.name.clone(),
                p.quantity.to_string(),
                format!(
                    "{:.3} per Mh",
                    p.base_rate_per_million_hours
                        * p.quantity as f64
                        * p.stress_factor
                        * p.quality_factor
                ),
            ]
        })
        .collect();
    g.set_lib_rows(cells_model(&lib_rows));
    g.set_lib_results(pairs_model(&w.pred_lib_rows()));
    g.set_p217_file_index(idx_of(&MIL217_FILES, &w.pred217_file));
    let category_text = match &w.pred217_json {
        Some(cat) => {
            let name = cat
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or(&w.pred217_file);
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
    g.set_p217_category(ss(&category_text));
    g.set_p217_misc(w.pred217_is_misc());
    g.set_p217_pth(w.pred217_is_pth());
    let specs = pred217_specs(w);
    let fields: Vec<FieldSpec> = specs
        .iter()
        .map(|(named, key, label, options)| {
            let value = pred217_value(w, *named, key);
            FieldSpec {
                id: ss("p217"),
                key: ss(key),
                label: ss(label),
                named: *named,
                options: strings_model(&options.iter().map(|s| s.clone()).collect::<Vec<_>>()),
                value: ss(&value),
                sel: options
                    .iter()
                    .position(|o| *o == value)
                    .map(|i| i as i32)
                    .unwrap_or(0),
            }
        })
        .collect();
    g.set_p217_fields(mrc(fields));
    let part_options = w.pred217_part_options();
    g.set_p217_part_options(strings_model(&part_options));
    let part = w.pred217_named.get("part").cloned().unwrap_or_default();
    g.set_p217_part_index(
        part_options
            .iter()
            .position(|p| *p == part)
            .map(|i| i as i32)
            .unwrap_or(0),
    );
    let env = w
        .pred217_named
        .get("environment")
        .cloned()
        .unwrap_or_else(|| "GB".to_string());
    g.set_p217_env_index(idx_of(&ENVIRONMENTS, &env));
    g.set_p217_results(rows_model(&w.pred217_result));
    g.set_alloc_target(ss(&w.alloc_target));
    g.set_alloc_n(ss(&w.alloc_n));
    g.set_alloc_weights(ss(&w.alloc_weights));
    g.set_alloc_results(rows_model(&w.alloc_result));
    g.set_der_name(ss(&w.der_name));
    g.set_der_operating(ss(&w.der_operating));
    g.set_der_rated(ss(&w.der_rated));
    g.set_der_max(ss(&w.der_max));
    g.set_der_results(rows_model(&w.der_result));
    g.set_sr_generic(ss(&w.sr_generic));
    g.set_sr_pi_q(ss(&w.sr_pi_q));
    g.set_sr_pi_s(ss(&w.sr_pi_s));
    g.set_sr_ea(ss(&w.sr_ea));
    g.set_sr_temp(ss(&w.sr_temp));
    g.set_sr_ref_temp(ss(&w.sr_ref_temp));
    g.set_sr_pi_e(ss(&w.sr_pi_e));
    g.set_sr_qty_rate(ss(&w.sr_qty_rate));
    g.set_sr_obs_rate(ss(&w.sr_obs_rate));
    g.set_sr_pit_obs(ss(&w.sr_pit_obs));
    g.set_sr_pit_tgt(ss(&w.sr_pit_tgt));
    g.set_sr_pis_obs(ss(&w.sr_pis_obs));
    g.set_sr_pis_tgt(ss(&w.sr_pis_tgt));
    g.set_sr_ci_failures(ss(&w.sr_ci_failures));
    g.set_sr_ci_exposure(ss(&w.sr_ci_exposure));
    g.set_sr_ci_conf(ss(&w.sr_ci_conf));
    g.set_sr_m3_pred(ss(&w.sr_m3_pred));
    g.set_sr_m3_pw(ss(&w.sr_m3_pw));
    g.set_sr_m3_obs(ss(&w.sr_m3_obs));
    g.set_sr_m3_ow(ss(&w.sr_m3_ow));
    g.set_mp_base(ss(&w.mp_base));
    g.set_mp_ea(ss(&w.mp_ea));
    g.set_mp_ref(ss(&w.mp_ref));
    g.set_mp_cyc(ss(&w.mp_cyc_factor));
    g.set_mp_phases(ss(&w.mp_phases));
    g.set_clean_results(rows_model(&w.pred_clean_result));
}

fn push_doe(ui: &AppWindow, w: &Work) {
    let g = ui.global::<DoeUi>();
    g.set_k(ss(&w.doe_k));
    g.set_y(ss(&w.doe_y));
    g.set_results(rows_model(&w.doe_result));
}

fn push_flow(ui: &AppWindow, w: &Work) {
    let g = ui.global::<FlowUi>();
    g.set_json(ss(&w.flow_json));
    g.set_sims(ss(&w.flow_sims));
    g.set_seed(ss(&w.flow_seed));
    g.set_results(rows_model(&w.flow_result));
}

fn push_markov(ui: &AppWindow, w: &Work) {
    let g = ui.global::<MarkovUi>();
    g.set_states(strings_model(&w.mk_states));
    g.set_new_state(ss(&w.mk_new_state));
    let trs: Vec<Vec<String>> = w
        .mk_transitions
        .iter()
        .map(|t| vec![t[0].clone(), t[1].clone(), t[2].clone()])
        .collect();
    g.set_transitions(cells_model(&trs));
    g.set_sel_transition(w.mk_sel.map(|i| i as i32).unwrap_or(-1));
    match w.mk_sel.and_then(|i| w.mk_transitions.get(i)) {
        Some(t) => {
            g.set_tr_from(ss(&t[0]));
            g.set_tr_to(ss(&t[1]));
            g.set_tr_rate(ss(&t[2]));
        }
        None => {
            g.set_tr_from(ss(""));
            g.set_tr_to(ss(""));
            g.set_tr_rate(ss(""));
        }
    }
    g.set_start_index(
        w.mk_states
            .iter()
            .position(|s| *s == w.mk_start)
            .map(|i| i as i32)
            .unwrap_or(0),
    );
    g.set_absorbing_index(
        w.mk_states
            .iter()
            .position(|s| *s == w.mk_absorbing)
            .map(|i| i as i32)
            .unwrap_or(0),
    );
    g.set_time(ss(&w.mk_time));
    g.set_results(rows_model(&w.mk_result));
    match charts::markov_chart(&w.mk_series) {
        Some(img) => {
            g.set_chart(img);
            g.set_has_chart(true);
        }
        None => g.set_has_chart(false),
    }
}

fn push_qcp(ui: &AppWindow, w: &Work) {
    let g = ui.global::<QcpUi>();
    g.set_dist_index(idx_of(&ALL_DISTS, &w.qcp_dist));
    let specs = reliakit::distributions::dist_spec(&w.qcp_dist).unwrap_or_default();
    let fields: Vec<FieldSpec> = specs
        .iter()
        .map(|(name, _kind)| FieldSpec {
            id: ss("qcp"),
            key: ss(name),
            label: ss(name),
            named: false,
            options: mrc(Vec::new()),
            value: ss(&w
                .qcp_params
                .get(*name)
                .cloned()
                .unwrap_or_else(|| Work::qcp_default(&w.qcp_dist, name).to_string())),
            sel: 0,
        })
        .collect();
    g.set_params(mrc(fields));
    g.set_t(ss(&w.qcp_t));
    g.set_bx(ss(&w.qcp_bx));
    g.set_results(rows_model(&w.qcp_result));
    let headline = rows_of(&w.qcp_result)
        .into_iter()
        .find(|(l, _)| l.starts_with("Reliability"))
        .map(|(_, v)| v)
        .unwrap_or_default();
    g.set_headline_value(ss(&headline));
}

fn push_rcm(ui: &AppWindow, w: &Work) {
    let g = ui.global::<RcmUi>();
    let rows: Vec<Vec<String>> = w
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
                mode.criticality()
                    .map(|v| format!("{:.4}", v))
                    .unwrap_or_default(),
            ]
        })
        .collect();
    g.set_rows(cells_model(&rows));
    g.set_sel(w.rcm_sel.map(|i| i as i32).unwrap_or(-1));
    if let Some(r) = w.rcm_sel.and_then(|i| w.rcm_rows.get(i)) {
        g.set_ed_asset(ss(&r.asset));
        g.set_ed_function(ss(&r.function));
        g.set_ed_mode(ss(&r.mode));
        g.set_ed_consequence_index(idx_of(&CONSEQUENCES, &r.consequence));
        g.set_ed_monitorable(r.monitorable);
        g.set_ed_age(r.age_related);
        g.set_ed_restorable(r.restorable);
        g.set_ed_rate(ss(&r.rate));
        g.set_ed_alpha(ss(&r.alpha));
        g.set_ed_beta(ss(&r.beta));
        g.set_ed_hours(ss(&r.hours));
        let mode = r.to_mode();
        g.set_ed_task(ss(reliakit::rcm::select_task(&mode).as_str()));
        g.set_ed_crit(ss(&mode
            .criticality()
            .map(|v| format!("{:.4}", v))
            .unwrap_or_default()));
    }
}

fn push_fracas(ui: &AppWindow, w: &Work) {
    use reliakit::fracas::IncidentStatus;
    let g = ui.global::<FracasUi>();
    let rows: Vec<Vec<String>> = w
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
    g.set_rows(cells_model(&rows));
    g.set_sel(w.fracas_sel.map(|i| i as i32).unwrap_or(-1));
    if let Some(inc) = w.fracas_sel.and_then(|i| w.fracas_rows.get(i)) {
        g.set_ed_id(ss(&inc.id));
        g.set_ed_part(ss(&inc.part));
        g.set_ed_mode(ss(&inc.failure_mode));
        g.set_ed_hours(ss(&format!("{:.0}", inc.hours)));
        g.set_ed_date(ss(&inc.date));
        g.set_ed_status_index(match inc.status {
            IncidentStatus::Open => 0,
            IncidentStatus::Analysis => 1,
            IncidentStatus::CorrectiveAction => 2,
            IncidentStatus::Closed => 3,
        });
        g.set_ed_action(ss(&inc.corrective_action));
    }
    g.set_total_hours(ss(&w.fracas_total_hours));
    g.set_summary(pairs_model(&w.fracas_summary_rows()));
    g.set_path(ss(&w.fracas_path));
    g.set_status(ss(&w.fracas_status));
}

fn push_current(ui: &AppWindow, app: &App) {
    let w = &app.work;
    push_shell(ui, w);
    match current_kind(w) {
        Some(ItemKind::LifeData) => push_life(ui, w),
        Some(ItemKind::LimitedData) => push_limited(ui, w),
        Some(ItemKind::Mcmc) => push_mcmc(ui, w),
        Some(ItemKind::Degradation) => push_deg(ui, w),
        Some(ItemKind::Alt) => push_alt(ui, w),
        Some(ItemKind::Growth) => push_growth(ui, w),
        Some(ItemKind::Competing) => push_comp(ui, w),
        Some(ItemKind::Rbd) => push_rbd(ui, app, true),
        Some(ItemKind::FaultTree) => push_ft(ui, w),
        Some(ItemKind::Fmea) => push_fmea(ui, w),
        Some(ItemKind::Warranty) => push_war(ui, w),
        Some(ItemKind::Prediction) => push_pred(ui, w),
        Some(ItemKind::Doe) => push_doe(ui, w),
        Some(ItemKind::Flowchart) => push_flow(ui, w),
        Some(ItemKind::Markov) => push_markov(ui, w),
        Some(ItemKind::Qcp) => push_qcp(ui, w),
        Some(ItemKind::Rcm) => push_rcm(ui, w),
        Some(ItemKind::Fracas) => push_fracas(ui, w),
        None => {}
    }
}

// ---------------- pulls ----------------

fn pull_shell(ui: &AppWindow, w: &mut Work) {
    let a = ui.global::<AppState>();
    w.project_name = a.get_project_name().to_string();
    w.project_path = a.get_project_path().to_string();
    w.new_kind = KIND_DEFS
        .get(a.get_kind_index().max(0) as usize)
        .map(|(_, l, _)| l.to_string())
        .unwrap_or_else(|| "Life Data".to_string());
    w.new_name = a.get_new_name().to_string();
    w.rename_name = a.get_rename_name().to_string();
    let c = ui.global::<CsvUi>();
    w.csv_pick = at(&BUNDLED_CSV, c.get_pick_index());
    w.csv_path = c.get_path().to_string();
}

fn pull_current(ui: &AppWindow, w: &mut Work) {
    pull_shell(ui, w);
    match current_kind(w) {
        Some(ItemKind::LifeData) => {
            let g = ui.global::<LifeUi>();
            w.life_times = g.get_times().to_string();
            w.life_dist = at(&ALL_DISTS, g.get_dist_index());
            w.life_rank = g.get_rank();
            w.life_t = g.get_t().to_string();
            w.life_show_prob = g.get_show_prob();
            let rbd_ids: Vec<u64> = w
                .items
                .iter()
                .filter(|it| it.kind == ItemKind::Rbd)
                .map(|it| it.id)
                .collect();
            if let Some(id) = rbd_ids.get(g.get_target_index().max(0) as usize) {
                w.flow_target_id = *id;
            }
            w.flow_block_idx = g.get_block_index().max(0) as usize;
        }
        Some(ItemKind::LimitedData) => {
            let g = ui.global::<LimitedUi>();
            w.limited_groups = g.get_groups().to_string();
        }
        Some(ItemKind::Mcmc) => {
            let g = ui.global::<McmcUi>();
            w.mcmc_times = g.get_times().to_string();
            w.mcmc_dist = at(&MCMC_DISTS, g.get_dist_index());
            w.mcmc_draws = g.get_draws().to_string();
            w.mcmc_warmup = g.get_warmup().to_string();
            w.mcmc_chains = g.get_chains().to_string();
            w.mcmc_seed = g.get_seed().to_string();
            w.mcmc_t = g.get_t().to_string();
        }
        Some(ItemKind::Degradation) => {
            let g = ui.global::<DegUi>();
            w.deg_units = g.get_units().to_string();
            w.deg_threshold = g.get_threshold().to_string();
            w.deg_model = at(&DEG_MODELS, g.get_model_index());
            w.deg_dist = at(&ALL_DISTS, g.get_dist_index());
            w.deg_increasing = g.get_increasing();
        }
        Some(ItemKind::Alt) => {
            let g = ui.global::<AltUi>();
            w.alt_data = g.get_data().to_string();
            w.alt_model = at(&ALT_MODELS, g.get_model_index());
            w.alt_use_stress = g.get_use_stress().to_string();
            w.alt_t = g.get_t().to_string();
        }
        Some(ItemKind::Growth) => {
            let g = ui.global::<GrowthUi>();
            w.growth_times = g.get_times().to_string();
        }
        Some(ItemKind::Competing) => {
            let g = ui.global::<CompUi>();
            w.comp_data = g.get_data().to_string();
            w.comp_dist = at(&ALL_DISTS, g.get_dist_index());
            w.comp_t = g.get_t().to_string();
        }
        Some(ItemKind::Rbd) => {
            let g = ui.global::<RbdUi>();
            w.rbd_t = g.get_t().to_string();
            w.rbd_horizon = g.get_horizon().to_string();
            w.rbd_topology = at(&TOPOLOGIES, g.get_topology_index());
            w.rbd_k = g.get_k().to_string();
            w.ed_name = g.get_ed_name().to_string();
            w.ed_dist = at(&ALL_DISTS, g.get_ed_dist_index());
            w.ed_params = g.get_ed_params().to_string();
            w.ed_mtbf = g.get_ed_mtbf().to_string();
            w.ed_mttr = g.get_ed_mttr().to_string();
            w.ed_units = g.get_ed_units().to_string();
            w.ed_dormancy = g.get_ed_dormancy().to_string();
            w.ed_switch = g.get_ed_switch().to_string();
            w.nb_name = g.get_nb_name().to_string();
            w.nb_dist = at(&ALL_DISTS, g.get_nb_dist_index());
            w.nb_params = g.get_nb_params().to_string();
            w.nb_mtbf = g.get_nb_mtbf().to_string();
            w.nb_mttr = g.get_nb_mttr().to_string();
            w.nb_mode = at(&NB_MODES, g.get_nb_mode_index());
            w.nb_dormancy = g.get_nb_dormancy().to_string();
            w.rbd_json = g.get_json().to_string();
            w.rbd_path = g.get_path().to_string();
        }
        Some(ItemKind::FaultTree) => {
            let g = ui.global::<FtUi>();
            w.ft_json = g.get_json().to_string();
        }
        Some(ItemKind::Fmea) => {
            let g = ui.global::<FmeaUi>();
            w.fmea_type = at(&FMEA_TYPES, g.get_type_index());
            w.fm_item = g.get_fm_item().to_string();
            w.fm_mode = g.get_fm_mode().to_string();
            w.fm_sev = g.get_fm_sev().to_string();
            w.fm_occ = g.get_fm_occ().to_string();
            w.fm_det = g.get_fm_det().to_string();
            w.fm_alpha = g.get_fm_alpha().to_string();
            w.fm_beta = g.get_fm_beta().to_string();
            w.fm_lambda = g.get_fm_lambda().to_string();
            w.fm_hours = g.get_fm_hours().to_string();
            w.dv_id = g.get_dv_id().to_string();
            w.dv_ref = g.get_dv_ref().to_string();
            w.dv_test = g.get_dv_test().to_string();
            w.dv_sample = g.get_dv_sample().to_string();
            w.dv_criteria = g.get_dv_criteria().to_string();
            w.fmea_csv = g.get_csv().to_string();
            w.fmea_export_path = g.get_export_path().to_string();
            w.fmea_rcm_row = g.get_rcm_row().to_string();
        }
        Some(ItemKind::Warranty) => {
            let g = ui.global::<WarUi>();
            w.war_sales = g.get_sales().to_string();
            w.war_returns = g.get_returns().to_string();
            w.war_current = g.get_current().to_string();
            w.war_horizon = g.get_horizon().to_string();
        }
        Some(ItemKind::Prediction) => {
            let g = ui.global::<PredUi>();
            w.pred_source = at(&PRED_SOURCES, g.get_source_index());
            w.pred_life_dist = at(&ALL_DISTS, g.get_life_dist_index());
            w.pred_life_params = g.get_life_params().to_string();
            w.pred_life_t = g.get_life_t().to_string();
            w.pred_parts = g.get_parts().to_string();
            w.pred_t = g.get_t().to_string();
            w.pred_lib_path = g.get_lib_path().to_string();
            w.pred217_file = at(&MIL217_FILES, g.get_p217_file_index());
            if w.pred217_is_misc() {
                let opts = w.pred217_part_options();
                if let Some(p) = opts.get(g.get_p217_part_index().max(0) as usize) {
                    w.pred217_named.insert("part".to_string(), p.clone());
                }
                w.pred217_named.insert(
                    "environment".to_string(),
                    at(&ENVIRONMENTS, g.get_p217_env_index()),
                );
            }
            w.alloc_target = g.get_alloc_target().to_string();
            w.alloc_n = g.get_alloc_n().to_string();
            w.alloc_weights = g.get_alloc_weights().to_string();
            w.der_name = g.get_der_name().to_string();
            w.der_operating = g.get_der_operating().to_string();
            w.der_rated = g.get_der_rated().to_string();
            w.der_max = g.get_der_max().to_string();
            w.sr_generic = g.get_sr_generic().to_string();
            w.sr_pi_q = g.get_sr_pi_q().to_string();
            w.sr_pi_s = g.get_sr_pi_s().to_string();
            w.sr_ea = g.get_sr_ea().to_string();
            w.sr_temp = g.get_sr_temp().to_string();
            w.sr_ref_temp = g.get_sr_ref_temp().to_string();
            w.sr_pi_e = g.get_sr_pi_e().to_string();
            w.sr_qty_rate = g.get_sr_qty_rate().to_string();
            w.sr_obs_rate = g.get_sr_obs_rate().to_string();
            w.sr_pit_obs = g.get_sr_pit_obs().to_string();
            w.sr_pit_tgt = g.get_sr_pit_tgt().to_string();
            w.sr_pis_obs = g.get_sr_pis_obs().to_string();
            w.sr_pis_tgt = g.get_sr_pis_tgt().to_string();
            w.sr_ci_failures = g.get_sr_ci_failures().to_string();
            w.sr_ci_exposure = g.get_sr_ci_exposure().to_string();
            w.sr_ci_conf = g.get_sr_ci_conf().to_string();
            w.sr_m3_pred = g.get_sr_m3_pred().to_string();
            w.sr_m3_pw = g.get_sr_m3_pw().to_string();
            w.sr_m3_obs = g.get_sr_m3_obs().to_string();
            w.sr_m3_ow = g.get_sr_m3_ow().to_string();
            w.mp_base = g.get_mp_base().to_string();
            w.mp_ea = g.get_mp_ea().to_string();
            w.mp_ref = g.get_mp_ref().to_string();
            w.mp_cyc_factor = g.get_mp_cyc().to_string();
            w.mp_phases = g.get_mp_phases().to_string();
        }
        Some(ItemKind::Doe) => {
            let g = ui.global::<DoeUi>();
            w.doe_k = g.get_k().to_string();
            w.doe_y = g.get_y().to_string();
        }
        Some(ItemKind::Flowchart) => {
            let g = ui.global::<FlowUi>();
            w.flow_json = g.get_json().to_string();
            w.flow_sims = g.get_sims().to_string();
            w.flow_seed = g.get_seed().to_string();
        }
        Some(ItemKind::Markov) => {
            let g = ui.global::<MarkovUi>();
            w.mk_new_state = g.get_new_state().to_string();
            if let Some(s) = w
                .mk_states
                .get(g.get_start_index().max(0) as usize)
                .cloned()
            {
                w.mk_start = s;
            }
            if let Some(s) = w
                .mk_states
                .get(g.get_absorbing_index().max(0) as usize)
                .cloned()
            {
                w.mk_absorbing = s;
            }
            w.mk_time = g.get_time().to_string();
            if let Some(i) = w.mk_sel {
                let vals = [
                    g.get_tr_from().to_string(),
                    g.get_tr_to().to_string(),
                    g.get_tr_rate().to_string(),
                ];
                if let Some(t) = w.mk_transitions.get_mut(i) {
                    *t = vals;
                }
            }
        }
        Some(ItemKind::Qcp) => {
            let g = ui.global::<QcpUi>();
            w.qcp_dist = at(&ALL_DISTS, g.get_dist_index());
            w.qcp_t = g.get_t().to_string();
            w.qcp_bx = g.get_bx().to_string();
        }
        Some(ItemKind::Rcm) => {
            let g = ui.global::<RcmUi>();
            if let Some(r) = w.rcm_sel.and_then(|i| w.rcm_rows.get_mut(i)) {
                r.asset = g.get_ed_asset().to_string();
                r.function = g.get_ed_function().to_string();
                r.mode = g.get_ed_mode().to_string();
                r.consequence = at(&CONSEQUENCES, g.get_ed_consequence_index());
                r.monitorable = g.get_ed_monitorable();
                r.age_related = g.get_ed_age();
                r.restorable = g.get_ed_restorable();
                r.rate = g.get_ed_rate().to_string();
                r.alpha = g.get_ed_alpha().to_string();
                r.beta = g.get_ed_beta().to_string();
                r.hours = g.get_ed_hours().to_string();
            }
        }
        Some(ItemKind::Fracas) => {
            use reliakit::fracas::IncidentStatus;
            let g = ui.global::<FracasUi>();
            w.fracas_total_hours = g.get_total_hours().to_string();
            w.fracas_path = g.get_path().to_string();
            if let Some(inc) = w.fracas_sel.and_then(|i| w.fracas_rows.get_mut(i)) {
                inc.id = g.get_ed_id().to_string();
                inc.part = g.get_ed_part().to_string();
                inc.failure_mode = g.get_ed_mode().to_string();
                if let Ok(h) = g.get_ed_hours().to_string().trim().parse::<f64>() {
                    inc.hours = h;
                }
                inc.date = g.get_ed_date().to_string();
                inc.status = match g.get_ed_status_index() {
                    1 => IncidentStatus::Analysis,
                    2 => IncidentStatus::CorrectiveAction,
                    3 => IncidentStatus::Closed,
                    _ => IncidentStatus::Open,
                };
                inc.corrective_action = g.get_ed_action().to_string();
            }
        }
        None => {}
    }
}

/// Cross-cutting fixes applied between pull and dispatch: keep the
/// Life Data flow target valid, and seed MIL-HDBK-217F defaults.
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

// ---------------- dispatch ----------------

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

fn dispatch_act(w: &mut Work, name: &str) {
    match name {
        "life-fit" => {
            w.compute_life_fit();
        }
        "life-wizard" => w.compute_wizard(),
        "flow-rbd" => w.flow_assign_fit_to_rbd(),
        "flow-qcp" => w.flow_send_fit_to_qcp(),
        "flow-pred" => w.flow_send_fit_to_prediction(),
        "flow-target" => {}
        "limited-run" => {
            w.compute_limited();
        }
        "mcmc-run" => {
            w.compute_mcmc();
        }
        "deg-run" => {
            w.compute_degradation();
        }
        "alt-run" => {
            w.compute_alt();
        }
        "growth-run" => {
            w.compute_growth();
        }
        "comp-run" => {
            w.compute_competing();
        }
        "rbd-avail" => {
            w.compute_rbd_availability();
        }
        "rbd-importance" => w.compute_rbd_importance(),
        "rbd-topology" => w.rbd_apply_topology(),
        "rbd-apply-props" => w.rbd_apply_editor(),
        "rbd-duplicate" => w.rbd_duplicate_block(),
        "rbd-delete" => w.rbd_delete_block(),
        "rbd-undo" => w.undo_rbd(),
        "rbd-add-form" => w.rbd_add_block_form(),
        "rbd-load-json" => w.rbd_load_json_text(),
        "rbd-save-file" => w.rbd_save_file(),
        "rbd-load-file" => w.rbd_load_file(),
        "ft-run" => {
            w.compute_fault_tree();
        }
        "fmea-add-row" => w.fmea_add_row(),
        "fmea-to-rcm" => w.flow_rcm_row_from_field(),
        "fmea-add-dvpr" => w.fmea_add_dvpr(),
        "fmea-export-ws" => w.fmea_export_worksheet(),
        "fmea-export-rows" => w.fmea_export_rows(),
        "fmea-import" => w.fmea_import_csv(),
        "fmea-clear" => w.fmea_clear_rows(),
        "fmea-export-cp" => w.fmea_export_control_plan(),
        "fmea-export-dvpr" => w.fmea_export_dvpr(),
        "war-run" => {
            w.compute_warranty();
        }
        "pred-life" => {
            w.compute_pred_life();
        }
        "pred-run" => {
            w.compute_pred_parts();
        }
        "pred-lib-load" => w.pred_lib_load(),
        "pred217-load" => {
            w.pred217_load();
            pred217_seed_defaults(w);
        }
        "pred217-compute" => w.compute_pred217(),
        "pred-alloc" => {
            w.compute_alloc();
        }
        "pred-derate" => {
            w.compute_derating();
        }
        "pred-clean" => {
            w.compute_clean_room();
        }
        "pred-mission" => {
            w.compute_mission();
        }
        "doe-run" => {
            w.compute_doe();
        }
        "flow-run" => {
            w.compute_flowchart();
        }
        "mk-preset" => w.markov_preset(),
        "mk-run" => {
            w.compute_markov();
        }
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
        }
        "mk-add-trans" => {
            if w.mk_states.len() >= 2 {
                w.mk_transitions.push([
                    w.mk_states[0].clone(),
                    w.mk_states[1].clone(),
                    "0.01".to_string(),
                ]);
                w.mk_sel = Some(w.mk_transitions.len() - 1);
                w.note("Transition added. Edit it in the inspector.");
            } else {
                w.note("Add at least two states before adding a transition.");
            }
        }
        "mk-apply-trans" => w.note("Transition updated."),
        "mk-del-trans" => {
            if let Some(i) = w.mk_sel.take() {
                if i < w.mk_transitions.len() {
                    w.mk_transitions.remove(i);
                }
                w.note("Transition removed.");
            }
        }
        "qcp-dist" => w.qcp_reset_params(),
        "qcp-run" => {
            w.compute_qcp();
        }
        "rcm-add" => w.rcm_add_row(),
        "rcm-refresh" => w.note("RCM row updated."),
        "rcm-remove" => {
            if let Some(i) = w.rcm_sel.take() {
                if i < w.rcm_rows.len() {
                    w.rcm_rows.remove(i);
                    w.note("RCM row removed.");
                }
            }
        }
        "fracas-add" => w.fracas_add_incident(),
        "fracas-refresh" => {
            w.fracas_status = "Incident updated.".to_string();
            w.note("Incident updated.");
        }
        "fracas-remove" => {
            if let Some(i) = w.fracas_sel.take() {
                if i < w.fracas_rows.len() {
                    w.fracas_rows.remove(i);
                    w.fracas_status = "Incident removed.".to_string();
                    w.note("Incident removed.");
                }
            }
        }
        "fracas-export-csv" => w.fracas_export_csv(),
        "fracas-import-csv" => w.fracas_import_csv(),
        "flow-fracas-life" => w.flow_fracas_to_life(),
        _ => {}
    }
}

fn dispatch_row(w: &mut Work, name: &str, i: i32) {
    let i = i.max(0) as usize;
    match name {
        "wizard-use" => w.wizard_use(i),
        "fmea-rcm-row" => w.flow_fmea_to_rcm(i),
        "fmea-del" => w.fmea_delete_row(i),
        "rcm-select" => w.rcm_sel = Some(i),
        "fracas-select" => w.fracas_sel = Some(i),
        "mk-select" => w.mk_sel = Some(i),
        "mk-del-state" => markov_remove_state(w, i),
        _ => {}
    }
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

// ---------------- wiring ----------------

fn with_work(weak: &slint::Weak<AppWindow>, app: &AppRef, f: impl FnOnce(&mut Work)) {
    let ui = weak.upgrade().unwrap();
    let mut app = app.borrow_mut();
    pull_current(&ui, &mut app.work);
    normalize(&mut app.work);
    f(&mut app.work);
    push_current(&ui, &app);
}

fn bind(ui: &AppWindow, app: &AppRef) {
    let a = ui.global::<Actions>();
    macro_rules! on {
        ($method:ident, $body:expr) => {{
            let app2 = app.clone();
            let weak = ui.as_weak();
            a.$method(move || {
                with_work(&weak, &app2, $body);
            });
        }};
    }
    // No-argument project actions share the pull/run/push cycle.
    on!(on_go_start, |w: &mut Work| w.select_index(None));
    on!(on_add_item, |w: &mut Work| w.add_item_from_panel());
    on!(on_remove_item, |w: &mut Work| w.remove_selected_item());
    on!(on_rename_item, |w: &mut Work| w.rename_selected_item());
    on!(on_new_project, |w: &mut Work| w.new_project());
    on!(on_save_project, |w: &mut Work| w.save_project());
    on!(on_open_project, |w: &mut Work| w.open_project());
    on!(on_csv_bundled, |w: &mut Work| w.csv_load_bundled());
    on!(on_csv_file, |w: &mut Work| w.csv_load_file());
    {
        let app2 = app.clone();
        let weak = ui.as_weak();
        a.on_select_item(move |i| {
            with_work(&weak, &app2, |w| w.select_index(Some(i.max(0) as usize)));
        });
    }
    {
        let app2 = app.clone();
        let weak = ui.as_weak();
        a.on_card_action(move |id| {
            let id = id.to_string();
            with_work(&weak, &app2, |w| {
                if let Some(n) = id.strip_prefix("example:") {
                    if let Ok(n) = n.parse::<usize>() {
                        w.load_example_item(n);
                    }
                } else if let Some(f) = id.strip_prefix("dataset:") {
                    w.load_dataset_item(f);
                }
            });
        });
    }
    {
        let app2 = app.clone();
        let weak = ui.as_weak();
        a.on_act(move |name| {
            let name = name.to_string();
            with_work(&weak, &app2, |w| dispatch_act(w, &name));
        });
    }
    {
        let app2 = app.clone();
        let weak = ui.as_weak();
        a.on_row_act(move |name, i| {
            let name = name.to_string();
            with_work(&weak, &app2, |w| dispatch_row(w, &name, i));
        });
    }
    {
        let app2 = app.clone();
        let weak = ui.as_weak();
        a.on_field_changed(move |id, value| {
            let id = id.to_string();
            let value = value.to_string();
            with_work(&weak, &app2, |w| dispatch_field(w, &id, &value));
        });
    }
    {
        let app2 = app.clone();
        let weak = ui.as_weak();
        a.on_rbd_arm(move |name| {
            let name = name.to_string();
            with_work(&weak, &app2, |w| {
                w.rbd_armed = if w.rbd_armed.as_deref() == Some(name.as_str()) {
                    None
                } else {
                    Some(name.clone())
                };
            });
        });
    }
    {
        let app2 = app.clone();
        let weak = ui.as_weak();
        a.on_rbd_place(move |x, y| {
            with_work(&weak, &app2, |w| w.rbd_place_armed(x, y));
        });
    }
    {
        // Press: select and load the inspector WITHOUT rebuilding the
        // block delegates, so the pointer grab survives into the drag.
        let app2 = app.clone();
        let weak = ui.as_weak();
        a.on_rbd_press(move |idx| {
            let ui = weak.upgrade().unwrap();
            let mut app = app2.borrow_mut();
            pull_current(&ui, &mut app.work);
            normalize(&mut app.work);
            app.work.load_rbd_editor(idx.max(0) as usize);
            app.drag_grab = None;
            push_shell(&ui, &app.work);
            push_rbd(&ui, &app, false);
        });
    }
    {
        // Drag: move the spec, update only the dragged row and the
        // connector path. No model reset while the pointer is down.
        let app2 = app.clone();
        let weak = ui.as_weak();
        a.on_rbd_drag(move |idx, x, y| {
            let ui = weak.upgrade().unwrap();
            let mut app = app2.borrow_mut();
            let idx = idx.max(0) as usize;
            let paths = Work::leaf_paths(&app.work.rbd_spec);
            if let Some(p) = paths.get(idx) {
                if let Some(node) = Work::node_at(&app.work.rbd_spec, p) {
                    let (bx, by) = Work::leaf_pos(&node, idx);
                    let grab = *app.drag_grab.get_or_insert((x - bx, y - by));
                    app.work.rbd_move_block(idx, x - grab.0, y - grab.1);
                }
            }
            if let Some(geom) = block_geom(&app.work, idx) {
                app.blocks_model.set_row_data(idx, geom);
            }
            ui.global::<RbdUi>()
                .set_segments(slint::ModelRc::new(slint::VecModel::from(segments_vec(&app.work))));
        });
    }
    {
        let app2 = app.clone();
        let weak = ui.as_weak();
        a.on_rbd_release(move |_idx| {
            let ui = weak.upgrade().unwrap();
            let mut app = app2.borrow_mut();
            app.drag_grab = None;
            pull_current(&ui, &mut app.work);
            push_current(&ui, &app);
        });
    }
}

fn start_cards() -> (Vec<Vec<CardInfo>>, Vec<Vec<CardInfo>>) {
    let examples: Vec<CardInfo> = [
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
    .map(|(n, tag, title, sub, icon)| CardInfo {
        title: ss(title),
        sub: ss(sub),
        tag: ss(tag),
        icon: icon_image(icon),
        action_id: ss(&format!("example:{}", n)),
    })
    .collect();
    let datasets: Vec<CardInfo> = BUNDLED_CSV
        .iter()
        .map(|f| {
            let kind = dataset_kind(f);
            CardInfo {
                title: ss(f),
                sub: ss(&format!("Loads into a {} item.", kind.label())),
                tag: ss(&kind.label().to_uppercase()),
                icon: icon_image(kind.icon()),
                action_id: ss(&format!("dataset:{}", f)),
            }
        })
        .collect();
    let chunk = |v: Vec<CardInfo>| -> Vec<Vec<CardInfo>> {
        v.chunks(4).map(|c| c.to_vec()).collect()
    };
    (chunk(examples), chunk(datasets))
}

/// No-window smoke check, as documented in the README: builds the
/// Example 3 diagram, prints its reliability and availability, runs
/// one life fit, and exits 0 only when the reliability matches.
fn headless() -> i32 {
    let mut w = Work::new();
    w.init_project();
    w.load_example_item(3);
    let rel = w.rbd_reliability().unwrap_or(f64::NAN);
    println!("Example 3 RBD reliability at t=1000: {:.6}", rel);
    for (a, b) in w.compute_rbd_availability() {
        println!("{}: {}", a, b);
    }
    w.load_example_item(1);
    for (a, b) in w.compute_life_fit().iter().take(4) {
        println!("{}: {}", a, b);
    }
    if (rel - 0.921419).abs() < 1e-3 {
        println!("reliakit-app headless check OK");
        0
    } else {
        println!("reliakit-app headless check FAILED: unexpected reliability");
        1
    }
}

fn main() {
    if std::env::args().any(|a| a == "--headless") {
        std::process::exit(headless());
    }
    charts::init_fonts();
    let ui = AppWindow::new().expect("failed to create the ReliaKit window");
    let app: AppRef = Rc::new(RefCell::new(App::new()));
    {
        let mut a = app.borrow_mut();
        a.work.init_project();
        a.work.select_index(None);
    }
    {
        let a = app.borrow();
        ui.global::<RbdUi>().set_blocks(ModelRc::from(a.blocks_model.clone()));
        ui.global::<CsvUi>().set_files(str_model(&BUNDLED_CSV));
        let (examples, datasets) = start_cards();
        let st = ui.global::<AppState>();
        st.set_example_rows(mrc(examples
            .iter()
            .map(|row| mrc(row.clone()))
            .collect()));
        st.set_dataset_rows(mrc(datasets
            .iter()
            .map(|row| mrc(row.clone()))
            .collect()));
    }
    push_current(&ui, &app.borrow());
    bind(&ui, &app);
    ui.run().expect("UI loop failed");
}

