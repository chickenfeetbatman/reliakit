// ReliaKit web shell: toolbar, source list sidebar, content, status bar.
// One codebase for the Tauri desktop host and the browser demo. All
// engine calls go through dispatch(action, extraFields), which collects
// the current field values from the DOM, sends { action, fields } and
// re-renders from the full state the engine returns. Typing on its own
// never dispatches; only buttons, selects the Slint UI marked as change
// actions, and explicit change or blur on dynamic parameter fields do.
import { Transport, downloadTextFile, pickTextFile } from "./wasm.js";
import { normalizeChart } from "./charts.js";
import { KIND_OPTIONS, SCREEN_RENDERERS, renderStart, BUNDLED_CSV, MIL217_FILES } from "./screens.js";
import { renderRbd, bindRbdCanvas } from "./rbd.js";
import { KIND_META, FAMILIES, kindIndex, kindByKey } from "./kinds.js";
import { initTours } from "./tours.js";

const VERSION = "v0.3.1";
const REPO_URL = "https://github.com/chickenfeetbatman/reliakit";
// Quickstart dismissal persists in localStorage where the host
// allows it. Some webview configurations block storage on the app
// origin, so an in-memory flag backs it up: Dismiss must always
// work, even when the preference cannot be saved.
let qsDismissedMem = false;
function qsDismissed() {
  if (qsDismissedMem) return true;
  try { return localStorage.getItem("reliakit.quickstart.dismissed") === "1"; } catch { return false; }
}
function qsSetDismissed(v) {
  qsDismissedMem = v;
  try {
    if (v) localStorage.setItem("reliakit.quickstart.dismissed", "1");
    else localStorage.removeItem("reliakit.quickstart.dismissed");
  } catch { /* memory flag above already recorded it */ }
}
const COMPUTE_ACTIONS = new Set(["life-fit", "life-wizard", "limited-run", "mcmc-run", "deg-run", "alt-run", "growth-run", "comp-run", "rbd-avail", "rbd-importance", "rbd-apply-props", "rbd-add-form", "ft-run", "war-run", "pred-life", "pred-run", "pred217-load", "pred217-compute", "pred-alloc", "pred-derate", "pred-clean", "pred-mission", "doe-run", "flow-run", "mk-run", "qcp-run", "csv-bundled", "csv-file"]);
const KIND_ICON = { life: "life", limited: "limited", mcmc: "mcmc", degradation: "degradation", alt: "alt", growth: "growth", competing: "competing", rbd: "rbd", faulttree: "faulttree", fmea: "fmea", warranty: "warranty", prediction: "prediction", doe: "doe", flowchart: "flowchart", markov: "markov", qcp: "qcp", rcm: "rcm", fracas: "fracas" };
const KIND_LABEL = { life: "Life Data", limited: "Limited Data", mcmc: "MCMC", degradation: "Degradation", alt: "ALT", growth: "Growth", competing: "Competing Risks", rbd: "RBD", faulttree: "Fault Tree", fmea: "FMEA", warranty: "Warranty", prediction: "Prediction", doe: "DOE", flowchart: "Flowchart", markov: "Markov", qcp: "QCP", rcm: "RCM", fracas: "FRACAS" };

function esc(v) {
  return String(v == null ? "" : v).replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");
}
function pick(obj, keys, fallback) {
  if (!obj || typeof obj !== "object") return fallback;
  for (const k of keys) {
    if (obj[k] !== undefined && obj[k] !== null) return obj[k];
  }
  return fallback;
}
function asStr(v, fallback) { return v === undefined || v === null ? (fallback || "") : String(v); }
function asInt(v, fallback) { const n = parseInt(v, 10); return Number.isFinite(n) ? n : (fallback || 0); }
function asBool(v, fallback) {
  if (typeof v === "boolean") return v;
  if (v === "true") return true;
  if (v === "false") return false;
  return !!fallback;
}
function pairs(raw) {
  if (!raw) return [];
  if (typeof raw === "string") {
    return raw.split("\n").filter((l) => l.trim() !== "").map((l) => {
      const i = l.indexOf(": ");
      return i >= 0 ? [l.slice(0, i), l.slice(i + 2)] : ["", l];
    });
  }
  if (!Array.isArray(raw)) return [];
  return raw.map((r) => {
    if (Array.isArray(r)) return [asStr(r[0]), asStr(r[1])];
    if (r && typeof r === "object") return [asStr(r.label), asStr(r.value)];
    return ["", asStr(r)];
  });
}
function cellsRows(raw) {
  if (!Array.isArray(raw)) return [];
  return raw.map((r) => {
    if (Array.isArray(r)) return r.map((c) => asStr(c));
    if (r && Array.isArray(r.cells)) return r.cells.map((c) => asStr(c));
    return [asStr(r)];
  });
}
function fieldSpecs(raw) {
  if (!Array.isArray(raw)) return [];
  return raw.map((f) => ({
    id: asStr(f.id || f.key), key: asStr(f.key || f.id), label: asStr(f.label || f.key || f.id),
    named: !!f.named, options: Array.isArray(f.options) ? f.options.map(asStr) : [],
    value: asStr(f.value), sel: asInt(f.sel, 0),
  }));
}
function chartOf(obj, keys) {
  for (const k of keys) {
    const c = normalizeChart(obj[k]);
    if (c) return c;
  }
  return normalizeChart(obj);
}
function sub(raw, names) {
  for (const n of names) {
    if (raw && raw[n] && typeof raw[n] === "object") return raw[n];
  }
  return {};
}
function flatGet(raw, group, key, fallback) {
  const g = sub(raw, [group]);
  const direct = pick(g, [key], undefined);
  if (direct !== undefined) return direct;
  const flat = pick(raw, [group + "_" + key, group + "." + key], undefined);
  return flat !== undefined ? flat : fallback;
}
function cards(rawRows, fallbackRows) {
  if (!Array.isArray(rawRows) || rawRows.length === 0) return fallbackRows;
  const flat = [];
  for (const row of rawRows) {
    if (Array.isArray(row)) flat.push(row);
    else flat.push([row]);
  }
  return flat.map((row) => row.map((c) => ({
    title: asStr(c.title), sub: asStr(c.sub), tag: asStr(c.tag),
    icon: asStr(c.icon_name || c.iconName || KIND_ICON[asStr(c.icon)] || c.icon || "life"),
    actionId: asStr(c.action_id || c.actionId || c.action || ""),
  })));
}

function fallbackCards() {
  const examples = [
    [1, "EXAMPLE 1", "Bearing life data", "Weibull fit on the bundled bearing dataset.", "life"],
    [2, "EXAMPLE 2", "Limited data lots", "Small-sample lots for hierarchical shrinkage.", "limited"],
    [3, "EXAMPLE 3", "RBD with cold standby", "Series system, standby pumps, parallel controllers.", "rbd"],
    [4, "EXAMPLE 4", "FMEA sample", "Three sample worksheet rows with FMECA criticality.", "fmea"],
    [5, "EXAMPLE 5", "ALT dataset", "Arrhenius data for least squares and censored MLE.", "alt"],
    [6, "EXAMPLE 6", "Growth dataset", "Crow-AMSAA reliability growth data.", "growth"],
    [7, "EXAMPLE 7", "Degradation units", "Wear paths against a failure threshold.", "degradation"],
    [8, "EXAMPLE 8", "Markov preset", "Two-unit repairable system. MTTF is 650 hours.", "markov"],
  ].map(([n, tag, title, sub, icon]) => ({ title, sub, tag, icon, actionId: "example:" + n }));
  const kindFor = (f) => f.includes("_alt_") && !f.includes("degradation") ? "alt" : f.includes("degradation") ? "degradation" : f.includes("growth") ? "growth" : f.includes("warranty_nevada") ? "warranty" : "life";
  const datasets = BUNDLED_CSV.map((f) => {
    const k = kindFor(f);
    return { title: f, sub: "Loads into a " + (KIND_LABEL[k] || "Life Data") + " item.", tag: (KIND_LABEL[k] || "Life Data").toUpperCase(), icon: k, actionId: "dataset:" + f };
  });
  const chunk = (v) => { const out = []; for (let i = 0; i < v.length; i += 4) out.push(v.slice(i, i + 4)); return out; };
  return { exampleRows: chunk(examples), datasetRows: chunk(datasets) };
}

