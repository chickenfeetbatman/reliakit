# ReliaKit v0.2.0 feature research

This file is the research record behind the ReliaKit v0.2.0 feature
set. It has three parts: a per-product comparison against the
ReliaSoft (HBK) suite, a datasets section, and a standards section.
It is documentation only. It is not a promise that a listed feature
exists; the disposition column states what is built, what is
limited, and what is not planned.

Research date: November 5, 2026.

Method: features are taken from ReliaSoft and HBK product pages and
online help pages that were actually opened during the research.
Each section names its source pages, and each feature row cites the
page it came from. Dataset rows cite the documentation page each
file was transcribed from. Standards rows cite the published
handbook PDF each table was transcribed from, with page numbers.
No feature, dataset value, or table value in this file comes from
memory.

Dispositions:

- have: the row names the module that implements it. Rows marked
  "have in v0.2.0" are new or expanded in v0.2.0.
- limited: the row states the exact limit of the implementation.
- not planned v0.2.0: the row gives a one line reason, so the
  remaining gap is visible.
- out of scope: the row states why the item is outside this
  project.

Disposition totals for the 177 feature rows below: counted once
per row, under the first status word of its disposition. The
totals line is at the end of the feature tables.

## Weibull++ (life data analysis)

Source pages opened: https://www.hbkworld.com/en/products/software/reliability/weibull-life-data-analysis-software and https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm and https://help.reliasoft.com/weibull24/content/estimating_usage.htm

| Feature | Source URL | Disposition |
| --- | --- | --- |
| 2 parameter Weibull distribution fit | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | have: distributions (weibull2) |
| 3 parameter Weibull distribution fit | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | have: distributions (weibull3) |
| Lognormal distribution fit | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | have: distributions (lognormal) |
| Normal distribution fit | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | have: distributions (normal) |
| Exponential distribution fit | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | have: distributions (exponential, 1 parameter theta form, and exponential2, 2 parameter form, in v0.2.0) |
| Gamma distribution fit | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | have: distributions (gamma) |
| Gumbel distribution fit | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | have: distributions (gumbel) |
| Generalized gamma distribution fit | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | have in v0.2.0: distributions (gengamma) |
| Logistic distribution fit | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | have in v0.2.0: distributions (logistic) |
| Loglogistic distribution fit | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | have in v0.2.0: distributions (loglogistic) |
| Mixed Weibull (2 to 4 subpopulations) | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | limited: distributions has mixed_weibull with 2 subpopulations in v0.2.0; 3 and 4 subpopulation mixtures are not built |
| 1 parameter Weibull fit | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | have in v0.2.0: distributions (weibull1, shape fixed by the user) |
| Estimation methods RRX, RRY, NLRR, MLE | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | limited: v0.1.0 has censored MLE and median rank regression only, no NLRR |
| Confidence bounds: Fisher Matrix, Likelihood Ratio, Bayesian, Beta-Binomial | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | limited: v0.1.0 has Fisher matrix bounds only |
| Rank methods: SRM, RRM, Median Ranks, Kaplan-Meier | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | limited: v0.1.0 has median ranks and Kaplan-Meier only |
| Distribution Wizard (best fit selection) | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | have in v0.2.0: wizard (best-fit ranking) |
| Quick Calculation Pad (QCP) | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | have in v0.2.0: quickcalc (QCP) |
| Probability plots | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | have: distributions (probability plots) |
| Bayesian-Weibull analysis | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | limited: v0.1.0 has empirical Bayes shrinkage and bias correction in bayes, not a full Bayesian-Weibull folio |
| Competing Failure Modes analysis | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | have: competing (independent risks, unknown mode as own category) |
| Degradation analysis (non-destructive) | https://www.hbkworld.com/en/products/software/reliability/weibull-life-data-analysis-software | limited: degradation has linear, exponential and power paths only |
| Destructive degradation analysis | https://www.hbkworld.com/en/products/software/reliability/weibull-life-data-analysis-software | not planned v0.2.0: destructive test workflow not in plan |
| Warranty folio (multiple warranty data formats) | https://www.hbkworld.com/en/products/software/reliability/weibull-life-data-analysis-software | limited: warranty has Nevada table, Weibull fit to returns and forecast only |
| Warranty usage estimation (average usage and usage distribution methods) | https://help.reliasoft.com/weibull24/content/estimating_usage.htm | not planned v0.2.0: usage distribution estimation not in plan |
| Reliability demonstration test design | https://www.hbkworld.com/en/products/software/reliability/weibull-life-data-analysis-software | not planned v0.2.0: test design utility not in plan |
| Built-in RBDs linked to life data folios | https://www.hbkworld.com/en/products/software/reliability/weibull-life-data-analysis-software | limited: system RBDs are JSON defined and not linked live to folios |
| Goodness of fit tests (uncensored data) | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | not planned v0.2.0: no formal goodness of fit test suite in plan |
| Folio tools: Alter Data Type, Change Units, Auto Group Data, Alter Parameters, Specify Points, Color Data Values | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | not planned v0.2.0: folio editing utilities not in plan |

## ALTA (accelerated life testing, module of Weibull++)

Source pages opened: https://www.hbkworld.com/en/products/software/reliability/weibull-software-accelerated-life-testing-module and https://help.reliasoft.com/weibull24/content/alta_standard_folio_control_panel.htm

| Feature | Source URL | Disposition |
| --- | --- | --- |
| Life distributions: 2P Weibull, lognormal, 1P exponential | https://help.reliasoft.com/weibull24/content/alta_standard_folio_control_panel.htm | have: alt with distributions |
| Arrhenius life-stress relationship | https://www.hbkworld.com/en/products/software/reliability/weibull-software-accelerated-life-testing-module | have: alt |
| Eyring life-stress relationship | https://www.hbkworld.com/en/products/software/reliability/weibull-software-accelerated-life-testing-module | have: alt |
| Inverse Power Law relationship | https://www.hbkworld.com/en/products/software/reliability/weibull-software-accelerated-life-testing-module | have: alt |
| Temperature-Humidity relationship | https://www.hbkworld.com/en/products/software/reliability/weibull-software-accelerated-life-testing-module | limited: alt has two-stress Eyring with interaction, not a named Temperature-Humidity model |
| Temperature-Nonthermal relationship | https://www.hbkworld.com/en/products/software/reliability/weibull-software-accelerated-life-testing-module | limited: alt has two-stress Eyring with interaction, not a named Temperature-Nonthermal model |
| Generalized Eyring relationship (PRO) | https://www.hbkworld.com/en/products/software/reliability/weibull-software-accelerated-life-testing-module | have in v0.2.0: alt loglinear model (generalized Eyring form, two stresses) |
| Proportional Hazards model (PRO, up to 8 stresses) | https://www.hbkworld.com/en/products/software/reliability/weibull-software-accelerated-life-testing-module | not planned v0.2.0: proportional hazards not in plan |
| General Log-Linear model (PRO, up to 8 stresses, user transforms) | https://www.hbkworld.com/en/products/software/reliability/weibull-software-accelerated-life-testing-module | limited: alt has a loglinear multi-stress MLE in v0.2.0 for exactly two stresses (temperature and one nonthermal stress, weibull2 and lognormal lives); the PRO form with up to 8 stresses and user transforms is not built |
| Cumulative Damage model for time-varying stress (PRO) | https://www.hbkworld.com/en/products/software/reliability/weibull-software-accelerated-life-testing-module | have in v0.2.0: alt step-stress (step profiles only; general time-varying stress entry is not built) |
| Step-stress analysis | https://help.reliasoft.com/weibull24/content/alta_standard_folio_control_panel.htm | have in v0.2.0: alt step-stress |
| Indicator variables for stresses (PRO) | https://www.hbkworld.com/en/products/software/reliability/weibull-software-accelerated-life-testing-module | not planned v0.2.0: indicator variable handling not in plan |
| Stress profiles (PRO) | https://www.hbkworld.com/en/products/software/reliability/weibull-software-accelerated-life-testing-module | not planned v0.2.0: graphical stress profile editor not in plan |
| MLE estimation with confidence intervals | https://www.hbkworld.com/en/products/software/reliability/weibull-software-accelerated-life-testing-module | have: alt (LSQ and censored MLE) |
| Test plan utility | https://www.hbkworld.com/en/products/software/reliability/weibull-software-accelerated-life-testing-module | not planned v0.2.0: ALT test planning not in plan |
| Monte Carlo simulation for all models (PRO) | https://www.hbkworld.com/en/products/software/reliability/weibull-software-accelerated-life-testing-module | not planned v0.2.0: ALT Monte Carlo not in plan |
| Likelihood Ratio Test for common shape parameter | https://help.reliasoft.com/weibull24/content/alta_standard_folio_control_panel.htm | not planned v0.2.0: likelihood ratio test utility not in plan |
| Use Level Probability and Standardized Residuals plots | https://help.reliasoft.com/weibull24/content/alta_standard_folio_control_panel.htm | not planned v0.2.0: ALTA diagnostic plots not in plan |
| Transfer Data to Life Data Folio | https://help.reliasoft.com/weibull24/content/alta_standard_folio_control_panel.htm | not planned v0.2.0: cross-folio transfer not in plan |
| nCode GlyphWorks .s3t file import | https://www.hbkworld.com/en/products/software/reliability/weibull-software-accelerated-life-testing-module | out of scope: third party nCode file format import |

