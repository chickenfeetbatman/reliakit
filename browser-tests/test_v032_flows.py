#!/usr/bin/env python3
"""ReliaKit v0.3.2 browser flows, against the locally served demo.

Covers:
  1. CSV import with fractional times: stored field, fit, and the
     project download round trip keep the values unchanged.
  2. Edit after fit: the outdated badge appears and the headline is
     demoted.
  3. Fit, send to RBD: the destination block takes the fitted
     distribution parameters, and the send reports a snapshot.
  4. Save and reopen: field-by-field comparison of the Life Data
     item.
  5. Keyboard-only: load Example 1 and run the fit by keyboard,
     assert a result appears.
  7. RBD at 1280x800: fit-to-view leaves every block inside the
     visible canvas; drag a block and verify the new position after
     a save, reload, and reopen (the v0.3.1 drag regression, at the
     new viewport size).

Test 6 (tours) lives in test_tours_v032.py.
Prints PASS/FAIL lines; exit code 0 only if everything passes.
"""
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from common import (
    Checker, block_info, click_example, dismiss_recovery, field_value,
    find_spec_node, goto_demo, headline_value, launch_browser, life_state_of,
    open_project_file, result_pairs, save_project, start_server, status_text,
)
from playwright.sync_api import sync_playwright

PORT = 8961
TMP = Path(tempfile.mkdtemp(prefix="reliakit-browser-"))
CSV_PATH = TMP / "fractional.csv"
CSV_TEXT = (
    "time,failed\n"
    "100.25,1\n200.5,1\n300.75,1\n400.125,1\n"
    "500.5,1\n650.25,1\n800.75,0\n1000.5,0\n"
)
FRACTIONAL_TOKENS = ["100.25", "200.5", "300.75", "400.125", "500.5", "650.25", "800.75", "1000.5"]


def open_life_example(page, port):
    goto_demo(page, port)
    click_example(page, 1)
    page.wait_for_selector('[data-field="life.times"]', timeout=20000)


def run_fit(page):
    page.locator('[data-act="life-fit"]').click()
    page.wait_for_timeout(1500)


def test_import_roundtrip(page, port, ck):
    print("--- Test 1: CSV fractional import, fit, export round trip ---")
    open_life_example(page, port)
    CSV_PATH.write_text(CSV_TEXT)
    with page.expect_file_chooser(timeout=15000) as fc_info:
        page.locator('[data-file-import="csv"]').click()
    fc_info.value.set_files(str(CSV_PATH))
    page.wait_for_timeout(1200)
    stored = field_value(page, "life.times") or ""
    print(f"Stored life.times after import: {stored!r}")
    for tok in FRACTIONAL_TOKENS:
        ck.check(f"import: stored field keeps {tok}", tok in stored, f"stored={stored!r}")
    ck.check("import: no whole-number truncation of 100.25",
             "100.25" in stored and "100," not in stored.replace("100.25", ""), f"stored={stored!r}")
    page.locator('[data-field="life.t"]').fill("300")
    run_fit(page)
    hl = headline_value(page)
    print(f"Headline after fit on imported data: {hl!r}")
    ck.check("import: fit produces a headline", bool(hl) and "R(300)" in hl, f"headline={hl!r}")
    pairs = dict(result_pairs(page))
    ck.check("import: fit results include reliability, B10, and mean life rows",
             "R(300)" in pairs and "B10" in pairs and "Mean life" in pairs,
             f"pairs={list(pairs)}")
    saved_path = TMP / "roundtrip-project.json"
    project = save_project(page, saved_path)
    ck.check("export: Save downloaded reliakit-project.json", project is not None)
    if project:
        state = life_state_of(project)
        saved_times = state.get("life_times", "")
        print(f"Saved project life_times: {saved_times!r}")
        ck.check("export: saved project times equal the stored field",
                 saved_times == stored, f"saved={saved_times!r} stored={stored!r}")
        page.reload(wait_until="load")
        page.wait_for_selector('[data-card="example:1"]', timeout=30000)
        page.wait_for_timeout(1500)
        open_project_file(page, saved_path)
        # The saved project reopens on the Life Data item that was
        # selected at save time; if the start page shows, select it.
        if page.locator('[data-field="life.times"]').count() == 0:
            page.locator("[data-select-item]").filter(has_text="Example").first.click()
            page.wait_for_timeout(800)
        reopened = field_value(page, "life.times") or ""
        print(f"Reopened life.times: {reopened!r}")
        ck.check("round trip: reopened values unchanged, field by field",
                 reopened == stored, f"reopened={reopened!r} stored={stored!r}")


