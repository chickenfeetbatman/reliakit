// Inline SVG charts rendered from series arrays in the engine state.
// A chart spec is { title, xLabel, yLabel, series: [{ label, kind, points }] }
// where each point is [x, y] or { x, y }. A series with kind "points"
// renders as markers only; kind "line" (the default) renders as a
// connected line. Reliability and probability charts clamp the y
// domain to [0, 1] exactly. Colors follow docs/DESIGN.md.
export const SERIES_COLORS = ["#0A84FF", "#64D2FF", "#BF5AF2", "#FF6482", "#30D158"];

export function escapeHtml(value) {
  return String(value == null ? "" : value)
    .replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;").replaceAll("'", "&#39;");
}

function normPoints(points) {
  if (!Array.isArray(points)) return [];
  const out = [];
  for (const p of points) {
    if (Array.isArray(p) && p.length >= 2) {
      const x = Number(p[0]); const y = Number(p[1]);
      if (Number.isFinite(x) && Number.isFinite(y)) out.push([x, y]);
    } else if (p && typeof p === "object") {
      const x = Number(p.x); const y = Number(p.y);
      if (Number.isFinite(x) && Number.isFinite(y)) out.push([x, y]);
    }
  }
  return out;
}

export function normalizeChart(raw) {
  if (!raw || typeof raw !== "object") return null;
  const seriesRaw = raw.series || raw.chart_series || raw.chartSeries || raw.lines || [];
  const series = (Array.isArray(seriesRaw) ? seriesRaw : []).map((s, i) => {
    if (Array.isArray(s)) return { label: "Series " + (i + 1), kind: "line", color: null, points: normPoints(s) };
    return {
      label: s.label || s.name || ("Series " + (i + 1)),
      kind: s.kind === "points" ? "points" : "line",
      color: s.color || null,
      points: normPoints(s.points || s.data || s.values || []),
    };
  }).filter((s) => s.points.length > 0);
  if (series.length === 0) return null;
  const title = raw.title || raw.chart_title || raw.chartTitle || "";
  const yLabel = raw.y_label || raw.yLabel || raw.y_desc || "Value";
  // Reliability-kind charts (reliability curves and probability
  // plots) live on [0, 1] by definition; clamp the domain exactly.
  const clamp01 = !!(raw.clamp_y_01 || raw.clampY01 || /reliab|probab/i.test(title + " " + yLabel));
  return {
    title,
    xLabel: raw.x_label || raw.xLabel || raw.x_desc || "Time",
    yLabel,
    clamp01,
    series,
  };
}

function fmt(v) {
  if (!Number.isFinite(v)) return "";
  const a = Math.abs(v);
  if (a !== 0 && (a >= 10000 || a < 0.01)) return v.toExponential(1);
  return String(Math.round(v * 1000) / 1000);
}