## BlockSim (system reliability, availability, maintainability)

Source pages opened: https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software and https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features and https://help.reliasoft.com/blocksim21/content/quick_tour_of_blocksim.htm

| Feature | Source URL | Disposition |
| --- | --- | --- |
| RBD series, parallel and complex configurations | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | have: system |
| k-out-of-n redundancy | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | have: system |
| Load sharing blocks | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | not planned v0.2.0: load sharing model not in plan |
| Hot, warm and cold standby blocks | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | limited: standby has dormancy and switch reliability, quadrature practical to about 5 units then Monte Carlo |
| Mirrored blocks | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | not planned v0.2.0: mirrored block references not in plan |
| Multi blocks | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | not planned v0.2.0: multi block shorthand not in plan |
| Subdiagrams | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | have: system (nested diagrams from JSON) |
| Fault tree analysis (FTA) diagrams | https://help.reliasoft.com/blocksim21/content/quick_tour_of_blocksim.htm | have: faulttree (AND, OR, voting gates) |
| Minimal cut sets | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | limited: faulttree cut sets are exact for small trees only, no BDD engine |
| Markov diagrams with degraded states | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | have in v0.2.0: markov (diagrams with degraded states); v0.1.0 already has CTMC steady and transient availability in system |
| QCP metrics: reliability, reliable life, BX percent life, mean life, failure rate | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | have in v0.2.0: quickcalc (QCP) |
| Reliability importance measures and plots | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | have in v0.2.0: importance (Birnbaum, criticality and Fussell-Vesely measures; no importance plots) |
| FRED reports | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | not planned v0.2.0: FRED report format not in plan |
| Reliability allocation: Equal, Weighted, Cost Optimized | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | limited: prediction has allocate equal and weighted in v0.2.0; cost optimized allocation is not built |
| Discrete event simulation for repairable systems | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software | limited: system has Monte Carlo availability, not a full discrete event simulator |
| Restoration factor in simulation | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | not planned v0.2.0: restoration factor modelling not in plan |
| Duty cycles | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | not planned v0.2.0: duty cycle modelling not in plan |
| Crews and spare parts pools | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | not planned v0.2.0: crew and spares logistics not in plan |
| Maintenance groups and state change triggers | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | not planned v0.2.0: maintenance group logic not in plan |
| Optimum replacement and optimum inspection | https://help.reliasoft.com/blocksim21/content/quick_tour_of_blocksim.htm | not planned v0.2.0: optimum replacement solver not in plan |
| Phase diagrams (RPD) and maintenance phases | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | not planned v0.2.0: phase diagram modelling not in plan |
| Simulation Worksheets for what-if analysis | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | not planned v0.2.0: simulation worksheets not in plan |
| Throughput analysis with backlog | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | not planned v0.2.0: throughput and backlog modelling not in plan |
| Process Flow and Event Analysis flowcharts (formerly RENO) | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software | limited: flowchart is JSON defined with no graphical editor |
| Life cycle cost analysis | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software | not planned v0.2.0: life cycle costing not in plan |
| Multi-thread and batch mode simulation | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | not planned v0.2.0: batch simulation runner not in plan |
| Overlay plots | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | not planned v0.2.0: overlay plot type not in plan |


## DOE++ (design of experiments, in Weibull++)

Source page opened: https://help.reliasoft.com/reference/experiment_design_and_analysis/doe/doe_overview.html

| Feature | Source URL | Disposition |
| --- | --- | --- |
| One Factor designs | https://help.reliasoft.com/reference/experiment_design_and_analysis/doe/doe_overview.html | not planned v0.2.0: one factor design type not in plan |
| General Full Factorial designs | https://help.reliasoft.com/reference/experiment_design_and_analysis/doe/doe_overview.html | limited: doe covers 2 level full factorial only, not general multi-level full factorial |
| Two Level Full Factorial designs | https://help.reliasoft.com/reference/experiment_design_and_analysis/doe/doe_overview.html | have: doe (2^k full factorial, main effects and interactions) |
| Two Level Fractional Factorial designs | https://help.reliasoft.com/reference/experiment_design_and_analysis/doe/doe_overview.html | have in v0.2.0: doe (fractional factorial with defining relation, resolution and alias sets) |
| Plackett-Burman designs | https://help.reliasoft.com/reference/experiment_design_and_analysis/doe/doe_overview.html | have in v0.2.0: doe (Plackett-Burman) |
| Taguchi Orthogonal Arrays | https://help.reliasoft.com/reference/experiment_design_and_analysis/doe/doe_overview.html | have in v0.2.0: doe (Taguchi L8 and L9 arrays, signal to noise ratios) |
| Response Surface Method | https://help.reliasoft.com/reference/experiment_design_and_analysis/doe/doe_overview.html | have in v0.2.0: doe (central composite and Box-Behnken designs, quadratic fit and ANOVA) |
| Robust Parameter design (Taguchi) | https://help.reliasoft.com/reference/experiment_design_and_analysis/doe/doe_overview.html | have in v0.2.0: doe (Taguchi arrays and signal to noise ratios) |
| Reliability DOE (censored response) | https://help.reliasoft.com/reference/experiment_design_and_analysis/doe/doe_overview.html | not planned v0.2.0: reliability DOE with censored response not in plan |
| Optimal Custom Design | https://help.reliasoft.com/reference/experiment_design_and_analysis/doe/doe_overview.html | not planned v0.2.0: optimal custom designs not in plan |

## RGA (reliability growth analysis, module of Weibull++)

Source pages opened: https://www.hbkworld.com/en/products/software/reliability/weibull-software-reliability-growth-module and https://help.reliasoft.com/weibull25/content/rga_standard_folio.htm

| Feature | Source URL | Disposition |
| --- | --- | --- |
| Crow-AMSAA (NHPP) model | https://help.reliasoft.com/weibull25/content/rga_standard_folio.htm | have: growth |
| Duane model | https://help.reliasoft.com/weibull25/content/rga_standard_folio.htm | have: growth |
| Standard Gompertz model | https://help.reliasoft.com/weibull25/content/rga_standard_folio.htm | have in v0.2.0: growth (Gompertz) |
| Modified Gompertz model | https://help.reliasoft.com/weibull25/content/rga_standard_folio.htm | have in v0.2.0: growth (Gompertz scope; one Gompertz fit, no separate modified form) |
| Lloyd-Lipow model | https://help.reliasoft.com/weibull25/content/rga_standard_folio.htm | have in v0.2.0: growth (Lloyd-Lipow) |
| Logistic growth model | https://help.reliasoft.com/weibull25/content/rga_standard_folio.htm | not planned v0.2.0: logistic growth model not in plan (logistic distribution for life data is separate and is building) |
| Crow Extended model (single phase, A, BC, BD modes) | https://help.reliasoft.com/weibull25/content/rga_standard_folio.htm | not planned v0.2.0: Crow Extended not in plan |
| Crow Extended Continuous Evaluation (multi-phase) | https://help.reliasoft.com/weibull25/content/rga_standard_folio.htm | not planned v0.2.0: multi-phase continuous evaluation not in plan |
| Growth planning folio and multi-phase plot | https://help.reliasoft.com/weibull25/content/rga_standard_folio.htm | not planned v0.2.0: growth planning folio not in plan |
| Reliability growth projections | https://www.hbkworld.com/en/products/software/reliability/weibull-software-reliability-growth-module | not planned v0.2.0: projection workflow not in plan |
| Multi-phase analysis | https://www.hbkworld.com/en/products/software/reliability/weibull-software-reliability-growth-module | not planned v0.2.0: multi-phase growth analysis not in plan |
| Optimum overhaul analysis for repairable systems | https://help.reliasoft.com/weibull25/content/rga_standard_folio.htm | not planned v0.2.0: optimum overhaul not in plan |
| Repairable systems analysis (power law, minimal repair) | https://help.reliasoft.com/weibull25/content/rga_standard_folio.htm | not planned v0.2.0: repairable systems folio not in plan |
| Repairable system test plans | https://www.hbkworld.com/en/products/software/reliability/weibull-software-reliability-growth-module | not planned v0.2.0: growth test planning not in plan |
| Statistical tests and interval goodness of fit | https://help.reliasoft.com/weibull25/content/rga_standard_folio.htm | not planned v0.2.0: growth test suite not in plan |
| Simulation tools for growth | https://www.hbkworld.com/en/products/software/reliability/weibull-software-reliability-growth-module | not planned v0.2.0: growth simulation not in plan |

