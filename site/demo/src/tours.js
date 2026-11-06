// Guided tours for ReliaKit. Tours are data: each step names a CSS
// selector resolved against the live DOM when the step is shown, an
// optional action that runs when the step is entered, and plain text.
//
// The overlay lives outside #app (appended to document.body) so the
// app.js render cycle, which rebuilds #app innerHTML on every engine
// response, never wipes it. Repositioning follows window resizes, a
// MutationObserver on #app, and a short interval while a tour runs.
//
// Starting a tour never overwrites user items: it loads a bundled
// example or dataset as a new item, or adds a fresh item, then renames
// that new item to "Tour: <Label>" and selects it. Step 1 of every
// tour says so.
import { KIND_META, FAMILIES, kindIndex, kindByKey } from "./kinds.js";

const EXAMPLE_FOR_KIND = {
  life: "example:1",
  limited: "example:2",
  rbd: "example:3",
  fmea: "example:4",
  alt: "example:5",
  growth: "example:6",
  degradation: "example:7",
  markov: "example:8",
};
const WARRANTY_DATASET = "dataset:reliasoft_warranty_nevada.csv";

function esc(v) {
  return String(v == null ? "" : v).replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");
}
function sleep(ms) { return new Promise((r) => setTimeout(r, ms)); }
function dispatchAction(action, fields) {
  return { type: "dispatch", action, fields };
}