def test_outdated_badge(page, port, ck):
    print("--- Test 2: edit after fit shows the outdated badge ---")
    open_life_example(page, port)
    run_fit(page)
    hl = headline_value(page)
    ck.check("outdated: fresh fit headline is R(1000) = 0.9172", "0.9172" in hl, f"headline={hl!r}")
    fresh_badges = page.locator(".headline-card .badge-outdated").count()
    ck.check("outdated: no outdated badge on a fresh fit", fresh_badges == 0)
    original = field_value(page, "life.times") or ""
    page.locator('[data-field="life.times"]').fill(original + ", 5000")
    # Typing alone does not dispatch; go to the start page (which
    # pulls the edited field into the engine) and come back.
    page.locator('[data-shell="go-start"]').first.click()
    page.wait_for_timeout(800)
    page.locator("[data-select-item]").filter(has_text="Example").first.click()
    page.wait_for_timeout(1000)
    card = page.evaluate(
        "(() => { const el = document.querySelector('.headline-card'); "
        "return el ? { cls: el.className, text: el.textContent.trim() } : null; })()")
    print(f"Headline card after edit: {card}")
    ck.check("outdated: headline card carries the outdated class or badge",
             bool(card) and ("outdated" in card["cls"] or "Outdated" in card["text"]),
             f"card={card}")
    value = headline_value(page)
    ck.check("outdated: headline value is demoted with the Outdated prefix",
             value.startswith("Outdated:"), f"value={value!r}")
    line = page.locator(".outdated-line").count()
    ck.check("outdated: outdated line above the results", line >= 1)
    side_badges = page.locator("[data-select-item] .badge-outdated").count()
    ck.check("outdated: sidebar item row shows the Outdated badge", side_badges >= 1)