## Lambda Predict (standards based reliability prediction)

Source pages opened: https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software and https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software/key-features and https://help.reliasoft.com/lp20/current_application_intro.htm

| Feature | Source URL | Disposition |
| --- | --- | --- |
| MIL-HDBK-217F part stress method | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software/key-features | have in v0.2.0: standards217 with the MIL-HDBK-217F Notice 2 library (18 part categories, see Standards section) plus the prediction framework |
| MIL-HDBK-217F parts count method | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software/key-features | limited: prediction framework has a parts-count roll-up; the handbook Appendix A parts count tables are not built (outside part stress scope, see Standards section) |
| 217Plus (2015 Notice 1) with Process Grade Factors | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software/key-features | not planned v0.2.0: 217Plus model not in plan |
| 217Plus Bayesian tailoring | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software/key-features | not planned v0.2.0: 217Plus Bayesian tailoring not in plan |
| Neufelder software reliability model in 217Plus | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software/key-features | not planned v0.2.0: software reliability model not in plan |
| Telcordia SR-332 Issues 3, 2, 1 and TR-332 Issue 6, Method I (Black Box) | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software/key-features | limited: prediction framework only, SR-332 tables are not bundled; method-only per Evaluated and declined section |
| Telcordia SR-332 Method II (laboratory data) | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software/key-features | limited: method-only, no bundled tables; combination formulas only partly pinned by open sources, see Clean-room methods sources |
| Telcordia SR-332 Method III (field data) | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software/key-features | limited: method-only, no bundled tables; combination formulas only partly pinned by open sources, see Clean-room methods sources |
| FIDES 2009 and FIDES 2022 with auto-upgrade | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software/key-features | not planned v0.2.0: FIDES model not in plan; guide availability noted in Evaluated and declined section |
| FIDES life profiles and phases, pi factor assessments | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software/key-features | not planned v0.2.0: FIDES mission profile workflow not in plan |
| NSWC Mechanical (NSWC-07 and NSWC-11) | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software/key-features | not planned v0.2.0: NSWC mechanical models not in plan |
| Siemens SN 29500 | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software | not planned v0.2.0: SN 29500 model not in plan |
| IEC 62380 prediction | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software/key-features | limited: method-only per Evaluated and declined section; mission profile form logged in Clean-room methods sources |
| Component libraries (built-in) | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software | have in v0.2.0: prediction component library file format with local import (library CSV); ReliaSoft compiled libraries themselves are declined, see Evaluated and declined |
| PartLibraries.org access (MIL-M-38510, EPRD-97, NPRD-95 free; commercial parts by subscription) | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software | out of scope: external subscription parts service |
| Custom connections modelling | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software/key-features | not planned v0.2.0: custom connection models not in plan |
| Non-operational (dormant) failure rates | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software/key-features | not planned v0.2.0: non-operational rate tables not in plan |
| Derating analysis with built-in standards (NAVSEA-TE000-AB-GTP-010, MIL-STD-975M, MIL-STD-1547, AS-4613, ECSS-Q-30-11-A) plus custom derating | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software/key-features | limited: prediction has a derating check in v0.2.0 (operating stress against rated stress, user limit); the named built-in derating standards tables are not bundled |
| Allocation with 5 models: Equal, AGREE, Feasibility of Objectives, ARINC, Repairable Systems | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software/key-features | limited: prediction has allocate equal and weighted in v0.2.0; AGREE, Feasibility of Objectives, ARINC and Repairable Systems allocation are not built |
| Prediction plots: Failure Rate, MTBF, Mission Time, Unreliability | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software/key-features | not planned v0.2.0: prediction plot set not in plan |
| Import, export and copy-paste of prediction projects | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software | have in v0.2.0: GUI project file (save and load, ReliaKit own JSON format) |
| Template reports in Word and Excel | https://www.hbkworld.com/en/products/software/reliability/lambda-predict-standard-based-reliability-predictions-software/key-features | not planned v0.2.0: Word and Excel template reporting not in plan |

## XFMEA (failure mode and effects analysis)

Source pages opened: https://www.hbkworld.com/en/products/software/reliability/xfmea-failure-mode-effects-analysis-fmea-software/key-features and https://help.reliasoft.com/xfmea_rcm/content/quick_tour_of_xfmea_and_rcm.htm

| Feature | Source URL | Disposition |
| --- | --- | --- |
| Design FMEA (DFMEA) | https://www.hbkworld.com/en/products/software/reliability/xfmea-failure-mode-effects-analysis-fmea-software/key-features | have in v0.2.0: fmea (FMEA types) |
| Process FMEA (PFMEA) | https://www.hbkworld.com/en/products/software/reliability/xfmea-failure-mode-effects-analysis-fmea-software/key-features | have in v0.2.0: fmea (FMEA types) |
| System FMEA | https://www.hbkworld.com/en/products/software/reliability/xfmea-failure-mode-effects-analysis-fmea-software/key-features | have in v0.2.0: fmea (FMEA types) |
| FMEA worksheet with Severity, Occurrence, Detection and RPN | https://www.hbkworld.com/en/products/software/reliability/xfmea-failure-mode-effects-analysis-fmea-software/key-features | have: fmea (S, O, D, RPN and revised RPN, CSV import and export) |
| Risk methods: Severity x Occurrence, risk matrices, Priority Highlights and Risk Ranking Logic, multiple severity scores | https://www.hbkworld.com/en/products/software/reliability/xfmea-failure-mode-effects-analysis-fmea-software/key-features | not planned v0.2.0: extended risk ranking methods not in plan |
| FMECA criticality per MIL-STD-1629A, quantitative and qualitative, with time-dependent distributions | https://www.hbkworld.com/en/products/software/reliability/xfmea-failure-mode-effects-analysis-fmea-software/key-features | have in v0.2.0: fmea and rcm (FMECA criticality number); time-dependent distribution form is not built |
| QCPNs and Alpha/Beta Ratios worksheet | https://www.hbkworld.com/en/products/software/reliability/xfmea-failure-mode-effects-analysis-fmea-software/key-features | not planned v0.2.0: QCPN worksheet not in plan |
| P-Diagrams | https://help.reliasoft.com/xfmea_rcm/content/quick_tour_of_xfmea_and_rcm.htm | not planned v0.2.0: P-Diagrams not in plan |
| DFMEA to PFMEA transfer | https://www.hbkworld.com/en/products/software/reliability/xfmea-failure-mode-effects-analysis-fmea-software/key-features | not planned v0.2.0: DFMEA to PFMEA transfer not in plan |
| Test Plans and DVP and R (with Excel import and change log) | https://www.hbkworld.com/en/products/software/reliability/xfmea-failure-mode-effects-analysis-fmea-software/key-features | have in v0.2.0: fmea (DVP and R table with CSV round trip); Excel import and change log are not built |
| Control Plans | https://help.reliasoft.com/xfmea_rcm/content/quick_tour_of_xfmea_and_rcm.htm | have in v0.2.0: fmea (control plan export, rows with recommended actions) |
| Process Flow Diagrams | https://help.reliasoft.com/xfmea_rcm/content/quick_tour_of_xfmea_and_rcm.htm | not planned v0.2.0: FMEA process flow diagrams not in plan (flowchart module is a separate RENO-style analysis) |
| DRBFM (Design Review Based on Failure Mode) | https://www.hbkworld.com/en/products/software/reliability/xfmea-failure-mode-effects-analysis-fmea-software/key-features | not planned v0.2.0: DRBFM not in plan |
| Data tools: Excel import templates, change log, analysis planning, links and attachments, find and replace, email and SMS alerts, cause-effect diagrams, ancestry tracking | https://www.hbkworld.com/en/products/software/reliability/xfmea-failure-mode-effects-analysis-fmea-software/key-features | not planned v0.2.0: FMEA collaboration and tracking tools not in plan |
| FMRA with BlockSim RBD and fault tree sync, reliability and availability allocation | https://www.hbkworld.com/en/products/software/reliability/xfmea-failure-mode-effects-analysis-fmea-software/key-features | not planned v0.2.0: FMRA sync workflow not in plan (allocation itself is building under BlockSim and Lambda Predict) |
| Queries, reports in Word and Excel, SEP publishing, dashboards, ancestry status | https://www.hbkworld.com/en/products/software/reliability/xfmea-failure-mode-effects-analysis-fmea-software/key-features | not planned v0.2.0: FMEA reporting and dashboard layer not in plan |

## RCM++ (reliability centered maintenance)

