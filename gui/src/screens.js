// Screen renderers for the start page and all 18 item kinds.
// Structure mirrors gui/ui/screens_a.slint through screens_e.slint and
// gui/ui/app.slint: left input panel, center chart, table or canvas,
// right inspector with result cards. Every control carries a data-act,
// data-row-act, data-card or data-field hook that app.js dispatches.
import { escapeHtml, chartCardHtml } from "./charts.js";
import { rbdCanvasHtml } from "./rbd.js";
import { KIND_META, kindIndex } from "./kinds.js";

export const ALL_DISTS = ["weibull2", "weibull3", "lognormal", "normal", "exponential", "gamma", "gumbel", "loglogistic", "logistic", "gompertz", "gengamma", "weibull1", "exponential2", "mixed_weibull"];
export const MCMC_DISTS = ["weibull2", "lognormal"];
export const KIND_OPTIONS = ["Life Data", "Limited Data", "MCMC", "Degradation", "ALT", "Growth", "Competing Risks", "RBD", "Fault Tree", "FMEA", "Warranty", "Prediction", "DOE", "Flowchart", "Markov", "QCP", "RCM", "FRACAS"];
export const BUNDLED_CSV = ["reliasoft_alt_circuit_boards.csv", "reliasoft_alt_degradation_led.csv", "reliasoft_alt_paper_clips.csv", "reliasoft_alt_tensile.csv", "reliasoft_alt_two_stress.csv", "reliasoft_beta_shape_example.csv", "reliasoft_degradation_mosfet.csv", "reliasoft_degradation_turbine_blades.csv", "reliasoft_growth_crow_amsaa.csv", "reliasoft_growth_test_fix_test.csv", "reliasoft_life_data_example.csv", "reliasoft_unbiasing_complete.csv", "reliasoft_unbiasing_suspended.csv", "reliasoft_warranty_nevada.csv", "reliasoft_warranty_time_to_failure.csv", "smrd_bearingcage.csv"];
export const MIL217_FILES = ["capacitors.json", "circuit_breakers.json", "coils.json", "connections.json", "connector_sockets.json", "connectors.json", "elapsed_time_meters.json", "electronic_filters.json", "fuses.json", "interconnection_assemblies_pth.json", "miscellaneous_parts.json", "quartz_crystals.json", "relays_mechanical.json", "relays_solid_state.json", "resistors.json", "switches.json", "synchros_and_resolvers.json", "transformers.json"];

// Human-readable dataset titles (M5), keyed by file name, based on
// datasets/README.md. Source and file name render as secondary lines.
export const DATASET_TITLES = {
  "reliasoft_life_data_example.csv": "Prototype device life data (ReliaSoft example)",
  "reliasoft_unbiasing_complete.csv": "Unbiasing example, complete data (ReliaSoft)",
  "reliasoft_unbiasing_suspended.csv": "Unbiasing example, suspended data (ReliaSoft)",
  "reliasoft_beta_shape_example.csv": "Weibull shape parameter example data (ReliaSoft)",
  "reliasoft_alt_tensile.csv": "ALT tensile component data (ReliaSoft)",
  "reliasoft_alt_circuit_boards.csv": "ALT circuit board data (ReliaSoft)",
  "reliasoft_alt_paper_clips.csv": "ALT paper clip bend data (ReliaSoft)",
  "reliasoft_alt_two_stress.csv": "ALT two-stress data (ReliaSoft)",
  "reliasoft_degradation_turbine_blades.csv": "Turbine blade crack degradation data (ReliaSoft)",
  "reliasoft_degradation_mosfet.csv": "MOSFET degradation data (ReliaSoft)",
  "reliasoft_alt_degradation_led.csv": "LED accelerated degradation data (ReliaSoft)",
  "reliasoft_growth_crow_amsaa.csv": "Crow-AMSAA growth example data (ReliaSoft)",
  "reliasoft_growth_test_fix_test.csv": "Test-fix-test growth data (ReliaSoft)",
  "reliasoft_warranty_nevada.csv": "Warranty Nevada chart data (ReliaSoft)",
  "reliasoft_warranty_time_to_failure.csv": "Warranty time-to-failure data (ReliaSoft)",
  "smrd_bearingcage.csv": "Bearing cage life data (SMRD)",
};
export function datasetTitle(file) { return DATASET_TITLES[file] || file; }