// ---------------------------------------------------------------------
// adaptState(raw): the single adapter between the engine JSON and the
// renderers. It accepts the Slint global names (state.slint), snake
// case, camel case, nested per screen objects, or flat prefixed keys,
// so an engine field name mismatch is fixed here and nowhere else.
// ---------------------------------------------------------------------
export function adaptState(raw) {
  const r = raw && typeof raw === "object" ? raw : {};
  const app = sub(r, ["app", "app_state", "appState", "shell"]);
  const fb = fallbackCards();
  const itemsRaw = pick(r, ["items"], pick(app, ["items"], [])) || [];
  const items = (Array.isArray(itemsRaw) ? itemsRaw : []).map((it) => {
    const kind = asStr(it.kind || it.key || "life");
    return {
      id: it.id, kind,
      kindLabel: asStr(it.kind_label || it.kindLabel || KIND_LABEL[kind] || kind),
      name: asStr(it.name || KIND_LABEL[kind] || "Item"),
      icon: asStr(it.icon_name || KIND_ICON[kind] || "life"),
    };
  });
  const csvObj = sub(r, ["csv", "csv_ui", "csvUi"]);
  const g = (group, key, fb2) => flatGet(r, group, key, fb2);
  const gs = (group, key, fb2) => asStr(g(group, key, fb2), fb2 || "");
  const gi = (group, key, fb2) => asInt(g(group, key, fb2), fb2 || 0);
  const gb = (group, key, fb2) => asBool(g(group, key, fb2), fb2);
  const lifeObj = sub(r, ["life", "life_ui", "lifeUi"]);
  const rbdObj = sub(r, ["rbd", "rbd_ui", "rbdUi"]);
  const predObj = sub(r, ["pred", "prediction", "pred_ui", "predUi"]);
  const state = {
    screen: asStr(pick(r, ["screen"], pick(app, ["screen"], "start")), "start"),
    projectName: asStr(pick(r, ["project_name", "projectName"], pick(app, ["project_name", "projectName"], "Untitled project")), "Untitled project"),
    projectPath: asStr(pick(r, ["project_path", "projectPath"], pick(app, ["project_path", "projectPath"], "reliakit_project.json")), "reliakit_project.json"),
    status: asStr(pick(r, ["status"], pick(app, ["status"], "Ready.")), "Ready."),
    headline: asStr(pick(r, ["headline"], pick(app, ["headline"], "")), ""),
    errorText: asStr(pick(r, ["error", "error_text", "errorText"], ""), ""),
    isError: asBool(pick(r, ["is_error", "isError"], false), false),
    items,
    selectedIndex: asInt(pick(r, ["selected_index", "selectedIndex"], pick(app, ["selected_index", "selectedIndex"], -1)), -1),
    currentTitle: asStr(pick(r, ["current_title", "currentTitle"], pick(app, ["current_title", "currentTitle"], "Start page")), "Start page"),
    kindIndex: asInt(pick(r, ["kind_index", "kindIndex"], pick(app, ["kind_index", "kindIndex"], 0)), 0),
    newName: asStr(pick(r, ["new_name", "newName"], pick(app, ["new_name", "newName"], "")), ""),
    renameName: asStr(pick(r, ["rename_name", "renameName"], pick(app, ["rename_name", "renameName"], "")), ""),
    itemCount: asInt(pick(r, ["item_count", "itemCount"], pick(app, ["item_count", "itemCount"], items.length)), items.length),
    exampleRows: cards(pick(r, ["example_rows", "exampleRows"], pick(app, ["example_rows", "exampleRows"], null)), fb.exampleRows),
    datasetRows: cards(pick(r, ["dataset_rows", "datasetRows"], pick(app, ["dataset_rows", "datasetRows"], null)), fb.datasetRows),
    projectJson: asStr(pick(r, ["project_json", "projectJson", "save_text", "saveText"], ""), ""),
    download: (r.download && typeof r.download === "object") ? { filename: asStr(r.download.filename || "download.txt"), text: asStr(r.download.text || "") } : null,
    csv: {
      files: (pick(csvObj, ["files"], null) || pick(r, ["csv_files"], null) || BUNDLED_CSV).map(asStr),
      pickIndex: asInt(pick(csvObj, ["pick_index", "pickIndex"], pick(r, ["csv_pick_index"], 10)), 10),
      path: asStr(pick(csvObj, ["path"], pick(r, ["csv_path"], "")), ""),
    },
    life: {
      times: gs("life", "times", ""), distIndex: gi("life", "dist_index", 0), rank: gb("life", "rank", false),
      t: gs("life", "t", "1000"), showProb: gb("life", "show_prob", false),
      results: pairs(pick(lifeObj, ["results"], pick(r, ["life_results"], null))),
      headlineValue: gs("life", "headline_value", ""), chart: chartOf(lifeObj, ["chart", "chart_series", "chartSeries", "series"]),
      chartTitle: gs("life", "chart_title", "Reliability curve"), hasChart: gb("life", "has_chart", false),
      hasFit: gb("life", "has_fit", false),
      wizardRows: (pick(lifeObj, ["wizard_rows", "wizardRows"], pick(r, ["life_wizard_rows"], [])) || []).map((w) => ({
        dist: asStr(w.dist), label: asStr(w.label || w.dist), loglik: asStr(w.loglik), aic: asStr(w.aic), bic: asStr(w.bic), ad: asStr(w.ad), usable: asBool(w.usable, false),
      })),
      wizardNote: gs("life", "wizard_note", ""),
      rbdTargets: (pick(lifeObj, ["rbd_targets", "rbdTargets"], []) || []).map(asStr),
      targetIndex: gi("life", "target_index", 0),
      blockNames: (pick(lifeObj, ["block_names", "blockNames"], []) || []).map(asStr),
      blockIndex: gi("life", "block_index", 0),
    },
    limited: { groups: gs("limited", "groups", ""), results: pairs(g("limited", "results", null)) },
    mcmc: {
      times: gs("mcmc", "times", ""), distIndex: gi("mcmc", "dist_index", 0),
      draws: gs("mcmc", "draws", "300"), warmup: gs("mcmc", "warmup", "300"), chains: gs("mcmc", "chains", "2"),
      seed: gs("mcmc", "seed", "42"), t: gs("mcmc", "t", "1000"),
      results: pairs(g("mcmc", "results", null)), headlineValue: gs("mcmc", "headline_value", ""),
    },
    deg: {
      units: gs("deg", "units", gs("degradation", "units", "")), threshold: gs("deg", "threshold", gs("degradation", "threshold", "70")),
      modelIndex: gi("deg", "model_index", gi("degradation", "model_index", 0)), distIndex: gi("deg", "dist_index", gi("degradation", "dist_index", 0)),
      increasing: gb("deg", "increasing", gb("degradation", "increasing", false)),
      results: pairs(g("deg", "results", g("degradation", "results", null))),
      chart: chartOf(sub(r, ["deg", "degradation"]), ["chart", "chart_series", "chartSeries", "series"]),
    },
    alt: {
      data: gs("alt", "data", ""), modelIndex: gi("alt", "model_index", 0),
      useStress: gs("alt", "use_stress", "55"), t: gs("alt", "t", "1000"),
      results: pairs(g("alt", "results", null)),
      chart: chartOf(sub(r, ["alt"]), ["chart", "chart_series", "chartSeries", "series"]),
    },
    growth: {
      times: gs("growth", "times", ""), results: pairs(g("growth", "results", null)),
      chart: chartOf(sub(r, ["growth"]), ["chart", "chart_series", "chartSeries", "series"]),
    },
    comp: {
      data: gs("comp", "data", gs("competing", "data", "")), distIndex: gi("comp", "dist_index", gi("competing", "dist_index", 0)),
      t: gs("comp", "t", gs("competing", "t", "1000")), results: pairs(g("comp", "results", g("competing", "results", null))),
    },
    rbd: {
      t: gs("rbd", "t", "1000"), horizon: gs("rbd", "horizon", "20000"), topologyIndex: gi("rbd", "topology_index", 0),
      k: gs("rbd", "k", "2"), relLabel: gs("rbd", "rel_label", "Reliability"), relValue: gs("rbd", "rel_value", ""),
      availResults: pairs(g("rbd", "avail_results", g("rbd", "availResults", null))),
      armed: gs("rbd", "armed", ""),
      blocks: (pick(rbdObj, ["blocks"], []) || []).map((b) => ({ idx: b.idx, x: b.x, y: b.y, w: b.w, h: b.h, name: asStr(b.name), sub: asStr(b.sub), selected: asBool(b.selected, false) })),
      segments: pick(rbdObj, ["segments"], []) || [],
      frames: (pick(rbdObj, ["frames"], []) || []).map((f) => ({ x: f.x, y: f.y, w: f.w, h: f.h, label: asStr(f.label) })),
      ghosts: (pick(rbdObj, ["ghosts"], []) || []).map((g2) => ({ x: g2.x, y: g2.y, w: g2.w, h: g2.h, idx: g2.idx })),
      hasSelection: gb("rbd", "has_selection", false), selTitle: gs("rbd", "sel_title", ""),
      edName: gs("rbd", "ed_name", ""), edDistIndex: gi("rbd", "ed_dist_index", 0), edParams: gs("rbd", "ed_params", ""),
      edMtbf: gs("rbd", "ed_mtbf", ""), edMttr: gs("rbd", "ed_mttr", ""), edUnits: gs("rbd", "ed_units", "2"),
      edDormancy: gs("rbd", "ed_dormancy", "0"), edSwitch: gs("rbd", "ed_switch", "1"), edIsStandby: gb("rbd", "ed_is_standby", false),
      nbName: gs("rbd", "nb_name", "new block"), nbDistIndex: gi("rbd", "nb_dist_index", 0), nbParams: gs("rbd", "nb_params", "beta=1.5, eta=5000"),
      nbMtbf: gs("rbd", "nb_mtbf", "5000"), nbMttr: gs("rbd", "nb_mttr", "10"), nbModeIndex: gi("rbd", "nb_mode_index", 0), nbDormancy: gs("rbd", "nb_dormancy", "0.5"),
      json: gs("rbd", "json", ""), path: gs("rbd", "path", "rbd.json"), status: gs("rbd", "status", ""),
      importanceRows: cellsRows(pick(rbdObj, ["importance_rows", "importanceRows"], [])),
      importanceNote: gs("rbd", "importance_note", ""),
    },
    ft: { json: gs("ft", "json", gs("faulttree", "json", "")), results: pairs(g("ft", "results", g("faulttree", "results", null))) },
    fmea: {
      typeIndex: gi("fmea", "type_index", 0), fmItem: gs("fmea", "fm_item", ""), fmMode: gs("fmea", "fm_mode", ""),
      fmSev: gs("fmea", "fm_sev", "5"), fmOcc: gs("fmea", "fm_occ", "5"), fmDet: gs("fmea", "fm_det", "5"),
      fmAlpha: gs("fmea", "fm_alpha", ""), fmBeta: gs("fmea", "fm_beta", ""), fmLambda: gs("fmea", "fm_lambda", ""), fmHours: gs("fmea", "fm_hours", ""),
      rows: cellsRows(g("fmea", "rows", [])), dvprRows: cellsRows(g("fmea", "dvpr_rows", [])),
      dvId: gs("fmea", "dv_id", ""), dvRef: gs("fmea", "dv_ref", ""), dvTest: gs("fmea", "dv_test", ""),
      dvSample: gs("fmea", "dv_sample", "3"), dvCriteria: gs("fmea", "dv_criteria", ""),
      csv: gs("fmea", "csv", ""), exportPath: gs("fmea", "export_path", "fmea_export.csv"),
      rcmRow: gs("fmea", "rcm_row", ""), status: gs("fmea", "status", ""),
    },
    war: {
      sales: gs("war", "sales", gs("warranty", "sales", "")), returns: gs("war", "returns", gs("warranty", "returns", "")),
      current: gs("war", "current", gs("warranty", "current", "3")), horizon: gs("war", "horizon", gs("warranty", "horizon", "6")),
      results: pairs(g("war", "results", g("warranty", "results", null))),
      tableHeader: (g("war", "table_header", g("warranty", "table_header", [])) || []).map(asStr),
      tableRows: cellsRows(g("war", "table_rows", g("warranty", "table_rows", []))),
    },
    pred: {
      sourceIndex: gi("pred", "source_index", gi("prediction", "source_index", 0)),
      lifeDistIndex: gi("pred", "life_dist_index", 0), lifeParams: gs("pred", "life_params", ""), lifeT: gs("pred", "life_t", "1000"), lifeResult: gs("pred", "life_result", ""),
      parts: gs("pred", "parts", ""), t: gs("pred", "t", "1000"), partsResults: pairs(pick(predObj, ["parts_results", "partsResults"], pick(r, ["pred_parts_results"], null))),
      libPath: gs("pred", "lib_path", ""), libNote: gs("pred", "lib_note", ""),
      libRows: cellsRows(pick(predObj, ["lib_rows", "libRows"], [])), libResults: pairs(pick(predObj, ["lib_results", "libResults"], null)),
      p217FileIndex: gi("pred", "p217_file_index", 14), p217Category: gs("pred", "p217_category", ""),
      p217Fields: fieldSpecs(pick(predObj, ["p217_fields", "p217Fields"], [])), p217Misc: gb("pred", "p217_misc", false),
      p217PartOptions: (pick(predObj, ["p217_part_options", "p217PartOptions"], []) || []).map(asStr),
      p217PartIndex: gi("pred", "p217_part_index", 0), p217EnvIndex: gi("pred", "p217_env_index", 0),
      p217Results: pairs(pick(predObj, ["p217_results", "p217Results"], null)),
      allocTarget: gs("pred", "alloc_target", "0.99"), allocN: gs("pred", "alloc_n", "3"), allocWeights: gs("pred", "alloc_weights", "1, 2, 1"),
      allocResults: pairs(pick(predObj, ["alloc_results", "allocResults"], null)),
      derName: gs("pred", "der_name", "Resistor"), derOperating: gs("pred", "der_operating", "0.5"), derRated: gs("pred", "der_rated", "1.0"), derMax: gs("pred", "der_max", "0.7"),
      derResults: pairs(pick(predObj, ["der_results", "derResults"], null)),
      srGeneric: gs("pred", "sr_generic", "1.0"), srPiQ: gs("pred", "sr_pi_q", "1.0"), srPiS: gs("pred", "sr_pi_s", "1.0"),
      srEa: gs("pred", "sr_ea", "0.35"), srTemp: gs("pred", "sr_temp", "55"), srRefTemp: gs("pred", "sr_ref_temp", "25"), srPiE: gs("pred", "sr_pi_e", "1.0"),
      srQtyRate: gs("pred", "sr_qty_rate", "10,1.0\n5,2.0"), srObsRate: gs("pred", "sr_obs_rate", "1.2"),
      srPitObs: gs("pred", "sr_pit_obs", "1.0"), srPitTgt: gs("pred", "sr_pit_tgt", "1.5"),
      srPisObs: gs("pred", "sr_pis_obs", "1.0"), srPisTgt: gs("pred", "sr_pis_tgt", "1.0"),
      srCiFailures: gs("pred", "sr_ci_failures", "3"), srCiExposure: gs("pred", "sr_ci_exposure", "100000"), srCiConf: gs("pred", "sr_ci_conf", "0.90"),
      srM3Pred: gs("pred", "sr_m3_pred", "1.0"), srM3Pw: gs("pred", "sr_m3_pw", "1.0"), srM3Obs: gs("pred", "sr_m3_obs", "1.4"), srM3Ow: gs("pred", "sr_m3_ow", "1.0"),
      mpBase: gs("pred", "mp_base", "1.0"), mpEa: gs("pred", "mp_ea", "0.35"), mpRef: gs("pred", "mp_ref", "25"), mpCyc: gs("pred", "mp_cyc", "0.001"),
      mpPhases: gs("pred", "mp_phases", ""), cleanResults: pairs(pick(predObj, ["clean_results", "cleanResults"], null)),
    },
    doe: { k: gs("doe", "k", "3"), y: gs("doe", "y", ""), results: pairs(g("doe", "results", null)) },
    flow: { json: gs("flow", "json", gs("flowchart", "json", "")), sims: gs("flow", "sims", gs("flowchart", "sims", "5000")), seed: gs("flow", "seed", gs("flowchart", "seed", "1")), results: pairs(g("flow", "results", g("flowchart", "results", null))) },
    markov: {
      states: (g("markov", "states", []) || []).map(asStr), newState: gs("markov", "new_state", ""),
      transitions: cellsRows(g("markov", "transitions", [])), selTransition: gi("markov", "sel_transition", -1),
      trFrom: gs("markov", "tr_from", ""), trTo: gs("markov", "tr_to", ""), trRate: gs("markov", "tr_rate", ""),
      startIndex: gi("markov", "start_index", 0), absorbingIndex: gi("markov", "absorbing_index", 2), time: gs("markov", "time", "100"),
      results: pairs(g("markov", "results", null)),
      chart: chartOf(sub(r, ["markov"]), ["chart", "chart_series", "chartSeries", "series"]),
    },
    qcp: {
      distIndex: gi("qcp", "dist_index", 0), params: fieldSpecs(g("qcp", "params", [])),
      t: gs("qcp", "t", "1000"), bx: gs("qcp", "bx", "0.10"),
      results: pairs(g("qcp", "results", null)), headlineValue: gs("qcp", "headline_value", ""),
    },
    rcm: {
      rows: cellsRows(g("rcm", "rows", [])), sel: gi("rcm", "sel", -1),
      edAsset: gs("rcm", "ed_asset", ""), edFunction: gs("rcm", "ed_function", ""), edMode: gs("rcm", "ed_mode", ""),
      edConsequenceIndex: gi("rcm", "ed_consequence_index", 3), edMonitorable: gb("rcm", "ed_monitorable", false),
      edAge: gb("rcm", "ed_age", false), edRestorable: gb("rcm", "ed_restorable", false),
      edRate: gs("rcm", "ed_rate", ""), edAlpha: gs("rcm", "ed_alpha", ""), edBeta: gs("rcm", "ed_beta", ""), edHours: gs("rcm", "ed_hours", ""),
      edTask: gs("rcm", "ed_task", ""), edCrit: gs("rcm", "ed_crit", ""),
    },
    fracas: {
      rows: cellsRows(g("fracas", "rows", [])), sel: gi("fracas", "sel", -1),
      edId: gs("fracas", "ed_id", ""), edPart: gs("fracas", "ed_part", ""), edMode: gs("fracas", "ed_mode", ""),
      edHours: gs("fracas", "ed_hours", ""), edDate: gs("fracas", "ed_date", ""), edStatusIndex: gi("fracas", "ed_status_index", 0),
      edAction: gs("fracas", "ed_action", ""), totalHours: gs("fracas", "total_hours", "10000"),
      summary: pairs(g("fracas", "summary", null)), path: gs("fracas", "path", "fracas.csv"), status: gs("fracas", "status", ""),
    },
  };
  state.hostKind = transportKind;
  if (state.screen === "start" || !state.screen) state.screen = items.length && state.selectedIndex >= 0 ? state.screen : state.screen;
  return state;
}