export const TOURS = {
  life: {
    kind: "life", label: "Life Data",
    steps: [
      { target: ".screen-header", title: "Life Data tour", body: "Life Data fits a distribution to failure and suspension times. This tour runs in its own new item, Tour: Life Data, loaded from Example 1. Your other items are not changed." },
      { target: '[data-field="life.times"]', title: "Failure times", body: "Times go in this box, comma separated. Add c or + after a time to mark it as censored. The example bearing data is already loaded here." },
      { target: '[data-act="life-fit"]', title: "Run the fit", body: "Run fit estimates the distribution parameters by maximum likelihood and computes reliability at the mission time.", action: dispatchAction("life-fit") },
      { target: ".headline-card", title: "Headline result", body: "The headline card shows reliability at the mission time, R(1000) for this item. The same value appears in the status bar at the bottom." },
      { target: ".result-card", title: "Fit details", body: "The result cards list the fitted parameters, confidence bounds, B10 life, and mean life from the fit." },
      { target: '[data-act="flow-rbd"]', title: "Send the fit on", body: "After a fit, these buttons send it to an RBD block, to QCP, or to Prediction. The label above each button names the source and destination. Pick a target RBD item and block first, then use Assign fit to RBD block. A send copies a snapshot; it does not create a live link." },
    ],
  },
  limited: {
    kind: "limited", label: "Limited Data",
    steps: [
      { target: ".screen-header", title: "Limited Data tour", body: "Limited Data gives shrinkage estimates and honest bounds from very small samples. This tour runs in its own new item, Tour: Limited Data, loaded from Example 2. Your other items are not changed." },
      { target: '[data-field="limited.groups"]', title: "One lot per line", body: "Each line is one lot, with times comma separated and c marking censored values. The example lots are already loaded." },
      { target: '[data-act="limited-run"]', title: "Run the hierarchical fit", body: "Run hierarchical fit pools the lots, shrinks each lot toward the pooled estimate, and adds bias correction and conformal bounds.", action: dispatchAction("limited-run") },
      { target: ".result-card", title: "Read the results", body: "The cards show the pooled shape, the shrunk eta for each lot, the bias correction for lot 1, and conformal bounds you can quote with small samples." },
      { target: ".statusbar", title: "Status and saving", body: "The status bar at the bottom repeats the headline for the current item. Save the project from the toolbar to keep this item and its results." },
    ],
  },
  mcmc: {
    kind: "mcmc", label: "MCMC",
    steps: [
      { target: ".screen-header", title: "MCMC tour", body: "MCMC fits a distribution with Bayesian sampling and reports posterior summaries with convergence checks. This tour runs in its own new item, Tour: MCMC. Sample data is already filled in; replace it with yours. Your other items are not changed." },
      { target: '[data-field="mcmc.times"]', title: "Times and model", body: "Times go here in the same format as Life Data. Pick Weibull or lognormal under Model. The sample times are the bearing data." },
      { target: '[data-field="mcmc.draws"]', title: "Sampling settings", body: "Draws, warmup, chains, and seed control the sampler. The seed makes runs repeat exactly, so the defaults are fine for a first run." },
      { target: '[data-act="mcmc-run"]', title: "Run MCMC", body: "Run MCMC samples the posterior for the chosen model. It can take a few seconds.", action: dispatchAction("mcmc-run") },
      { target: ".headline-card", title: "Posterior reliability", body: "The headline card shows mean posterior reliability at the mission time, with a credible interval in the summary below." },
      { target: ".result-card", title: "Check convergence", body: "Read R-hat and ESS for each parameter before trusting the summary. R-hat near 1 and a large ESS mean the chains mixed well." },
    ],
  },
  degradation: {
    kind: "degradation", label: "Degradation",
    steps: [
      { target: ".screen-header", title: "Degradation tour", body: "Degradation projects measured wear paths to a failure threshold and fits a life distribution to the crossing times. This tour runs in its own new item, Tour: Degradation, loaded from Example 7. Your other items are not changed." },
      { target: '[data-field="deg.units"]', title: "Unit paths", body: "One unit per line: measurement times, a vertical bar, then the measurements. The example units are already loaded." },
      { target: '[data-field="deg.threshold"]', title: "Threshold and path model", body: "The threshold is the measurement value that counts as failure. Pick a path model and a life distribution to fit the pseudo failure times." },
      { target: '[data-act="deg-run"]', title: "Run the analysis", body: "Run degradation analysis fits each path, finds when each unit crosses the threshold, and fits the life distribution to those times.", action: dispatchAction("deg-run") },
      { target: ".result-card", title: "Results", body: "The cards report the fitted life parameters and reliability. The chart in the center plots each unit path against the threshold line." },
      { target: ".statusbar", title: "Status and saving", body: "The status bar at the bottom repeats the headline for the current item. Save the project from the toolbar to keep this item and its results." },
    ],
  },
  alt: {
    kind: "alt", label: "ALT",
    steps: [
      { target: ".screen-header", title: "ALT tour", body: "ALT fits accelerated test data and extrapolates life to normal use stress. This tour runs in its own new item, Tour: ALT, loaded from Example 5. Your other items are not changed." },
      { target: '[data-field="alt.data"]', title: "Stress and time pairs", body: "One unit per line: stress, time, with an optional failed flag of 1 or 0 at the end. The example Arrhenius data is already loaded." },
      { target: '[data-field="alt.use_stress"]', title: "Use stress", body: "Use stress is the normal operating stress. The fit extrapolates from the test stresses down to this value." },
      { target: '[data-act="alt-run"]', title: "Run the ALT fit", body: "Run ALT fit fits the life-stress model by least squares and censored maximum likelihood.", action: dispatchAction("alt-run") },
      { target: ".result-card", title: "Life at use stress", body: "The cards give the model parameters and reliability at use stress for the mission time. The chart shows the reliability curve at use stress." },
      { target: ".statusbar", title: "Status and saving", body: "The status bar at the bottom repeats the headline for the current item. Save the project from the toolbar to keep this item and its results." },
    ],
  },
  growth: {
    kind: "growth", label: "Growth",
    steps: [
      { target: ".screen-header", title: "Growth tour", body: "Growth tracks reliability growth across a test, fix, test program with Crow-AMSAA and Duane fits. This tour runs in its own new item, Tour: Growth, loaded from Example 6. Your other items are not changed." },
      { target: '[data-field="growth.times"]', title: "Failure times", body: "Cumulative failure times go here, comma separated. The example test program times are already loaded." },
      { target: '[data-act="growth-run"]', title: "Run the growth fit", body: "Run growth fit estimates the growth rate and the current MTBF from the failure times.", action: dispatchAction("growth-run") },
      { target: ".result-card", title: "Growth results", body: "The cards show the Crow-AMSAA shape and scale, the Duane fit, and instantaneous MTBF at the end of test. The chart plots expected failures against time." },
      { target: ".statusbar", title: "Status and saving", body: "The status bar at the bottom repeats the headline for the current item. Save the project from the toolbar to keep this item and its results." },
    ],
  },
  competing: {
    kind: "competing", label: "Competing Risks",
    steps: [
      { target: ".screen-header", title: "Competing Risks tour", body: "Competing Risks separates several failure causes fitted to the same units and shows cumulative incidence per cause. This tour runs in its own new item, Tour: Competing Risks. Sample data is already filled in; replace it with yours. Your other items are not changed." },
      { target: '[data-field="comp.data"]', title: "Times and modes", body: "One unit per line: time, mode. A blank mode means the unit was censored. The sample data has wear and electrical modes." },
      { target: '[data-act="comp-run"]', title: "Run the fit", body: "Run competing risks fit fits each cause on its own and combines them into incidence at the mission time.", action: dispatchAction("comp-run") },
      { target: ".result-card", title: "Incidence by cause", body: "The cards give each cause its parameters and its share of failures by the mission time." },
      { target: ".statusbar", title: "Status and saving", body: "The status bar at the bottom repeats the headline for the current item. Save the project from the toolbar to keep this item and its results." },
    ],
  },
  rbd: {
    kind: "rbd", label: "RBD",
    steps: [
      { target: ".screen-header", title: "RBD tour", body: "RBD builds a block diagram of a system and computes reliability and availability from the blocks. This tour runs in its own new item, Tour: RBD, loaded from Example 3. Your other items are not changed." },
      { target: "#rbd-canvas", title: "The diagram", body: "The canvas holds the Example 3 system: blocks in series, a standby pump group, and parallel controllers. Drag a block to move it; positions are saved with the project." },
      { target: "[data-rbd-block]", title: "Select a block", body: "Click any block to select it. Its distribution, parameters, MTBF, and MTTR open in the Block inspector on the right, where Apply properties saves edits." },
      { target: '[data-field="rbd.t"]', title: "Mission time and topology", body: "Mission time is where reliability is read. Topology and k set how the blocks combine, and Apply topology rebuilds the layout from the current blocks." },
      { target: '[data-act="rbd-avail"]', title: "Run availability", body: "Run availability simulates the repairable system over the horizon and reports availability and downtime. The Simulations field beside the horizon sets how many Monte Carlo runs it makes, and the results include the Monte Carlo standard error.", action: dispatchAction("rbd-avail") },
      { target: ".result-card", title: "Availability results", body: "The cards under the button report simulated availability. The headline card above them shows reliability at the mission time." },
      { target: '[data-act="rbd-importance"]', title: "Importance and next steps", body: "Compute importance measures ranks the blocks by Birnbaum and Fussell-Vesely importance, so you can see which block drives system reliability. Save the project to keep the diagram and its positions." },
    ],
  },
  faulttree: {
    kind: "faulttree", label: "Fault Tree",
    steps: [
      { target: ".screen-header", title: "Fault Tree tour", body: "Fault Tree models failure logic with gates and finds the minimal cut sets for the top event. This tour runs in its own new item, Tour: Fault Tree. Sample data is already filled in; replace it with yours. Your other items are not changed." },
      { target: '[data-field="ft.json"]', title: "The gate tree", body: "The tree is FtNode JSON: gates such as Or and And with Basic events that carry a probability. The sample tree has a pump and a valve under an Or gate." },
      { target: '[data-act="ft-run"]', title: "Evaluate the tree", body: "Evaluate computes the top event probability from the gate logic.", action: dispatchAction("ft-run") },
      { target: ".result-card", title: "Cut sets", body: "The cards show the top event probability and the minimal cut sets, the smallest combinations of basic events that cause the top event." },
      { target: ".statusbar", title: "Status and saving", body: "The status bar at the bottom repeats the headline for the current item. Save the project from the toolbar to keep this item and its results." },
    ],
  },
  fmea: {
    kind: "fmea", label: "FMEA",
    steps: [
      { target: ".screen-header", title: "FMEA tour", body: "FMEA is a worksheet for failure modes, risk priority numbers, and FMECA criticality. This tour runs in its own new item, Tour: FMEA, loaded from Example 4. Your other items are not changed." },
      { target: '[data-field="fmea.fm_item"]', title: "Add row form", body: "Item and failure mode go here, with severity, occurrence, and detection scores from 1 to 10. FMECA fields are optional. The worksheet already holds the three example rows." },
      { target: '[data-act="fmea-add-row"]', title: "Add a row", body: "This step adds one tour row with the Add row button, a valve that sticks. In your own worksheet, fill the form first, then use the same button.", action: dispatchAction("fmea-add-row", { "fmea.fm_item": "Tour valve", "fmea.fm_mode": "Valve sticks", "fmea.fm_sev": "6", "fmea.fm_occ": "4", "fmea.fm_det": "5" }) },
      { target: ".data-table-wrap", title: "Read the worksheet", body: "Each row shows its RPN, the product of severity, occurrence, and detection, plus revised RPN and criticality when FMECA values are set. The new tour row is at the bottom." },
      { target: '[data-act="fmea-to-rcm"]', title: "Send a row to RCM", body: "Type a row number and use Create RCM row to start a maintenance row from a failure mode. Each worksheet row also has its own To RCM button in the table." },
      { target: '[data-field="fmea.csv"]', title: "Export the worksheet", body: "Export worksheet to CSV text writes the worksheet into this box, ready to copy or import back later. File exports below it write CSV files on the desktop." },
    ],
  },
  warranty: {
    kind: "warranty", label: "Warranty",
    steps: [
      { target: ".screen-header", title: "Warranty tour", body: "Warranty forecasts returns from sales and claims using a Nevada table. This tour runs in its own new item, Tour: Warranty, loaded from the bundled Nevada dataset. Your other items are not changed." },
      { target: '[data-field="war.sales"]', title: "Sales per period", body: "Sales per period go here, comma separated. The dataset values are already loaded." },
      { target: '[data-field="war.returns"]', title: "Returns", body: "Returns go here, one per line: sale period, return period, both 0 based. Current period and horizon set how far the forecast runs." },
      { target: '[data-act="war-run"]', title: "Run the warranty analysis", body: "Run warranty analysis builds the Nevada table and fits a Weibull forecast to the returns.", action: dispatchAction("war-run") },
      { target: ".data-table-wrap", title: "Nevada table", body: "Rows are sale periods and columns are age in periods. Each cell counts returns at that age for that sale period." },
      { target: ".result-card", title: "Forecast", body: "The cards total the returns and give the forecast Weibull parameters, expected returns over the horizon, and units still in service." },
    ],
  },
  prediction: {
    kind: "prediction", label: "Prediction",
    steps: [
      { target: ".screen-header", title: "Prediction tour", body: "Prediction estimates a system failure rate from a parts list, with allocation, derating, and standards based part stress in the same item. This tour runs in its own new item, Tour: Prediction. Sample data is already filled in; replace it with yours. Your other items are not changed." },
      { target: '[data-field="pred.parts"]', title: "Parts list", body: "One part per line: name, base rate per million hours, quantity, stress factor, quality factor. Three sample parts are already listed." },
      { target: '[data-act="pred-run"]', title: "Run the prediction", body: "Run prediction sums the part rates into a system failure rate, MTBF, and reliability at the mission time.", action: dispatchAction("pred-run") },
      { target: ".result-card", title: "System prediction", body: "The cards show the system failure rate per million hours, MTBF, and reliability at the mission time." },
      { target: '[data-act="pred-alloc"]', title: "Allocation and derating", body: "Allocate splits a system reliability target across subsystems by weight. Check derating, below it, compares a part operating stress with its rated limit." },
      { target: ".statusbar", title: "Standards library and saving", body: "Switch Standard / source to the MIL-HDBK-217F library to compute a single part rate from the public tables. Save the project from the toolbar to keep this item." },
    ],
  },
  doe: {
    kind: "doe", label: "DOE",
    steps: [
      { target: ".screen-header", title: "DOE tour", body: "DOE analyzes two-level factorial experiments and estimates factor effects. This tour runs in its own new item, Tour: DOE. Sample data is already filled in; replace it with yours. Your other items are not changed." },
      { target: '[data-field="doe.y"]', title: "Design and responses", body: "Set the number of factors k, then list the 2^k responses in Yates order. The sample run has k = 3 and eight responses." },
      { target: '[data-act="doe-run"]', title: "Analyze the design", body: "Analyze estimates every main effect and interaction from the responses.", action: dispatchAction("doe-run") },
      { target: ".result-card", title: "Effects", body: "The cards list effects, sums of squares, and percent contribution in Yates order. Large effects stand out by size." },
      { target: ".statusbar", title: "Status and saving", body: "The status bar at the bottom repeats the headline for the current item. Save the project from the toolbar to keep this item and its results." },
    ],
  },
  flowchart: {
    kind: "flowchart", label: "Flowchart",
    steps: [
      { target: ".screen-header", title: "Flowchart tour", body: "Flowchart simulates a process network with Monte Carlo runs to estimate total time and cost. This tour runs in its own new item, Tour: Flowchart. Sample data is already filled in; replace it with yours. Your other items are not changed." },
      { target: '[data-field="flow.json"]', title: "Process network", body: "The process is JSON: start, task, and end nodes linked by next fields, with a time distribution and cost per task. The sample process inspects, then assembles." },
      { target: '[data-act="flow-run"]', title: "Run the simulation", body: "Run simulation runs the network for the set number of simulations with the given seed, so results repeat.", action: dispatchAction("flow-run") },
      { target: ".result-card", title: "Simulation results", body: "The cards summarize run time and cost across the simulations, with mean and spread." },
      { target: ".statusbar", title: "Status and saving", body: "The status bar at the bottom repeats the headline for the current item. Save the project from the toolbar to keep this item and its results." },
    ],
  },
  markov: {
    kind: "markov", label: "Markov",
    steps: [
      { target: ".screen-header", title: "Markov tour", body: "Markov models a repairable system as states with transition rates, and computes steady state, transient probabilities, and mean time to absorption. This tour runs in its own new item, Tour: Markov, loaded from Example 8. Your other items are not changed." },
      { target: '[data-field="markov.new_state"]', title: "States", body: "The preset has three states: Both up, One down, and System down. Add a state here, or remove one from the list." },
      { target: ".data-table-wrap", title: "Transitions", body: "Each row is a transition rate per hour between two states. Select a row to edit it in the inspector on the right." },
      { target: '[data-act="mk-run"]', title: "Compute the chain", body: "Compute Markov results solves the chain for the start and absorbing states set on the left.", action: dispatchAction("mk-run") },
      { target: ".result-card", title: "Results", body: "The headline result is MTTF from Both up, 650 hours for this preset. The cards also give steady state and transient probabilities, and the chart plots them over time." },
      { target: ".statusbar", title: "Status and saving", body: "The status bar at the bottom repeats the headline for the current item. Save the project from the toolbar to keep this item and its results." },
    ],
  },
  qcp: {
    kind: "qcp", label: "QCP",
    steps: [
      { target: ".screen-header", title: "QCP tour", body: "QCP is a quick calculation pad for one distribution: reliability, hazard, mean life, and Bx life. This tour runs in its own new item, Tour: QCP. Sample values are already filled in; replace them with yours. Your other items are not changed." },
      { target: '[data-field="qcp.dist_index"]', title: "Distribution and parameters", body: "Pick a distribution, then set its parameters in the list below. Weibull starts with beta and eta filled in." },
      { target: '[data-field="qcp.t"]', title: "Mission time and Bx", body: "Mission t is where reliability and hazard are read. Bx fraction 0.10 asks for B10 life." },
      { target: '[data-act="qcp-run"]', title: "Calculate", body: "Calculate reads the distribution at the mission time and computes the Bx life.", action: dispatchAction("qcp-run") },
      { target: ".headline-card", title: "Reliability", body: "The headline card shows reliability at the mission time." },
      { target: ".result-card", title: "Full results", body: "The cards add hazard, mean life, and the Bx life. Change t or the parameters and calculate again to compare." },
    ],
  },
  rcm: {
    kind: "rcm", label: "RCM",
    steps: [
      { target: ".screen-header", title: "RCM tour", body: "RCM chooses a maintenance task for each failure mode from its consequence class and failure pattern. This tour runs in its own new item, Tour: RCM. One sample row is already in the worksheet; replace it with yours. Your other items are not changed." },
      { target: ".data-table-wrap", title: "Worksheet", body: "Each row is one failure mode with its consequence class, selected task, and criticality. Rows can also arrive from an FMEA worksheet." },
      { target: '[data-act="rcm-add"]', title: "Add a row", body: "Add row creates a blank row and opens it in the inspector on the right. This step just added one.", action: dispatchAction("rcm-add") },
      { target: '[data-field="rcm.ed_asset"]', title: "Row inspector", body: "Fill asset, function, and failure mode here. Consequence, the monitorable and age flags, and the rate values drive the task choice." },
      { target: '[data-act="rcm-refresh"]', title: "Refresh the task", body: "Refresh task and criticality recomputes the selected task and the criticality number for the edited row." },
      { target: ".statusbar", title: "Status and saving", body: "The status bar at the bottom repeats the headline for the current item. Save the project from the toolbar to keep this item and its rows." },
    ],
  },
  fracas: {
    kind: "fracas", label: "FRACAS",
    steps: [
      { target: ".screen-header", title: "FRACAS tour", body: "FRACAS logs failures, tracks corrective actions, and watches MTBF over total operating hours. This tour runs in its own new item, Tour: FRACAS. Sample incidents are already in the log; replace them with yours. Your other items are not changed." },
      { target: ".data-table-wrap", title: "Incident log", body: "Each row is one incident with part, failure mode, hours, date, status, and corrective action. Select a row to edit it in the inspector on the right." },
      { target: '[data-field="fracas.total_hours"]', title: "Hours and summary", body: "Total operating hours sets the MTBF in the summary cards on the left. Keep it current as the fleet runs." },
      { target: '[data-act="fracas-add"]', title: "Add an incident", body: "Add incident creates the next FR number and opens it in the inspector on the right. This step just added one.", action: dispatchAction("fracas-add") },
      { target: '[data-field="fracas.ed_part"]', title: "Incident inspector", body: "Fill part, failure mode, hours, and date here, then set the status as work moves from Open to Closed. Apply edits saves the row." },
      { target: '[data-act="flow-fracas-life"]', title: "Send incidents to Life Data", body: "Export to Life Data item turns every incident hour value into a failure time in a new Life Data item, ready for a fit." },
    ],
  },
};