const E = escapeHtml;
function field(name, label, value, extra) {
  return '<div class="field"><label for="f-' + E(name) + '">' + E(label) + '</label><input type="text" id="f-' + E(name) + '" data-field="' + E(name) + '" value="' + E(value == null ? "" : value) + '"' + (extra || "") + "></div>";
}
function area(name, label, value, rows) {
  return '<div class="field"><label for="f-' + E(name) + '">' + E(label) + '</label><textarea id="f-' + E(name) + '" data-field="' + E(name) + '" rows="' + (rows || 4) + '">' + E(value == null ? "" : value) + "</textarea></div>";
}
function selectField(name, label, options, index, changeAct) {
  const opts = (options || []).map((o, i) => '<option value="' + i + '"' + (i === Number(index || 0) ? " selected" : "") + ">" + E(o) + "</option>").join("");
  const ch = changeAct ? ' data-change-act="' + E(changeAct) + '"' : "";
  return '<div class="field"><label for="f-' + E(name) + '">' + E(label) + '</label><select id="f-' + E(name) + '" data-field="' + E(name) + '" data-field-kind="index"' + ch + ">" + opts + "</select></div>";
}
function checkField(name, label, checked) {
  return '<label class="check-field"><input type="checkbox" data-field="' + E(name) + '" data-field-kind="bool"' + (checked ? " checked" : "") + "> " + E(label) + "</label>";
}
function btn(act, text, opts) {
  const o = opts || {};
  const cls = "btn" + (o.primary ? " btn-primary" : "") + (o.danger ? " btn-danger" : "") + (o.small ? " small" : "") + (o.active ? " active" : "");
  const busy = o.busyKey ? ' data-busy-key="' + E(o.busyKey) + '"' : "";
  return '<button type="button" class="' + cls + '" data-act="' + E(act) + '"' + busy + ">" + E(text) + "</button>";
}
function rowBtn(act, index, text) {
  return '<button type="button" class="btn small" data-row-act="' + E(act) + '" data-row-index="' + Number(index || 0) + '">' + E(text) + "</button>";
}
function titleBlock(title, sub) {
  // The app header now carries the item name, kind label, and kind
  // description, so screens keep only their guidance text as a caption.
  return sub ? '<p class="caption screen-caption">' + E(sub) + "</p>" : "";
}
function section(t) { return '<h3 class="section-title">' + E(t) + "</h3>"; }
function caption(t) { return t ? '<p class="caption">' + E(t) + "</p>" : ""; }
// Compact results (M3): a summary grid of label/value pairs plus an
// expandable Details section holding every row as a table, so long
// values stay available without stacking tall cards.
function rowPair(r) {
  return [Array.isArray(r) ? (r[0] || "") : (r.label || ""), Array.isArray(r) ? (r[1] || "") : (r.value || "")];
}
function resultCards(rows, emptyText) {
  const list = Array.isArray(rows) ? rows : [];
  if (list.length === 0) return '<p class="faint-note">' + E(emptyText || "No results yet. Run the analysis to see results here.") + "</p>";
  const grid = '<div class="summary-grid">' + list.map((r) => {
    const [label, value] = rowPair(r);
    return '<div class="result-card summary-cell">' + (label ? '<div class="rc-label">' + E(label) + "</div>" : "") + '<div class="rc-value">' + E(value) + "</div></div>";
  }).join("") + "</div>";
  const details = '<details class="details-rows"><summary>Details (' + list.length + ' rows)</summary><table class="details-table"><tbody>' +
    list.map((r) => { const [label, value] = rowPair(r); return "<tr><th>" + E(label) + "</th><td>" + E(value) + "</td></tr>"; }).join("") +
    "</tbody></table></details>";
  return grid + details;
}
function headlineCard(label, value, outdated) {
  if (!value) return "";
  return '<div class="headline-card' + (outdated ? " outdated" : "") + '"><div class="hc-label">' + E(label) + (outdated ? ' <span class="badge badge-outdated">Outdated</span>' : "") + '</div><div class="hc-value">' + (outdated ? "Outdated: " : "") + E(value) + "</div></div>";
}
// Persistent explanatory text as a compact collapsed Help disclosure (M6).
function helpDetails(text) {
  if (!text) return "";
  return '<details class="help-disclosure"><summary>Help</summary><p class="caption">' + E(text) + "</p></details>";
}
// Fit status badge (B/C): colored by the engine fit status, with the
// status note under it and any warnings listed.
export function fitBadgeHtml(scr) {
  if (!scr || !scr.fitStatus) return "";
  const st = scr.fitStatus;
  if (st === "success" && !scr.fitNote && !(scr.fitWarnings || []).length) {
    return '<div class="fit-line"><span class="badge badge-fit-success">Fit OK</span></div>';
  }
  const cls = st === "success" ? "badge-fit-success" : st === "failed" ? "badge-fit-failed" : "badge-fit-warn";
  const label = st === "success" ? "Fit OK" : st === "success_with_warnings" ? "Fit OK, with warnings" : st === "boundary_or_degenerate" ? "Boundary or degenerate fit" : st === "failed" ? "Fit failed" : st;
  let html = '<div class="fit-line"><span class="badge ' + cls + '">' + E(label) + "</span></div>";
  if (scr.fitNote) html += '<p class="fit-note">' + E(scr.fitNote) + "</p>";
  if ((scr.fitWarnings || []).length) {
    html += '<ul class="fit-warnings">' + scr.fitWarnings.map((w) => "<li>" + E(w) + "</li>").join("") + "</ul>";
  }
  return html;
}
export function outdatedBadgeHtml(state) {
  return state && state.outdated ? '<p class="outdated-line"><span class="badge badge-outdated">Outdated</span> Inputs changed after these results were computed. Run the analysis again to refresh them.</p>' : "";
}
// Life Data fit summary (M3): two structured tables built from the
// engine result rows. Parameters come from the "Parameter <name>"
// rows; the standard error is read from the matching segment of the
// "Fit" summary string when the engine provides one. Diagnostics
// come from that same summary line (n, failures, log-likelihood)
// plus the fit status.
export function lifeFitTables(rows, scr) {
  const list = (Array.isArray(rows) ? rows : []).map(rowPair);
  if (!list.length) return "";
  let fitSummary = "";
  const params = [];
  const rest = [];
  for (const [label, value] of list) {
    if (label === "Fit") { fitSummary = value; continue; }
    if (label === "Fit status") continue; // shown by the fit badge
    if (label.startsWith("Parameter ")) { params.push([label.slice("Parameter ".length), value]); continue; }
    rest.push([label, value]);
  }
  const segs = fitSummary.split("|").map((x) => x.trim());
  const seByParam = {};
  let distLine = segs.length ? segs[0] : "";
  for (const seg of segs.slice(1)) {
    const m = seg.match(/^(\S+)\s*=\s*([-\d.eE+]+)\s*(?:\(se\s*([-\d.eE+]+)\))?/);
    if (m) seByParam[m[1]] = m[3] || "";
  }
  let html = "";
  if (rest.length) html += '<div class="summary-grid">' + rest.map(([label, value]) =>
    '<div class="result-card summary-cell">' + (label ? '<div class="rc-label">' + E(label) + "</div>" : "") + '<div class="rc-value">' + E(value) + "</div></div>").join("") + "</div>";
  if (params.length) {
    html += '<h4 class="table-title">Parameters</h4><div class="data-table-wrap"><table class="data-table fit-table"><thead><tr><th>Name</th><th>Estimate</th><th>Std. error</th></tr></thead><tbody>' +
      params.map(([name, value]) => "<tr><td>" + E(name) + "</td><td>" + E(value) + "</td><td>" + E(seByParam[name] || "") + "</td></tr>").join("") +
      "</tbody></table></div>";
  }
  const diag = [];
  const nM = distLine.match(/n=(\d+)/); const fM = distLine.match(/failures=(\d+)/); const lM = distLine.match(/loglik=([-\d.eE+]+)/);
  const dM = distLine.match(/^(.*?)\s+fit by\s+(\S+):/);
  if (dM) { diag.push(["Distribution", dM[1]]); diag.push(["Method", dM[2]]); }
  if (nM) diag.push(["n", nM[1]]);
  if (fM) diag.push(["Failures", fM[1]]);
  if (nM && fM) diag.push(["Suspensions", String(Number(nM[1]) - Number(fM[1]))]);
  if (lM) diag.push(["Log-likelihood", lM[1]]);
  if (scr && scr.fitStatus) diag.push(["Fit status", scr.fitStatus + (scr.fitNote ? ": " + scr.fitNote : "")]);
  if (diag.length) {
    html += '<h4 class="table-title">Diagnostics</h4><div class="data-table-wrap"><table class="data-table fit-table"><tbody>' +
      diag.map(([label, value]) => "<tr><th>" + E(label) + "</th><td>" + E(value) + "</td></tr>").join("") +
      "</tbody></table></div>";
  }
  return html || resultCards(rows);
}
function selectedItemName(state) {
  const it = state && state.selectedIndex >= 0 && Array.isArray(state.items) ? state.items[state.selectedIndex] : null;
  return it ? it.name : "This item";
}
function emptyBox(text) { return '<div class="empty-box">' + E(text) + "</div>"; }
function chartOrEmpty(chart, title, emptyText) {
  if (chart) return chartCardHtml(chart, title);
  return emptyBox(emptyText);
}
function dataTable(headers, rows, opts) {
  const o = opts || {};
  const head = "<tr>" + headers.map((h) => "<th>" + E(h) + "</th>").join("") + (o.rowActions ? "<th>Actions</th>" : "") + "</tr>";
  const body = (rows || []).map((cells, i) => {
    const list = Array.isArray(cells) ? cells : (cells.cells || []);
    const sel = o.selectedIndex === i ? ' class="row-selectable row-selected"' : ' class="row-selectable"';
    const click = o.selectAct ? ' data-row-act="' + E(o.selectAct) + '" data-row-index="' + i + '"' : "";
    let tds = list.map((c) => "<td>" + E(c) + "</td>").join("");
    if (o.rowActions) {
      tds += "<td><div class=\"btn-row\">" + o.rowActions.map((a) => rowBtn(a.act, i, a.text)).join("") + "</div></td>";
    }
    return "<tr" + (o.selectAct ? sel : "") + click + ">" + tds + "</tr>";
  }).join("");
  const empty = (!rows || rows.length === 0) ? '<p class="faint-note" style="padding:10px">' + E(o.emptyText || "No rows yet.") + "</p>" : "";
  return '<div class="data-table-wrap"><table class="data-table"><thead>' + head + "</thead><tbody>" + body + "</tbody></table>" + empty + "</div>";
}
function csvImport(state, hostKind) {
  const csv = state.csv || {};
  const files = csv.files && csv.files.length ? csv.files : BUNDLED_CSV;
  let html = '<div style="display:flex;flex-direction:column;gap:8px">' + section("Import data");
  html += selectField("csv.pick_index", "Bundled dataset", files, csv.pickIndex || 0);
  html += '<div class="btn-row">' + btn("csv-bundled", "Load bundled CSV", { busyKey: "csv-bundled" }) + "</div>";
  if (hostKind === "wasm") {
    html += helpDetails("In the browser demo, Load CSV file opens a file picker and sends the file text to the engine. No local path is needed.");
    html += '<div class="btn-row"><button type="button" class="btn" data-file-import="csv">Load CSV file</button></div>';
  } else {
    html += field("csv.path", "CSV file path", csv.path || "");
    html += '<div class="btn-row">' + btn("csv-file", "Load CSV file", { busyKey: "csv-file" }) + "</div>";
  }
  return html + "</div>";
}
function dynFields(fields, prefix) {
  return (fields || []).map((f) => {
    const key = prefix + ":" + (f.key || f.id || "");
    if (f.named || (f.options && f.options.length)) {
      const opts = (f.options || []).map((o, i) => '<option value="' + E(o) + '"' + (i === Number(f.sel || 0) ? " selected" : "") + ">" + E(o) + "</option>").join("");
      return '<div class="dyn-row"><span class="dyn-label">' + E(f.label || f.key || "") + '</span><select class="dyn-input" data-dyn-field="' + E(key) + '">' + opts + "</select></div>";
    }
    return '<div class="dyn-row"><span class="dyn-label">' + E(f.label || f.key || "") + '</span><input type="text" class="dyn-input" data-dyn-field="' + E(key) + '" value="' + E(f.value || "") + '"></div>';
  }).join("");
}
function shell3(left, center, right) {
  return '<div class="screen"><div class="panel-left">' + left + '</div><div class="panel-center">' + center + "</div>" + (right != null ? '<div class="panel-right">' + right + "</div>" : "") + "</div>";
}
function shell2(left, center) { return '<div class="screen"><div class="panel-left">' + left + '</div><div class="panel-center">' + center + "</div></div>"; }