let transportKind = "wasm";
const transport = new Transport();
let state = adaptState({});
let aboutOpen = false;
let lastStatus = "";
let localFmeaRow = -1;
const pending = new Set();
// Navigation state (v0.3.1): picker, grouped tree, quick switcher.
// These live outside engine state so re-renders keep them.
let pickerOpen = false;
let switcherOpen = false;
let switcherQuery = "";
let switcherActive = 0;
let switcherFiltered = [];
const collapsedFamilies = new Set();
let renamePendingValue = null;

function fuzzyScore(label, query) {
  if (!query) return 0;
  const text = String(label || "").toLowerCase();
  const q = String(query || "").toLowerCase();
  let ti = 0, first = -1, last = -1;
  for (let qi = 0; qi < q.length; qi++) {
    const found = text.indexOf(q[qi], ti);
    if (found < 0) return null;
    if (first < 0) first = found;
    last = found;
    ti = found + 1;
  }
  // Shorter span and earlier start are better matches.
  return (last - first) * 100 + first * 2 + text.length;
}
function switcherEntries() {
  const entries = [];
  (state.items || []).forEach((it, i) => {
    entries.push({ type: "item", index: i, label: it.name, sub: it.kindLabel, icon: it.icon, searchText: it.name + " " + it.kindLabel });
  });
  entries.push({ type: "add", label: "Add item...", sub: "Open the visual picker", icon: "plus", searchText: "Add item" });
  KIND_META.forEach((m) => {
    entries.push({ type: "new-kind", kindKey: m.key, kindIndex: kindIndex(m.key), label: "New " + m.label, sub: m.desc, icon: m.icon, searchText: "New " + m.label + " " + m.key });
  });
  entries.push({ type: "save", label: "Save project", sub: "Save the current project", icon: "save", searchText: "Save project" });
  entries.push({ type: "open", label: "Open project", sub: "Open a project file", icon: "open", searchText: "Open project" });
  entries.push({ type: "start", label: "Start page", sub: "Go to the start page", icon: "home", searchText: "Start page" });
  entries.push({ type: "tours", label: "Tours...", sub: "Guided tours", icon: "play", searchText: "Tours" });
  return entries;
}
function computeSwitcherFiltered() {
  const all = switcherEntries();
  const q = switcherQuery.trim();
  if (!q) { switcherFiltered = all; return switcherFiltered; }
  const scored = [];
  all.forEach((e, order) => {
    const s = fuzzyScore(e.searchText || e.label, q);
    if (s !== null) scored.push({ e, s, order });
  });
  scored.sort((a, b) => (a.s - b.s) || (a.order - b.order));
  switcherFiltered = scored.map((x) => x.e);
  return switcherFiltered;
}
function switcherListHtml() {
  const list = switcherFiltered;
  if (!list.length) return '<p class="caption switcher-empty">No matches.</p>';
  return list.map((e, i) =>
    '<div class="switcher-row' + (i === switcherActive ? " active" : "") + '" data-switcher-row="' + i + '" role="option" aria-selected="' + (i === switcherActive ? "true" : "false") + '"><img src="assets/icons/' + esc(e.icon || "life") + '.svg" alt=""><span class="switcher-label">' + esc(e.label) + '</span><span class="switcher-sub">' + esc(e.sub || "") + "</span></div>"
  ).join("");
}
function updateSwitcherListDom() {
  const listEl = document.querySelector("#switcher-list");
  if (listEl) listEl.innerHTML = switcherListHtml();
}
function openSwitcher() {
  switcherOpen = true;
  switcherQuery = "";
  switcherActive = 0;
  computeSwitcherFiltered();
  render();
  const inp = document.querySelector("#switcher-input");
  if (inp) { inp.focus(); inp.select(); }
}
function closeSwitcher() {
  switcherOpen = false;
  render();
}
function openPicker() {
  pickerOpen = true;
  render();
  const inp = document.querySelector("#picker-name");
  if (inp) inp.focus();
}
function closePicker() {
  pickerOpen = false;
  render();
}
function addKindByIndex(idx, nameValue) {
  const kindSel = document.querySelector("#kind-select");
  if (kindSel) kindSel.value = String(idx);
  if (nameValue !== undefined) {
    const nn = document.querySelector("#new-name");
    if (nn) nn.value = nameValue;
  }
  return dispatch("add-item");
}
function runSwitcherEntry(entry) {
  if (!entry) return;
  switcherOpen = false;
  if (entry.type === "add") { pickerOpen = true; render(); const inp = document.querySelector("#picker-name"); if (inp) inp.focus(); return; }
  render();
  if (entry.type === "item") { dispatch("select-item", { index: String(entry.index), selected_index: String(entry.index) }); return; }
  if (entry.type === "new-kind") {
    const kindSel = document.querySelector("#kind-select");
    if (kindSel) kindSel.value = String(entry.kindIndex);
    const nn = document.querySelector("#new-name"); if (nn) nn.value = "";
    dispatch("add-item");
    return;
  }
  if (entry.type === "save") { saveProjectFlow(); return; }
  if (entry.type === "open") { openProjectFlow(); return; }
  if (entry.type === "start") { dispatch("go-start"); return; }
  if (entry.type === "tours") { if (tourApi) tourApi.openList(); return; }
}
function commitRename(el) {
  if (!el) return;
  const value = (el.value || "").trim();
  if (!value) return;
  const sel = state.selectedIndex >= 0 ? state.items[state.selectedIndex] : null;
  if (!sel) return;
  if (value === sel.name) return;
  if (renamePendingValue === value) return;
  renamePendingValue = value;
  Promise.resolve(dispatch("rename-item")).finally(() => { renamePendingValue = null; });
}

