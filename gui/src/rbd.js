// RBD screen: palette, canvas, and block inspector.
// Mirrors gui/ui/screens_c.slint (RbdScreen). Palette choices arm a
// placement mode, a canvas click places the block, pointer drag moves
// a block, and a click selects it for the right hand inspector.
// All geometry changes go through dispatch actions so the engine stays
// the single source of truth for the diagram.
import { escapeHtml } from "./charts.js";

const E = escapeHtml;
const DISTS = ["weibull2", "weibull3", "lognormal", "normal", "exponential", "gamma", "gumbel", "loglogistic", "logistic", "gompertz", "gengamma", "weibull1", "exponential2", "mixed_weibull"];

function field(name, label, value) {
  return '<div class="field"><label for="f-' + E(name) + '">' + E(label) + '</label><input type="text" id="f-' + E(name) + '" data-field="' + E(name) + '" value="' + E(value == null ? "" : value) + '"></div>';
}
function selectField(name, label, options, index) {
  const opts = (options || []).map((o, i) => '<option value="' + i + '"' + (i === Number(index || 0) ? " selected" : "") + ">" + E(o) + "</option>").join("");
  return '<div class="field"><label for="f-' + E(name) + '">' + E(label) + '</label><select id="f-' + E(name) + '" data-field="' + E(name) + '" data-field-kind="index">' + opts + "</select></div>";
}
function btn(act, text, opts) {
  const o = opts || {};
  return '<button type="button" class="btn' + (o.primary ? " btn-primary" : "") + (o.active ? " active" : "") + '" data-act="' + E(act) + '"' + (o.busyKey ? ' data-busy-key="' + E(o.busyKey) + '"' : "") + ">" + E(text) + "</button>";
}
function resultCards(rows) {
  const list = Array.isArray(rows) ? rows : [];
  if (!list.length) return '<p class="faint-note">No results yet. Run the analysis to see results here.</p>';
  return list.map((r) => {
    const label = Array.isArray(r) ? r[0] : (r.label || "");
    const value = Array.isArray(r) ? r[1] : (r.value || "");
    return '<div class="result-card">' + (label ? '<div class="rc-label">' + E(label) + "</div>" : "") + '<div class="rc-value">' + E(value) + "</div></div>";
  }).join("");
}

// Canvas HTML: connector segments from state drawn as one SVG layer,
// blocks as positioned elements. Segment values are CSS pixels in the
// Slint model ({ x, y, w, h } bars), so each bar becomes one rect.
export function rbdCanvasHtml(rbd) {
  const blocks = rbd.blocks || [];
  const segs = (rbd.segments || []).map((s) => {
    const x = Number(s.x || 0), y = Number(s.y || 0), w = Number(s.w != null ? s.w : (s.width || 0)), h = Number(s.h != null ? s.h : (s.height || 0));
    return '<rect x="' + x + '" y="' + y + '" width="' + Math.max(w, 0) + '" height="' + Math.max(h, 0) + '" rx="1" fill="#64D2FF"/>';
  }).join("");
  const ghostsHtml = (rbd.ghosts || []).map((g) => {
    const x = Number(g.x || 0), y = Number(g.y || 0), w = Number(g.w != null ? g.w : 128), h = Number(g.h != null ? g.h : 56);
    return '<div class="rbd-ghost" style="left:' + x + "px;top:" + y + "px;width:" + w + "px;height:" + h + 'px"></div>';
  }).join("");
  const framesHtml = (rbd.frames || []).map((f) => {
    const x = Number(f.x || 0), y = Number(f.y || 0), w = Number(f.w != null ? f.w : 0), h = Number(f.h != null ? f.h : 0);
    return '<div class="rbd-frame" style="left:' + x + "px;top:" + y + "px;width:" + w + "px;height:" + h + 'px"><span class="rbd-frame-label">' + E(f.label || "") + "</span></div>";
  }).join("");
  const blockHtml = blocks.map((b) => {
    const idx = Number(b.idx != null ? b.idx : 0);
    return '<div class="rbd-block' + (b.selected ? " selected" : "") + '" data-rbd-block="' + idx + '" style="left:' + Number(b.x || 0) + "px;top:" + Number(b.y || 0) + 'px" role="button" tabindex="0" aria-label="Block ' + E(b.name || "") + '"><div class="rb-name">' + E(b.name || "") + '</div><div class="rb-sub">' + E(b.sub || "") + "</div></div>";
  }).join("");
  const empty = blocks.length === 0 ? '<div class="rbd-empty">No blocks yet. Arm a palette item and click here to place a block.</div>' : "";
  return '<div class="rbd-canvas" id="rbd-canvas">' + framesHtml + ghostsHtml + '<svg class="wires" aria-hidden="true">' + segs + "</svg>" + blockHtml + empty + "</div>";
}