// Render a chart spec to an SVG string with axes, tick labels, and a
// legend when more than one series is present. The SVG is emitted at
// its natural 720x330 size and CSS keeps a minimum rendered width, so
// axis text (12px in viewBox units) never renders below about 10px.
export function chartSvg(spec, width, height) {
  const W = width || 720; const H = height || 330;
  if (!spec || !spec.series || spec.series.length === 0) return "";
  const m = { left: 62, right: 16, top: 14, bottom: 44 };
  const plotW = W - m.left - m.right; const plotH = H - m.top - m.bottom;
  let xmin = Infinity, xmax = -Infinity, ymin = Infinity, ymax = -Infinity;
  for (const s of spec.series) for (const [x, y] of s.points) {
    xmin = Math.min(xmin, x); xmax = Math.max(xmax, x);
    ymin = Math.min(ymin, y); ymax = Math.max(ymax, y);
  }
  if (!Number.isFinite(xmin)) { xmin = 0; xmax = 1; ymin = 0; ymax = 1; }
  if (xmin === xmax) { xmax = xmin + 1; }
  if (spec.clamp01) {
    ymin = 0; ymax = 1;
  } else {
    if (ymin === ymax) { ymax = ymin + 1; }
    const ypad = (ymax - ymin) * 0.06; ymin -= ypad; ymax += ypad;
  }
  const X = (x) => m.left + ((x - xmin) / (xmax - xmin)) * plotW;
  const Y = (y) => {
    const cy = spec.clamp01 ? Math.min(1, Math.max(0, y)) : y;
    return m.top + plotH - ((cy - ymin) / (ymax - ymin)) * plotH;
  };
  const parts = [];
  parts.push('<svg viewBox="0 0 ' + W + " " + H + '" width="' + W + '" height="' + H + '" role="img" aria-label="' + escapeHtml(spec.title || "Chart") + '">');
  for (let i = 0; i <= 4; i++) {
    const yv = ymin + ((ymax - ymin) * i) / 4; const yy = Y(yv);
    parts.push('<line x1="' + m.left + '" y1="' + yy + '" x2="' + (W - m.right) + '" y2="' + yy + '" stroke="rgba(255,255,255,0.10)" stroke-width="1" vector-effect="non-scaling-stroke"/>');
    parts.push('<text x="' + (m.left - 8) + '" y="' + (yy + 4) + '" fill="#A7A7AD" font-size="12" text-anchor="end">' + escapeHtml(fmt(yv)) + "</text>");
  }
  for (let i = 0; i <= 5; i++) {
    const xv = xmin + ((xmax - xmin) * i) / 5; const xx = X(xv);
    parts.push('<line x1="' + xx + '" y1="' + m.top + '" x2="' + xx + '" y2="' + (m.top + plotH) + '" stroke="rgba(255,255,255,0.06)" stroke-width="1" vector-effect="non-scaling-stroke"/>');
    parts.push('<text x="' + xx + '" y="' + (m.top + plotH + 19) + '" fill="#A7A7AD" font-size="12" text-anchor="middle">' + escapeHtml(fmt(xv)) + "</text>");
  }
  parts.push('<line x1="' + m.left + '" y1="' + (m.top + plotH) + '" x2="' + (W - m.right) + '" y2="' + (m.top + plotH) + '" stroke="#3A3C40" stroke-width="1"/>');
  parts.push('<line x1="' + m.left + '" y1="' + m.top + '" x2="' + m.left + '" y2="' + (m.top + plotH) + '" stroke="#3A3C40" stroke-width="1"/>');
  spec.series.forEach((s, si) => {
    const color = s.color || SERIES_COLORS[si % SERIES_COLORS.length];
    const pts = s.points.slice().sort((a, b) => a[0] - b[0]);
    if (s.kind === "points") {
      // Markers only, no connecting line (probability and raw data).
      for (const p of pts) {
        parts.push('<circle cx="' + X(p[0]).toFixed(1) + '" cy="' + Y(p[1]).toFixed(1) + '" r="3.6" fill="' + color + '" stroke="#17181a" stroke-width="0.8"/>');
      }
      return;
    }
    if (pts.length === 1) {
      parts.push('<circle cx="' + X(pts[0][0]) + '" cy="' + Y(pts[0][1]) + '" r="4" fill="' + color + '"/>');
    } else {
      const d = pts.map((p, i) => (i === 0 ? "M" : "L") + X(p[0]).toFixed(1) + " " + Y(p[1]).toFixed(1)).join(" ");
      parts.push('<path d="' + d + '" fill="none" stroke="' + color + '" stroke-width="2.2" stroke-linejoin="round" stroke-linecap="round"/>');
    }
  });
  parts.push('<text x="' + (m.left + plotW / 2) + '" y="' + (H - 4) + '" fill="#A7A7AD" font-size="12.5" text-anchor="middle">' + escapeHtml(spec.xLabel || "") + "</text>");
  parts.push('<text transform="rotate(-90 14 ' + (m.top + plotH / 2) + ')" x="14" y="' + (m.top + plotH / 2) + '" fill="#A7A7AD" font-size="12.5" text-anchor="middle">' + escapeHtml(spec.yLabel || "") + "</text>");
  if (spec.series.length > 1) {
    let lx = m.left + 8;
    spec.series.forEach((s, si) => {
      const color = s.color || SERIES_COLORS[si % SERIES_COLORS.length];
      const label = String(s.label || "");
      if (s.kind === "points") {
        parts.push('<circle cx="' + (lx + 5) + '" cy="' + (m.top + 12) + '" r="4" fill="' + color + '"/>');
      } else {
        parts.push('<line x1="' + lx + '" y1="' + (m.top + 12) + '" x2="' + (lx + 20) + '" y2="' + (m.top + 12) + '" stroke="' + color + '" stroke-width="3"/>');
      }
      parts.push('<text x="' + (lx + 25) + '" y="' + (m.top + 16) + '" fill="#F2F2F7" font-size="12">' + escapeHtml(label) + "</text>");
      lx += 30 + label.length * 7 + 18;
    });
  }
  parts.push("</svg>");
  return parts.join("");
}

export function chartCardHtml(spec, fallbackTitle) {
  if (!spec) return "";
  const title = spec.title || fallbackTitle || "Chart";
  return '<div class="chart-card"><div class="chart-title">' + escapeHtml(title) + "</div>" + chartSvg(spec) + "</div>";
}