function collectFields() {
  const fields = {};
  const put = (key, value) => {
    fields[key] = value;
    const under = key.replaceAll(".", "_").replaceAll("-", "_");
    if (!(under in fields)) fields[under] = value;
  };
  document.querySelectorAll("[data-field]").forEach((el) => {
    const key = el.getAttribute("data-field");
    const kind = el.getAttribute("data-field-kind");
    if (el.type === "checkbox") { put(key, el.checked ? "true" : "false"); return; }
    if (el.tagName === "SELECT") {
      put(key, el.value);
      const text = el.options[el.selectedIndex] ? el.options[el.selectedIndex].text : "";
      put(key + "_text", text);
      if (key.endsWith("_index")) put(key.slice(0, -6), text);
      if (kind === "index") put(key + "_value", text);
      return;
    }
    put(key, el.value);
  });
  document.querySelectorAll("[data-dyn-field]").forEach((el) => {
    put(el.getAttribute("data-dyn-field"), el.value);
  });
  // Shell fields that live outside the content area.
  const pn = document.querySelector("#project-name"); if (pn) { put("app.project_name", pn.value); put("project_name", pn.value); }
  const pp = document.querySelector("#project-path"); if (pp) { put("app.project_path", pp.value); put("project_path", pp.value); }
  const kindSel = document.querySelector("#kind-select"); if (kindSel) { put("app.kind_index", kindSel.value); put("kind_index", kindSel.value); put("app.kind", kindSel.options[kindSel.selectedIndex].text); }
  const nn = document.querySelector("#new-name"); if (nn) { put("app.new_name", nn.value); put("new_name", nn.value); }
  const rn = document.querySelector("#rename-name"); if (rn) { put("app.rename_name", rn.value); put("rename_name", rn.value); }
  // Canonical aliases the engine JSON API uses for the shared CSV pick.
  if (fields["csv.pick_index"] !== undefined) {
    const files = state.csv.files;
    const idx = parseInt(fields["csv.pick_index"], 10);
    if (files[idx]) { fields["csv_pick"] = files[idx]; fields["csv.pick"] = files[idx]; }
  }
  if (fields["pred.p217_file_index"] !== undefined) {
    const idx = parseInt(fields["pred.p217_file_index"], 10);
    if (MIL217_FILES[idx]) fields["pred.p217_file"] = MIL217_FILES[idx];
  }
  return fields;
}