// ---------------- tour engine ----------------

let api = null;
let current = null; // { kind, tour, index, found, root, spot, card, observer, timer, onResize }
let listRoot = null;

function findTarget(selector) {
  if (!selector) return null;
  let el = null;
  try { el = document.querySelector(selector); } catch (e) { return null; }
  if (!el) return null;
  const r = el.getBoundingClientRect();
  if (!r || (r.width <= 0 && r.height <= 0)) return null;
  return el;
}

async function pollTarget(selector, timeoutMs) {
  const limit = timeoutMs || 2500;
  const start = Date.now();
  for (;;) {
    const el = findTarget(selector);
    if (el) return el;
    if (Date.now() - start >= limit) return null;
    await sleep(100);
  }
}

async function runAction(action) {
  if (!action || !api) return;
  if (action.type === "dispatch") {
    await api.dispatch(action.action, action.fields || {});
  } else if (action.type === "card") {
    await api.dispatch("card-action", { id: action.id, card_id: action.id, action_id: action.id });
  } else if (action.type === "rename") {
    const rn = document.querySelector("#rename-name");
    if (rn) rn.value = action.name;
    await api.dispatch("rename-item");
  }
  // "addItem" is handled by the start sequence, not by a step.
}

function position() {
  if (!current) return;
  const step = current.tour.steps[current.index];
  const el = step ? findTarget(step.target) : null;
  current.found = !!el;
  const spot = current.spot;
  const card = current.card;
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  const cardW = Math.min(380, vw - 20);
  card.style.width = cardW + "px";
  const cardH = card.offsetHeight || 190;
  if (!el) {
    spot.style.display = "none";
    card.style.left = Math.max(10, (vw - cardW) / 2) + "px";
    card.style.top = Math.max(10, (vh - cardH) / 2) + "px";
    return;
  }
  const r = el.getBoundingClientRect();
  const pad = 6;
  spot.style.display = "block";
  spot.style.left = (r.left - pad) + "px";
  spot.style.top = (r.top - pad) + "px";
  spot.style.width = (r.width + pad * 2) + "px";
  spot.style.height = (r.height + pad * 2) + "px";
  let left = Math.min(Math.max(10, r.left), vw - cardW - 10);
  let top = r.bottom + pad + 14;
  if (top + cardH > vh - 10) top = r.top - pad - 14 - cardH;
  if (top < 10) top = Math.min(Math.max(10, r.bottom + pad + 14), Math.max(10, vh - cardH - 10));
  card.style.left = left + "px";
  card.style.top = top + "px";
}

