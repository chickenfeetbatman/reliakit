#!/usr/bin/env python3
"""ReliaKit v0.3.2 tours coverage, against the locally served demo.

Covers task item 6:
  - All 18 tours: every step target is found (selector smoke over
    the tour test hook, the same hook the v0.3.1 suite used).
  - The Life Data and RBD tours are played end to end, with a
    content assertion on the final step card and on the result the
    tour produced (Life headline R(1000) = 0.9172, RBD headline
    reliability 0.9214).
  - The first-run wizard's three paths are clicked through, each on
    a fresh page: each starts its tour in a new tour item.

Prints PASS/FAIL lines; exit code 0 only if everything passes.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from common import (
    Checker, goto_demo, headline_value, launch_browser, start_server,
)
from playwright.sync_api import sync_playwright

PORT = 8962
ALL_KINDS = ["life", "limited", "mcmc", "degradation", "alt", "growth",
             "competing", "rbd", "faulttree", "fmea", "warranty",
             "prediction", "doe", "flowchart", "markov", "qcp", "rcm",
             "fracas"]
WIZARD_PATHS = [
    ("failure-data", "life", "Tour: Life Data"),
    ("system", "rbd", "Tour: RBD"),
    ("maintenance", "fracas", "Tour: FRACAS"),
]


def item_names(page):
    return page.evaluate(
        "(() => [...document.querySelectorAll('.items-list .item-name')]"
        ".map(e => e.textContent.trim()))()")


def tour_card_text(page):
    return page.evaluate(
        "(() => { const t = document.querySelector('.tour-card-title'); "
        "const b = document.querySelector('.tour-card-body'); "
        "return { title: t ? t.textContent.trim() : '', body: b ? b.textContent.trim() : '' }; })()")


def full_play(page, kind):
    """Play one tour to the end. Returns (count, missing, final_card)."""
    page.evaluate("(k) => window.__reliakitTour.start(k)", kind)
    page.wait_for_function("window.__reliakitTour.active()", timeout=30000)
    count = page.evaluate("window.__reliakitTour.stepCount()")
    missing = []
    final_card = {}
    for step in range(count):
        idx = page.evaluate("window.__reliakitTour.stepIndex()")
        found = page.evaluate("window.__reliakitTour.targetFound()")
        card = tour_card_text(page)
        print(f"  [{kind}] step {idx + 1}/{count} '{card['title']}' targetFound={found}", flush=True)
        if not found:
            missing.append(idx)
        if step == count - 1:
            final_card = card
        page.evaluate("window.__reliakitTour.next()")
        page.wait_for_timeout(200)
    active_after = page.evaluate("window.__reliakitTour.active()")
    return count, missing, final_card, active_after


def main():
    httpd = start_server(PORT)
    ck = Checker()
    print(f"Serving site/ on port {PORT}")
    try:
        with sync_playwright() as p:
            browser = launch_browser(p)
            page = browser.new_page(viewport={"width": 1600, "height": 1000})
            page.on("console", lambda m: print(f"[console] {m.type}: {m.text}", flush=True))
            goto_demo(page, PORT)

            # Life Data tour, end to end with final-step content.
            count, missing, final_card, closed = full_play(page, "life")
            ck.check("life tour: 4 to 8 steps", 4 <= count <= 8, f"steps={count}")
            ck.check("life tour: all step targets found", missing == [], f"missing={missing}")
            ck.check("life tour: closed after Finish", closed is False)
            ck.check("life tour: final step is the send-on step",
                     "Send the fit on" in final_card.get("title", ""),
                     f"final={final_card}")
            ck.check("life tour: final step body names the send actions",
                     "RBD" in final_card.get("body", ""), f"final={final_card}")
            names = item_names(page)
            ck.check("life tour: 'Tour: Life Data' item exists",
                     "Tour: Life Data" in names, f"items={names}")
            hl = headline_value(page)
            ck.check("life tour: headline is R(1000) = 0.9172", "0.9172" in hl, f"headline={hl!r}")

            # RBD tour, end to end with final-step content.
            count, missing, final_card, closed = full_play(page, "rbd")
            ck.check("rbd tour: 4 to 8 steps", 4 <= count <= 8, f"steps={count}")
            ck.check("rbd tour: all step targets found", missing == [], f"missing={missing}")
            ck.check("rbd tour: closed after Finish", closed is False)
            ck.check("rbd tour: final step is the importance step",
                     "Importance" in final_card.get("title", ""), f"final={final_card}")
            ck.check("rbd tour: final step body names importance measures",
                     "Birnbaum" in final_card.get("body", ""), f"final={final_card}")
            names = item_names(page)
            ck.check("rbd tour: 'Tour: RBD' item exists", "Tour: RBD" in names, f"items={names}")
            hl = headline_value(page)
            blocks = page.evaluate(
                "(() => [...document.querySelectorAll('[data-rbd-block] .rb-name')]"
                ".map(e => e.textContent.trim()))()")
            ck.check("rbd tour: Example 3 blocks on screen",
                     len(blocks) >= 4 and "power supply" in blocks, f"blocks={blocks}")
            ck.check("rbd tour: headline reliability is 0.9214 or shown on screen",
                     "0.9214" in hl or "0.9214" in page.locator("body").inner_text(),
                     f"headline={hl!r}")

            # Selector smoke for the other 16 tours.
            for kind in ALL_KINDS:
                if kind in ("life", "rbd"):
                    continue
                try:
                    count, missing, _final, closed = full_play(page, kind)
                    ck.check(f"smoke {kind}: all step targets found", missing == [],
                             f"missing={missing} steps={count}")
                    ck.check(f"smoke {kind}: closed after Finish", closed is False)
                except Exception as e:
                    ck.check(f"smoke {kind}: completed without exception", False, str(e))
                    try:
                        page.evaluate("window.__reliakitTour.exit()")
                    except Exception:
                        pass
            page.close()

            # Wizard: three paths, each on a fresh page.
            for path_key, want_kind, want_item in WIZARD_PATHS:
                wp = browser.new_page(viewport={"width": 1600, "height": 1000})
                try:
                    goto_demo(wp, PORT)
                    btn = wp.locator(f'[data-wizard-path="{path_key}"]')
                    ck.check(f"wizard {path_key}: path button on the start page", btn.count() >= 1)
                    btn.first.click()
                    try:
                        wp.wait_for_function("window.__reliakitTour.active()", timeout=30000)
                        started = True
                    except Exception:
                        started = False
                    ck.check(f"wizard {path_key}: clicking starts a tour", started)
                    kind = wp.evaluate("window.__reliakitTour.kind()") if started else None
                    ck.check(f"wizard {path_key}: tour kind is {want_kind}", kind == want_kind,
                             f"kind={kind}")
                    names = item_names(wp)
                    ck.check(f"wizard {path_key}: creates {want_item}", want_item in names,
                             f"items={names}")
                    if started:
                        wp.evaluate("window.__reliakitTour.exit()")
                except Exception as e:
                    ck.check(f"wizard {path_key}: completed without exception", False, str(e))
                wp.close()
            browser.close()
    finally:
        httpd.shutdown()
    return ck.verdict("all 18 tours, Life and RBD full plays, and the three wizard paths all passed")


if __name__ == "__main__":
    sys.exit(main())
