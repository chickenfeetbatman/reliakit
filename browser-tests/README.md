# ReliaKit browser tests

Playwright tests for the ReliaKit web interface, run against the
browser demo build served locally from `site/`. They are committed
in the repo (the v0.3.1 harness lived outside it) and they run in
CI as a hard gate.

## What is covered

- `test_v032_flows.py`: CSV import with fractional times (stored
  field, fit, project download round trip), the outdated badge and
  headline demotion after an edit, fit then send to RBD with the
  destination block parameters verified, save and reopen with a
  field-by-field Life Data comparison, a keyboard-only Example 1
  load and fit, and the RBD viewport at 1280x800 (fit-to-view keeps
  every block inside the canvas; a drag persists through save,
  reload, and reopen).
- `test_tours_v032.py`: all 18 guided tours with every step target
  found, the Life Data and RBD tours played end to end with content
  assertions on the final step, and the first-run wizard's three
  paths clicked through.

## How to run

From the repo root:

```bash
python3 -m venv /tmp/reliakit-browser-venv
/tmp/reliakit-browser-venv/bin/pip install playwright
/tmp/reliakit-browser-venv/bin/playwright install chromium

bash scripts/build-demo.sh

/tmp/reliakit-browser-venv/bin/python browser-tests/run_all.py
```

Or run one file at a time:

```bash
/tmp/reliakit-browser-venv/bin/python browser-tests/test_v032_flows.py
/tmp/reliakit-browser-venv/bin/python browser-tests/test_tours_v032.py
```

Each file serves `site/` itself on its own port (8961 and 8962),
so no separate server step is needed. A file prints PASS or FAIL
per check and an OVERALL line, and exits nonzero if any check
failed. The assertions are not softened to force a pass; a red run
means the behavior named in the check did not happen.

Selectors prefer stable hooks (data attributes, roles, and visible
text) over CSS structure, so routine front-end restyling does not
break the suite.