def test_fit_to_rbd(page, port, ck):
    print("--- Test 3: fit, send to RBD, block params match the fit ---")
    goto_demo(page, port)
    click_example(page, 3)
    page.wait_for_selector("#rbd-canvas", timeout=20000)
    click_example_back_to_start(page)
    click_example(page, 1)
    page.wait_for_selector('[data-field="life.times"]', timeout=20000)
    run_fit(page)
    # The fit itself is read from the project snapshot: the Life
    # Data item's stored fit object is exactly what the send action
    # copies, so it is the right reference for the destination
    # block. (The current Life screen grid shows reliability rows,
    # not the parameter rows.)
    probe_path = TMP / "fit-probe-project.json"
    probe = save_project(page, probe_path)
    ck.check("send: project snapshot captured for the fit reference", probe is not None)
    fit_params = (life_state_of(probe).get("life_fit") or {}).get("params") or {} if probe else {}
    beta = float(fit_params.get("beta", "nan"))
    eta = float(fit_params.get("eta", "nan"))
    print(f"Fit params from the project snapshot: beta={beta} eta={eta}")
    ck.check("send: fit parameters readable", beta == beta and eta == eta, f"params={fit_params}")
    send_btn = page.locator('[data-act="flow-rbd"]')
    ck.check("send: Assign fit to RBD block button present after fit", send_btn.count() >= 1)
    send_btn.first.click()
    page.wait_for_timeout(1200)
    if page.locator('[data-dialog="send-anyway"]').count() > 0:
        ck.check("send: no outdated confirm expected on a fresh fit", False,
                 "the outdated dialog appeared for a fresh fit")
        page.locator('[data-dialog="cancel"]').click()
        page.wait_for_timeout(500)
        return
    st = status_text(page)
    transfer = page.evaluate(
        "(() => { const el = document.querySelector('.transfer-text'); "
        "return el ? el.textContent.trim() : ''; })()")
    print(f"Status after send: {st!r} transfer note: {transfer!r}")
    ck.check("send: status reports a copied snapshot (N3 wording)",
             "snapshot" in transfer.lower() and "will not update it" in transfer,
             f"transfer={transfer!r}")
    page.locator("[data-select-item]").filter(has_text="RBD").first.click()
    page.wait_for_selector("#rbd-canvas", timeout=20000)
    page.wait_for_timeout(800)
    info = block_info(page)
    ck.check("send: RBD has blocks", info is not None)
    page.mouse.click(info["rect"]["x"] + info["rect"]["width"] / 2,
                     info["rect"]["y"] + info["rect"]["height"] / 2)
    page.wait_for_timeout(900)
    params_text = field_value(page, "rbd.ed_params") or ""
    print(f"Destination block params in inspector: {params_text!r}")
    got = {}
    for part in params_text.replace(";", ",").split(","):
        if "=" in part:
            k, v = part.split("=", 1)
            try:
                got[k.strip()] = float(v.strip())
            except ValueError:
                pass
    ck.check("send: destination block beta matches the fit within 1 percent",
             "beta" in got and abs(got["beta"] - beta) <= 0.01 * max(1.0, abs(beta)),
             f"got={got} fit beta={beta}")
    ck.check("send: destination block eta matches the fit within 1 percent",
             "eta" in got and abs(got["eta"] - eta) <= 0.01 * max(1.0, abs(eta)),
             f"got={got} fit eta={eta}")


def click_example_back_to_start(page):
    page.locator('[data-shell="go-start"]').first.click()
    page.wait_for_selector('[data-card="example:1"]', timeout=20000)
    page.wait_for_timeout(600)


def test_save_reopen_fields(page, port, ck):
    print("--- Test 4: save and reopen, Life Data field by field ---")
    open_life_example(page, port)
    run_fit(page)
    before = {
        "life.times": field_value(page, "life.times"),
        "life.t": field_value(page, "life.t"),
        "life.dist_index": field_value(page, "life.dist_index"),
    }
    print(f"Life Data fields before save: {before}")
    saved_path = TMP / "fields-project.json"
    project = save_project(page, saved_path)
    ck.check("save: project downloaded", project is not None)
    if not project:
        return
    state = life_state_of(project)
    ck.check("save: project stores the same times text",
             state.get("life_times") == before["life.times"],
             f"stored={state.get('life_times')!r} field={before['life.times']!r}")
    try:
        same_t = float(state.get("life_t")) == float(before["life.t"])
    except (TypeError, ValueError):
        same_t = str(state.get("life_t")) == str(before["life.t"])
    ck.check("save: project stores the same mission t", same_t,
             f"stored={state.get('life_t')!r} field={before['life.t']!r}")
    page.reload(wait_until="load")
    page.wait_for_selector('[data-card="example:1"]', timeout=30000)
    page.wait_for_timeout(1500)
    open_project_file(page, saved_path)
    if page.locator('[data-field="life.times"]').count() == 0:
        page.locator("[data-select-item]").filter(has_text="Example").first.click()
        page.wait_for_timeout(800)
    after = {
        "life.times": field_value(page, "life.times"),
        "life.t": field_value(page, "life.t"),
        "life.dist_index": field_value(page, "life.dist_index"),
    }
    print(f"Life Data fields after reopen: {after}")
    for key in before:
        ck.check(f"reopen: field {key} unchanged", after[key] == before[key],
                 f"before={before[key]!r} after={after[key]!r}")