export function renderStart(state, ui) {
  const exRows = (state.exampleRows || []).map((row) => '<div class="cards-grid">' + row.map((c) => '<button type="button" class="card hoverable start-card" data-card="' + E(c.actionId || "") + '"><img src="assets/icons/' + E(c.icon || "life") + '.svg" alt=""><span><span class="sc-tag">' + E(c.tag || "") + '</span><br><span class="sc-title">' + E(c.title || "") + '</span><br><span class="sc-sub">' + E(c.sub || "") + "</span></span></button>").join("") + "</div>").join("");
  const dsRows = (state.datasetRows || []).map((row) => '<div class="cards-grid">' + row.map((c) => {
    const file = (c.actionId || "").startsWith("dataset:") ? c.actionId.slice("dataset:".length) : "";
    const title = file ? datasetTitle(file) : (c.title || "");
    const sub = (file ? "Source: datasets/README.md. File: " + file + ". " : "") + (c.sub || "");
    return '<button type="button" class="card hoverable start-card" data-card="' + E(c.actionId || "") + '"><img src="assets/icons/' + E(c.icon || "dataset") + '.svg" alt=""><span><span class="sc-tag">' + E(c.tag || "") + '</span><br><span class="sc-title">' + E(title) + '</span><br><span class="sc-sub">' + E(sub) + "</span></span></button>";
  }).join("") + "</div>").join("");
  const count = state.itemCount || 0;
  const quickBody = "<ol><li><strong>Projects hold items.</strong> One project file holds every analysis item, and each item keeps its own inputs and results.</li>" +
    "<li><strong>Add an item.</strong> Choose Add item and pick the kind of analysis from the visual picker, give it a name if you want one, and the new item opens ready to use.</li>" +
    "<li><strong>Load data or an example.</strong> Type or paste data, load a bundled dataset, or open an example card below.</li>" +
    "<li><strong>Run.</strong> Use the primary button on the item screen. Results appear as cards and in the status bar.</li>" +
    "<li><strong>Send results between items.</strong> A Life Data fit can go to an RBD block, QCP, or Prediction. An FMEA row can create an RCM row, and FRACAS incidents can go to Life Data.</li></ol>";
  // Quickstart (M4): expanded until first dismissal, then a compact
  // collapsible card, collapsed by default.
  const quick = ui.quickstartDismissed
    ? '<details class="card quickstart quickstart-compact"><summary>Quickstart</summary>' + quickBody + "</details>"
    : '<div class="card quickstart"><div class="btn-row" style="justify-content:space-between"><h3 class="section-title">Quickstart</h3><button type="button" class="btn small" data-local="dismiss-quickstart">Dismiss</button></div>' + quickBody + "</div>";
  const startKeys = ["life", "rbd", "fmea", "prediction", "markov", "fracas"];
  const startCards = '<div class="cards-grid start-analysis-grid">' + startKeys.map((key) => {
    const m = KIND_META.find((k) => k.key === key);
    return '<button type="button" class="card hoverable start-card start-analysis-card" data-add-kind="' + kindIndex(key) + '"><img src="assets/icons/' + E(m.icon) + '.svg" alt=""><span><span class="sc-title">' + E(m.label) + '</span><br><span class="sc-sub">' + E(m.desc) + "</span></span></button>";
  }).join("") + "</div>";
  const recent = Array.isArray(ui.recentProjects) ? ui.recentProjects : [];
  const recentHtml = recent.length
    ? '<div style="display:flex;flex-direction:column;gap:10px"><h3 class="section-title">Recent projects</h3><div class="recent-list">' +
      recent.map((r, i) => '<button type="button" class="recent-row" data-recent="' + i + '"><span class="recent-label">' + E(r.label || "Untitled project") + '</span><span class="recent-sub">' + E(r.sub || "") + "</span></button>").join("") + "</div></div>"
    : "";
  const wizard = '<div class="card wizard-card"><h3 class="section-title">New here? Build your first project</h3>' +
    '<p class="caption">Each path sets up a starter item and walks you through it, step by step. Your existing items are not changed.</p>' +
    '<div class="btn-row wizard-paths">' +
    '<button type="button" class="btn btn-primary" data-wizard-path="failure-data">Analyze failure data</button>' +
    '<button type="button" class="btn" data-wizard-path="system">Model a system</button>' +
    '<button type="button" class="btn" data-wizard-path="maintenance">Track failures and maintenance</button>' +
    "</div></div>";
  // Slim hero banner (M4): logo, one line, version. The project
  // actions, analysis picker cards, and recent projects follow
  // directly so they sit above the fold; examples and datasets move
  // below them.
  return '<div class="start-scroll">' +
    '<div class="hero hero-slim"><div class="hero-inner"><div class="hero-top"><img class="logo-lg" src="assets/logo.svg" alt="ReliaKit logo"><div><h1>ReliaKit</h1><p class="hero-sub">One project workspace. Each item holds its own analysis, and items send results to each other.</p></div><span class="hero-version">v0.3.2</span></div></div></div>' +
    '<div class="start-grid">' +
    '<div class="card start-box"><h3 class="section-title">New project</h3><p class="caption">Start empty, then add analysis items from the project panel.</p>' + btn("new-project", "New project", { primary: true }) + "</div>" +
    '<div class="card start-box"><h3 class="section-title">Open project</h3><p class="caption">Project file: ' + E(state.projectPath || "reliakit_project.json") + "</p>" + btn("open-project", "Open project") + "</div>" +
    '<div class="card start-box"><h3 class="section-title">This project</h3><p class="caption">' + E(count === 0 ? "No items yet. Load an example or a dataset below, or add an item from the project panel." : count + " items. Click one in the project panel to open it.") + "</p>" + btn("save-project", "Save project") + "</div>" +
    "</div>" +
    '<div style="display:flex;flex-direction:column;gap:10px"><h3 class="section-title">Start an analysis</h3>' + startCards + "</div>" +
    recentHtml +
    wizard +
    quick +
    '<div style="display:flex;flex-direction:column;gap:10px"><h3 class="section-title">Examples</h3><p class="caption">Each example loads as a new item in this project.</p>' + exRows + "</div>" +
    '<div style="display:flex;flex-direction:column;gap:10px"><h3 class="section-title">Bundled datasets</h3><p class="caption">Documented example datasets from the ReliaSoft documentation and public textbooks, each with its source recorded in datasets/README.md. Life files go to Life Data, ALT files to ALT, degradation files to Degradation, growth files to Growth, and the Nevada warranty file to Warranty.</p>' + dsRows + "</div>" +
    '<div class="card start-box"><h3 class="section-title">About ReliaKit</h3><p class="caption">ReliaKit v0.3.2. Reliability engineering suite: life data, Bayesian methods, degradation, ALT, growth, RBD, fault trees, FMEA, warranty, prediction, DOE, Markov, RCM, and FRACAS.</p><p class="caption">Built with Tauri (MIT/Apache-2.0). Inter font, SIL Open Font License 1.1. ReliaKit is MIT licensed.</p><div class="btn-row"><button type="button" class="btn" data-local="open-about">About and attributions</button><button type="button" class="btn" data-local="show-quickstart">Show Quickstart</button></div></div>' +
    "</div>";
}

