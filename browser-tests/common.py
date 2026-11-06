"""Shared helpers for the ReliaKit browser test suite.

The suite runs against the locally served browser demo build in
site/ (build it with scripts/build-demo.sh). Helpers prefer stable
hooks: data attributes, roles, and visible text. Ports are picked
per test file so files can run in any order.
"""
import functools
import http.server
import json
import socketserver
import threading
from pathlib import Path

from playwright.sync_api import sync_playwright

REPO_ROOT = Path(__file__).resolve().parent.parent
SITE_DIR = REPO_ROOT / "site"


class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, fmt, *args):
        pass


class ReuseTCPServer(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True


def start_server(port):
    handler = functools.partial(QuietHandler, directory=str(SITE_DIR))
    httpd = ReuseTCPServer(("127.0.0.1", port), handler)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    return httpd


def launch_browser(p):
    exe = Path.home() / ".cache" / "ms-playwright" / "chromium-1148" / "chrome-linux" / "chrome"
    kwargs = {"headless": True}
    if exe.exists():
        kwargs["executable_path"] = str(exe)
    return p.chromium.launch(**kwargs)


class Checker:
    def __init__(self):
        self.failures = []

    def check(self, label, ok, detail=""):
        print(f"{'PASS' if ok else 'FAIL'}: {label}" + (f" - {detail}" if detail else ""), flush=True)
        if not ok:
            self.failures.append(label)

    def verdict(self, summary):
        if self.failures:
            print(f"OVERALL: FAIL ({len(self.failures)} failing checks: {self.failures})", flush=True)
            return 1
        print(f"OVERALL: PASS - {summary}", flush=True)
        return 0


def demo_url(port):
    return f"http://127.0.0.1:{port}/demo/"


def dismiss_recovery(page):
    """Dismiss the unsaved-project recovery prompt if it is up."""
    btn = page.locator('[data-dialog="dismiss-recovery"]')
    if btn.count() > 0:
        try:
            btn.first.click(timeout=3000)
            page.wait_for_timeout(400)
        except Exception:
            pass


def goto_demo(page, port):
    page.goto(demo_url(port), wait_until="load")
    page.wait_for_selector('[data-card="example:1"]', timeout=30000)
    page.wait_for_function("!!window.__reliakitTour", timeout=30000)
    page.wait_for_timeout(2000)
    dismiss_recovery(page)


def click_example(page, n):
    page.click(f'[data-card="example:{n}"]')
    page.wait_for_timeout(1200)


def headline_value(page):
    return page.evaluate(
        "(() => { const el = document.querySelector('.headline-card .hc-value'); "
        "return el ? el.textContent.trim() : ''; })()"
    )


def status_text(page):
    return page.evaluate(
        "(() => { const el = document.querySelector('#status-text'); "
        "return el ? el.textContent.trim() : ''; })()"
    )


def field_value(page, name):
    return page.evaluate(
        "(n) => { const el = document.querySelector('[data-field=\"' + n + '\"]'); "
        "return el ? el.value : null; }",
        name,
    )


def result_pairs(page):
    """Result label/value pairs from the summary grid, merged with
    the Details table (the full row set lives there; the grid is
    the compact view)."""
    return page.evaluate(
        "(() => { const out = {};"
        " [...document.querySelectorAll('.summary-grid .result-card')].forEach(c => {"
        "   const l = c.querySelector('.rc-label'); const v = c.querySelector('.rc-value');"
        "   if (l) out[l.textContent.trim()] = v ? v.textContent.trim() : ''; });"
        " [...document.querySelectorAll('.details-rows table tr')].forEach(tr => {"
        "   const th = tr.querySelector('th'); const td = tr.querySelector('td');"
        "   if (th && td) out[th.textContent.trim()] = td.textContent.trim(); });"
        " return Object.entries(out); })()"
    )


def block_info(page, name=None):
    return page.evaluate(
        """(name) => {
          const blocks = [...document.querySelectorAll('[data-rbd-block]')];
          const el = name ? blocks.find(b => {
            const n = b.querySelector('.rb-name');
            return n && n.textContent.trim() === name;
          }) : blocks[0];
          if (!el) return null;
          const r = el.getBoundingClientRect();
          return {
            idx: el.getAttribute('data-rbd-block'),
            left: el.offsetLeft, top: el.offsetTop,
            selected: el.classList.contains('selected'),
            rect: { x: r.x, y: r.y, width: r.width, height: r.height,
                    right: r.right, bottom: r.bottom },
          };
        }""",
        name,
    )


def save_project(page, dest):
    """Click Save and capture the reliakit-project.json download."""
    downloads = []
    handler = lambda d: downloads.append(d)
    page.on("download", handler)
    try:
        page.locator('[data-shell="save-project"]').first.click()
        page.wait_for_timeout(2500)
    finally:
        page.remove_listener("download", handler)
    proj = [d for d in downloads
            if "project" in d.suggested_filename and d.suggested_filename.endswith(".json")]
    if not proj:
        return None
    proj[0].save_as(str(dest))
    return json.loads(Path(dest).read_text())


def open_project_file(page, path):
    """Open a saved project through the demo file picker."""
    dismiss_recovery(page)
    with page.expect_file_chooser(timeout=15000) as fc_info:
        page.locator('[data-shell="open-project"]').first.click()
    fc_info.value.set_files(str(path))
    page.wait_for_timeout(1500)
    dismiss_recovery(page)


def find_spec_node(spec, name):
    if isinstance(spec, dict):
        if spec.get("name") == name:
            return spec
        for child in spec.get("children") or []:
            found = find_spec_node(child, name)
            if found is not None:
                return found
    return None


def life_state_of(project, kind_part="life"):
    """State of the Life Data item that carries the work: kinds in
    the project file are PascalCase (LifeData), and a project can
    hold more than one Life Data item, so prefer the one with a
    stored fit, then the one with the longest times text."""
    candidates = [
        item.get("state") or {}
        for item in project.get("items", [])
        if kind_part in str(item.get("kind", "")).lower()
    ]
    if not candidates:
        return {}
    for state in candidates:
        if state.get("life_fit"):
            return state
    return max(candidates, key=lambda s: len(str(s.get("life_times", ""))))