function showToast(message, info) {
  const stack = document.querySelector("#toast-stack");
  if (!stack || !message) return;
  const el = document.createElement("div");
  el.className = "toast" + (info ? " info" : "");
  const span = document.createElement("span"); span.textContent = message; span.style.flex = "1";
  const close = document.createElement("button"); close.type = "button"; close.textContent = "×"; close.setAttribute("aria-label", "Dismiss message");
  close.addEventListener("click", () => el.remove());
  el.append(span, close); stack.appendChild(el);
  setTimeout(() => { if (el.isConnected) el.remove(); }, 12000);
}

function setBusy(action, on) {
  document.querySelectorAll('[data-act="' + CSS.escape(action) + '"], [data-busy-key="' + CSS.escape(action) + '"]').forEach((b) => {
    if (on) {
      if (!b.querySelector(".spinner")) { const s = document.createElement("span"); s.className = "spinner"; b.prepend(s); }
      b.setAttribute("disabled", "true");
    }
  });
}

async function dispatch(action, extra, opts) {
  const silent = !!(opts && opts.silent);
  const fields = Object.assign(collectFields(), extra || {});
  fields.action = action;
  if (!silent && COMPUTE_ACTIONS.has(action)) setBusy(action, true);
  pending.add(action);
  try {
    if (!transport.ready) throw new Error(transport.error || "The engine is not connected in this view. Start the desktop app or open the hosted demo so the engine can load.");
    const raw = await transport.dispatch({ action, fields });
    const next = adaptState(raw);
    state = next;
    if (state.download && state.download.text && transport.kind === "wasm") {
      downloadTextFile(state.download.filename, state.download.text);
    }
    if (action === "save-project" && transport.kind === "wasm") {
      if (state.projectJson) downloadTextFile("reliakit-project.json", state.projectJson);
    }
    render();
    if (state.errorText) showToast(state.errorText, false);
    else if (state.isError && state.status && state.status !== lastStatus) showToast(state.status, false);
    else if (/error|failed|invalid|could not|unable to/i.test(state.status || "") && state.status !== lastStatus) showToast(state.status, false);
    lastStatus = state.status;
    return state;
  } catch (err) {
    const msg = err && err.message ? err.message : String(err);
    showToast(msg, false);
    const bar = document.querySelector("#status-text");
    if (bar) bar.textContent = "Status: " + msg;
    return state;
  } finally {
    pending.delete(action);
    if (!silent && COMPUTE_ACTIONS.has(action)) setBusy(action, false);
  }
}