export function renderLife(state, ui) {
  const s = state.life;
  const outdated = !!state.outdated;
  const srcName = selectedItemName(state);
  const left = titleBlock("Life Data", "Fit a life distribution to failure and suspension times.") +
    area("life.times", "Times (comma separated; suffix c or + = censored)", s.times, 5) +
    selectField("life.dist_index", "Distribution", ALL_DISTS, s.distIndex) +
    '<div class="field-row">' + field("life.t", "Mission t", s.t) + checkField("life.rank", "Rank regression", s.rank) + "</div>" +
    btn("life-fit", "Run fit", { primary: true, busyKey: "life-fit" }) +
    checkField("life.show_prob", "Empirical vs fitted probability", s.showProb) +
    csvImport(state, ui.hostKind);
  let center = headlineCard("Reliability at mission time", s.headlineValue, outdated) +
    chartOrEmpty(s.chart, s.chartTitle || "Reliability curve", "Run a fit to see the reliability curve here.") +
    '<div style="display:flex;flex-direction:column;gap:8px">' + section("Distribution wizard") +
    caption("Rank candidate distributions by AIC on the current life data.") +
    '<div class="btn-row">' + btn("life-wizard", "Rank distributions", { busyKey: "life-wizard" }) + caption(s.wizardNote) + "</div>";
  if ((s.wizardRows || []).length) {
    center += '<p class="caption">Use copies the ranked distribution into ' + E(srcName) + " as its fit.</p>" +
      '<div class="data-table-wrap"><table class="data-table"><thead><tr><th>Distribution</th><th>Loglik</th><th>AIC</th><th>BIC</th><th>AD</th><th>Status</th><th></th></tr></thead><tbody>' +
      s.wizardRows.map((r, i) => {
        const statusCell = r.status ? E(r.status) + (r.reason ? ": " + E(r.reason) : "") : "";
        return "<tr><td>" + E(r.label || r.dist || "") + "</td><td>" + E(r.loglik || "") + "</td><td>" + E(r.aic || "") + "</td><td>" + E(r.bic || "") + "</td><td>" + E(r.ad || "") + "</td><td>" + statusCell + "</td><td>" + (r.usable ? rowBtn("wizard-use", i, "Use") : "") + "</td></tr>";
      }).join("") + "</tbody></table></div>";
  }
  center += "</div>";
  let right = section("Results") + outdatedBadgeHtml(state) + fitBadgeHtml(s) +
    ((s.results || []).length ? lifeFitTables(s.results, s) : resultCards(s.results));
  right += section("Send this fit");
  if (s.hasFit) {
    const targetName = (s.rbdTargets || [])[s.targetIndex] || "the selected RBD item";
    const blockName = (s.blockNames || [])[s.blockIndex] || "the selected block";
    right += selectField("life.target_index", "Target RBD item", s.rbdTargets || [], s.targetIndex, "flow-target") +
      selectField("life.block_index", "Block", s.blockNames || [], s.blockIndex) +
      '<p class="caption transfer-label">Send ' + E(srcName) + " fit to " + E(targetName) + ", block: " + E(blockName) + ".</p>" +
      btn("flow-rbd", "Assign fit to RBD block", { primary: true, busyKey: "flow-rbd" }) +
      '<p class="caption transfer-label">Send ' + E(srcName) + " fit to QCP.</p>" +
      btn("flow-qcp", "Send fit to QCP") +
      '<p class="caption transfer-label">Send ' + E(srcName) + " fit to Prediction.</p>" +
      btn("flow-pred", "Send fit to Prediction");
  } else {
    right += caption("Run a fit first, then the send actions appear here.");
  }
  return shell3(left, center, right);
}

