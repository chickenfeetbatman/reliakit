#!/usr/bin/env python3
"""Run the whole ReliaKit browser test suite and report per file.

Run from the repo root or from this directory, after building the
demo (scripts/build-demo.sh). Exit code is 0 only if every file
passes. See README.md in this directory for setup.
"""
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
FILES = ["test_v032_flows.py", "test_tours_v032.py"]


def main():
    results = {}
    for name in FILES:
        print(f"===== {name} =====", flush=True)
        proc = subprocess.run([sys.executable, str(HERE / name)])
        results[name] = proc.returncode
        print(f"===== {name}: exit {proc.returncode} =====\n", flush=True)
    failed = [n for n, rc in results.items() if rc != 0]
    if failed:
        print(f"SUITE: FAIL ({failed})")
        return 1
    print("SUITE: PASS - all browser test files passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