// Browser demo file flows: read the file locally, then send its text.
async function openProjectFlow() {
  if (transport.kind !== "wasm") return dispatch("open-project");
  const file = await pickTextFile(".json");
  if (!file) return state;
  return dispatch("project_open_data", { file_text: file.text, text: file.text, content: file.text, file_name: file.name, name: file.name, act: "project_open_data" });
}
async function csvFileFlow() {
  if (transport.kind !== "wasm") return dispatch("csv-file");
  const file = await pickTextFile(".csv,.txt");
  if (!file) return state;
  return dispatch("csv_load_text", { file_text: file.text, text: file.text, content: file.text, csv_text: file.text, file_name: file.name, act: "csv_load_text" });
}
async function genericFileFlow(kind) {
  const file = await pickTextFile(kind === "rbd-load" ? ".json" : ".csv,.txt");
  if (!file) return state;
  if (kind === "csv") return dispatch("csv_load_text", { file_text: file.text, text: file.text, content: file.text, csv_text: file.text, file_name: file.name });
  if (kind === "library") return dispatch("pred_lib_load_text", { file_text: file.text, text: file.text, content: file.text, csv_text: file.text, file_name: file.name, act: "pred-lib-load" });
  if (kind === "fracas") return dispatch("fracas_import_text", { file_text: file.text, text: file.text, content: file.text, csv_text: file.text, file_name: file.name, act: "fracas-import-csv" });
  if (kind === "rbd-load") {
    const ta = document.querySelector('[data-field="rbd.json"]');
    if (ta) ta.value = file.text;
    return dispatch("rbd-load-json", { file_text: file.text, text: file.text });
  }
  if (kind === "rbd-save") { downloadTextFile("rbd.json", state.rbd.json || ""); return state; }
  return state;
}
async function saveProjectFlow() {
  return dispatch("save-project");
}

function render() {
  const root = document.querySelector("#app");
  const ui = { hostKind: transport.kind, quickstartDismissed: qsDismissed() };
  const s = state;
  // Grouped project tree: items grouped under collapsible family headers.
  const rowHtml = (it, i) =>
    '<div class="side-row' + (i === s.selectedIndex ? " selected" : "") + '" data-select-item="' + i + '" role="button" tabindex="0"><img src="assets/icons/' + esc(it.icon) + '.svg" alt=""><span style="min-width:0"><span class="item-name">' + esc(it.name) + '</span><br><span class="kind-label">' + esc(it.kindLabel) + "</span></span></div>";
  let itemsHtml = "";
  FAMILIES.forEach((fam) => {
    const famItems = s.items.map((it, i) => ({ it, i })).filter(({ it }) => { const m = kindByKey(it.kind); return m && m.family === fam; });
    if (!famItems.length) return;
    const collapsed = collapsedFamilies.has(fam);
    itemsHtml += '<div class="family-group"><div class="family-header" data-family-toggle="' + esc(fam) + '" role="button" tabindex="0" aria-expanded="' + (collapsed ? "false" : "true") + '"><span class="family-name">' + esc(fam) + '</span><span class="family-count">' + famItems.length + '</span><span class="family-chevron">' + (collapsed ? "▸" : "▾") + "</span></div>";
    if (!collapsed) itemsHtml += famItems.map(({ it, i }) => rowHtml(it, i)).join("");
    itemsHtml += "</div>";
  });
  const otherItems = s.items.map((it, i) => ({ it, i })).filter(({ it }) => !kindByKey(it.kind));
  if (otherItems.length) {
    const collapsed = collapsedFamilies.has("Other");
    itemsHtml += '<div class="family-group"><div class="family-header" data-family-toggle="Other" role="button" tabindex="0" aria-expanded="' + (collapsed ? "false" : "true") + '"><span class="family-name">Other</span><span class="family-count">' + otherItems.length + '</span><span class="family-chevron">' + (collapsed ? "▸" : "▾") + "</span></div>";
    if (!collapsed) itemsHtml += otherItems.map(({ it, i }) => rowHtml(it, i)).join("");
    itemsHtml += "</div>";
  }
  const kindOptions = KIND_OPTIONS.map((k, i) => '<option value="' + i + '"' + (i === s.kindIndex ? " selected" : "") + ">" + esc(k) + "</option>").join("");
  let content = "";
  if (s.screen === "start" || s.selectedIndex < 0 && s.screen === "start") content = renderStart(s, ui);
  else if (s.screen === "rbd") content = renderRbd({ ...s, hostKind: transport.kind });
  else if (SCREEN_RENDERERS[s.screen]) content = SCREEN_RENDERERS[s.screen](s, ui);
  else content = renderStart(s, ui);
  const demoNote = transport.kind === "wasm"
    ? '<div class="demo-note">Browser demo. Project Save downloads reliakit-project.json. Project Open and CSV import use a file picker. Controls that need a local file path on the desktop show a short note here instead.</div>'
    : "";
  // Screen header: only on item screens, not the start page.
  let headerHtml = "";
  const selectedItem = s.selectedIndex >= 0 ? s.items[s.selectedIndex] : null;
  if (selectedItem && s.screen !== "start") {
    const meta = kindByKey(selectedItem.kind);
    const hIcon = meta ? meta.icon : selectedItem.icon;
    const hLabel = meta ? meta.label : selectedItem.kindLabel;
    const hDesc = meta ? meta.desc : "";
    headerHtml = '<div class="screen-header"><img class="screen-header-icon" src="assets/icons/' + esc(hIcon) + '.svg" alt="">' +
      '<div class="screen-header-main"><div class="screen-header-top"><input type="text" id="rename-name" class="rename-input" aria-label="Rename selected item" value="' + esc(s.renameName || selectedItem.name) + '"><span class="screen-header-kind">' + esc(hLabel) + '</span></div>' +
      '<p class="screen-header-desc">' + esc(hDesc) + "</p></div>" +
      '<div class="screen-header-actions"><button type="button" class="btn" data-local="tour-screen">Tour this screen</button><button type="button" class="btn" data-shell="remove-item">Remove</button></div></div>';
  }
  const aboutHtml = aboutOpen ? '<div class="overlay" id="about-overlay"><div class="dialog" role="dialog" aria-modal="true" aria-label="About ReliaKit"><h2><img src="assets/logo.svg" alt=""> ReliaKit ' + VERSION + '</h2><p class="caption">Reliability engineering suite for life data, system reliability, and maintenance analysis.</p><p class="caption">Reliability engineering suite: life data, limited-data Bayesian methods, degradation, ALT, growth, system RBD with standby, fault trees, FMEA, warranty, prediction, DOE, flowchart, Markov, QCP, RCM, and FRACAS.</p><p class="caption">Repository: <a href="' + REPO_URL + '" target="_blank" rel="noopener">' + REPO_URL + '</a></p><p class="caption">Built with Tauri (MIT/Apache-2.0)</p><p class="caption">Inter font, SIL Open Font License 1.1</p><p class="caption">ReliaKit is MIT licensed</p><p class="caption">No proprietary data is bundled; the MIL-HDBK-217F library is a public US government document.</p><div class="btn-row"><button type="button" class="btn" data-local="show-quickstart">Show Quickstart</button><button type="button" class="btn btn-primary" data-local="close-about">Close</button></div></div></div>' : "";
  let pickerHtml = "";
  if (pickerOpen) {
    const groups = FAMILIES.map((fam) => {
      const cards = KIND_META.filter((m) => m.family === fam).map((m) =>
        '<button type="button" class="picker-card kind-card" data-picker-kind="' + esc(m.key) + '" data-kind-key="' + esc(m.key) + '" data-picker-index="' + kindIndex(m.key) + '"><img src="assets/icons/' + esc(m.icon) + '.svg" alt=""><span class="picker-card-label">' + esc(m.label) + '</span><span class="picker-card-desc">' + esc(m.desc) + "</span></button>"
      ).join("");
      return '<div class="picker-family"><h3 class="picker-family-title">' + esc(fam) + '</h3><div class="picker-grid">' + cards + "</div></div>";
    }).join("");
    pickerHtml = '<div class="overlay picker-overlay" id="picker-overlay"><div class="dialog dialog-wide picker-dialog" role="dialog" aria-modal="true" aria-label="Add item"><h2>Add item</h2>' +
      '<div class="field"><label for="picker-name">Name (optional)</label><input type="text" id="picker-name" placeholder="Name (optional)" value=""></div>' +
      groups +
      '<div class="btn-row"><button type="button" class="btn" data-local="close-picker">Cancel</button></div></div></div>';
  }
  let switcherHtml = "";
  if (switcherOpen) {
    computeSwitcherFiltered();
    if (switcherActive >= switcherFiltered.length) switcherActive = Math.max(0, switcherFiltered.length - 1);
    if (switcherActive < 0) switcherActive = 0;
    switcherHtml = '<div class="overlay switcher-overlay" id="switcher-overlay"><div class="dialog switcher-dialog" role="dialog" aria-modal="true" aria-label="Jump to">' +
      '<input type="text" id="switcher-input" class="switcher-input" placeholder="Jump to..." aria-label="Jump to" value="' + esc(switcherQuery) + '" autocomplete="off">' +
      '<div class="switcher-list" id="switcher-list" role="listbox">' + switcherListHtml() + "</div></div></div>";
  }
  root.innerHTML =
    '<div class="toolbar"><img class="logo" src="assets/logo.svg" alt=""><span class="brand">ReliaKit</span><span class="divider"></span>' +
    '<button type="button" class="btn" data-shell="new-project">New</button><button type="button" class="btn" data-shell="open-project">Open</button><button type="button" class="btn" data-shell="save-project">Save</button>' +
    '<button type="button" class="btn" data-local="open-switcher">Jump to...</button><button type="button" class="btn" data-local="open-tours">Tours</button>' +
    '<span class="current-title">' + esc(s.currentTitle) + '</span><span class="version">' + VERSION + '</span><button type="button" class="btn" data-local="open-about">About</button></div>' +
    '<div class="body-row"><aside class="sidebar"><h4>Project</h4><input type="text" id="project-name" aria-label="Project name" value="' + esc(s.projectName) + '">' +
    '<div class="side-row' + (s.screen === "start" ? " selected" : "") + '" data-shell="go-start" role="button" tabindex="0"><img src="assets/icons/home.svg" alt=""><span class="item-name">Start page</span></div>' +
    "<h4>Items (" + s.itemCount + ')</h4><div class="items-list">' + itemsHtml + "</div>" +
    (s.itemCount === 0 ? '<p class="caption">No items yet. Add one below, or load an example from the start page.</p>' : "") +
    demoNote +
    '<div class="side-sep"></div><h4>Add item</h4><div class="visually-hidden" aria-hidden="true"><select id="kind-select" aria-label="Item kind">' + kindOptions + '</select><input type="text" id="new-name" aria-label="New item name" placeholder="Name (optional)" value="' + esc(s.newName) + '"></div><button type="button" class="btn btn-primary" data-local="open-picker">Add item</button>' +
    '<div class="side-sep"></div><h4>Project file</h4><input type="text" id="project-path" aria-label="Project file path" value="' + esc(s.projectPath) + '"><div class="btn-row"><button type="button" class="btn" data-shell="save-project">Save</button><button type="button" class="btn" data-shell="open-project">Open</button></div></aside>' +
    '<main class="content">' + headerHtml + content + "</main></div>" +
    '<div class="statusbar"><span class="status-text" id="status-text">Status: ' + esc(s.status) + '</span><span class="headline-text">' + esc(s.headline) + '</span><span class="transport-tag">' + (transport.kind === "tauri" ? "Desktop" : "Browser demo") + "</span></div>" +
    '<div class="toast-stack" id="toast-stack"></div>' + aboutHtml + pickerHtml + switcherHtml;
  bindDynamic();
}

