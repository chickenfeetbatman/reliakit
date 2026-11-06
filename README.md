# ReliaKit (Rust)

A reliability engineering suite in Rust: a library, a CLI, and a
desktop app. It rewrites an earlier Python suite of the same name.
Same math, same JSON diagram format, much faster on simulation and
MCMC workloads.

The crate has its own special functions, optimizer, seeded RNG, and
Metropolis sampler, so runs repeat exactly. No proprietary data is
bundled. The MIL-HDBK-217F Notice 2 prediction library is bundled
because the handbook is a public US government document. Data
tables from licensed standards are not bundled; you load your own
through the local library import.

## Try it in two minutes

```bash
cargo build --release

# Example 1: fit the bearing life data (Weibull), then evaluate it
./target/release/reliakit fit examples/example1_bearing_life.csv --dist weibull2
./target/release/reliakit rel weibull2 1000 beta=2.2168492389077707 eta=4006.6133366126537
#   fitted beta = 2.217, eta = 4006.6 h; R(1000 h) = 0.955; B10 = 1451.8 h

# Example 2: limited data (bias correction, shrinkage, conformal, MCMC)
cargo run --release --example example2_limited_data
#   bias-corrected beta 1.928; conformal lower bound on R(1000) = 0.778;
#   MCMC max R-hat 1.007, min ESS 455

# Example 3: system diagram with a cold standby group
./target/release/reliakit rbd examples/example3_rbd.json --t 1000
#   { "reliability": 0.9214192151709761, "t": 1000.0 }
./target/release/reliakit availability examples/example3_rbd.json --horizon 20000 --sims 200 --seed 3
#   mean availability 0.99630, expected downtime 86.0 h

# Example 4: FMEA worksheet and warranty forecast
./target/release/reliakit fmea examples/example4_fmea.csv
./target/release/reliakit warranty --sales examples/example4_warranty_sales.csv \
    --returns examples/example4_warranty_returns.csv --current 3 --horizon 3

# Example 5: censored ALT maximum likelihood and competing risks
cargo run --release --example example5_alt_and_competing
#   Ea = 0.714 eV recovered (0.7 in generation); competing-risk CIFs printed

./target/release/reliakit selftest     # built-in closed-form checks
./target/release/reliakit-app --headless   # desktop binary smoke check
```

Full outputs: [examples/OUTPUTS.md](examples/OUTPUTS.md). The
desktop app has an Examples page that loads each one into its
screen, so the downloaded binary runs without data of your own.

## Downloads

- From source: clone this repo and run
  `cargo run --release --bin reliakit-app` for the desktop app or
  `cargo run --release --bin reliakit` for the CLI.
