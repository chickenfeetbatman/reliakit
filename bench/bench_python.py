"""Benchmark workloads for the Python ReliaKit suite.

Same workloads and parameters as the Rust reliakit-bench binary.
Usage: python3 bench_python.py [path-to-python-suite]
Default suite path: ../hardware-reliability-suite relative to this
repository, then ~/workspace/hardware-reliability-suite.
"""
import os
import sys
import time

candidates = [
    sys.argv[1] if len(sys.argv) > 1 else "",
    os.path.join(os.path.dirname(__file__), "..", "..", "hardware-reliability-suite"),
    os.path.expanduser("~/workspace/hardware-reliability-suite"),
]
suite = next((c for c in candidates if c and os.path.isdir(os.path.join(c, "reliakit"))), None)
if suite is None:
    sys.exit("Python ReliaKit suite not found; pass its path as argv[1]")
sys.path.insert(0, suite)

import numpy as np

from reliakit.system import simulate_availability
from reliakit.mcmc import fit_bayes
from reliakit.distributions import fit_mle
from reliakit.bayes import conformal_reliability_lower

# 1) Monte Carlo availability: single pump, horizon 20000, 300 runs
spec = {"type": "block", "name": "pump", "mtbf": 1000.0, "mttr": 10.0}
t0 = time.perf_counter()
res = simulate_availability(spec, horizon=20000.0, n_sim=300, seed=3, grid_points=50)
t_avail = time.perf_counter() - t0
print(f"availability_mc  mean_A={res['mean_availability']:.6f}  time={t_avail:.3f}s")

# 2) MCMC: 60 Weibull draws, 4 chains x (2000 warmup + 2000 draws)
rng = np.random.default_rng(21)
times = rng.weibull(1.5, 60) * 1000.0
t0 = time.perf_counter()
fit = fit_bayes("weibull2", times, n_draws=2000, warmup=2000, n_chains=4, seed=99)
t_mcmc = time.perf_counter() - t0
summ = fit.summary()
print(f"mcmc  beta_mean={summ['beta']['mean']:.4f}  time={t_mcmc:.3f}s")

# 3) Conformal mini-study: 200 replicates, fit + conformal bound each
rng = np.random.default_rng(7)
t0 = time.perf_counter()
cover = 0
for rep in range(200):
    tr = rng.weibull(1.5, 40) * 1000.0
    fit_r = fit_mle("weibull2", tr)
    calib = rng.weibull(1.5, 40) * 1000.0
    rl = conformal_reliability_lower(calib, 500.0, 0.10)
    true_r = float(np.exp(-(500.0 / 1000.0) ** 1.5))
    cover += int(rl <= true_r)
t_study = time.perf_counter() - t0
print(f"conformal_study  coverage={cover}/200  time={t_study:.3f}s")