function bindDynamic() {
  const root = document.querySelector("#app");
  bindRbdCanvas(root, (action, extra, opts) => dispatch(action, extra, opts));
  // FMEA worksheet row selection for the Delete shortcut. The Slint
  // worksheet has per row buttons and no row selection state, so the
  // selection is tracked locally and Delete sends the row action.
  if (state.screen === "fmea") {
    const wrap = root.querySelectorAll(".data-table-wrap")[0];
    if (wrap) wrap.querySelectorAll("tbody tr").forEach((tr, i) => {
      tr.style.cursor = "pointer";
      if (i === localFmeaRow) tr.classList.add("row-selected");
      tr.addEventListener("click", () => { localFmeaRow = i; wrap.querySelectorAll("tbody tr").forEach((x) => x.classList.remove("row-selected")); tr.classList.add("row-selected"); });
    });
  }
}

document.addEventListener("click", (ev) => {
  if (ev.target.id === "picker-overlay") { pickerOpen = false; render(); return; }
  if (ev.target.id === "switcher-overlay") { switcherOpen = false; render(); return; }
  const pickerCard = ev.target.closest && ev.target.closest("[data-picker-kind]");
  if (pickerCard) {
    const key = pickerCard.getAttribute("data-picker-kind");
    const idx = kindIndex(key);
    const nameEl = document.querySelector("#picker-name");
    const nameValue = nameEl ? nameEl.value : "";
    pickerOpen = false;
    render();
    addKindByIndex(idx, nameValue);
    return;
  }
  const swRow = ev.target.closest && ev.target.closest("[data-switcher-row]");
  if (swRow) {
    const entry = switcherFiltered[parseInt(swRow.getAttribute("data-switcher-row"), 10)];
    if (entry) runSwitcherEntry(entry);
    return;
  }
  const wiz = ev.target.closest && ev.target.closest("[data-wizard-path]");
  if (wiz) {
    const path = wiz.getAttribute("data-wizard-path");
    if (tourApi) {
      if (path === "failure-data") tourApi.start("life");
      else if (path === "system") tourApi.start("rbd");
      else if (path === "maintenance") tourApi.start("fracas");
    }
    return;
  }
  const fam = ev.target.closest && ev.target.closest("[data-family-toggle]");
  if (fam) {
    const name = fam.getAttribute("data-family-toggle");
    if (collapsedFamilies.has(name)) collapsedFamilies.delete(name); else collapsedFamilies.add(name);
    render();
    return;
  }
  const addKindEl = ev.target.closest && ev.target.closest("[data-add-kind]");
  if (addKindEl) {
    const idx = parseInt(addKindEl.getAttribute("data-add-kind"), 10);
    addKindByIndex(idx, "");
    return;
  }
  const local = ev.target.closest("[data-local]");
  if (local) {
    const name = local.getAttribute("data-local");
    if (name === "open-about") { aboutOpen = true; render(); }
    if (name === "close-about") { aboutOpen = false; render(); }
    if (name === "open-picker") { openPicker(); }
    if (name === "close-picker") { pickerOpen = false; render(); }
    if (name === "open-switcher") { openSwitcher(); }
    if (name === "open-tours") { if (tourApi) tourApi.openList(); }
    if (name === "tour-screen") { if (tourApi) tourApi.startForCurrent(); }
    if (name === "dismiss-quickstart") { qsSetDismissed(true); render(); }
    if (name === "show-quickstart") { qsSetDismissed(false); aboutOpen = false; dispatch("go-start"); }
    return;
  }
  if (ev.target.id === "about-overlay") { aboutOpen = false; render(); return; }
  const fileBtn = ev.target.closest("[data-file-import]");
  if (fileBtn) { genericFileFlow(fileBtn.getAttribute("data-file-import")); return; }
  const shell = ev.target.closest("[data-shell]");
  if (shell) {
    const name = shell.getAttribute("data-shell");
    if (name === "save-project") saveProjectFlow();
    else if (name === "open-project") openProjectFlow();
    else if (name === "go-start") dispatch("go-start");
    else dispatch(name);
    return;
  }
  const sel = ev.target.closest("[data-select-item]");
  if (sel) { dispatch("select-item", { index: sel.getAttribute("data-select-item"), selected_index: sel.getAttribute("data-select-item") }); return; }
  const card = ev.target.closest("[data-card]");
  if (card) { dispatch("card-action", { id: card.getAttribute("data-card"), card_id: card.getAttribute("data-card"), action_id: card.getAttribute("data-card") }); return; }
  const arm = ev.target.closest("[data-rbd-arm]");
  if (arm) { dispatch("rbd-arm", { armed: arm.getAttribute("data-rbd-arm"), kind: arm.getAttribute("data-rbd-arm"), name: arm.getAttribute("data-rbd-arm") }); return; }
  const rowBtnEl = ev.target.closest("[data-row-act]");
  if (rowBtnEl) {
    const name = rowBtnEl.getAttribute("data-row-act");
    const idx = rowBtnEl.getAttribute("data-row-index");
    dispatch(name, { row_index: idx, index: idx, act: name, row_action: name });
    return;
  }
  const tr = ev.target.closest("tr[data-row-act]");
  if (tr) {
    const name = tr.getAttribute("data-row-act");
    const idx = tr.getAttribute("data-row-index");
    dispatch(name, { row_index: idx, index: idx, act: name, row_action: name });
    return;
  }
  const actBtn = ev.target.closest("[data-act]");
  if (actBtn) {
    const name = actBtn.getAttribute("data-act");
    if (name === "save-project") { saveProjectFlow(); return; }
    if (name === "open-project") { openProjectFlow(); return; }
    if (name === "csv-file") { csvFileFlow(); return; }
    dispatch(name, { act: name });
  }
});