export function renderLimited(state) {
  const s = state.limited;
  const left = titleBlock("Limited Data", "Hierarchical shrinkage for small lots, with bias correction and conformal bounds.") +
    area("limited.groups", "One lot per line, times comma separated (c = censored)", s.groups, 7) +
    btn("limited-run", "Run hierarchical fit", { primary: true, busyKey: "limited-run" });
  const center = section("Results") + outdatedBadgeHtml(state) + fitBadgeHtml(s) + resultCards(s.results);
  return shell2(left, center);
}

export function renderMcmc(state) {
  const s = state.mcmc;
  const left = titleBlock("Bayesian MCMC", "Adaptive Metropolis sampling with split R-hat and ESS diagnostics.") +
    area("mcmc.times", "Times (c = censored)", s.times, 4) +
    selectField("mcmc.dist_index", "Model", MCMC_DISTS, s.distIndex) +
    '<div class="field-row">' + field("mcmc.draws", "Draws", s.draws) + field("mcmc.warmup", "Warmup", s.warmup) + field("mcmc.chains", "Chains", s.chains) + "</div>" +
    '<div class="field-row">' + field("mcmc.seed", "Seed", s.seed) + field("mcmc.t", "Mission t", s.t) + "</div>" +
    btn("mcmc-run", "Run MCMC", { primary: true, busyKey: "mcmc-run" }) +
    caption("Sampling runs on the seeded library sampler, so runs repeat exactly.");
  const center = headlineCard("Posterior reliability", s.headlineValue, !!state.outdated) + outdatedBadgeHtml(state) + section("Posterior summary") + resultCards(s.results);
  return shell2(left, center);
}

export function renderDegradation(state, ui) {
  const s = state.deg;
  const left = titleBlock("Degradation", "Path fits per unit, pseudo-failure times at the threshold, then a life fit.") +
    area("deg.units", "One unit per line: times | measurements", s.units, 6) +
    '<div class="field-row">' + field("deg.threshold", "Threshold", s.threshold) + selectField("deg.model_index", "Path model", ["linear", "exponential", "power"], s.modelIndex) + "</div>" +
    selectField("deg.dist_index", "Life distribution", ALL_DISTS, s.distIndex) +
    checkField("deg.increasing", "Increasing failure direction", s.increasing) +
    btn("deg-run", "Run degradation analysis", { primary: true, busyKey: "deg-run" }) + csvImport(state, ui.hostKind);
  const center = chartOrEmpty(s.chart, "Degradation paths", "Run the analysis to see the degradation paths here.") + section("Results") + outdatedBadgeHtml(state) + fitBadgeHtml(s) + resultCards(s.results);
  return shell2(left, center);
}

export function renderAlt(state, ui) {
  const s = state.alt;
  const left = titleBlock("Accelerated Life Test", "Least squares and censored maximum likelihood fits, extrapolated to use stress.") +
    area("alt.data", "One unit per line: stress,time (Eyring: stress,stress2,time). Optional trailing failed flag 1 or 0.", s.data, 6) +
    selectField("alt.model_index", "Model", ["arrhenius", "ipl", "eyring"], s.modelIndex) +
    '<div class="field-row">' + field("alt.use_stress", "Use stress", s.useStress) + field("alt.t", "Mission t", s.t) + "</div>" +
    btn("alt-run", "Run ALT fit", { primary: true, busyKey: "alt-run" }) + csvImport(state, ui.hostKind);
  const center = chartOrEmpty(s.chart, "Reliability at use stress", "Run the fit to see reliability at use stress here.") + section("Results") + outdatedBadgeHtml(state) + fitBadgeHtml(s) + resultCards(s.results);
  return shell2(left, center);
}

export function renderGrowth(state, ui) {
  const s = state.growth;
  const left = titleBlock("Reliability Growth", "Crow-AMSAA and Duane fits on cumulative failure times.") +
    area("growth.times", "Failure times (comma separated)", s.times, 5) +
    btn("growth-run", "Run growth fit", { primary: true, busyKey: "growth-run" }) + csvImport(state, ui.hostKind);
  const center = chartOrEmpty(s.chart, "Growth curve", "Run the fit to see the growth curve here.") + section("Results") + outdatedBadgeHtml(state) + resultCards(s.results);
  return shell2(left, center);
}

export function renderCompeting(state, ui) {
  const s = state.comp;
  const left = titleBlock("Competing Risks", "Cause-specific fits and cumulative incidence per failure mode.") +
    area("comp.data", "One unit per line: time,mode (blank mode = censored)", s.data, 6) +
    selectField("comp.dist_index", "Distribution", ALL_DISTS, s.distIndex) +
    field("comp.t", "Mission t", s.t) +
    btn("comp-run", "Run competing risks fit", { primary: true, busyKey: "comp-run" }) + csvImport(state, ui.hostKind);
  const center = section("Results") + outdatedBadgeHtml(state) + fitBadgeHtml(s) + resultCards(s.results);
  return shell2(left, center);
}

export function renderFaultTree(state) {
  const s = state.ft;
  const left = titleBlock("Fault Tree", "Top event probability and minimal cut sets from the gate tree.") +
    area("ft.json", "FtNode JSON (serde format)", s.json, 9) +
    btn("ft-run", "Evaluate", { primary: true, busyKey: "ft-run" });
  const center = section("Results") + outdatedBadgeHtml(state) + resultCards(s.results);
  return shell2(left, center);
}