export function renderRbd(state) {
  const s = state.rbd;
  let left = '<p class="caption screen-caption">Blocks evaluate with the system reliability engine, including standby groups.</p>';
  left += '<h3 class="section-title">Palette</h3><p class="caption">Pick a palette item, then click the canvas to place it. Click a block to select and edit it. Drag blocks to move them.</p>';
  left += '<div class="btn-row">' +
    '<button type="button" class="btn' + (s.armed === "block" ? " active" : "") + '" data-rbd-arm="block">Single block</button>' +
    '<button type="button" class="btn' + (s.armed === "standby_group" ? " active" : "") + '" data-rbd-arm="standby_group">Standby group</button></div>';
  left += '<div class="btn-row">' +
    '<button type="button" class="btn' + (s.armed === "parallel" ? " active" : "") + '" data-rbd-arm="parallel">Parallel group</button>' +
    '<button type="button" class="btn' + (s.armed === "kofn" ? " active" : "") + '" data-rbd-arm="kofn">k-out-of-n group</button></div>';
  if (s.armed) left += '<p class="caption">Placement armed. Click the canvas to place.</p>';
  left += '<h3 class="section-title">Evaluation</h3>' +
    '<div class="field-row">' + field("rbd.t", "Mission t", s.t) + selectField("rbd.topology_index", "Topology", ["series", "parallel", "k-of-n", "series with standby group"], s.topologyIndex) + field("rbd.k", "k", s.k) + "</div>" +
    btn("rbd-topology", "Apply topology") +
    '<div class="headline-card"><div class="hc-label">' + E(s.relLabel || "Reliability") + '</div><div class="hc-value">' + E(s.relValue || "") + "</div></div>" +
    field("rbd.horizon", "Availability horizon", s.horizon) +
    btn("rbd-avail", "Run availability (40 sims)", { primary: true, busyKey: "rbd-avail" }) +
    resultCards(s.availResults) +
    '<h3 class="section-title">Reliability importance</h3>' + btn("rbd-importance", "Compute importance measures", { busyKey: "rbd-importance" }) +
    (s.importanceNote ? '<p class="caption">' + E(s.importanceNote) + "</p>" : "") +
    (s.importanceRows || []).map((r) => {
      const c = Array.isArray(r) ? r : (r.cells || []);
      return '<div><div style="font-size:13px;font-weight:600">' + E(c[0] || "") + '</div><div class="caption">Birnbaum ' + E(c[1] || "") + " · F-V " + E(c[2] || "") + " · Crit " + E(c[3] || "") + "</div></div>";
    }).join("");
  const center = rbdCanvasHtml(s) + (s.status ? '<p class="caption">' + E(s.status) + "</p>" : "");
  let right = '<h3 class="section-title">Block inspector</h3>';
  if (s.hasSelection) {
    right += '<p class="caption">' + E(s.selTitle || "") + "</p>" +
      field("rbd.ed_name", "Name", s.edName) +
      selectField("rbd.ed_dist_index", "Distribution", DISTS, s.edDistIndex) +
      field("rbd.ed_params", "Params (beta=1.5, eta=5000)", s.edParams) +
      '<div class="field-row">' + field("rbd.ed_mtbf", "MTBF", s.edMtbf) + field("rbd.ed_mttr", "MTTR", s.edMttr) + "</div>";
    if (s.edIsStandby) {
      right += '<div class="field-row">' + field("rbd.ed_units", "Units", s.edUnits) + field("rbd.ed_dormancy", "Dormancy", s.edDormancy) + field("rbd.ed_switch", "Switch rel.", s.edSwitch) + "</div>";
    }
    right += btn("rbd-apply-props", "Apply properties", { primary: true, busyKey: "rbd-apply-props" }) +
      '<div class="btn-row">' + btn("rbd-duplicate", "Duplicate") + btn("rbd-delete", "Delete") + btn("rbd-undo", "Undo") + "</div>";
  } else {
    right += '<p class="caption">Click a placed block to select it. Its properties open here, in place, without leaving the canvas.</p>';
  }
  right += '<h3 class="section-title">Add block form</h3>' +
    field("rbd.nb_name", "Name", s.nbName) +
    selectField("rbd.nb_dist_index", "Distribution", DISTS, s.nbDistIndex) +
    field("rbd.nb_params", "Params", s.nbParams) +
    '<div class="field-row">' + field("rbd.nb_mtbf", "MTBF", s.nbMtbf) + field("rbd.nb_mttr", "MTTR", s.nbMttr) + "</div>" +
    selectField("rbd.nb_mode_index", "Mode", ["active", "hot standby", "cold standby", "warm"], s.nbModeIndex) +
    field("rbd.nb_dormancy", "Dormancy (warm)", s.nbDormancy) +
    btn("rbd-add-form", "Add block", { primary: true, busyKey: "rbd-add-form" }) +
    '<h3 class="section-title">Diagram JSON</h3><p class="caption">Same schema as the library RBD format. Leaf nodes may carry an additive pos field with canvas coordinates.</p>' +
    '<div class="field"><label for="f-rbd-json">Diagram JSON</label><textarea id="f-rbd-json" data-field="rbd.json" rows="6">' + E(s.json || "") + "</textarea></div>" +
    btn("rbd-load-json", "Load from JSON text");
  if (state.hostKind === "wasm") {
    right += '<p class="caption">In the browser demo, Save downloads the diagram JSON and Load opens a file picker. The file path field is a desktop only control.</p>' +
      field("rbd.path", "File path (desktop only)", s.path) +
      '<div class="btn-row"><button type="button" class="btn" data-file-import="rbd-save">Save</button><button type="button" class="btn" data-file-import="rbd-load">Load</button></div>';
  } else {
    right += field("rbd.path", "File path", s.path) +
      '<div class="btn-row">' + btn("rbd-save-file", "Save") + btn("rbd-load-file", "Load") + "</div>";
  }
  return '<div class="screen"><div class="panel-left">' + left + '</div><div class="panel-center">' + center + '</div><div class="panel-right">' + right + "</div></div>";
}