function renderCardContent() {
  if (!current) return;
  const step = current.tour.steps[current.index];
  const n = current.tour.steps.length;
  current.card.querySelector(".tour-card-kind").textContent = current.tour.label + " tour";
  current.card.querySelector(".tour-card-title").textContent = step.title;
  current.card.querySelector(".tour-card-body").textContent = step.body;
  current.card.querySelector(".tour-step-count").textContent = "Step " + (current.index + 1) + " of " + n;
  const backBtn = current.card.querySelector('[data-tour-btn="back"]');
  const nextBtn = current.card.querySelector('[data-tour-btn="next"]');
  if (backBtn) backBtn.disabled = current.index === 0;
  if (nextBtn) nextBtn.textContent = current.index === n - 1 ? "Finish" : "Next";
}

async function enterStep(index) {
  if (!current) return;
  current.index = index;
  current.found = false;
  renderCardContent();
  position();
  const step = current.tour.steps[index];
  if (step.action) {
    try { await runAction(step.action); } catch (e) { /* target poll below reports the outcome */ }
  }
  if (!current) return; // exited while the action ran
  const el = await pollTarget(step.target, 2500);
  if (!current) return;
  current.found = !!el;
  if (el) {
    try { el.scrollIntoView({ block: "nearest", inline: "nearest" }); } catch (e) { /* best effort */ }
    await sleep(60);
  }
  renderCardContent();
  position();
}

