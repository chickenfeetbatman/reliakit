// RBD screen: palette, canvas, and block inspector.
// Mirrors gui/ui/screens_c.slint (RbdScreen). Palette choices arm a
// placement mode, a canvas click places the block, pointer drag moves
// a block, and a click selects it for the right hand inspector.
// All geometry changes go through dispatch actions so the engine stays
// the single source of truth for the diagram.
//
// The canvas has a view transform (pan and zoom) kept in this module:
// blocks live in world coordinates inside .rbd-world, and the world
// element carries the transform. Pointer positions are converted
// through the transform before they become world coordinates, so
// dragging and placement behave exactly as they did in v0.3.1, in
// world units, at any zoom.
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
  return '<div class="summary-grid">' + list.map((r) => {
    const label = Array.isArray(r) ? r[0] : (r.label || "");
    const value = Array.isArray(r) ? r[1] : (r.value || "");
    return '<div class="result-card summary-cell">' + (label ? '<div class="rc-label">' + E(label) + "</div>" : "") + '<div class="rc-value">' + E(value) + "</div></div>";
  }).join("") + "</div>";
}

// ---------------- view transform ----------------
const view = { x: 24, y: 24, zoom: 1 };
let lastFitSignature = "";
let suppressPlaceUntil = 0;

function viewTransform() {
  return "translate(" + view.x + "px, " + view.y + "px) scale(" + view.zoom + ")";
}
export function rbdViewSignature(rbd) {
  const blocks = (rbd && rbd.blocks) || [];
  return blocks.map((b) => String(b.idx)).sort().join(",") + "|f" + (((rbd && rbd.frames) || []).length);
}
function diagramBounds(rbd) {
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  const eat = (x, y, w, h) => {
    if (!Number.isFinite(x) || !Number.isFinite(y)) return;
    minX = Math.min(minX, x); minY = Math.min(minY, y);
    maxX = Math.max(maxX, x + (Number(w) || 0)); maxY = Math.max(maxY, y + (Number(h) || 0));
  };
  ((rbd && rbd.blocks) || []).forEach((b) => eat(Number(b.x || 0), Number(b.y || 0), 128, 56));
  ((rbd && rbd.frames) || []).forEach((f) => eat(Number(f.x || 0), Number(f.y || 0), Number(f.w || 0), Number(f.h || 0)));
  ((rbd && rbd.ghosts) || []).forEach((g) => eat(Number(g.x || 0), Number(g.y || 0), Number(g.w || 128), Number(g.h || 56)));
  if (!Number.isFinite(minX)) return null;
  return { minX, minY, maxX, maxY, w: Math.max(maxX - minX, 1), h: Math.max(maxY - minY, 1) };
}
function fitView(canvas, rbd) {
  const b = diagramBounds(rbd);
  if (!canvas || !b) return;
  const cw = canvas.clientWidth || 800; const ch = canvas.clientHeight || 430;
  const zoom = Math.max(0.2, Math.min(1.6, Math.min((cw - 32) / b.w, (ch - 32) / b.h)));
  view.zoom = zoom;
  view.x = (cw - b.w * zoom) / 2 - b.minX * zoom;
  view.y = (ch - b.h * zoom) / 2 - b.minY * zoom;
  applyView(canvas);
}
function applyView(canvas) {
  const world = canvas ? canvas.querySelector(".rbd-world") : null;
  if (world) world.style.transform = viewTransform();
}
function toWorld(canvas, clientX, clientY) {
  const rect = canvas.getBoundingClientRect();
  return {
    x: (clientX - rect.left - view.x) / view.zoom,
    y: (clientY - rect.top - view.y) / view.zoom,
  };
}
function zoomAt(canvas, clientX, clientY, factor) {
  const rect = canvas.getBoundingClientRect();
  const cx = clientX - rect.left; const cy = clientY - rect.top;
  const wx = (cx - view.x) / view.zoom; const wy = (cy - view.y) / view.zoom;
  view.zoom = Math.max(0.2, Math.min(2.5, view.zoom * factor));
  view.x = cx - wx * view.zoom;
  view.y = cy - wy * view.zoom;
  applyView(canvas);
}