def tab_until(page, predicate_js, max_tabs=80):
    """Press Tab until the focused element matches predicate_js(el)."""
    for _ in range(max_tabs):
        page.keyboard.press("Tab")
        page.wait_for_timeout(60)
        ok = page.evaluate(
            "(pred) => { const el = document.activeElement; "
            "return el ? eval(pred)(el) : false; }",
            predicate_js,
        )
        if ok:
            return True
    return False


def test_keyboard_only(page, port, ck):
    print("--- Test 5: keyboard-only Example 1 load and fit ---")
    goto_demo(page, port)
    page.locator("body").click(position={"x": 5, "y": 5})
    found = tab_until(page, "(el) => el.getAttribute && el.getAttribute('data-card') === 'example:1'")
    ck.check("keyboard: Tab reaches the Example 1 card", found)
    if not found:
        return
    page.keyboard.press("Enter")
    page.wait_for_selector('[data-field="life.times"]', timeout=20000)
    page.wait_for_timeout(600)
    ck.check("keyboard: Example 1 Life Data screen opened by Enter", True)
    found_fit = tab_until(page, "(el) => el.getAttribute && el.getAttribute('data-act') === 'life-fit'")
    ck.check("keyboard: Tab reaches the Run fit button", found_fit)
    if not found_fit:
        return
    page.keyboard.press("Enter")
    page.wait_for_timeout(1800)
    hl = headline_value(page)
    print(f"Headline after keyboard fit: {hl!r}")
    ck.check("keyboard: a fit result appears", "0.9172" in hl, f"headline={hl!r}")
    pairs = result_pairs(page)
    ck.check("keyboard: result grid is populated", len(pairs) >= 3, f"rows={len(pairs)}")