function buildOverlay() {
  const root = document.createElement("div");
  root.className = "tour-root";
  root.id = "reliakit-tour";
  root.innerHTML =
    '<div class="tour-backdrop"></div>' +
    '<div class="tour-spotlight"></div>' +
    '<div class="tour-card" role="dialog" aria-label="Guided tour">' +
    '<div class="tour-card-kind"></div>' +
    '<h3 class="tour-card-title"></h3>' +
    '<p class="tour-card-body"></p>' +
    '<div class="tour-card-foot"><span class="tour-step-count"></span>' +
    '<div class="tour-btns">' +
    '<button type="button" class="btn" data-tour-btn="back">Back</button>' +
    '<button type="button" class="btn btn-primary" data-tour-btn="next">Next</button>' +
    '<button type="button" class="btn" data-tour-btn="exit">Exit</button>' +
    "</div></div></div>";
  root.querySelector('[data-tour-btn="back"]').addEventListener("click", () => { void backStep(); });
  root.querySelector('[data-tour-btn="next"]').addEventListener("click", () => { void nextStep(); });
  root.querySelector('[data-tour-btn="exit"]').addEventListener("click", () => { exitTour(); });
  document.body.appendChild(root);
  return root;
}

function exitTour() {
  if (!current) return;
  const c = current;
  current = null;
  if (c.observer) { try { c.observer.disconnect(); } catch (e) { /* already gone */ } }
  if (c.timer) clearInterval(c.timer);
  if (c.onResize) window.removeEventListener("resize", c.onResize);
  if (c.root && c.root.isConnected) c.root.remove();
}