Source pages opened: https://www.hbkworld.com/en/products/software/reliability/rcm-reliability-centered-maintenance-software and https://www.hbkworld.com/en/products/software/reliability/rcm-reliability-centered-maintenance-software/key-features and https://help.reliasoft.com/xfmea_rcm/content/quick_tour_of_xfmea_and_rcm.htm

| Feature | Source URL | Disposition |
| --- | --- | --- |
| RCM analysis supporting ATA MSG-3, SAE JA1011 and SAE JA1012 logic | https://www.hbkworld.com/en/products/software/reliability/rcm-reliability-centered-maintenance-software | have in v0.2.0: rcm (RCM worksheet); MPC and MSG-3 program tooling itself is out of scope, see Synthesis platform layer |
| Risk Discovery equipment selection | https://www.hbkworld.com/en/products/software/reliability/rcm-reliability-centered-maintenance-software/key-features | not planned v0.2.0: Risk Discovery selection workflow not in plan |
| Functional failure analysis (Hierarchy, Worksheet and Filtered views) | https://www.hbkworld.com/en/products/software/reliability/rcm-reliability-centered-maintenance-software/key-features | have in v0.2.0: rcm (RCM worksheet covers functional failure analysis) |
| Failure effect categorization (FEC) and task selection logic charts (customizable) | https://www.hbkworld.com/en/products/software/reliability/rcm-reliability-centered-maintenance-software/key-features | have in v0.2.0: rcm (RCM worksheet covers FEC and task selection) |
| Task packaging | https://www.hbkworld.com/en/products/software/reliability/rcm-reliability-centered-maintenance-software/key-features | not planned v0.2.0: task packaging not in plan |
| FMEA inside RCM analysis | https://www.hbkworld.com/en/products/software/reliability/rcm-reliability-centered-maintenance-software | have: fmea (worksheet) and rcm (RCM worksheet in v0.2.0) |
| 10 distributions with Weibull++ and BlockSim integration and QPE | https://www.hbkworld.com/en/products/software/reliability/rcm-reliability-centered-maintenance-software/key-features | limited: distributions cover 14 types in v0.2.0; QPE is not a ReliaKit feature |
| Simulation comparison by Average Availability, Cost per Uptime and Total Operating Cost | https://help.reliasoft.com/xfmea_rcm/content/quick_tour_of_xfmea_and_rcm.htm | not planned v0.2.0: RCM simulation comparison not in plan |
| FMRA in RCM++ | https://www.hbkworld.com/en/products/software/reliability/rcm-reliability-centered-maintenance-software/key-features | not planned v0.2.0: FMRA in RCM context not in plan |
| Optimum replacement interval (single and group) | https://www.hbkworld.com/en/products/software/reliability/rcm-reliability-centered-maintenance-software/key-features | not planned v0.2.0: optimum replacement calculation not in plan |
| Queries, reports, Plot Viewer and dashboards | https://www.hbkworld.com/en/products/software/reliability/rcm-reliability-centered-maintenance-software/key-features | not planned v0.2.0: RCM reporting layer not in plan |
| Excel import | https://www.hbkworld.com/en/products/software/reliability/rcm-reliability-centered-maintenance-software/key-features | not planned v0.2.0: RCM Excel import not in plan |
| Import from XFMEA, MPC and XFRACAS; FRACAS and BOM sync | https://www.hbkworld.com/en/products/software/reliability/rcm-reliability-centered-maintenance-software/key-features | not planned v0.2.0: cross-product sync not in plan |
| MPC (MSG-3 aircraft maintenance program tooling) | https://www.hbkworld.com/en/products/software/reliability | out of scope: aerospace program tooling |


## XFRACAS (failure reporting, analysis and corrective action system)

Source pages opened: https://www.hbkworld.com/en/products/software/reliability/xfracas-failure-reporting-analysis-corrective-action-system-fracas-software and https://www.hbkworld.com/en/products/software/reliability/xfracas-failure-reporting-analysis-corrective-action-system-fracas-software/key-features

| Feature | Source URL | Disposition |
| --- | --- | --- |
| Incident reporting | https://www.hbkworld.com/en/products/software/reliability/xfracas-failure-reporting-analysis-corrective-action-system-fracas-software | have in v0.2.0: fracas (incident log) |
| Failure analysis on returned parts (RMA, sales and work order links, inspection findings, fault history and sensor data) | https://www.hbkworld.com/en/products/software/reliability/xfracas-failure-reporting-analysis-corrective-action-system-fracas-software/key-features | limited: fracas incident log in v0.2.0 has part, failure mode, operating hours, date, status and corrective action; RMA, sales and work order links, inspection findings, fault history and sensor data fields are not built |
| Serialized part tracking (install, incidents, operating time, as-maintained configuration, status) | https://www.hbkworld.com/en/products/software/reliability/xfracas-failure-reporting-analysis-corrective-action-system-fracas-software | limited: fracas incident log in v0.2.0 records part and operating hours per incident; install history, as-maintained configuration and unit status tracking are not built |
| Root cause analysis records | https://www.hbkworld.com/en/products/software/reliability/xfracas-failure-reporting-analysis-corrective-action-system-fracas-software | have in v0.2.0: fracas (failure mode and corrective action recorded per incident; no separate structured root cause workflow) |
| Team problem resolution with 4 to 8 step methods (8D, DMAIC, DCOV, CAPA) and FRB review and sign-off | https://www.hbkworld.com/en/products/software/reliability/xfracas-failure-reporting-analysis-corrective-action-system-fracas-software/key-features | not planned v0.2.0: full problem solving workflow with sign-off not in plan |
| Action tracking with email notifications | https://www.hbkworld.com/en/products/software/reliability/xfracas-failure-reporting-analysis-corrective-action-system-fracas-software/key-features | not planned v0.2.0: action tracking with email notification service not in plan |
| Projects grouping incidents and problems with statistics | https://www.hbkworld.com/en/products/software/reliability/xfracas-failure-reporting-analysis-corrective-action-system-fracas-software/key-features | not planned v0.2.0: FRACAS project statistics layer not in plan |
| Data feed to Weibull++ life and growth analysis | https://www.hbkworld.com/en/products/software/reliability/xfracas-failure-reporting-analysis-corrective-action-system-fracas-software | not planned v0.2.0: live cross-module data feed not in plan |
| XFMEA and RCM++ configuration and failure mode sharing | https://www.hbkworld.com/en/products/software/reliability/xfracas-failure-reporting-analysis-corrective-action-system-fracas-software | not planned v0.2.0: cross-product sharing not in plan |
| Configurable entities, fields, permissions, security levels, audit snapshot, email digest, auto-disable | https://www.hbkworld.com/en/products/software/reliability/xfracas-failure-reporting-analysis-corrective-action-system-fracas-software/key-features | out of scope: multi-user web administration and permissions system |
| Personalized web portal | https://www.hbkworld.com/en/products/software/reliability/xfracas-failure-reporting-analysis-corrective-action-system-fracas-software/key-features | out of scope: hosted web portal |
| Report Builder with custom SQL, export to xlsx, rtf, csv and pdf | https://www.hbkworld.com/en/products/software/reliability/xfracas-failure-reporting-analysis-corrective-action-system-fracas-software/key-features | not planned v0.2.0: custom SQL report builder not in plan |
| Charts (bar, pie, area, step, scatter, line), dashboards and custom XSLT | https://www.hbkworld.com/en/products/software/reliability/xfracas-failure-reporting-analysis-corrective-action-system-fracas-software/key-features | not planned v0.2.0: FRACAS dashboard layer not in plan |
| FMEA and SEP integration, including incident counts feeding RPNs | https://www.hbkworld.com/en/products/software/reliability/xfracas-failure-reporting-analysis-corrective-action-system-fracas-software/key-features | not planned v0.2.0: SEP integration not in plan |
| XML import and export with scheduled Windows service import; BOM via Excel xlsx or XFMEA and RCM++ sync | https://www.hbkworld.com/en/products/software/reliability/xfracas-failure-reporting-analysis-corrective-action-system-fracas-software/key-features | not planned v0.2.0: scheduled service import not in plan |
| Web architecture (Windows Server, .NET, IIS, SQL Server or Oracle, SSO) | https://www.hbkworld.com/en/products/software/reliability/xfracas-failure-reporting-analysis-corrective-action-system-fracas-software/key-features | out of scope: hosted web service stack |

## Synthesis platform layer

Source pages opened: https://www.hbkworld.com/en/products/software/reliability and https://help.reliasoft.com/xfmea_rcm_rbi20/importing_exporting_projects.htm and https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm

| Feature | Source URL | Disposition |
| --- | --- | --- |
| Project and folio organization inside a Synthesis database | https://help.reliasoft.com/xfmea_rcm_rbi20/importing_exporting_projects.htm | have in v0.2.0: GUI project file |
| Import and export projects between databases (.rsr20 standard, .rserp enterprise), with resources and FMEAs travelling with the project | https://help.reliasoft.com/xfmea_rcm_rbi20/importing_exporting_projects.htm | have in v0.2.0: GUI project file (ReliaKit own format); ReliaSoft .rsr20 and .rserp formats themselves are not planned |
| QCP quick calculation pad (probability, life and rate calculations) | https://help.reliasoft.com/weibull23/content/weibull_standard_folio_control_panel.htm | have in v0.2.0: quickcalc (QCP) |
| Plot types including probability, reliability and overlay plots with Plot Viewer | https://www.hbkworld.com/en/products/software/reliability/blocksim-system-reliability-availability-maintainability-ram-analysis-software/key-features | limited: v0.1.0 has probability plots and reliability curves, no overlay plots and no separate Plot Viewer |
| Import and export (Excel, CSV, XML depending on product) | https://help.reliasoft.com/xfmea_rcm_rbi20/importing_exporting_projects.htm | limited: v0.2.0 has CSV import and export for FMEA, FRACAS, datasets and the component library, JSON system definitions, and the GUI project file; Excel and XML import and export are not built |
| ReliaSoft Cloud (web SaaS for FMEA and platform access) | https://www.hbkworld.com/en/products/software/reliability | out of scope: hosted service |
| Synthesis Enterprise Portal (SEP) publishing and dashboards | https://www.hbkworld.com/en/products/software/reliability/xfmea-failure-mode-effects-analysis-fmea-software/key-features | out of scope: hosted enterprise portal |
| Multi-user database with permissions (enterprise Synthesis) | https://help.reliasoft.com/xfmea_rcm_rbi20/importing_exporting_projects.htm | out of scope: multi-user server database |

Disposition totals (177 rows): have 60, limited 29, not planned v0.2.0 79, out of scope 9.

## Datasets

ReliaKit ships 16 CSV files in `datasets/`. Every file is
transcribed from a documentation page that was opened and read in
full. Nothing in the folder is generated or invented. Each CSV has
comment lines that name the dataset, give the source URL, name the
source example, and record any note about the printed source.
`datasets/README.md` in the repo has the per-file detail. Tests:
`tests/datasets_tests.rs`, 19 tests (one row and column sum test
per file, plus three fit verification tests).

ReliaSoft life data:

| File | Rows | Source |
| --- | --- | --- |
| reliasoft_life_data_example.csv | 10 | https://help.reliasoft.com/weibull22/content/life_data_analysis_example.htm |
| reliasoft_unbiasing_complete.csv | 10 | https://help.reliasoft.com/articles/content/hotwire/issue109/relbasics109.htm |
| reliasoft_unbiasing_suspended.csv | 10 | https://help.reliasoft.com/articles/content/hotwire/issue109/relbasics109.htm |
| reliasoft_beta_shape_example.csv | 20 | https://help.reliasoft.com/articles/content/hotwire/issue110/hottopics110.htm |

ReliaSoft ALT:

| File | Rows | Source |
| --- | --- | --- |
| reliasoft_alt_tensile.csv | 15 | https://help.reliasoft.com/articles/content/additional_examples/alta/tensile_components.html |
| reliasoft_alt_circuit_boards.csv | 29, expanded from grouped counts | https://help.reliasoft.com/articles/content/additional_examples/alta/circuit_boards_example.html |
| reliasoft_alt_paper_clips.csv | 18 | https://help.reliasoft.com/articles/content/additional_examples/alta/paper_clip_example.html |
| reliasoft_alt_two_stress.csv | 12 | http://help.synthesisplatform.net/weibull_alta8/alta_two-stress_example.htm |

ReliaSoft degradation:

| File | Rows | Source |
| --- | --- | --- |
| reliasoft_degradation_turbine_blades.csv | 15 | http://help.synthesisplatform.net/weibull_alta8/weibull_degradation_analysis_example.htm |
| reliasoft_degradation_mosfet.csv | 175 | https://help.reliasoft.com/articles/content/reference_examples/weibull/degradation_data_analysis_with_a_power_regression_model.html |
| reliasoft_alt_degradation_led.csv | 50 | https://help.reliasoft.com/weibull22/content/alta_degradation_analysis_example.htm |

ReliaSoft growth:

| File | Rows | Source |
| --- | --- | --- |
| reliasoft_growth_crow_amsaa.csv | 27 | https://reliawiki.com/index.php?title=Failure_Times_Crow-AMSAA_Example&oldid=48924 |
| reliasoft_growth_test_fix_test.csv | 23 | https://help.reliasoft.com/articles/content/reference_examples/rga/test-fix-test_data_reference_example.html |

ReliaSoft warranty:

| File | Rows | Source |
| --- | --- | --- |
| reliasoft_warranty_nevada.csv | 6 | https://help.reliasoft.com/reference/life_data_analysis/lda/warranty_data_analysis.html |
| reliasoft_warranty_time_to_failure.csv | 4 grouped rows | https://help.reliasoft.com/reference/life_data_analysis/lda/warranty_data_analysis.html |

Textbook (Meeker and Escobar, Statistical Methods for Reliability
Data, via the SMRD package):

| File | Rows | Source |
| --- | --- | --- |
| smrd_bearingcage.csv | 25 grouped rows, 1703 assemblies, 6 failures | https://rdrr.io/github/Auburngrads/SMRD/src/inst/test_objs/wqmmlesss/bearingcage.R |

### Fit verification

Printed page values against ReliaKit `fit_mle` for weibull2.
Tolerance used in tests: beta within 0.002, eta within 0.005.
Three fits reproduce the printed values:

| Dataset | Printed beta / eta | Fitted beta / eta | Result |
| --- | --- | --- | --- |
| Life data example | 1.5070 / 128.6414 | 1.5069 / 128.6430 | Pass |
| Unbiasing, complete data | 1.7255 / 103.8609 | 1.7255 / 103.8606 | Pass |
| Unbiasing, suspended data | 1.7670 / 100.2278 | 1.7669 / 100.2279 | Pass |

The eta gap of 0.0016 on the life data example is larger than last
digit rounding and looks like optimizer convergence in one of the
two tools. It is far below any transcription effect.

Beta shape example: shipped as transcribed, fit not reproduced.
The page prints beta 0.9812, eta 73.6036 and does not state the
estimation method. ReliaKit MLE gives beta 1.2280, eta 65.3757.
ReliaKit RRX gives beta 0.7394, eta 165.7638. A hand RRY check
gives beta 0.7468, eta 162.3489. None reproduces the printed
values. The printed subgroup fits on the same page also sit
between the MLE and rank regression values, so the page most
likely used a rank method with suspension-adjusted ranks, which
ReliaKit does not implement. The transcription is cross-validated
by Tables 2 and 3 on the same page, which re-list the same 20
rows as subgroups and match the file exactly. The tests make no
fit assertion for this file, only row and sum checks.

Circuit boards source inconsistency: the page text says 27
circuit boards, but the printed grouped counts sum to 29 units
(19 at 463 K, 10 at 488 K). The file ships the table as printed
(29 rows) and the CSV comments and `datasets/README.md` flag the
inconsistency.

Bearing cage note: the SMRD source factor codes are 1 = Censored
and 2 = Failed. They are converted to failed 1 or 0 in the file.
Weighted totals reproduce the documented 1703 assemblies, 6
failures, 1697 censored.

### Attempted but not shipped

- RGA Failure Times Data example on help.reliasoft.com (40
  failures, DMTBF 166.2217), opened in full at
  https://help.reliasoft.com/weibull20/example_using_the_crow-amsaa_model.htm .
  The 40 failure times appear only inside a screenshot image, not
  as page text, so they cannot be transcribed exactly. Not
  shipped. The ReliaWiki version of the Crow-AMSAA failure times
  example (27 failures, 300 hours) prints its table as text and
  was shipped instead.
- Warranty Dates of Failure example (same warranty reference
  page): the sales table (20 rows) and returns table (18 rows)
  are printed in full, but the native shape is calendar dates,
  which does not fit the numeric import formats used by the other
  files. Not shipped. The Nevada chart and time-to-failure
  examples from the same page cover the warranty category.
- SMRD lzbearing (Lieblein and Zelen ball bearing fatigue, 23
  complete times in megacycles): the SMRD echapter source renders
  the values only through a JavaScript data table, so the values
  are not printed in the page text that can be opened. Not shipped
  rather than copied from memory.
- SMRD printedcircuitboard (Meeker and LuValle PCB accelerated
  test): interval censored counts in 140 interval rows. The
  documentation pages opened describe the structure but do not
  print the 140 rows. Not shipped.
- ALTA cumulative damage step-stress reference example
  (help.reliasoft.com reference examples, Nelson cable insulation
  data): opened via search results. The data are six step-stress
  profiles plus a results table that does not fit the flat ALT
  import shape. Not shipped.