// Canvas HTML: connector segments from state drawn as one SVG layer,
// blocks as positioned elements, all inside the transformed world.
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
  const world = '<div class="rbd-world" style="transform:' + viewTransform() + '">' + framesHtml + ghostsHtml +
    '<svg class="wires" aria-hidden="true" width="4000" height="3000" style="width:4000px;height:3000px;overflow:visible">' + segs + "</svg>" + blockHtml + "</div>";
  return '<div class="rbd-canvas" id="rbd-canvas">' + world + empty + "</div>";
}

function rbdToolbarHtml() {
  return '<div class="rbd-toolbar" role="toolbar" aria-label="Diagram view">' +
    '<button type="button" class="btn small" data-rbd-view="fit">Fit to view</button>' +
    '<button type="button" class="btn small" data-rbd-view="zoom-in" aria-label="Zoom in">+</button>' +
    '<button type="button" class="btn small" data-rbd-view="zoom-out" aria-label="Zoom out">-</button>' +
    '<span class="rbd-zoom-label" data-rbd-zoom-label>' + Math.round(view.zoom * 100) + '%</span>' +
    '<button type="button" class="btn small" data-rbd-view="reset">Reset view</button>' +
    '<button type="button" class="btn small" data-rbd-view="auto-layout">Auto-layout</button>' +
    '<span class="caption">Drag empty canvas to pan. Ctrl+scroll zooms.</span></div>';
}