export function renderFmea(state, ui) {
  const s = state.fmea;
  let left = titleBlock("FMEA Worksheet", "Design, Process, or System FMEA with RPN and FMECA criticality.") +
    selectField("fmea.type_index", "Worksheet type (stored with the worksheet)", ["Design", "Process", "System"], s.typeIndex) +
    section("Add row") +
    field("fmea.fm_item", "Item", s.fmItem) + field("fmea.fm_mode", "Failure mode", s.fmMode) +
    '<div class="field-row">' + field("fmea.fm_sev", "Sev", s.fmSev) + field("fmea.fm_occ", "Occ", s.fmOcc) + field("fmea.fm_det", "Det", s.fmDet) + "</div>" +
    '<div class="field-row">' + field("fmea.fm_alpha", "FMECA alpha", s.fmAlpha) + field("fmea.fm_beta", "FMECA beta", s.fmBeta) + field("fmea.fm_hours", "Mission hours", s.fmHours) + "</div>" +
    field("fmea.fm_lambda", "FMECA lambda per hour", s.fmLambda) +
    btn("fmea-add-row", "Add row", { primary: true, busyKey: "fmea-add-row" }) +
    section("Send a row to RCM") +
    caption("Send a row of " + selectedItemName(state) + " to RCM as a new maintenance row.") +
    '<div class="field-row">' + field("fmea.rcm_row", "Row number (1 based)", s.rcmRow) + btn("fmea-to-rcm", "Create RCM row") + "</div>" +
    section("DVP&R entry") +
    field("fmea.dv_id", "Id", s.dvId) + field("fmea.dv_ref", "Failure mode ref", s.dvRef) + field("fmea.dv_test", "Test", s.dvTest) +
    '<div class="field-row">' + field("fmea.dv_sample", "Sample size", s.dvSample) + field("fmea.dv_criteria", "Acceptance criteria", s.dvCriteria) + "</div>" +
    btn("fmea-add-dvpr", "Add DVP&R entry") + caption(s.status);
  const center = section("Worksheet") +
    dataTable(["Item", "Failure mode", "Sev", "Occ", "Det", "RPN", "Rev RPN", "Criticality"], s.rows, { rowActions: [{ act: "fmea-rcm-row", text: "To RCM" }, { act: "fmea-del", text: "Delete" }], emptyText: "No worksheet rows yet. Add a row from the panel on the left." }) +
    section("DVP&R entries") +
    dataTable(["Id", "Failure mode ref", "Test", "Samples", "Acceptance criteria"], s.dvprRows, { emptyText: "No DVP&R entries yet. Add one from the panel on the left." });
  let right = section("Worksheet CSV") + helpDetails("Worksheet CSV carries a first line with the FMEA type, then the rows CSV.") +
    area("fmea.csv", "CSV text", s.csv, 7) +
    btn("fmea-export-ws", "Export worksheet to CSV text") + btn("fmea-export-rows", "Export rows only") +
    btn("fmea-import", "Import CSV") + btn("fmea-clear", "Clear rows") +
    section("File exports");
  if (ui.hostKind === "wasm") {
    right += helpDetails("In the browser demo, file exports download as files and CSV import uses the CSV text box above. The export file path field is a desktop only control.") + field("fmea.export_path", "Export file path (desktop only)", s.exportPath);
  } else {
    right += field("fmea.export_path", "Export file path", s.exportPath);
  }
  right += btn("fmea-export-cp", "Export control plan CSV") + btn("fmea-export-dvpr", "Export DVP&R CSV");
  return shell3(left, center, right);
}

export function renderWarranty(state, ui) {
  const s = state.war;
  const left = titleBlock("Warranty", "Nevada table aggregation and a Weibull returns forecast.") +
    field("war.sales", "Sales per period (comma separated)", s.sales) +
    area("war.returns", "Returns, one per line: sale period, return period (0-based)", s.returns, 5) +
    '<div class="field-row">' + field("war.current", "Current period", s.current) + field("war.horizon", "Horizon periods", s.horizon) + "</div>" +
    btn("war-run", "Run warranty analysis", { primary: true, busyKey: "war-run" }) + csvImport(state, ui.hostKind);
  let center = section("Nevada table") + caption("Rows are sale periods, columns are age in periods.");
  if ((s.tableRows || []).length === 0) center += emptyBox("Run the analysis to build the Nevada table.");
  else center += dataTable(s.tableHeader || [], s.tableRows, {});
  center += section("Results") + outdatedBadgeHtml(state) + resultCards(s.results);
  return shell2(left, center);
}