- RGA Grouped Data Crow Extended example (ReliaWiki): seen in
  search results with a grouped table including classification
  and mode columns. Not opened in full and the shape is a
  classification table rather than a plain growth event table.
  Not shipped.

## Standards

### MIL-HDBK-217F Notice 2 library

The library is in `libraries/mil217/` (one JSON file per category,
plus INDEX.md there). The Rust module is `src/standards217.rs`.
Tests: `tests/standards217_tests.rs`, 8 tests. Every number in
the data files comes from the handbook PDFs named below. No
SR-332, IEC 62380, or ReliaSoft data was used in any form.

Source PDFs (kept with the research notes, not in the repo):

1. Notice 2 (primary source). File MIL-HDBK-217F_NOTICE-2.pdf,
   downloaded from Everyspec:
   https://everyspec.com/MIL-HDBK/MIL-HDBK-0200-0299/download.php?spec=MIL-HDBK-217F_NOTICE-2.014590.PDF .
   Size 13,062,097 bytes, 80 PDF pages, PDF version 1.3. Cover:
   MIL-HDBK-217F, NOTICE 2, 28 February 1995, MILITARY HANDBOOK,
   RELIABILITY PREDICTION OF ELECTRONIC EQUIPMENT. Contents: the
   Notice 2 change notice and all of the pages Notice 2 issued:
   Sections 9 and 10 complete in their new condensed form,
   Sections 11 to 17 complete as revised, front matter and table
   of contents, selected replacement pages for Sections 5, 6 and
   7, and Appendix A and C pages.
2. Base handbook (for sections Notice 2 did not revise). File
   MIL-HDBK-217F_base_Quanterion.pdf, downloaded from Quanterion:
   https://www.quanterion.com/wp-content/uploads/2014/09/MIL-HDBK-217F.pdf .
   Size 27,858,510 bytes, 205 PDF pages. Cover: MIL-HDBK-217F,
   2 DECEMBER 1991, SUPERSEDING MIL-HDBK-217E, Notice 1,
   2 January 1990. Used only for Sections 19, 21, 22 and 23,
   which do not appear in the Notice 2 list of revised pages, so
   their base pages are the pages in force under Notice 2. Each
   JSON file from this source says so in its source_pdf field.
   Pages revised by Notice 2 were never sourced from this file.

Notice 2 condensed Sections 9 and 10 finding: Notice 2
(28 February 1995) replaced Sections 9 (Resistors) and 10
(Capacitors) of Revision F with condensed unified models: a
single Section 9.1 (handbook pages 9-1 to 9-3, superseding pages
9-1 to 9-29) and a single Section 10.1 (pages 10-1 to 10-6,
superseding pages 10-1 to 10-32). Evidence: the change list table
on PDF page 1 of the Everyspec Notice 2 file ("9-1 through 9-3"
new pages supersede "9-1 through 9-29", and "10-1 through 10-6"
supersede "10-1 through 10-32"), the footers of the condensed
pages, and a cross-check against the IHS combined reprint of
MIL-HDBK-217F with Notice 2 (used as a cross-check only, not as a
source). Consequence: the detailed per subtype models of the base
handbook are superseded and were not transcribed. resistors.json
and capacitors.json hold the Notice 2 condensed models, which are
the current ones.

Section numbering note: the handbook table of contents is 17
Connections, 18 Meters, 19 Quartz Crystals, 20 Lamps, 21
Electronic Filters, 22 Fuses, 23 Miscellaneous Parts. The data
files use the handbook numbering.

Categories built (18, all coverage full). PDF pages are pages in
the source PDF named in each file. Handbook pages are the printed
page numbers.

| File | Section | Handbook pages | PDF pages | Source |
| --- | --- | --- | --- | --- |
| resistors.json | 9.1 | 9-1 to 9-3 | 32 to 34 | Notice 2 |
| capacitors.json | 10.1 | 10-1 to 10-6 | 35 to 40 | Notice 2 |
| transformers.json | 11.1 | 11-1 | 41 | Notice 2 |
| coils.json | 11.2 | 11-3 | 43 | Notice 2 |
| synchros_and_resolvers.json | 12.2 | 12-4 | 48 | Notice 2 |
| elapsed_time_meters.json | 12.3 | 12-5 | 49 | Notice 2 |
| relays_mechanical.json | 13.1 | 13-1, 13-2 | 50, 51 | Notice 2 |
| relays_solid_state.json | 13.2 | 13-3 | 52 | Notice 2 |
| switches.json | 14.1 | 14-1, 14-2 | 53, 54 | Notice 2 |
| circuit_breakers.json | 14.2 | 14-3 | 55 | Notice 2 |
| connectors.json | 15.1 | 15-1, 15-2 | 56, 57 | Notice 2 |
| connector_sockets.json | 15.2 | 15-3 | 58 | Notice 2 |
| interconnection_assemblies_pth.json | 16.1 | 16-1 | 59 | Notice 2 |
| connections.json | 17.1 | 17-1 | 63 | Notice 2 |
| quartz_crystals.json | 19.1 | 19-1 | 170 | base |
| electronic_filters.json | 21.1 | 21-1 | 172 | base |
| fuses.json | 22.1 | 22-1 | 173 | base |
| miscellaneous_parts.json | 23.1 | 23-1, 23-2 | 174, 175 | base |

Implementation notes:

- `standards217` has LambdaBTable (axes plus grid) with lambda_b()
  doing bilinear interpolation on two axis tables and linear
  interpolation on one axis series. Exact grid points return
  exact printed values. Blank grid entries give NaN.
  part_failure_rate(category, inputs) evaluates lambda_p =
  lambda_b times the product of the pi factors, from a map of
  factor name to numeric or named input. pi_E is looked up by
  environment name (GB, GF, and so on) and pi_Q by the quality
  names the handbook prints. Handbook formulas are implemented as
  helpers: arrhenius_pi_t, resistor_pi_p, resistor_pi_s,
  capacitor_pi_c, capacitor_pi_v, load_stress_pi_l, switch_pi_c,
  socket_pi_p, pth_pi_c, quartz_lambda_b, relay_lambda_b_formula,
  relay_pi_cyc, connector_delta_t.
- Section 16.1 is not a plain product model. Its printed equation
  lambda_p = lambda_b * [N1 * pi_C + N2 * (pi_C + 13)] * pi_Q *
  pi_E is implemented as pth_failure_rate.
- Section 23.1 is a fixed rate list, stored as such in
  miscellaneous_parts.json. The generic product function does not
  apply to it.
- Synchros: the formula printed with the lambda_b table does not
  reproduce the table at spot values, so the library stores and
  uses the printed table. This is recorded in the JSON notes.

Spot checks: values were extracted with pdftotext -layout per
page, and the pages listed as image checked in the research notes
were also rendered at 150 dpi and read back as images. The PDF
value equals the file value in every spot check recorded (resistor,
capacitor, transformer, coil, relay, switch, circuit breaker,
connector, socket, interconnection, connection, synchro, elapsed
time meter, quartz crystal and fuse values, plus the Section 10.2
worked example: handbook result lambda_p = 0.049
failures/10^6 hours, library result 0.04898 from the printed
factors). OCR traps caught by the image checks: resistor pi_E for
CL is 1728 (OCR read 1720), and the switch pi_E value for AUC is
13 as printed, lower than AIF. Electronic filters and
miscellaneous parts were checked by pdftotext only, with no page
image read.

Sections partial or skipped, with reasons:

- Section 5, Microcircuits: partial in the Notice 2 PDF, not
  built. The Notice 2 PDF contains only replacement handbook
  pages 5-3, 5-4, 5-5, 5-6, 5-9, 5-10, 5-23 and 5-24 (PDF pages
  20 to 27). The pi_T table (page 5-13), the C2 table (page
  5-14) and the pi_E, pi_L and pi_Q tables (page 5-15) are not
  in this PDF, so no complete microcircuit model can be computed
  from it alone.
- Section 6, Discrete Semiconductors: partial in the Notice 2
  PDF, not built. The Notice 2 PDF contains only handbook pages
  6-1 and 6-2 (PDF pages 28 and 29). The models continue on
  pages Notice 2 did not reissue.
- Section 7, Tubes: skipped per scope. Only replacement pages
  7-3 and 7-4 are present.
- Section 8, Lasers: skipped per scope. Not in the Notice 2 PDF.
- Section 12.1, Motors (PDF pages 45 to 47, handbook pages 12-1
  to 12-3): not built as a pi model. It is a Weibull
  characteristic life model (alpha_B and alpha_W tables by
  ambient temperature, A and B constants by motor type, an LC
  ratio table), not lambda_b times pi factors.
- Section 16.2, Surface Mount Technology (PDF pages 60 to 62,
  handbook pages 16-2 to 16-4): not built. It is a thermal
  cycling fatigue (Weibull) calculation, not a lambda_b product
  model. Its worked example on page 16-4 gives lambda_SMT = 0.4
  failures/10^6 hours for its stated inputs.