async function nextStep() {
  if (!current) return;
  if (current.index >= current.tour.steps.length - 1) { exitTour(); return; }
  await enterStep(current.index + 1);
}

async function backStep() {
  if (!current || current.index <= 0) return;
  await enterStep(current.index - 1);
}

async function renameCurrent(name) {
  const rn = document.querySelector("#rename-name");
  if (rn) rn.value = name;
  await api.dispatch("rename-item");
}

async function startTour(kind) {
  if (!api) return;
  const tour = TOURS[kind];
  if (!tour) return;
  if (current) exitTour();
  closeToursList();
  const meta = kindByKey(kind);
  const label = meta ? meta.label : tour.label;
  const tourName = "Tour: " + label;
  // Start sequence: always end with a NEW item named Tour: <Label>
  // selected. Examples and the warranty dataset load as new items;
  // the other kinds are added fresh from the panel fields.
  if (EXAMPLE_FOR_KIND[kind]) {
    const id = EXAMPLE_FOR_KIND[kind];
    await api.dispatch("card-action", { id, card_id: id, action_id: id });
    await renameCurrent(tourName);
  } else if (kind === "warranty") {
    await api.dispatch("card-action", { id: WARRANTY_DATASET, card_id: WARRANTY_DATASET, action_id: WARRANTY_DATASET });
    await renameCurrent(tourName);
  } else {
    const sel = document.querySelector("#kind-select");
    if (sel) sel.value = String(kindIndex(kind));
    const nn = document.querySelector("#new-name");
    if (nn) nn.value = tourName;
    await api.dispatch("add-item");
  }
  const root = buildOverlay();
  const app = document.querySelector("#app");
  const onResize = () => position();
  window.addEventListener("resize", onResize);
  let observer = null;
  if (app && typeof MutationObserver !== "undefined") {
    let queued = false;
    observer = new MutationObserver(() => {
      if (queued) return;
      queued = true;
      requestAnimationFrame(() => { queued = false; position(); });
    });
    observer.observe(app, { childList: true, subtree: true, attributes: true });
  }
  const timer = setInterval(() => { position(); }, 350);
  current = {
    kind, tour, index: 0, found: false,
    root, spot: root.querySelector(".tour-spotlight"), card: root.querySelector(".tour-card"),
    observer, timer, onResize,
  };
  await enterStep(0);
}