export function renderPrediction(state, ui) {
  const s = state.pred;
  let left = titleBlock("Prediction", "Parts-count prediction, MIL-HDBK-217F part stress, allocation, derating, and clean-room method shapes.") +
    section("Life model (from a Life Data fit)") + helpDetails("A fitted distribution sent from a Life Data item, or set by hand here.") +
    selectField("pred.life_dist_index", "Distribution", ALL_DISTS, s.lifeDistIndex) +
    field("pred.life_params", "Params", s.lifeParams) +
    '<div class="field-row">' + field("pred.life_t", "Mission t", s.lifeT) + btn("pred-life", "Compute R(t)") + "</div>" +
    caption(s.lifeResult) +
    selectField("pred.source_index", "Standard / source", ["Generic user data", "MIL-HDBK-217F library", "Local library file"], s.sourceIndex);
  if (s.sourceIndex === 0) {
    left += section("Parts-count prediction") + area("pred.parts", "One part per line: name, base rate per million hours, quantity, stress factor, quality factor", s.parts, 5) +
      '<div class="field-row">' + field("pred.t", "Mission t (hours)", s.t) + btn("pred-run", "Run prediction", { primary: true, busyKey: "pred-run" }) + "</div>";
  } else if (s.sourceIndex === 2) {
    left += section("Local component library CSV") + helpDetails("Format: name, base rate per million hours, quantity, stress factor, quality factor. Comment lines starting with # are skipped.");
    if (ui.hostKind === "wasm") {
      left += helpDetails("In the browser demo, Load library opens a file picker and sends the file text to the engine. No local path is needed.") +
        '<div class="btn-row"><button type="button" class="btn" data-file-import="library">Load library</button></div>' + caption(s.libNote) + field("pred.t", "Mission t (hours)", s.t);
    } else {
      left += field("pred.lib_path", "Library CSV path", s.libPath) + btn("pred-lib-load", "Load library") + caption(s.libNote) + field("pred.t", "Mission t (hours)", s.t);
    }
  } else {
    left += section("MIL-HDBK-217F part stress") + selectField("pred.p217_file_index", "Category file", MIL217_FILES, s.p217FileIndex) +
      btn("pred217-load", "Load category", { busyKey: "pred217-load" }) + caption(s.p217Category);
  }
  let center = "";
  if (s.sourceIndex === 0) {
    center = section("System prediction") + resultCards(s.partsResults);
  } else if (s.sourceIndex === 2) {
    const libRows = (s.libRows || []).map((r) => { const c = Array.isArray(r) ? r : (r.cells || []); return [c[0] || "", "Qty " + (c[1] || ""), c[2] || ""]; });
    center = section("Library parts") + (libRows.length ? dataTable(["Part", "Quantity", "Rate"], libRows, {}) : emptyBox("Load a library CSV to list its parts here.")) + section("System totals") + resultCards(s.libResults);
  } else {
    center = section("Category inputs");
    if (s.p217Misc) {
      center += caption("This category holds fixed rates. Pick a part and an environment; the rate shown is the fixed rate times pi_E.") +
        selectField("pred.p217_part_index", "Part", s.p217PartOptions || [], s.p217PartIndex) +
        selectField("pred.p217_env_index", "Environment", ["GB", "GF", "GM", "NS", "NU", "AIC", "CL"], s.p217EnvIndex);
    } else if ((s.p217Fields || []).length) {
      center += '<div class="card" style="padding:12px;display:flex;flex-direction:column;gap:8px">' + dynFields(s.p217Fields, "p217") + "</div>";
    } else {
      center += emptyBox("Load a category to see its inputs.");
    }
    center += btn("pred217-compute", "Compute part failure rate", { primary: true, busyKey: "pred217-compute" }) + section("Result") + resultCards(s.p217Results);
  }
  let right = section("Reliability allocation") +
    '<div class="field-row">' + field("pred.alloc_target", "System target R", s.allocTarget) + field("pred.alloc_n", "Subsystems n", s.allocN) + "</div>" +
    field("pred.alloc_weights", "Weights (weighted)", s.allocWeights) + btn("pred-alloc", "Allocate") + resultCards(s.allocResults, "No allocation yet. Choose Allocate to split the target.") +
    section("Derating check") + field("pred.der_name", "Part", s.derName) +
    '<div class="field-row">' + field("pred.der_operating", "Operating", s.derOperating) + field("pred.der_rated", "Rated", s.derRated) + field("pred.der_max", "Max ratio", s.derMax) + "</div>" +
    btn("pred-derate", "Check derating") + resultCards(s.derResults, "No derating result yet. Choose Check derating.") +
    section("Clean-room methods") + helpDetails("SR-332 style and 62380-style method shapes only. Every rate and factor is a value you supply; no standard table data is built in.") +
    '<div class="field-row">' + field("pred.sr_generic", "Generic rate", s.srGeneric) + field("pred.sr_pi_q", "pi_q", s.srPiQ) + field("pred.sr_pi_s", "pi_s", s.srPiS) + "</div>" +
    '<div class="field-row">' + field("pred.sr_ea", "Ea (eV)", s.srEa) + field("pred.sr_temp", "Use temp C", s.srTemp) + field("pred.sr_ref_temp", "Ref temp C", s.srRefTemp) + "</div>" +
    field("pred.sr_pi_e", "pi_e", s.srPiE) +
    area("pred.sr_qty_rate", "Quantity, device rate lines", s.srQtyRate, 2) +
    '<div class="field-row">' + field("pred.sr_obs_rate", "M-II observed rate", s.srObsRate) + field("pred.sr_pit_tgt", "pi_t target", s.srPitTgt) + "</div>" +
    '<div class="field-row">' + field("pred.sr_pit_obs", "pi_t observed", s.srPitObs) + field("pred.sr_pis_obs", "pi_s observed", s.srPisObs) + "</div>" +
    '<div class="field-row">' + field("pred.sr_pis_tgt", "pi_s target", s.srPisTgt) + "</div>" +
    '<div class="field-row">' + field("pred.sr_ci_failures", "CI failures", s.srCiFailures) + field("pred.sr_ci_exposure", "Exposure hours", s.srCiExposure) + field("pred.sr_ci_conf", "Confidence", s.srCiConf) + "</div>" +
    '<div class="field-row">' + field("pred.sr_m3_pred", "M-III predicted", s.srM3Pred) + field("pred.sr_m3_pw", "Pred weight", s.srM3Pw) + "</div>" +
    '<div class="field-row">' + field("pred.sr_m3_obs", "M-III observed", s.srM3Obs) + field("pred.sr_m3_ow", "Obs weight", s.srM3Ow) + "</div>" +
    btn("pred-clean", "Run clean-room calculations", { primary: true, busyKey: "pred-clean" }) +
    section("62380-style mission profile") + helpDetails("A mission profile framework with user supplied base rate and factors. It holds no IEC 62380 table data.") +
    '<div class="field-row">' + field("pred.mp_base", "Base rate", s.mpBase) + field("pred.mp_ea", "Ea (eV)", s.mpEa) + field("pred.mp_ref", "Ref temp C", s.mpRef) + "</div>" +
    field("pred.mp_cyc", "Cycling factor per cycle", s.mpCyc) +
    area("pred.mp_phases", "Phases, one per line: name, duration hours, temp C, cycles, dormant (1 or 0), dormant factor", s.mpPhases, 3) +
    btn("pred-mission", "Compute mission profile rate") + resultCards(s.cleanResults, "No clean-room results yet. Run the calculations above.");
  return shell3(left, center, right);
}

export function renderDoe(state) {
  const s = state.doe;
  const left = titleBlock("Factorial DOE", "Two-level factorial designs with effect estimates and sums of squares.") +
    helpDetails("The library also builds fractional factorials, Plackett-Burman, Taguchi, and response surface designs; this panel runs the full 2^k analysis.") +
    field("doe.k", "Factors k", s.k) + field("doe.y", "Responses y (2^k values, Yates order)", s.y) +
    btn("doe-run", "Analyze", { primary: true, busyKey: "doe-run" });
  const center = section("Effects") + outdatedBadgeHtml(state) + resultCards(s.results) + section("Results") + caption("Effects, sums of squares, and percent contribution appear in the cards above, in Yates order.");
  return shell2(left, center);
}

export function renderFlowchart(state) {
  const s = state.flow;
  const left = titleBlock("Flowchart Simulation", "Process network simulation with run time and cost distributions.") +
    area("flow.json", "Process network JSON", s.json, 9) +
    '<div class="field-row">' + field("flow.sims", "Simulations", s.sims) + field("flow.seed", "Seed", s.seed) + "</div>" +
    btn("flow-run", "Run simulation", { primary: true, busyKey: "flow-run" });
  const center = section("Results") + outdatedBadgeHtml(state) + resultCards(s.results);
  return shell2(left, center);
}

export function renderMarkov(state) {
  const s = state.markov;
  const states = s.states || [];
  let left = titleBlock("Markov Chain", "Continuous time Markov models with steady state, transient, and mean time to absorption.") +
    section("States") + btn("mk-preset", "Load two-unit preset") +
    helpDetails("Preset: two-unit parallel repairable system. The preset gives MTTF 650 hours from Both up. Add a repair transition out of System down for a non-trivial steady state.");
  left += '<div style="display:flex;flex-direction:column;gap:6px">' + states.map((name, i) => '<div class="btn-row" style="justify-content:space-between"><span>' + E(name) + "</span>" + rowBtn("mk-del-state", i, "Remove") + "</div>").join("") + "</div>";
  if (states.length === 0) left += caption("No states yet. Add a state below or load the preset.");
  left += '<div class="field-row">' + field("markov.new_state", "New state", s.newState) + btn("mk-add-state", "Add state") + "</div>" +
    section("Compute") +
    selectField("markov.start_index", "Start state", states, s.startIndex) +
    selectField("markov.absorbing_index", "Absorbing state (for MTTF)", states, s.absorbingIndex) +
    field("markov.time", "Transient time", s.time) +
    btn("mk-run", "Compute Markov results", { primary: true, busyKey: "mk-run" });
  let center = section("Transitions") +
    dataTable(["From", "To", "Rate per hour"], s.transitions, { selectAct: "mk-select", selectedIndex: s.selTransition, emptyText: "No transitions yet. Add one from the inspector panel." }) +
    section("Results") + outdatedBadgeHtml(state) + resultCards(s.results) +
    (s.chart ? chartCardHtml(s.chart, "Transient probabilities") : emptyBox("Run the computation to see transient probabilities here."));
  const right = section("Transition inspector") +
    (s.selTransition >= 0
      ? field("markov.tr_from", "From state", s.trFrom) + field("markov.tr_to", "To state", s.trTo) + field("markov.tr_rate", "Rate per hour", s.trRate) + btn("mk-apply-trans", "Apply edits") + btn("mk-del-trans", "Remove transition", { danger: true })
      : caption("Select a transition to edit it here.")) +
    btn("mk-add-trans", "Add transition");
  return shell3(left, center, right);
}

