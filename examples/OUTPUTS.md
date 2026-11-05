# Worked examples: real run outputs

Everything below was produced by actually running the release build
(`cargo build --release`) on the machine that built this repository.
Commands are run from the repository root. No output has been edited
except that the long JSON of Example 1 is summarised to its key
fields (the full JSON is what the command prints).

## Example 1: life data (bearing test, CLI)

Data: `examples/example1_bearing_life.csv` (24 units, synthetic,
Weibull beta 1.8, eta 4200 h in generation, truncated at 6000 h;
22 failures, 2 suspensions).

```
$ reliakit fit examples/example1_bearing_life.csv --dist weibull2
  "dist": "weibull2",
  "params": { "beta": 2.2168, "eta": 4006.6 },
  "n_fail": 22,
  "loglik": -196.246,
  (plus the Fisher covariance matrix, standard errors, and the raw
  times/failed arrays, all printed as JSON)

$ reliakit rel weibull2 1000 beta=2.2168492389077707 eta=4006.6133366126537
{
  "b10": 1451.832618428267,
  "dist": "weibull2",
  "hazard": 0.00010220437035821302,
  "mean_life": 3548.460964588939,
  "reliability": 0.9549431801358215,
  "t": 1000.0,
  "unreliability": 0.04505681986417853
}
```

Headline: fitted Weibull beta = 2.217, eta = 4006.6 h;
R(1000 h) = 0.955; B10 life = 1451.8 h; mean life = 3548.5 h.

## Example 2: limited data (bias correction, shrinkage, conformal, MCMC)

Data: `examples/example2_small_sample_lots.csv` (4 production lots,
8 units each, synthetic, censored at 4500 h).

```
$ cargo run --release --example example2_limited_data
Example 2: limited data, 4 lots
  lot 1: n = 8, failures = 8, MLE beta = 2.526, eta = 2292.5 h
  lot 2: n = 8, failures = 7, MLE beta = 1.983, eta = 3027.4 h
  lot 3: n = 8, failures = 7, MLE beta = 2.205, eta = 3147.4 h
  lot 4: n = 8, failures = 5, MLE beta = 3.237, eta = 4494.6 h
Pooled MLE: beta = 2.026, eta = 3262.5 h
Bias-corrected: beta = 1.928 (factor 0.952), eta = 3254.3 h
Hierarchical EB: pooled beta = 2.374, tau2 (log eta) = 0.0707
  lot 1: shrunk eta = 2466.4 h (shrinkage B = 0.22)
  lot 2: shrunk eta = 3077.0 h (shrinkage B = 0.35)
  lot 3: shrunk eta = 3155.0 h (shrinkage B = 0.30)
  lot 4: shrunk eta = 4162.0 h (shrinkage B = 0.22)
Split-conformal (fit lots 1-2, calibrate lots 3-4, alpha = 0.10): certified mission time = 477.4 h
Conformal lower bound on R(1000 h) = 0.778 (model estimate 0.872)
MCMC (4 chains x 1000 draws): beta mean = 1.940, eta mean = 3337.7 h
  max R-hat = 1.007, min ESS = 455
  R(1000 h) posterior mean = 0.899, 90% credible interval [0.827, 0.957]
```

## Example 3: system RBD with standby (CLI, library, desktop app model)

Diagram: `examples/example3_rbd.json` - a series path: power supply
(Weibull), a cold standby pump group (2 exponential units, switch
reliability 0.98, swap time 0.5 h), and a parallel controller pair.

```
$ reliakit rbd examples/example3_rbd.json --t 1000
{ "reliability": 0.9214192151709761, "t": 1000.0 }

$ reliakit availability examples/example3_rbd.json --horizon 20000 --sims 200 --seed 3
{
  "expected_downtime": 85.99,
  "mean_availability": 0.99630,
  "mean_failures": 7.555,
  "mean_repairs": 7.55
}

$ cargo run --release --example example3_ctmc_check
Pump standby group availability: CTMC steady state = 0.9999974
Pump standby group availability: Monte Carlo (2000 runs, horizon 20000 h) = 1.0000000, expected downtime 0.017 h per run
(The CTMC check idealises the group: perfect switching, no swap delay. The diagram itself uses switch reliability 0.98 and a 0.5 h swap time.)
Full diagram reliability at t = 1000 h: 0.921419
Full diagram Monte Carlo availability (200 runs, horizon 20000 h): 0.99630, expected downtime 86.0 h

$ reliakit-app --headless
Example 3 RBD reliability at t=1000: 0.921419
Example 3 mean availability (horizon 20000, 40 sims, seed 3): 0.996000
Example 3 expected downtime: 64.873
reliakit-app headless check OK
```

The desktop app ships with this diagram loaded on its System RBD
page, and its Examples page loads all of the examples in this folder.

## Example 4: FMEA worksheet and warranty forecast (CLI)

```
$ reliakit fmea examples/example4_fmea.csv
rows sorted by RPN (JSON): Pump seal leaks RPN 160, revised 64;
Power supply output drops RPN 140, revised 70; Controller watchdog
timeout RPN 54, no revised scores.

$ reliakit warranty --sales examples/example4_warranty_sales.csv \
    --returns examples/example4_warranty_returns.csv --current 3 --horizon 3
  "nevada_table": [[1,2,1],[1,2,1],[1,3,0],[2,0,0]],
  "returns_by_age": [5, 7, 2],
  "total_returns": 14,
  "forecast": { "beta": 2.405, "eta": 14.352,
                "expected_returns": 101.35 (over the 3-period horizon),
                "units_in_service": 1856 }
```

## Example 5: censored ALT MLE and competing risks

```
$ cargo run --release --example example5_alt_and_competing
Example 5A: censored Arrhenius MLE (105 of 120 units failed before censoring at 3000 h)
  Ea = 0.714 eV (data generated with 0.7 eV), sigma = 0.452, loglik = -78.35
  R(20000 h at 55 C) = 0.526, 90% bounds [0.310, 0.734], median life at 55 C = 20607 h (data generated with 20000 h)
Example 5B: competing risks (weibull2 per mode)
  mode seal: beta = 1.168, eta = 6527.2 h, failures = 99
  mode wear: beta = 1.758, eta = 3827.1 h, failures = 169
  overall R(2000 h) = 0.565; P(fail by wear) = 0.237, P(fail by seal) = 0.198
```

(Part B data was generated with wear Weibull(1.6, 4000) and seal
Weibull(1.2, 6500), censoring at 5000 h, n = 300.)

## CLI self test

```
$ reliakit selftest
PASS weibull R(eta): got 0.3678794412 want 0.3678794412
PASS series: got 0.7200000000 want 0.7200000000
PASS parallel: got 0.9800000000 want 0.9800000000
PASS cold standby: got 0.7357588823 want 0.7357588823
PASS arrhenius AF: got 937.2536490517 want 936.5000000000
PASS conformal zero-fail: got 0.8608916593 want 0.8608916593
PASS competing risks CIF + R = 1: got 0.9999664564 want 1.0000000000
selftest: all checks passed
```