// Pointer wiring for the canvas. Called by app.js after each render.
// dispatch(action, fields) is the app level dispatcher.
//
// Drag handling stays local to this closure. app.js dispatch() ends
// with an unconditional full render() that rebuilds root.innerHTML,
// which destroys the element that received pointerdown and implicitly
// releases its pointer capture. So pointerdown and pointermove must
// never dispatch: the drag is tracked here, the block element is moved
// directly through its style, and only pointerup commits - one
// rbd-drag with the final position when the block actually moved, or
// one rbd-press when it did not (a plain click still selects).
export function bindRbdCanvas(root, dispatch) {
  const canvas = root.querySelector("#rbd-canvas");
  if (!canvas) return;
  canvas.addEventListener("click", (ev) => {
    if (ev.target.closest("[data-rbd-block]")) return;
    const rect = canvas.getBoundingClientRect();
    dispatch("rbd-place", { x: String(ev.clientX - rect.left), y: String(ev.clientY - rect.top), rbd_x: String(ev.clientX - rect.left), rbd_y: String(ev.clientY - rect.top) });
  });
  let drag = null;
  canvas.addEventListener("pointerdown", (ev) => {
    const el = ev.target.closest("[data-rbd-block]");
    if (!el) return;
    const idx = Number(el.getAttribute("data-rbd-block"));
    if (!Number.isFinite(idx)) return;
    const rect = canvas.getBoundingClientRect();
    drag = {
      idx,
      el,
      pointerId: ev.pointerId,
      startX: ev.clientX,
      startY: ev.clientY,
      grabX: ev.clientX - rect.left - el.offsetLeft,
      grabY: ev.clientY - rect.top - el.offsetTop,
      x: el.offsetLeft,
      y: el.offsetTop,
      moved: false,
    };
    try { el.setPointerCapture(ev.pointerId); } catch (err) { /* capture is best effort; the move handlers still track the drag */ }
    ev.preventDefault();
  });
  canvas.addEventListener("pointermove", (ev) => {
    if (!drag || ev.pointerId !== drag.pointerId) return;
    if (!drag.moved) {
      // A few pixels of travel separate a drag from a click, so plain
      // clicks keep selecting without moving the block.
      if (Math.hypot(ev.clientX - drag.startX, ev.clientY - drag.startY) <= 4) return;
      drag.moved = true;
      drag.el.classList.add("dragging");
    }
    const rect = canvas.getBoundingClientRect();
    const x = ev.clientX - rect.left - drag.grabX;
    const y = ev.clientY - rect.top - drag.grabY;
    if (!Number.isFinite(x) || !Number.isFinite(y)) return;
    drag.x = Math.max(0, x);
    drag.y = Math.max(0, y);
    // Move only the dragged element. No dispatch, no re-render.
    drag.el.style.left = drag.x + "px";
    drag.el.style.top = drag.y + "px";
  });
  const finish = async (ev) => {
    if (!drag || (ev && ev.pointerId !== undefined && ev.pointerId !== drag.pointerId)) return;
    const d = drag; drag = null;
    d.el.classList.remove("dragging");
    try {
      if (ev && d.el.hasPointerCapture && d.el.hasPointerCapture(ev.pointerId)) d.el.releasePointerCapture(ev.pointerId);
    } catch (err) { /* capture may already be released */ }
    if (!d.moved) {
      // Plain click: select the block and open its inspector.
      await dispatch("rbd-press", { index: String(d.idx), block_index: String(d.idx) });
      return;
    }
    if (!Number.isFinite(d.x) || !Number.isFinite(d.y)) {
      // Never send coordinates we cannot stand behind; just select.
      await dispatch("rbd-press", { index: String(d.idx), block_index: String(d.idx) });
      return;
    }
    const x = Math.max(0, d.x);
    const y = Math.max(0, d.y);
    // x and y are the block top-left (no grab fields are sent, and the
    // engine reads them that way). Then select so the block ends up
    // selected with the inspector showing it at its new position.
    await dispatch("rbd-drag", { index: String(d.idx), block_index: String(d.idx), x: String(x), y: String(y) });
    await dispatch("rbd-press", { index: String(d.idx), block_index: String(d.idx) });
  };
  canvas.addEventListener("pointerup", finish);
  canvas.addEventListener("pointercancel", finish);
}