- Binaries: the [v0.2.0 release](https://github.com/chickenfeetbatman/reliakit/releases/tag/v0.2.0)
  has the desktop app and CLI for Windows, macOS, and Linux. New
  version tags build new releases the same way.

## Desktop application

`reliakit-app` is one compiled binary (egui/eframe, no server).
It opens as one project workspace: a start page, a project tree
in the sidebar, and analysis items in the main pane. Item kinds:
Life Data (CSV load, fit, reliability curve and probability
plot), Limited Data, MCMC, Degradation, ALT, Growth, Competing
Risks, System RBD, Fault Tree, FMEA, Warranty, Prediction, DOE,
Flowchart, Markov, QCP, RCM, and FRACAS. Each item keeps its own
inputs and results. Results move between items: a Life Data fit
can be assigned to an RBD block or sent to QCP and Prediction
items, an FMEA row can create an RCM row, and FRACAS incidents
can be exported to a Life Data item. One JSON project file saves
and loads the whole project in ReliaKit own format. The design
system is documented in docs/DESIGN.md.

On the System RBD screen you drag blocks with the mouse. Each block
carries a name, distribution, parameters, MTBF/MTTR, and a mode
(active, hot standby, cold standby, warm with dormancy). The
topology evaluates as you edit, availability is computed on the
same screen, and diagrams save and load as the JSON the library and
CLI accept. `reliakit-app --headless` runs a no-window check: it
builds the Example 3 diagram and prints its reliability and
availability.

## Command line

`reliakit` prints JSON results for: `fit`, `rel`, `rbd`,
`availability`, `growth`, `af`, `alt-mle`, `competing`, `standby`,
`flow`, `fmea`, `warranty`, `faulttree`, `predict`, `doe`,
`wizard`, `qcp`, `importance`, `markov`, `predict217`, `sr332`,
`mission`, `allocate`, `library`, `fracas`, `degradation`, and
`selftest`. `reliakit <command> --help` shows the arguments.

## ReliaSoft parity matrix

Implemented means tested code. Limited means the limit is stated
in the row.

| ReliaSoft area | Rust implementation | Status |
| --- | --- | --- |
| Weibull++ life data: 14 distributions (weibull2, weibull3, weibull1, lognormal, normal, exponential, exponential2, gamma, gumbel, loglogistic, logistic, gompertz, gengamma, mixed_weibull) | `distributions` | Implemented (mixed_weibull has 2 subpopulations) |
| Censored (suspended) MLE, Fisher matrix bounds | `distributions` | Implemented |
| Median rank regression, probability plots, Kaplan-Meier | `distributions` | Implemented |
| Distribution wizard (best-fit ranking by AIC, with BIC and Anderson-Darling) | `wizard` | Implemented (weibull1 is not a wizard candidate; its shape is fixed by the user) |
| QCP (quick calculation pad: reliability, unreliability, hazard, mean life, pdf, Bx life at one mission time) | `quickcalc` | Implemented |
| Limited data: Weibull shape bias correction | `bayes` | Implemented |
| Limited data: hierarchical empirical-Bayes shrinkage (DerSimonian-Laird) | `bayes` | Implemented |
| Limited data: split-conformal certified mission time and reliability bound | `bayes` | Implemented |
| Bayesian MCMC with R-hat / ESS diagnostics, hierarchical model | `mcmc` | Implemented |
| Degradation analysis (linear, exponential, power paths) | `degradation` | Implemented |
| ALTA: Arrhenius, inverse power law, Eyring (least squares) | `alt` | Implemented |
| ALTA: censored maximum-likelihood fitting, all three models, use-level extrapolation with bounds | `alt` (`fit_alt_mle`) | Implemented |
| ALTA: two-stress Eyring with interaction term | `alt` (LSQ and MLE) | Implemented |
| ALTA: loglinear multi-stress MLE (generalized Eyring form) | `alt` (`fit_alt_mle_loglinear`) | Implemented for exactly two stresses (temperature and one nonthermal stress, weibull2 and lognormal lives); up to 8 stresses and user transforms are not built |
| ALTA: step-stress profiles | `alt` (step-stress reliability and CDF) | Implemented for step profiles; general time-varying stress entry is not built |
| RGA: Crow-AMSAA and Duane growth models | `growth` | Implemented |
| RGA: Gompertz and Lloyd-Lipow growth models | `growth` | Implemented (one Gompertz fit; no separate modified Gompertz form) |
| BlockSim: series, parallel, k-out-of-n, nested RBD from JSON | `system` | Implemented |
| BlockSim: standby redundancy, dormancy (cold / warm / hot), switch reliability | `system`, `standby` | Implemented |
| Standby groups of any size: exact exponential closed form, general distributions by recursive convolution quadrature, Monte Carlo cross-check | `standby` | Implemented (quadrature is practical to about 5 units; larger groups fall back to Monte Carlo) |
| BlockSim: CTMC steady-state and transient availability, Monte Carlo availability with swap-time downtime | `system` | Implemented |
| BlockSim: posterior system reliability propagation | `system` | Implemented |
| BlockSim: Markov diagrams (steady state, transient, mean time to absorption) | `markov` | Implemented |
| BlockSim: reliability importance (Birnbaum, criticality, Fussell-Vesely) | `importance` | Implemented (measures only; no importance plots) |
| Reliability allocation (target reliability split across blocks or parts) | `prediction` (`allocate_equal`, `allocate_weighted`) | Implemented for equal and weighted allocation; cost optimized, AGREE, Feasibility of Objectives, ARINC and Repairable Systems allocation are not built |
| FTA: AND / OR / voting gates, top-event probability, minimal cut sets | `faulttree` | Implemented (cut sets are exact for small trees; no binary decision diagram engine) |
| Competing risks / failure-mode analysis, cause-specific cumulative incidence | `competing` | Implemented (independent risks; a failure with unknown mode is treated as its own "unknown" mode category) |
| XFMEA: worksheet with severity, occurrence, detection, RPN, actions, revised RPN, CSV import/export | `fmea` | Implemented |
| XFMEA: FMEA types (Design, Process, System), FMECA criticality, control plan export, DVP&R table | `fmea`, `rcm` | Implemented (control plan export has rows with recommended actions; DVP&R has CSV round trip) |
| RCM++: RCM worksheet (consequence classes, task selection, mode criticality) | `rcm` | Implemented |
| XFRACAS: incident log (part, failure mode, hours, date, status, corrective action; counts and MTBF; CSV round trip) | `fracas` | Implemented as a local log; the hosted web portal and multi-user workflow are out of scope |
| Warranty: Nevada-table aggregation, Weibull fit to returns, forecast for units in service | `warranty` | Implemented |
| Lambda Predict style prediction: parts-count and parts-stress framework, user-entered base rates and stress / quality / environment factors, series roll-up | `prediction` | Implemented |
| Standards based prediction (MIL-HDBK-217F, SR-332, IEC 62380) | `standards217`, `prediction` | MIL-HDBK-217F Notice 2 library built from the published handbook PDF: 18 part categories with per-table provenance (sections listed in FEATURE_RESEARCH.md). Sections 5 and 6 are not included because the Notice 2 document carries only replacement pages for them. NSWC not included. SR-332 and IEC 62380: methods implemented clean-room from public secondary sources; the standards own data tables are licensed documents and load through the local library import. No table data from those standards is bundled. |
| Derating check (operating stress against rated stress, user limit) | `prediction` (`derating_check`) | Implemented; built-in derating standards tables are not bundled |
| Component library file format with local import (library CSV of parts and rates) | `prediction` (`library_from_csv`) | Implemented; ReliaSoft compiled libraries are not used |
| DOE++: two-level factorial designs, effect estimates, sums of squares | `doe` | Implemented for 2^k designs (main effects and interactions) |
| DOE++ expansion: fractional factorial, Plackett-Burman, Taguchi arrays, response surface (central composite, Box-Behnken, quadratic fit) | `doe` | Implemented |
| RENO style flowchart simulation: start, task, decision, loop/counter, end blocks, Monte Carlo run-time and cost distributions | `flowchart` | Implemented (JSON-defined flows; no graphical flow editor) |
| Bundled datasets: 16 sourced CSV files with CSV import, plus synthetic example data generators | `datasets/`, `datasets` | Implemented (sources and verification fits are in FEATURE_RESEARCH.md) |
| GUI project file (save and load app state, ReliaKit own JSON format) | `reliakit-app` | Implemented; ReliaSoft project formats are not read |
| Project workspace (Synthesis-style project and item organization, data flows between items) | `reliakit-app` | Implemented; one project open at a time, no multi-user database, no cloud sync |
| ReliaSoft Cloud | none | Out of scope: hosted service, not desktop software |
| MPC / MSG-3 | none | Out of scope: aerospace program tooling |

Each implemented row has at least one test with a known or
cross-checked value. FEATURE_TEST_LEDGER.md lists them feature by
feature.

## Tests

```bash
cargo test
```

167 integration tests in 13 files, all passing, plus the CLI self
test:

| Test file | Tests |
| --- | --- |
| tests/ported_tests.rs (the Python suite's 31 tests, ported) | 30 |
| tests/parity_tests.rs (FMEA, warranty, fault tree, prediction, DOE) | 7 |
| tests/qa_tests.rs (independent cross-checks) | 13 |
| tests/coverage_tests.rs (lognormal, normal, power path, IPL fit) | 4 |
| tests/alt_mle_tests.rs | 4 |
| tests/competing_tests.rs | 4 |
| tests/standby_tests.rs | 6 |
| tests/flowchart_tests.rs | 5 |
| tests/dist_tests.rs (v0.2.0 distributions) | 20 |
| tests/hbk_tests.rs (Markov, allocation, derating, library, RCM, FRACAS, FMEA additions, SR-332 and 62380 style methods) | 29 |
| tests/expansion_tests.rs (wizard, QCP, importance, DOE expansion, Gompertz, Lloyd-Lipow, loglinear ALT, step-stress) | 18 |
| tests/datasets_tests.rs (bundled dataset sums and fit verification) | 19 |
| tests/standards217_tests.rs (MIL-HDBK-217F Notice 2 library) | 8 |

The Python suite has 31 tests. The ported file groups the same
assertions, with the same known values and tolerances, into 30
test functions.

## Benchmarks

Same workloads, same parameters, same machine. Python numbers come
from the original suite; the script is in `bench/`.

| Workload | Python | Rust | Speedup |
| --- | --- | --- | --- |
| Monte Carlo availability, 300 runs, horizon 20000 h | 0.033 s | 0.002 s | about 16x |
| MCMC, 4 chains x (2000 warmup + 2000 draws), 60 points | 0.402 s | 0.031 s | about 13x |
| Conformal coverage study, 200 replicates | 2.663 s | 0.049 s | about 54x |

Coverage in the study: Python 192/200, Rust 191/200 at nominal
90 percent. Run the Rust side with
`cargo run --release --bin reliakit-bench`.

## Crate layout

```
src/
  lib.rs            module map and crate docs
  specfunc.rs       gamma / beta / normal special functions
  rng.rs            xoshiro256** RNG, seedable, distribution samplers
  optim.rs          Nelder-Mead, golden section, linear algebra, least squares
  distributions.rs  life distributions, MLE, rank regression, bounds, Kaplan-Meier
  bayes.rs          bias correction, shrinkage, hierarchical EB, conformal bounds
  mcmc.rs           adaptive Metropolis-Hastings, R-hat, ESS
  degradation.rs    degradation paths and pseudo-failure times
  alt.rs            accelerated life testing (LSQ, censored MLE, loglinear, step-stress)
  growth.rs         Crow-AMSAA, Duane, Gompertz, Lloyd-Lipow
  competing.rs      competing risks and cumulative incidence
  system.rs         RBD evaluation, standby, CTMC, availability simulation
  standby.rs        n-unit standby by recursive convolution quadrature
  flowchart.rs      flowchart (process network) simulation
  faulttree.rs      fault trees and minimal cut sets
  fmea.rs           FMEA worksheet, types, FMECA, control plan, DVP&R, CSV
  warranty.rs       Nevada table and returns forecast
  prediction.rs     parts-count / parts-stress prediction framework, allocation, derating, component library, SR-332 and 62380 style methods
  doe.rs            factorial designs and effects, fractional, Plackett-Burman, Taguchi, response surface
  wizard.rs         distribution wizard (best-fit ranking)
  quickcalc.rs      QCP (quick calculation pad)
  importance.rs     reliability importance measures
  markov.rs         Markov diagrams (CTMC models with named states)
  rcm.rs            RCM worksheet and mode criticality
  fracas.rs         FRACAS incident log
  standards217.rs   MIL-HDBK-217F Notice 2 part stress models
  datasets.rs       synthetic example data generators (labelled synthetic)
  bin/reliakit.rs       CLI
  bin/reliakit_app.rs   desktop app (egui)
  bin/reliakit_bench.rs benchmarks
examples/           worked examples with captured outputs (OUTPUTS.md)
datasets/           16 bundled sourced CSV datasets (see FEATURE_RESEARCH.md)
libraries/mil217/   MIL-HDBK-217F Notice 2 library, one JSON file per category
bench/              the Python benchmark script used above
tests/              the test files listed above
```

Four dependencies: serde, serde_json, clap, and eframe/egui for
the desktop app. The RNG is the crate's own seeded xoshiro256**.

## Toolchain

Built with rustc 1.99.1 and cargo 1.99.1, installed from the
official rustup script on Ubuntu 24.04. CI runs `cargo test` on
Linux. The release workflow builds Windows, macOS, and Linux
binaries on version tags.

## License

MIT. See LICENSE.