function startForCurrent() {
  if (!api || typeof api.getState !== "function") { openToursList(); return; }
  const st = api.getState();
  const item = st && st.selectedIndex >= 0 && Array.isArray(st.items) ? st.items[st.selectedIndex] : null;
  if (item && item.kind && TOURS[item.kind]) { void startTour(item.kind); return; }
  openToursList();
}

// ---------------- tours list overlay ----------------

function closeToursList() {
  if (listRoot && listRoot.isConnected) listRoot.remove();
  listRoot = null;
}

function openToursList() {
  if (listRoot) { closeToursList(); }
  const root = document.createElement("div");
  root.className = "tours-list-overlay";
  root.id = "reliakit-tours-list";
  const groups = FAMILIES.map((fam) => {
    const rows = KIND_META.filter((m) => m.family === fam).map((m) => {
      const t = TOURS[m.key];
      const count = t ? t.steps.length : 0;
      return '<div class="tours-list-row"><img src="assets/icons/' + esc(m.icon) + '.svg" alt="">' +
        '<span class="tours-list-main"><span class="tours-list-label">' + esc(m.label) + '</span>' +
        '<span class="tours-list-desc">' + esc(m.desc) + '</span></span>' +
        '<span class="tours-list-count">' + count + " steps</span>" +
        '<button type="button" class="btn btn-primary" data-tour-start="' + esc(m.key) + '">Start</button></div>';
    }).join("");
    return '<div class="tours-list-group"><h3 class="tours-list-family">' + esc(fam) + "</h3>" + rows + "</div>";
  }).join("");
  root.innerHTML =
    '<div class="tours-list-backdrop" data-tour-close="1"></div>' +
    '<div class="dialog dialog-wide tours-list-dialog" role="dialog" aria-modal="true" aria-label="Guided tours">' +
    '<div class="tours-list-head"><h2>Guided tours</h2><button type="button" class="btn" data-tour-close="1">Close</button></div>' +
    '<p class="caption">Each tour sets up its own new item and walks through one analysis, step by step. Your existing items are not changed.</p>' +
    groups +
    "</div>";
  root.addEventListener("click", (ev) => {
    const startBtn = ev.target.closest && ev.target.closest("[data-tour-start]");
    if (startBtn) {
      const kind = startBtn.getAttribute("data-tour-start");
      closeToursList();
      void startTour(kind);
      return;
    }
    const closer = ev.target.closest && ev.target.closest("[data-tour-close]");
    if (closer) closeToursList();
  });
  document.body.appendChild(root);
  listRoot = root;
}

export function initTours(apiArg) {
  api = apiArg;
  const handle = {
    start: (kind) => startTour(kind),
    next: () => nextStep(),
    back: () => backStep(),
    exit: () => exitTour(),
    active: () => !!current,
    kind: () => (current ? current.kind : null),
    stepIndex: () => (current ? current.index : -1),
    stepCount: () => (current ? current.tour.steps.length : 0),
    targetFound: () => (current ? !!current.found : false),
  };
  window.__reliakitTour = handle;
  return {
    start: startTour,
    startForCurrent,
    openList: openToursList,
    closeList: closeToursList,
    exit: exitTour,
    TOURS,
  };
}