def test_rbd_viewport_and_drag(page, port, ck):
    print("--- Test 7: RBD fit-to-view and drag at 1280x800 ---")
    page.set_viewport_size({"width": 1280, "height": 800})
    goto_demo(page, port)
    click_example(page, 3)
    page.wait_for_selector("#rbd-canvas", timeout=20000)
    page.wait_for_function(
        "document.querySelectorAll('[data-rbd-block]').length >= 4", timeout=20000)
    page.wait_for_timeout(800)
    fit_btn = page.locator('[data-rbd-view="fit"]')
    ck.check("viewport: Fit to view control exists", fit_btn.count() >= 1)
    if fit_btn.count() >= 1:
        fit_btn.first.click()
        page.wait_for_timeout(700)
    bounds = page.evaluate(
        """() => {
          const canvas = document.querySelector('#rbd-canvas');
          const cr = canvas.getBoundingClientRect();
          const blocks = [...document.querySelectorAll('[data-rbd-block]')].map(b => {
            const r = b.getBoundingClientRect();
            return { name: (b.querySelector('.rb-name') || {}).textContent,
                     x: r.x, y: r.y, right: r.right, bottom: r.bottom };
          });
          return { canvas: { x: cr.x, y: cr.y, right: cr.right, bottom: cr.bottom }, blocks };
        }""")
    print(f"Canvas bounds and blocks: {bounds}")
    inside = all(
        b["x"] >= bounds["canvas"]["x"] - 1 and b["y"] >= bounds["canvas"]["y"] - 1
        and b["right"] <= bounds["canvas"]["right"] + 1
        and b["bottom"] <= bounds["canvas"]["bottom"] + 1
        for b in bounds["blocks"]
    )
    ck.check("viewport: fit-to-view leaves every block inside the visible canvas",
             inside and len(bounds["blocks"]) >= 4, f"blocks={bounds['blocks']}")
    # Drag the first block by (+100, +60) with stepped mouse input.
    info = block_info(page, "power supply") or block_info(page)
    name = "power supply"
    cx = info["rect"]["x"] + info["rect"]["width"] / 2
    cy = info["rect"]["y"] + info["rect"]["height"] / 2
    page.mouse.move(cx, cy)
    page.wait_for_timeout(100)
    page.mouse.down()
    page.wait_for_timeout(80)
    for i in range(1, 11):
        page.mouse.move(cx + 100 * i / 10, cy + 60 * i / 10)
        page.wait_for_timeout(40)
    page.mouse.up()
    page.wait_for_timeout(1000)
    after = block_info(page, name)
    print(f"Drag at 1280x800: layout before=({info['left']},{info['top']}) "
          f"after=({after['left']},{after['top']})")
    # The pointer moves in screen pixels; the canvas view transform
    # scales them into world (layout) units, so the on-screen block
    # must move by the pointer delta while the layout position moves
    # by the scaled amount. Check the rendered movement here and the
    # persisted layout position below.
    screen_dx = after["rect"]["x"] - info["rect"]["x"]
    screen_dy = after["rect"]["y"] - info["rect"]["y"]
    ck.check("drag: rendered block moved by about (+100,+60) on screen",
             abs(screen_dx - 100) <= 8 and abs(screen_dy - 60) <= 8,
             f"screen delta=({screen_dx:.1f},{screen_dy:.1f})")
    ck.check("drag: layout position changed in the drag direction",
             after["left"] > info["left"] and after["top"] > info["top"],
             f"layout delta=({after['left'] - info['left']},{after['top'] - info['top']})")
    expected = (after["left"], after["top"])
    saved_path = TMP / "drag1280-project.json"
    project = save_project(page, saved_path)
    ck.check("drag: project downloaded", project is not None)
    saved_pos = None
    if project:
        for item in project.get("items", []):
            spec = (item.get("state") or {}).get("rbd_spec")
            if spec:
                node = find_spec_node(spec, name)
                if node is not None:
                    saved_pos = node.get("pos")
        print(f"Saved pos for {name}: {saved_pos}")
        ck.check("drag: saved JSON holds the new position",
                 saved_pos is not None and abs(saved_pos[0] - expected[0]) <= 2
                 and abs(saved_pos[1] - expected[1]) <= 2,
                 f"pos={saved_pos} expected={expected}")
    page.reload(wait_until="load")
    page.wait_for_selector('[data-card="example:3"]', timeout=30000)
    page.wait_for_timeout(1500)
    open_project_file(page, saved_path)
    if page.locator("#rbd-canvas").count() == 0:
        page.locator("[data-select-item]").filter(has_text="RBD").first.click()
    page.wait_for_selector("#rbd-canvas", timeout=20000)
    page.wait_for_function(
        "document.querySelectorAll('[data-rbd-block]').length >= 4", timeout=20000)
    page.wait_for_timeout(600)
    restored = block_info(page, name)
    print(f"Restored position after reopen: ({restored['left']},{restored['top']})")
    ck.check("drag: position persists after reload and reopen",
             abs(restored["left"] - expected[0]) <= 2 and abs(restored["top"] - expected[1]) <= 2,
             f"restored=({restored['left']},{restored['top']}) expected={expected}")


def main():
    httpd = start_server(PORT)
    ck = Checker()
    print(f"Serving site/ on port {PORT}; scratch dir {TMP}")
    try:
        with sync_playwright() as p:
            browser = launch_browser(p)
            tests = [
                test_import_roundtrip, test_outdated_badge, test_fit_to_rbd,
                test_save_reopen_fields, test_keyboard_only,
                test_rbd_viewport_and_drag,
            ]
            for test in tests:
                page = browser.new_page(viewport={"width": 1600, "height": 1000})
                page.on("console", lambda m: print(f"[console] {m.type}: {m.text}"))
                try:
                    test(page, PORT, ck)
                except Exception as e:
                    ck.check(f"{test.__name__} completed without exception", False, str(e))
                page.close()
            browser.close()
    finally:
        httpd.shutdown()
    return ck.verdict("import round trip, outdated badge, fit to RBD, save and reopen, keyboard workflow, RBD viewport and drag all passed")


if __name__ == "__main__":
    sys.exit(main())