- Section 18, Meters: skipped per scope.
- Section 20, Lamps: skipped per scope.
- Appendix A, Parts Count: not built. It is outside the part
  stress scope of this library.

### NSWC

NSWC is not included and was not started. The MIL-HDBK-217F
scope (locating the genuine Notice 2 pages, confirming the
condensed Sections 9 and 10, transcribing and image checking 18
categories, and building and testing the Rust module) used the
time available in that research stream. No NSWC data was
downloaded or transcribed, and nothing was reproduced from memory
in its place.

### FIDES

FIDES is not built. Check only, with no FIDES tables built:

- The genuine guide is on the FIDES group official site.
  Pages opened November 5, 2026: https://www.fides-reliability.org/
  (states FIDES guide 2022 Edition A is now available in English,
  accounts validated manually by administrator) and
  https://www.fides-reliability.org/en/node/6 (guide page
  describing the two parts: reliability prediction guide and
  reliability process control and audit guide, covering
  electrical, electronic and electromechanical components and
  PWAs and subassemblies, mission profile, overstresses and
  process factors, in French and English). The guide page for
  FIDES Guide 2022 Edition A (English) at
  https://www.fides-reliability.org/en/node/1205 lists the file
  FIDES_Guide_2022_Edition_A_(July 2023).pdf.
- Outcome: the guide is free of charge from the official site,
  but the download is behind site registration and login. The
  site states that accounts are validated manually by an
  administrator and can take several days. Access is registration
  gated, not payment gated. The guide was not downloaded in this
  research, and no FIDES model or table is built.

## Evaluated and declined

(a) Telcordia SR-332: no free release from the rights holder; distributed through Telcordia/iconectiv document store; full-text copies online are third-party uploads of a copyrighted document. Declined as a data source; method-only stands.

(b) IEC TR 62380: sold through the IEC webstore; the document front matter states copyright all rights reserved; online copies are third-party uploads. Declined; method-only stands.

(c) ReliaSoft compiled component libraries: no open release found. Declined; ReliaKit builds its own libraries from the public standards documents instead.

## Clean-room methods sources

This section is the audit trail for the SR-332 and IEC 62380 formulas. Only sources actually opened are logged as sources. Anything no opened source pins down is flagged Limited or "no opened source" and must not be silently assumed by the library stream.

### SR-332 Method I: black-box formula and 0.35 eV default

- ICE Components, "Failure Rate Calculation" (ICE Components, Inc., document published 13/12/2019, SR-332 Issue 3). Opened: https://www.icecomponents.com/wp-content/uploads/2023/10/FailureRateCalculation.pdf . States: mean black box steady-state failure rate lambda_BBi = lambda_Gi x pi_Qi x pi_Si x pi_Ti, where lambda_Gi is the mean generic steady-state rate (Section 8), pi_Qi is the quality factor (Section 9.3, 1.0 in the sheet), pi_Si is the electrical stress factor (Section 9.2, based on percent electrical stress; if unknown use 1, assumes 50 percent stress), pi_Ti is the temperature factor (Section 9.1). MTBF = 10^9 / FITs. There is no pi_E at device level in this form. Device-specific note in the sheet: magnetic components use pi_Ti = 2.0 at 85C with Ea = 0.15 eV, which is not the general default.
- Relyence, "A Guide to Reliability Prediction Standards and Failure Rate" (Relyence, 2018-07-16). Opened: https://relyence.com/2018/07/16/guide-reliability-prediction-methods/ . States: lambda_BB = lambda_G x pi_Q x pi_S x pi_T and sigma_BB = sigma_G x pi_Q x pi_S x pi_T, with lambda_BB in FITs (failures per 1e9 hours); pi_Q is quality, pi_S is electrical stress, pi_T is temperature stress. States separately: the pi_E environmental factor is factored into the overall failure rate calculation, and unit and system rates are calculated from device rates. Also states Telcordia can compute an upper confidence level and can integrate lab data and field data with or without burn-in.
- ReliaSoft article reprint hosted by Fermilab, "MIL-217, Bellcore/Telcordia and Other Reliability Prediction Methods for Electronic Products" (Prenscia HBK / ReliaSoft, reprinted with permission). Opened: https://ad.fnal.gov/EPG/localincludes/Reliability_explained.pdf . States: SR-332 assumes a serial model and addresses infant mortality and steady-state rates with Methods I, II and III; Method I is similar to MIL-HDBK-217F parts count and part stress; the standard provides generic rates and three part stress factors: pi_Q, pi_S and the temperature factor. Worked SR-332 Issue 3 capacitor example in the article: 9.655 FITs.
- Broadcom / Avago, "AFBR-5725PZ Reliability Data Sheet" AV02-3440EN (Broadcom, April 3, 2012, Telcordia SR-332 Issue 3, Parts Count). Opened: https://docs.broadcom.com/wcs-public/products/quality--reliability/reliability-data-sheet/928/966/av02-3440en.pdf . States: module failure rate is the sum of device component failure rates; modifiers cover environment, quality, temperature and stress; header factors Temperature Factor at 40C = 1, Stress Factor at 50 percent = 1, Environmental Factor = 1; total 105.0 FITs at 40C, MTTF 9.53E+06 hours. Key sentence: "FITs at other temperatures can be derived following the procedure of Telcordia SR-332, assuming activation energy, Ea, of 0.35eV to determine the component temperature factor pi_T." Gives a FITs versus Tcase table from 40C (105.0) to 85C (535.3). This sheet does not print the Arrhenius expression for pi_T itself.
- Broadcom / Avago, "AFBR-700SDZ Reliability Data Sheet" AV02-2778EN (Avago Technologies, December 24, 2010, Telcordia SR-332 Issue 2, Parts Count). Opened: https://docs.broadcom.com/wcs-public/products/quality--reliability/reliability-data-sheet/41/534/av02-2778en.pdf . States the same structure: module rate is the sum of device rates, modifiers cover operation environment, temperature and stress, Temperature Factor at 40C = 1, Stress Factor at 50 percent = 1, Environmental Factor = 1, total 138.1 FITs at 40C. Same key sentence: FITs at other temperatures follow SR-332 assuming activation energy Ea of 0.35 eV to determine pi_T. This is a second independent vendor citation of the 0.35 eV default.
- TDK-Lambda, "CUS800M Reliability Data" CA969-57-01C (TDK-Lambda). Opened: https://product.tdk.com/system/files/dam/doc/product/power/switching-power/ac-dc-converter/reliability_data/cus800m_rel.pdf . States the equipment level form explicitly: lambda_equip = pi_E x sum over i of (Ni x lambda_SSi), with lambda_SSi = lambda_Gi x pi_Qi x pi_Si x pi_Ti; lambda_Gi is the generic failure rate, pi_Qi quality, pi_Si stress, pi_Ti temperature, Ni quantity, pi_E the equipment environmental factor; MTBF = 1 x 10^9 / lambda_equip hours; calculated per SR-332 Issue 3, Method I, Quality Level II context, environmental factor GB (Ground, Benign). Example values: CUS800M-12 MTBF about 425,854 hours at Ta = 40C.
- Quanterion, "Making Sense Out of the Reliability Prediction Business" (Quanterion Solutions). Opened: https://www.quanterion.com/making-sense-out-of-the-reliability-prediction-business-mil-hdbk-217-bellcore-rdf-2000-prism/ . States for the most widely used Method I case: lambda_SSi = lambda_Gi x pi_Qi x pi_Si x pi_Ti; the Bellcore approach covers five use environments, assumes an exponential distribution and reports FITs (failures per billion part operating hours); Method I includes a first year modifier for infant mortality.

Synthesis for Method I: the five-factor product in the task wording (generic rate x pi_Q x pi_S x pi_T x pi_E) is supported by opened sources only in split form: the device rate is lambda_G x pi_Q x pi_S x pi_T, and pi_E is applied once at unit or equipment level as lambda_equip = pi_E x sum(Ni x lambda_SSi), per TDK-Lambda, Relyence and Broadcom above. No opened source prints a single device equation containing all five factors multiplied together. Flag: do not implement or document a per-device five-factor product as if it were verbatim from a source. The 0.35 eV default activation energy for pi_T is sourced (Broadcom AV02-3440EN and AV02-2778EN). The Arrhenius expression for pi_T itself (the exponential form in Ea and temperatures) has no opened source in this log: flag "no opened source" for the pi_T closed form; only the Ea value and the factor role are sourced.

### SR-332 Method II (laboratory data) and Method III (field data)