export function renderRbd(state) {
  const s = state.rbd;
  const outdated = !!state.outdated;
  let left = '<p class="caption screen-caption">Blocks evaluate with the system reliability engine, including standby groups.</p>';
  left += '<h3 class="section-title">Palette</h3><details class="help-disclosure"><summary>Help</summary><p class="caption">Pick a palette item, then click the canvas to place it. Click a block to select and edit it. Drag blocks to move them, or focus a block and use the arrow keys (Shift for 10px steps).</p></details>';
  left += '<div class="btn-row">' +
    '<button type="button" class="btn' + (s.armed === "block" ? " active" : "") + '" data-rbd-arm="block">Single block</button>' +
    '<button type="button" class="btn' + (s.armed === "standby_group" ? " active" : "") + '" data-rbd-arm="standby_group">Standby group</button></div>';
  left += '<div class="btn-row">' +
    '<button type="button" class="btn' + (s.armed === "parallel" ? " active" : "") + '" data-rbd-arm="parallel">Parallel group</button>' +
    '<button type="button" class="btn' + (s.armed === "kofn" ? " active" : "") + '" data-rbd-arm="kofn">k-out-of-n group</button></div>';
  if (s.armed) left += '<p class="caption">Placement armed. Click the canvas to place.</p>';
  const relCard = '<div class="headline-card' + (outdated ? " outdated" : "") + '"><div class="hc-label">' + E(s.relLabel || "Reliability") + (outdated ? ' <span class="badge badge-outdated">Outdated</span>' : "") + '</div><div class="hc-value">' + (outdated ? "Outdated: " : "") + E(s.relValue || "") + "</div></div>";
  left += '<h3 class="section-title">Evaluation</h3>' +
    '<div class="field-row">' + field("rbd.t", "Mission t", s.t) + selectField("rbd.topology_index", "Topology", ["series", "parallel", "k-of-n", "series with standby group"], s.topologyIndex) + field("rbd.k", "k", s.k) + "</div>" +
    btn("rbd-topology", "Apply topology") +
    relCard +
    '<div class="field-row">' + field("rbd.horizon", "Availability horizon", s.horizon) + field("rbd.sims", "Simulations", s.sims != null && s.sims !== "" ? s.sims : "40") + "</div>" +
    btn("rbd-avail", "Run availability", { primary: true, busyKey: "rbd-avail" }) +
    (outdated && (s.availResults || []).length ? '<p class="caption"><span class="badge badge-outdated">Outdated</span> Inputs changed after this availability run.</p>' : "") +
    resultCards(s.availResults) +
    '<h3 class="section-title">Reliability importance</h3>' + btn("rbd-importance", "Compute importance measures", { busyKey: "rbd-importance" }) +
    (s.importanceNote ? '<p class="caption">' + E(s.importanceNote) + "</p>" : "") +
    (s.importanceRows || []).map((r) => {
      const c = Array.isArray(r) ? r : (r.cells || []);
      return '<div><div style="font-size:13px;font-weight:600">' + E(c[0] || "") + '</div><div class="caption">Birnbaum ' + E(c[1] || "") + " · F-V " + E(c[2] || "") + " · Crit " + E(c[3] || "") + "</div></div>";
    }).join("");
  const center = rbdToolbarHtml() + rbdCanvasHtml(s) + (s.status ? '<p class="caption">' + E(s.status) + "</p>" : "");
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
    '<h3 class="section-title">Diagram JSON</h3><details class="help-disclosure"><summary>Help</summary><p class="caption">Same schema as the library RBD format. Leaf nodes may carry an additive pos field with canvas coordinates.</p></details>' +
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
  // View toolbar (rendered just above the canvas in the center panel).
  const panel = canvas.parentElement;
  const syncZoomLabel = () => {
    const lbl = panel ? panel.querySelector("[data-rbd-zoom-label]") : null;
    if (lbl) lbl.textContent = Math.round(view.zoom * 100) + "%";
  };
  if (panel) panel.querySelectorAll("[data-rbd-view]").forEach((b) => {
    b.addEventListener("click", async () => {
      const act = b.getAttribute("data-rbd-view");
      const rect = canvas.getBoundingClientRect();
      if (act === "fit") {
        const sigData = readDomDiagram(canvas);
        fitView(canvas, sigData); syncZoomLabel();
      } else if (act === "zoom-in") { zoomAt(canvas, rect.left + rect.width / 2, rect.top + rect.height / 2, 1.25); syncZoomLabel(); }
      else if (act === "zoom-out") { zoomAt(canvas, rect.left + rect.width / 2, rect.top + rect.height / 2, 0.8); syncZoomLabel(); }
      else if (act === "reset") { view.x = 24; view.y = 24; view.zoom = 1; applyView(canvas); syncZoomLabel(); }
      else if (act === "auto-layout") {
        if (window.confirm("Auto-layout rebuilds the diagram from the topology and clears manual block positions. Continue?")) {
          lastFitSignature = ""; // refit after the layout dispatch
          await dispatch("rbd-topology", { act: "rbd-topology" });
        }
      }
    });
  });
  function readDomDiagram(cv) {
    const blocks = Array.from(cv.querySelectorAll("[data-rbd-block]")).map((el) => ({
      idx: el.getAttribute("data-rbd-block"), x: el.offsetLeft, y: el.offsetTop,
    }));
    const frames = Array.from(cv.querySelectorAll(".rbd-frame")).map((el) => ({
      x: el.offsetLeft, y: el.offsetTop, w: el.offsetWidth, h: el.offsetHeight,
    }));
    const ghosts = Array.from(cv.querySelectorAll(".rbd-ghost")).map((el) => ({
      x: el.offsetLeft, y: el.offsetTop, w: el.offsetWidth, h: el.offsetHeight,
    }));
    return { blocks, frames, ghosts };
  }
  // Automatic fit when a new diagram (new set of blocks) appears, so
  // no block is clipped at load. Position-only changes keep the view.
  const domDiagram = readDomDiagram(canvas);
  const sig = domDiagram.blocks.map((b) => String(b.idx)).sort().join(",") + "|f" + domDiagram.frames.length;
  if (domDiagram.blocks.length && sig !== lastFitSignature) {
    lastFitSignature = sig;
    fitView(canvas, domDiagram);
    syncZoomLabel();
  }
  canvas.addEventListener("click", (ev) => {
    if (ev.target.closest("[data-rbd-block]")) return;
    if (Date.now() < suppressPlaceUntil) return; // the click after a pan
    const p = toWorld(canvas, ev.clientX, ev.clientY);
    dispatch("rbd-place", { x: String(p.x), y: String(p.y), rbd_x: String(p.x), rbd_y: String(p.y) });
  });
  canvas.addEventListener("wheel", (ev) => {
    if (!ev.ctrlKey) return;
    ev.preventDefault();
    zoomAt(canvas, ev.clientX, ev.clientY, ev.deltaY < 0 ? 1.12 : 0.89);
    syncZoomLabel();
  }, { passive: false });
  // Keyboard: Enter and Space select the focused block; arrow keys
  // move the selected block (1px, Shift for 10px) through rbd-drag,
  // persisted exactly like a mouse drag.
  canvas.addEventListener("keydown", async (ev) => {
    const el = ev.target.closest ? ev.target.closest("[data-rbd-block]") : null;
    if (!el) return;
    const idx = Number(el.getAttribute("data-rbd-block"));
    if (!Number.isFinite(idx)) return;
    if (ev.key === "Enter" || ev.key === " ") {
      ev.preventDefault();
      await dispatch("rbd-press", { index: String(idx), block_index: String(idx) });
      return;
    }
    const step = ev.shiftKey ? 10 : 1;
    let dx = 0, dy = 0;
    if (ev.key === "ArrowLeft") dx = -step;
    else if (ev.key === "ArrowRight") dx = step;
    else if (ev.key === "ArrowUp") dy = -step;
    else if (ev.key === "ArrowDown") dy = step;
    else return;
    ev.preventDefault();
    const x = Math.max(0, el.offsetLeft + dx);
    const y = Math.max(0, el.offsetTop + dy);
    el.style.left = x + "px";
    el.style.top = y + "px";
    await dispatch("rbd-drag", { index: String(idx), block_index: String(idx), x: String(x), y: String(y) });
    await dispatch("rbd-press", { index: String(idx), block_index: String(idx) });
  });
  let drag = null;
  let pan = null;
  canvas.addEventListener("pointerdown", (ev) => {
    const el = ev.target.closest("[data-rbd-block]");
    if (!el) {
      // Empty canvas: candidate pan. A plain click still places.
      pan = { pointerId: ev.pointerId, startX: ev.clientX, startY: ev.clientY, viewX: view.x, viewY: view.y, moved: false };
      return;
    }
    const idx = Number(el.getAttribute("data-rbd-block"));
    if (!Number.isFinite(idx)) return;
    const p = toWorld(canvas, ev.clientX, ev.clientY);
    drag = {
      idx,
      el,
      pointerId: ev.pointerId,
      startX: ev.clientX,
      startY: ev.clientY,
      grabX: p.x - el.offsetLeft,
      grabY: p.y - el.offsetTop,
      x: el.offsetLeft,
      y: el.offsetTop,
      moved: false,
    };
    try { el.setPointerCapture(ev.pointerId); } catch (err) { /* capture is best effort; the move handlers still track the drag */ }
    ev.preventDefault();
  });
  canvas.addEventListener("pointermove", (ev) => {
    if (pan && ev.pointerId === pan.pointerId) {
      const dx = ev.clientX - pan.startX; const dy = ev.clientY - pan.startY;
      if (!pan.moved && Math.hypot(dx, dy) <= 4) return;
      pan.moved = true;
      view.x = pan.viewX + dx;
      view.y = pan.viewY + dy;
      applyView(canvas);
      return;
    }
    if (!drag || ev.pointerId !== drag.pointerId) return;
    if (!drag.moved) {
      if (Math.hypot(ev.clientX - drag.startX, ev.clientY - drag.startY) <= 4) return;
      drag.moved = true;
      drag.el.classList.add("dragging");
    }
    const p = toWorld(canvas, ev.clientX, ev.clientY);
    const x = p.x - drag.grabX;
    const y = p.y - drag.grabY;
    if (!Number.isFinite(x) || !Number.isFinite(y)) return;
    drag.x = Math.max(0, x);
    drag.y = Math.max(0, y);
    drag.el.style.left = drag.x + "px";
    drag.el.style.top = drag.y + "px";
  });
  const finish = async (ev) => {
    if (pan && (!ev || ev.pointerId === undefined || ev.pointerId === pan.pointerId)) {
      if (pan.moved) suppressPlaceUntil = Date.now() + 250;
      pan = null;
    }
    if (!drag || (ev && ev.pointerId !== undefined && ev.pointerId !== drag.pointerId)) return;
    const d = drag; drag = null;
    d.el.classList.remove("dragging");
    try {
      if (ev && d.el.hasPointerCapture && d.el.hasPointerCapture(ev.pointerId)) d.el.releasePointerCapture(ev.pointerId);
    } catch (err) { /* capture may already be released */ }
    if (!d.moved) {
      await dispatch("rbd-press", { index: String(d.idx), block_index: String(d.idx) });
      return;
    }
    if (!Number.isFinite(d.x) || !Number.isFinite(d.y)) {
      await dispatch("rbd-press", { index: String(d.idx), block_index: String(d.idx) });
      return;
    }
    const x = Math.max(0, d.x);
    const y = Math.max(0, d.y);
    await dispatch("rbd-drag", { index: String(d.idx), block_index: String(d.idx), x: String(x), y: String(y) });
    await dispatch("rbd-press", { index: String(d.idx), block_index: String(d.idx) });
  };
  canvas.addEventListener("pointerup", finish);
  canvas.addEventListener("pointercancel", finish);
}