export function renderQcp(state) {
  const s = state.qcp;
  const left = titleBlock("QCP", "Quick calculation pad: reliability, hazard, mean life, and Bx life for any distribution.") +
    selectField("qcp.dist_index", "Distribution", ALL_DISTS, s.distIndex, "qcp-dist") +
    section("Parameters") + '<div style="display:flex;flex-direction:column;gap:4px">' + dynFields(s.params, "qcp") + "</div>" +
    '<div class="field-row">' + field("qcp.t", "Mission t", s.t) + field("qcp.bx", "Bx fraction", s.bx) + "</div>" +
    caption("Bx fraction 0.10 means B10 life.") +
    btn("qcp-run", "Calculate", { primary: true, busyKey: "qcp-run" });
  const center = headlineCard("Reliability", s.headlineValue, !!state.outdated) + outdatedBadgeHtml(state) + section("Results") + resultCards(s.results);
  return shell2(left, center);
}

export function renderRcm(state) {
  const s = state.rcm;
  const left = titleBlock("RCM Worksheet", "Reliability centered maintenance: consequence classes drive the task selection.") +
    helpDetails("Task selection and criticality update from the row inputs. Criticality needs failure rate, mode ratio, effect probability, and mission hours.") +
    btn("rcm-add", "Add row", { primary: true, busyKey: "rcm-add" }) +
    helpDetails("Select a row in the worksheet to edit it in the inspector. Rows can also arrive from an FMEA worksheet.");
  const center = section("Worksheet") +
    dataTable(["Asset", "Function", "Failure mode", "Consequence", "Selected task", "Criticality"], s.rows, { selectAct: "rcm-select", selectedIndex: s.sel, emptyText: "No RCM rows yet. Add a row, or send one from an FMEA worksheet." });
  let right = section("Row inspector");
  if (s.sel >= 0) {
    right += field("rcm.ed_asset", "Asset", s.edAsset) + field("rcm.ed_function", "Function", s.edFunction) + field("rcm.ed_mode", "Failure mode", s.edMode) +
      selectField("rcm.ed_consequence_index", "Consequence", ["Hidden", "Safety", "Environmental", "Operational", "NonOperational"], s.edConsequenceIndex) +
      checkField("rcm.ed_monitorable", "Condition monitorable", s.edMonitorable) + checkField("rcm.ed_age", "Age related", s.edAge) + checkField("rcm.ed_restorable", "Restorable", s.edRestorable) +
      '<div class="field-row">' + field("rcm.ed_rate", "Rate per hour", s.edRate) + field("rcm.ed_hours", "Hours", s.edHours) + "</div>" +
      '<div class="field-row">' + field("rcm.ed_alpha", "Alpha", s.edAlpha) + field("rcm.ed_beta", "Beta", s.edBeta) + "</div>" +
      resultCards([["Selected task", s.edTask || ""], ["Criticality Cm", s.edCrit || ""]]) +
      btn("rcm-refresh", "Refresh task and criticality") + btn("rcm-remove", "Remove row", { danger: true });
  } else {
    right += caption("Select a worksheet row to edit it here.");
  }
  return shell3(left, center, right);
}

export function renderFracas(state, ui) {
  const s = state.fracas;
  let left = titleBlock("FRACAS", "Failure reporting and corrective action: incident log with status workflow.") +
    section("Summary") + resultCards(s.summary, "No summary yet. Add an incident to start the log.") +
    field("fracas.total_hours", "Total operating hours (for MTBF)", s.totalHours) +
    btn("fracas-add", "Add incident", { primary: true, busyKey: "fracas-add" }) +
    section("Send incidents to Life Data") + caption("Send " + selectedItemName(state) + " incidents to a new Life Data item.") + btn("flow-fracas-life", "Export to Life Data item") +
    helpDetails("Each incident hour value becomes a failure time. All incidents count as failures, because the FRACAS log has no censoring notion.") +
    section("CSV import and export");
  if (ui.hostKind === "wasm") {
    left += helpDetails("In the browser demo, Export CSV downloads the log as a file and Import CSV opens a file picker. The CSV file path field is a desktop only control.") +
      field("fracas.path", "CSV file path (desktop only)", s.path) +
      '<div class="btn-row">' + btn("fracas-export-csv", "Export CSV") + '<button type="button" class="btn" data-file-import="fracas">Import CSV</button></div>';
  } else {
    left += field("fracas.path", "CSV file path", s.path) +
      '<div class="btn-row">' + btn("fracas-export-csv", "Export CSV") + btn("fracas-import-csv", "Import CSV") + "</div>";
  }
  left += caption(s.status);
  const center = section("Incidents") +
    dataTable(["Id", "Part", "Failure mode", "Hours", "Date", "Status", "Corrective action"], s.rows, { selectAct: "fracas-select", selectedIndex: s.sel, emptyText: "No incidents yet. Choose Add incident to start the log." });
  let right = section("Incident inspector");
  if (s.sel >= 0) {
    right += field("fracas.ed_id", "Id", s.edId) + field("fracas.ed_part", "Part", s.edPart) + field("fracas.ed_mode", "Failure mode", s.edMode) +
      '<div class="field-row">' + field("fracas.ed_hours", "Hours", s.edHours) + field("fracas.ed_date", "Date", s.edDate) + "</div>" +
      selectField("fracas.ed_status_index", "Status", ["Open", "Analysis", "CorrectiveAction", "Closed"], s.edStatusIndex) +
      field("fracas.ed_action", "Corrective action", s.edAction) +
      btn("fracas-refresh", "Apply edits") + btn("fracas-remove", "Remove incident", { danger: true });
  } else {
    right += caption("Select an incident to edit it here.");
  }
  return shell3(left, center, right);
}

export const SCREEN_RENDERERS = {
  life: renderLife, limited: renderLimited, mcmc: renderMcmc, degradation: renderDegradation,
  alt: renderAlt, growth: renderGrowth, competing: renderCompeting, faulttree: renderFaultTree,
  fmea: renderFmea, warranty: renderWarranty, prediction: renderPrediction, doe: renderDoe,
  flowchart: renderFlowchart, markov: renderMarkov, qcp: renderQcp, rcm: renderRcm, fracas: renderFracas,
};