- Isograph, "Telcordia SR-332" (Isograph, Reliability Workbench Prediction Module). Opened: https://www.isograph.com/software/reliability-workbench/prediction-software/telcordia-sr-332/ . States only: Method I is Parts Count; Method II is combining laboratory test data with Parts Count data; Method III is combining field tracking data with Parts Count data. Also describes the First Year Multiplier (FYM) for first-year and wear-in rates as a multiplier on the steady-state rate, influenced by burn-in times and temperatures at system, unit and device level and auto-calculated in the module. No equations, weights or eligibility criteria are given.
- ALD, "MIL-HDBK-217 vs Telcordia SR-332" (ALD, aldservice.com; states SR-332 latest Issue 4, March 2016). Opened: https://aldservice.com/Which-Reliability-Prediction-Standard-Is-Better-MIL-HDBK-217-vs.-Telcordia-SR-332.html . States only: Method I is Black Box (no lab or field data); Method II is Black Box Integrated with Laboratory Data; Method III is Black Box Integrated with Field Data; Methods II and III give more realistic predictions and account for burn-in and quality. No formulas.
- Relyence guide (same opened URL as above). States: predictions can integrate laboratory data and field data from devices, units or systems, with or without burn-in; predictions are weighted according to the amount of test data available; field adjustment makes predictions reflect product performance; the extra data is optional and analyses often start as black-box and update as lab, field and burn-in data arrive. No weighting formula, prior or case structure is given.
- Fermilab / ReliaSoft article (same opened URL as above). States: Method II is based on combining Method I predictions with data from laboratory tests performed in accordance with specific SR-332 criteria. Method III is a statistical prediction based on field tracking data collected in accordance with specific SR-332 criteria, and in Method III the predicted failure rate is a weighted average of the generic steady-state failure rate and the field failure rate. No weights and no criteria details are given.
- Quanterion article (same opened URL as above). States: Method II includes a Bayes weighting procedure covering three approaches depending on the level of previous burn-in the part or unit has undergone. Method III includes a Bayes weighting procedure based on three different cases depending on how similar the equipment is to the equipment from which the data was collected. This is the only opened source that names the Bayes weighting structure and the case discriminators (burn-in level for II, similarity for III). No numerical weights, priors or formulas are given.

Limited flags for Methods II and III (details no opened source above pins down): the exact Bayesian weighting or prior form for Method II or III; the numerical case weights; the Method II case labels and thresholds (search snippets use L1 to L4 style labels, no opened source confirms them); the effective time on test definition and the temperature acceleration conversion for lab data; the minimum data quantity and quality criteria (sources reference "specific SR-332 criteria" without enumerating them); the field-data similarity weighting values; how burn-in enters quantitatively beyond the First Year Multiplier being influenced by burn-in time and temperature; upper confidence level and standard deviation updating under Methods II and III; whether Method III combines at device, unit or system level in general (Relyence says data can come from devices, units or systems; the Fermilab article states the weighted average at prediction level only). Any exact Method II or Method III combination formula is therefore flagged "no opened source" in this log.

### IEC 62380 mission profile approach

- ReliabilityEducation, "IEC 62380 Reliability Prediction" (reliabilityeducation.com, paraphrased key concepts of IEC 62380). Opened: https://www.reliabilityeducation.com/intro-iec62380.html . States: thermal cycling during mission profiles replaces the environment factor used by other standards; models support permanent working, on/off cycling and dormant operation; soldering and mounting failure rate is included in the component rate; IEC 62380 does not support parts-count-only prediction because mission profiles are needed for credible results. The mission profile is decomposed into phases on the basis of a typical year: (1) on/off working phases with various average outside temperatures, (2) permanent-working phases with various average outside temperature changes, (3) storage and dormant phases with various average outside temperature changes. Time is counted in calendar hours including working and storage or dormant hours. Per-phase parameters: average outside ambient temperature, average PCB ambient temperature near components, annual time ratios (per-phase permanent working, total permanent working tau_on, total non-working or dormant tau_off), annual number of thermal cycles n_i for the phase, and average thermal swing Delta_T_i. For storage and permanent phases Delta_T is the mean max-min per cycle, values below 3C are treated as 0; typically one cycle per day from daylight and night difference; daily variation is superimposed on permanent and on/off phases but only the greater variation counts.
- Texas Instruments, "Understanding Functional Safety FIT Base Failure Rate Estimates per IEC 62380 and SN 29500", SLOA294A (Texas Instruments, June 2020, revised April 2024). Opened: https://www.ti.com/lit/SLOA294 . Section 6 states: IEC TR 62380 (published 2004, subsequently obsoleted, incorporated into ISO 26262 Part 11) gives the IC failure rate as die rate plus package rate plus EOS rate; the die formula includes IC type and technology, transistor count, the thermal mission profile, junction temperature, and operating and non-operating lifetime (variables described include N transistor count by type, lambda_1 type scale factor, lambda_2 technology base FIT, alpha manufacture-year factor; the equation itself is a graphic and did not extract as text); the package formula includes thermal expansion mismatch, thermal cycles, the thermal mission profile, and package type and materials (including pi_alpha expansion difference and lambda_3 package scale factor); EOS defaults to 0 unless the part is listed for an application with an external interface. Table 6-1 gives example automotive mission profiles with per-phase ambient temperatures and time ratios, on/off ratios ton/toff, annual thermal cycles n and swings Delta_T per start type (for example Motor Control at about 500 working hours per year, 4 daytime starts, 2 nighttime starts, 30 days per year non-use). Section 10 states the IEC 62380 base failure rate accounts for tau_ON and tau_OFF, unlike SN 29500 which uses tau_W. TI states system integrators must refer to the IEC 62380 standard itself for the data needed, so TI does not reproduce the full tables or formulas.
- Texas Instruments, "IEC TR 62380 FIT Rate Scaling for different mission profile", D. Tatman, 06/2020 (Texas Instruments, TI E2E). Opened: https://e2e.ti.com/cfs-file/__key/communityserver-discussions-components-files/388/IEC-TR-62380-failure-rate-mission-profile-scaling.pdf . States: Total FIT = Die FIT + Package FIT + EOS FIT. Die rate includes IC type and technology, transistor count, thermal mission profile, junction temperature, and operating and non-operating lifetime. Package rate includes mechanical stress from thermal expansion, thermal cycles, thermal mission profile, package type and materials. Scaling form shown by worked example: the die mission profile factor is the time-ratio weighted average of per-phase temperature factors, sum over phases of (tau_i x pi_t_i) divided by (tau_on + tau_off), where pi_t_i = exp(A x (1/328 - 1/(273 + Tj_i))), Tj_i is the phase junction temperature (phase ambient plus Delta_Tj), and A is a technology constant (4640 in the HV BiCMOS example). Worked example: Motor Control die FIT 3.5 rescaled to Passenger Compartment 1.4 FIT by the ratio of the two profile factors (0.0659 / 0.1746). Package scaling shown by worked example: package factor is the sum over phases of (Delta_T_i^0.68 x pi_n_i), where pi_n_i = n_i^0.76 (for example 670^0.76 = 140.5), and Delta_T_i for an on/off phase is Delta_Tj/3 plus the phase thermal swing; Motor Control package FIT 1.9 rescaled to Passenger Compartment 1.3 FIT by the ratio 3764.5 / 5741.6. This presentation is the only opened source that shows the phase weighting arithmetic in numbers.
- EnCo Software SOX Help, "Mission Profiles" (EnCo Software). Opened: https://enco-software.atlassian.net/wiki/spaces/SUD/pages/3440584192/Mission+Profiles . States: mission profiles apply to IEC 62380 only; a profile has any number of working phases and is broken into homogeneous phases on a typical year basis with the same three phase kinds (on/off, permanent, storage or dormant); time is calendar hours including dormant time; per-phase inputs are phase type (ON_OFF, Permanent, Dormant), t_ae average outside environmental temperature for phase i, n annual thermal cycles for phase i, and Delta_T average thermal swing for phase i; the profile is assigned in FIT Details for IEC 62380 catalogs.

Limited flags for IEC 62380: no opened source reproduces the full per-component failure rate formulas in symbolic form for all component families (the TI SLOA294A die equation is a graphic that did not extract, and component lambda_0 tables, pi factor tables and cycling damage exponents beyond the 0.68 and 0.76 exponents visible in the TI scaling example are not in any opened source); the general phase combination equation is sourced only through the TI scaling example arithmetic, not as a stated general formula; dormant-phase treatment (whether dormant phases contribute cycling damage only or also a base rate) is described qualitatively only. Those items are flagged Limited / "no opened source" for the full symbolic forms, consistent with method-only for IEC 62380.

Note on the Arrhenius closed form: the exponential form of pi_T
in activation energy and temperature has no opened SR-332 source
in the log above. Only the 0.35 eV default value and the factor
role are sourced from SR-332 material. The closed form used in
ReliaKit is the standard Arrhenius relation, and it is the form
the crate ALT module uses for its temperature acceleration factor.
It is not presented here as a quotation from SR-332.