document.addEventListener("mouseover", (ev) => {
  const row = ev.target.closest && ev.target.closest("[data-switcher-row]");
  if (row) {
    const idx = parseInt(row.getAttribute("data-switcher-row"), 10);
    if (Number.isFinite(idx) && idx !== switcherActive) {
      switcherActive = idx;
      document.querySelectorAll("[data-switcher-row]").forEach((r) => {
        const on = parseInt(r.getAttribute("data-switcher-row"), 10) === switcherActive;
        r.classList.toggle("active", on);
        r.setAttribute("aria-selected", on ? "true" : "false");
      });
    }
  }
});
document.addEventListener("input", (ev) => {
  const el = ev.target;
  if (el && el.id === "switcher-input") {
    switcherQuery = el.value;
    switcherActive = 0;
    computeSwitcherFiltered();
    updateSwitcherListDom();
  }
});

document.addEventListener("change", (ev) => {
  const el = ev.target;
  if (el && el.id === "rename-name") { commitRename(el); return; }
  if (el.hasAttribute && el.hasAttribute("data-change-act")) {
    dispatch(el.getAttribute("data-change-act"), { act: el.getAttribute("data-change-act") });
    return;
  }
  if (el.hasAttribute && el.hasAttribute("data-dyn-field")) {
    const id = el.getAttribute("data-dyn-field");
    dispatch("field-changed", { field_id: id, id, value: el.value, act: "field-changed" });
  }
});
document.addEventListener("focusout", (ev) => {
  const el = ev.target;
  if (el && el.id === "rename-name") { commitRename(el); return; }
  if (el.hasAttribute && el.hasAttribute("data-dyn-field")) {
    const id = el.getAttribute("data-dyn-field");
    // Change already dispatched for selects and edited inputs. Blur is
    // the explicit commit point the Slint LineEdit edited signal used.
    if (el.tagName === "INPUT") dispatch("field-changed", { field_id: id, id, value: el.value, act: "field-changed" });
  }
});

document.addEventListener("keydown", (ev) => {
  const typing = ev.target && (ev.target.tagName === "INPUT" || ev.target.tagName === "TEXTAREA" || ev.target.tagName === "SELECT" || ev.target.isContentEditable);
  // Quick switcher is global, including when focus is in an input.
  if ((ev.ctrlKey || ev.metaKey) && ev.key.toLowerCase() === "k") { ev.preventDefault(); if (!switcherOpen) openSwitcher(); return; }
  if (switcherOpen) {
    if (ev.key === "Escape") { ev.preventDefault(); closeSwitcher(); return; }
    if (ev.key === "ArrowDown") { ev.preventDefault(); if (switcherFiltered.length) { switcherActive = Math.min(switcherActive + 1, switcherFiltered.length - 1); updateSwitcherListDom(); const r = document.querySelector('[data-switcher-row="' + switcherActive + '"]'); if (r) r.scrollIntoView({ block: "nearest" }); } return; }
    if (ev.key === "ArrowUp") { ev.preventDefault(); if (switcherFiltered.length) { switcherActive = Math.max(switcherActive - 1, 0); updateSwitcherListDom(); const r = document.querySelector('[data-switcher-row="' + switcherActive + '"]'); if (r) r.scrollIntoView({ block: "nearest" }); } return; }
    if (ev.key === "Enter") { ev.preventDefault(); const entry = switcherFiltered[switcherActive]; if (entry) runSwitcherEntry(entry); return; }
    return;
  }
  if (pickerOpen && ev.key === "Escape") { ev.preventDefault(); pickerOpen = false; render(); return; }
  if (ev.key === "Escape" && aboutOpen) { aboutOpen = false; render(); return; }
  // Rename input: Enter blurs to commit through change/blur handling.
  if (ev.target && ev.target.id === "rename-name" && ev.key === "Enter") { ev.preventDefault(); ev.target.blur(); return; }
  if ((ev.ctrlKey || ev.metaKey) && ev.key.toLowerCase() === "s") { ev.preventDefault(); saveProjectFlow(); return; }
  if ((ev.ctrlKey || ev.metaKey) && ev.key.toLowerCase() === "o") { ev.preventDefault(); openProjectFlow(); return; }
  if (ev.key === "Delete" && !typing) {
    if (state.screen === "rbd" && state.rbd.hasSelection) { ev.preventDefault(); dispatch("rbd-delete", { act: "rbd-delete" }); return; }
    if (state.screen === "fracas" && state.fracas.sel >= 0) { ev.preventDefault(); dispatch("fracas-remove", { act: "fracas-remove" }); return; }
    if (state.screen === "rcm" && state.rcm.sel >= 0) { ev.preventDefault(); dispatch("rcm-remove", { act: "rcm-remove" }); return; }
    if (state.screen === "fmea" && localFmeaRow >= 0) { ev.preventDefault(); dispatch("fmea-del", { row_index: String(localFmeaRow), index: String(localFmeaRow), act: "fmea-del", row_action: "fmea-del" }); }
  }
});

// Guided tours: body-level overlay engine (gui/src/tours.js). It
// dispatches through the same dispatch() as the rest of the shell and
// reads the current adapted state through getState.
const tourApi = initTours({ dispatch, getState: () => state });

async function boot() {
  transportKind = transport.kind;
  render();
  const ok = await transport.init();
  transportKind = transport.kind;
  if (!ok) {
    showToast(transport.error + " The interface is shown with its start page so the layout can be reviewed; connect the engine to run analyses.", true);
    render();
    return;
  }
  await dispatch("init", { act: "init" });
  // Engines that name their first load differently still get a shell
  // refresh through the start action when init returns no items.
  if (!state.status) await dispatch("go-start");
}

boot();
