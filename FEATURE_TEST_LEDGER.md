# ReliaKit Rust: Feature Test Ledger

Every feature in the crate, the test file that covers it, and the
result in the final full test run: 167 passed, 0 failed. Totals are
in the README.

Status meanings:

- PASS: the named test exists and passes in the final gate run.
- NO TEST: no test for this feature was found in tests/.
- IGNORED: the test exists but is marked ignored in its file, with
  the reason recorded in a comment at the test.

| Feature | Test file | Test name(s) | Status |
| --- | --- | --- | --- |
| Distribution: Weibull (2P) | tests/ported_tests.rs | test_weibull_reliability_at_eta, test_mle_recovers_weibull | PASS |
| Distribution: Weibull (3P) | tests/qa_tests.rs | qa_weibull3_fit_gamma_below_min_time | PASS |
| Distribution: Lognormal | tests/coverage_tests.rs | lognormal_known_values_and_mle | PASS |
| Distribution: Normal | tests/coverage_tests.rs | normal_known_values_and_mle | PASS |
| Distribution: Exponential | tests/ported_tests.rs | test_weibull_reliability_at_eta, test_cold_standby_exponential | PASS |
| Distribution: Gamma | tests/qa_tests.rs | qa_gamma_cdf_derivative_matches_pdf, qa_gamma_cdf_at_mean_in_range | PASS |
| Distribution: Gumbel | tests/qa_tests.rs | qa_gumbel_cdf_quantile_roundtrip, qa_gumbel_pdf_integrates_to_one | PASS |
| MLE fitting | tests/ported_tests.rs | test_mle_recovers_weibull, test_censored_fit_and_bounds | PASS |
| Rank regression | tests/ported_tests.rs | test_rank_regression, test_median_ranks | PASS |
| Fisher bounds | tests/ported_tests.rs | test_censored_fit_and_bounds | PASS |
| Kaplan-Meier | tests/ported_tests.rs | test_kaplan_meier | PASS |
| Bias correction | tests/ported_tests.rs | test_bias_correction_exact | PASS |
| Shrinkage (normal-normal empirical Bayes) | tests/ported_tests.rs | test_shrink_normal | PASS |
| Hierarchical empirical Bayes | tests/ported_tests.rs | test_hierarchical_eb | PASS |
| Conformal: reliability lower bound | tests/ported_tests.rs, tests/qa_tests.rs | test_conformal_zero_failures, qa_specfunc_beta_ppf_closed_form | PASS |
| Conformal: mission time | tests/ported_tests.rs | test_conformal_mission_time_monotone | PASS |
| MCMC, single distribution | tests/ported_tests.rs | test_mcmc_weibull_recovery | PASS |
| MCMC, hierarchical | tests/ported_tests.rs | test_mcmc_hierarchical_shape | PASS |
| Degradation: linear path | tests/ported_tests.rs | test_degradation_crossing, test_degradation_end_to_end | PASS |
| Degradation: exponential path | tests/qa_tests.rs | qa_degradation_exponential_pseudo_failure_time | PASS |
| Degradation: power path | tests/coverage_tests.rs | degradation_power_path_crossing | PASS |
| ALT LSQ: Arrhenius | tests/ported_tests.rs | test_alt_fit_recovers_ea, test_arrhenius_af | PASS |
| ALT LSQ: inverse power law | tests/coverage_tests.rs, tests/ported_tests.rs | alt_ipl_lsq_fit_recovers_exponent, test_ipl_af | PASS |
| ALT LSQ: Eyring | tests/qa_tests.rs | qa_alt_eyring_lsq_recovers_b_and_orders_reliability | PASS |
| ALT MLE and Eyring interaction | tests/alt_mle_tests.rs | censored_arrhenius_mle_recovers_ea_and_sigma, ipl_mle_recovers_exponent, eyring_interaction_lsq_recovers_coefficients, eyring_interaction_mle_runs_and_bounds_are_ordered | PASS |
| Competing risks | tests/competing_tests.rs | test_hand_exponential_overall_reliability, test_hand_exponential_cumulative_incidence, test_unknown_mode_is_its_own_category, test_end_to_end_fit_recovers_competing_exponentials | PASS |
| Standby, 2-unit | tests/ported_tests.rs | test_cold_standby_exponential, test_standby_ordering | PASS |
| Standby, n-unit quadrature | tests/standby_tests.rs | exponential_cold_standby_matches_closed_forms, closed_form_two_units_is_poisson_sum, two_unit_weibull_matches_system_quadrature, three_unit_weibull_matches_monte_carlo, ordering_cold_warm_hot_single, guards_single_unit_and_hot_standby | PASS |
| CTMC | tests/ported_tests.rs | test_ctmc_single, test_mc_availability_standby_vs_ctmc | PASS |
| Monte Carlo availability | tests/ported_tests.rs | test_mc_availability_single_block, test_mc_availability_standby_vs_ctmc, test_block_availability | PASS |
| Posterior propagation | tests/ported_tests.rs | test_posterior_system_reliability | PASS |
| RBD JSON | tests/ported_tests.rs | test_nested_rbd, test_series_parallel, test_kofn | PASS |
| Flowchart simulation | tests/flowchart_tests.rs | deterministic_chain, known_mixture, loop_semantics, cost_accumulates, unbounded_loop_errors | PASS |
| FMEA | tests/parity_tests.rs | fmea_rpn_and_revised_rpn, fmea_csv_round_trip_preserves_row_with_comma | PASS |
| Warranty | tests/parity_tests.rs, tests/qa_tests.rs | warranty_nevada_table_aggregation, qa_warranty_forecast_expected_returns_nonnegative_finite | PASS |
| Fault tree | tests/parity_tests.rs, tests/qa_tests.rs | faulttree_gate_probabilities, faulttree_minimal_cut_sets, qa_faulttree_voting_cut_sets_are_the_three_pairs | PASS |
| Prediction, parts count and parts stress | tests/parity_tests.rs, tests/qa_tests.rs | prediction_parts_count, qa_prediction_parts_stress_rate_and_mtbf | PASS |
| DOE, two-level factorial | tests/parity_tests.rs, tests/qa_tests.rs | doe_two_factor_effects, qa_doe_three_factor_effects | PASS |
| Growth: Crow-AMSAA | tests/ported_tests.rs | test_crow_amsaa_hand_calc | PASS |
| Growth: Duane | tests/ported_tests.rs | test_duane | PASS |
| Distribution: Loglogistic | tests/dist_tests.rs | loglogistic_closed_form, loglogistic_mle_recovery, loglogistic_rank_regression_recovery | PASS |
| Distribution: Logistic | tests/dist_tests.rs | logistic_closed_form, logistic_mle_recovery, logistic_rank_regression_not_provided | PASS |
| Distribution: Gompertz | tests/dist_tests.rs | gompertz_closed_form, gompertz_pdf_integrates_against_reliability, gompertz_mle_recovery | PASS |
| Distribution: Generalized gamma | tests/dist_tests.rs | gengamma_closed_form, gengamma_reduces_to_gamma_at_p1, gengamma_reduces_to_weibull_at_d_eq_p, gengamma_mle_recovery | PASS |
| Distribution: Weibull (1P) | tests/dist_tests.rs | weibull1_closed_form, weibull1_eta_closed_form, weibull1_fit_mle_requires_fixed_beta | PASS |
| Distribution: Exponential (2P) | tests/dist_tests.rs | exponential2_closed_form, exponential2_mle_closed_form | PASS |
| Distribution: Mixed Weibull (2 subpopulations) | tests/dist_tests.rs | mixed_weibull_closed_form, mixed_weibull_mle_recovery | PASS |
| Distributions file total (tests/dist_tests.rs) | tests/dist_tests.rs | 20 tests | PASS |
| Distribution wizard, best-fit ranking | tests/expansion_tests.rs | wizard_weibull_ranks_first_on_weibull_data, wizard_anderson_darling_hand_case | PASS |
| QCP (quick calculation pad) | tests/expansion_tests.rs | quickcalc_weibull_hand_values | PASS |
| Reliability importance (Birnbaum, criticality, Fussell-Vesely) | tests/expansion_tests.rs | importance_series_hand_values, importance_parallel_hand_values, importance_nested_and_default_names | PASS |
| DOE, fractional factorial | tests/expansion_tests.rs | doe_fractional_factorial_2_5_minus_2 | PASS |
| DOE, Plackett-Burman | tests/expansion_tests.rs | doe_plackett_burman_12_orthogonal, doe_plackett_burman_20_and_24 | PASS |
| DOE, Taguchi arrays and signal to noise | tests/expansion_tests.rs | doe_taguchi_arrays_orthogonal, doe_signal_to_noise_hand_values | PASS |
| DOE, response surface (CCD, Box-Behnken, quadratic fit) | tests/expansion_tests.rs | doe_ccd_recovers_exact_quadratic, doe_box_behnken_shape | PASS |
| Growth: Gompertz | tests/expansion_tests.rs | growth_gompertz_recovers_parameters | PASS |
| Growth: Lloyd-Lipow | tests/expansion_tests.rs | growth_lloyd_lipow_recovers_parameters | PASS |
| ALT loglinear multi-stress MLE | tests/expansion_tests.rs | alt_loglinear_weibull_recovers_b1_and_monotone_use_reliability | PASS |
| ALT step-stress | tests/expansion_tests.rs | alt_step_stress_one_step_equals_plain_weibull, alt_step_stress_two_step_hand_example | PASS |
| Expansion file total (tests/expansion_tests.rs) | tests/expansion_tests.rs | 18 tests | PASS |
| Markov diagrams (steady state, MTTF, transient) | tests/hbk_tests.rs | markov_steady_state_availability_matches_balance_equations, markov_mttf_matches_closed_form_650, markov_expected_time_in_state_sums_to_mttf, markov_transient_rows_sum_to_one | PASS |
| Reliability allocation (equal, weighted) | tests/hbk_tests.rs | allocation_product_equals_target, allocation_weighted_doubling_weight_lowers_that_target | PASS |
| Derating check | tests/hbk_tests.rs | derating_arithmetic_and_thresholds | PASS |
| Component library file format, local import | tests/hbk_tests.rs | library_csv_round_trip_and_hand_sum, example_library_file_loads | PASS |
| RCM worksheet (task selection, criticality) | tests/hbk_tests.rs | rcm_select_task_decision_table, rcm_criticality_arithmetic | PASS |
| FRACAS incident log | tests/hbk_tests.rs | fracas_aggregation_counts, fracas_mtbf_arithmetic, fracas_csv_round_trip | PASS |
| FMEA types, FMECA criticality, control plan, DVP and R | tests/hbk_tests.rs | fmea_worksheet_round_trip_keeps_type_and_rows, fmea_control_plan_only_rows_with_actions, fmea_dvpr_csv_round_trip, fmea_criticality_arithmetic | PASS |
| SR-332 clean-room methods (Method I, II, III, temperature factor, failure rate CI) | tests/hbk_tests.rs | sr332_temperature_factor_equal_temps_and_arrhenius_value, sr332_method1_product_arithmetic, sr332_equipment_rate_applies_pi_e_once, sr332_method2_extrapolate_factor_ratios, chi2_ppf_known_values, failure_rate_ci_zero_failures_uses_known_two_df_value, sr332_method3_combine_weighted_average | PASS |
| IEC 62380 style mission profile method | tests/hbk_tests.rs | iec62380_style_single_phase_at_ref_temp_returns_base_rate, iec62380_style_hotter_phase_raises_rate, iec62380_style_two_phase_hand_arithmetic, iec62380_style_dormant_phase_uses_dormant_factor | PASS |
| HBK gaps and methods file total (tests/hbk_tests.rs) | tests/hbk_tests.rs | 29 tests | PASS |
| Bundled datasets, row and column sums (16 files) | tests/datasets_tests.rs | life_data_example_rows_and_sums, unbiasing_complete_rows_and_sums, unbiasing_suspended_rows_and_sums, beta_shape_example_rows_and_sums, alt_tensile_rows_and_sums, alt_circuit_boards_rows_and_sums, alt_paper_clips_rows_and_sums, alt_two_stress_rows_and_sums, degradation_turbine_blades_rows_and_sums, degradation_mosfet_rows_and_sums, alt_degradation_led_rows_and_sums, growth_crow_amsaa_rows_and_sums, growth_test_fix_test_rows_and_sums, warranty_nevada_rows_and_sums, warranty_time_to_failure_rows_and_sums, smrd_bearingcage_rows_and_sums | PASS |
| Bundled datasets, fit verification against printed values | tests/datasets_tests.rs | fit_life_data_example_matches_printed_mle, fit_unbiasing_complete_matches_printed_mle, fit_unbiasing_suspended_matches_printed_mle | PASS |
| Datasets file total (tests/datasets_tests.rs) | tests/datasets_tests.rs | 19 tests | PASS |
| MIL-HDBK-217F Notice 2 library (standards217) | tests/standards217_tests.rs | relay_lambda_b_exact_grid_points, synchro_lambda_b_series_exact_point, relay_lambda_b_bilinear_midpoint, resistor_end_to_end_handbook_tables, capacitor_end_to_end_handbook_worked_example, pth_end_to_end_section_16_1, quartz_end_to_end, all_category_files_have_provenance | PASS |
| Standards217 file total (tests/standards217_tests.rs) | tests/standards217_tests.rs | 8 tests | PASS |
| Specfunc: beta quantile closed form | tests/qa_tests.rs | qa_specfunc_beta_ppf_closed_form | PASS |
| Specfunc: normal quantile and cdf round trip | tests/qa_tests.rs | qa_specfunc_norm_ppf_cdf_roundtrip | PASS |
| CLI selftest | src/bin/reliakit.rs | selftest (Selftest subcommand, built-in checks) | PASS |
| GUI headless check | gui/src/main.rs | reliakit-app --headless (builds the Example 3 RBD in the app state engine, prints reliability 0.921419 and availability rows plus the bearing life fit, exits 0; verified by the coordinator in the gate run) | PASS |

Note on the specfunc row: qa_specfunc_norm_ppf_cdf_roundtrip was
briefly ignored because norm_cdf was built on the Abramowitz and
Stegun erf approximation (error 1.28e-6 at x = 2.0 against a 1e-6
tolerance). The coordinator replaced erf and norm_cdf with
incomplete-gamma based versions accurate to about 1e-14; the test
now runs un-ignored with its original tolerance and passes.

Features with no test at all: none. The four gaps QA found
(lognormal, normal, degradation power path, IPL least squares fit)
are covered by tests/coverage_tests.rs.

## v0.3.0 GUI rebuild (Slint)

The desktop app was rebuilt for v0.3.0. The egui front end
(src/bin/reliakit_app.rs, deleted) was replaced by a separate
workspace crate, gui/ (package reliakit-gui, binary reliakit-app):
the interface is written in Slint (declarative files under
gui/ui/), and charts are rendered with plotters into an image
buffer. Slint is used under its royalty-free license
(LicenseRef-Slint-Royalty-free-2.0); the About dialog carries the
AboutSlint widget that license asks for, and the README names the
license. Inter is bundled under the SIL Open Font License 1.1.

The library, the CLI, the datasets, and the project format
(reliakit-project-2, v1 files still open) are unchanged. The full
test run for this version is the same 167 tests:
cargo test --release --workspace reports 167 passed, 0 failed.
The headless check above runs on the new binary and prints the
same Example 3 reliability, 0.921419.

GUI evidence for this version is by screenshot, not unit test.
Screenshots taken under Xvfb at a 1600x1000 window with scripted
XTest input are kept with the GUI test material: the start page,
Life Data with a fit and its reliability chart, the distribution
wizard ranking table, the RBD screen after a palette placement and
a drag with the block selected and its inspector open, an MCMC run
with posterior cards, the Markov example (MTTF 650.00 hours, as
the preset states), and the FRACAS log with its summary cards and
incident inspector. The project tree in the later screenshots
holds five item kinds. The FRACAS to Life Data flow was also
exercised live: it created a Life Data item holding the two
incident hour values as failure times.

Two v0.2.0 caveats are closed by this rebuild. Synthetic clicks
under Xvfb could not select an RBD block in the egui app; in the
Slint app, click-to-select and drag both work and are visible in
the screenshots. The one-line fit summary that overflowed its card
in the egui app renders as a wrapped card that takes its full
height.

Behavior notes. Global keyboard shortcuts from the egui build
(Ctrl+S, Ctrl+O, Ctrl+Z, Delete) did not carry over; save, open,
and undo are buttons. RBD connector lines chain leaves in reading
order; leaves inside a parallel group are drawn on the same line
as their neighbours, and the diagram JSON carries the real
topology. At the default window size the FMEA worksheet table is
wider than the center panel, so its row action buttons are partly
clipped until the window is widened.

## v0.3.0 correction: Tauri pivot (supersedes the Slint section)

The Slint build above was completed and briefly carried the
v0.3.0 tag. Before the version was announced, the user directed a
toolkit change: use whatever engine makes the product feel modern
and fast, with Stripe's site as the polish benchmark and a live
browser demo on GitHub Pages. The parent settled the question:
the shipped v0.3.0 is a Tauri 2 app with a web UI. The tag was
moved to the Tauri commit and the release assets were rebuilt.
The Slint section above stays as the record of that attempt; its
screenshots were replaced in the v030 set by the Tauri captures
of the same screens.

What shipped: the engine moved, logic unchanged, from the Slint
crate into a new workspace crate, engine/ (package
reliakit-engine): the same Work state, computes, project format,
and data flows, exposed through one JSON dispatcher
(engine/src/handle.rs, contract in engine/API.md). Chart series
are computed as data in engine/src/series.rs and drawn by the
front end as inline SVG. The desktop host (gui/src-tauri/) is a
thin Tauri 2 shell with a single dispatch command. The front end
(gui/index.html, gui/src/, gui/styles/, gui/assets/) is vanilla
JavaScript modules and hand-written CSS; the Slint icon set,
logo, hero artwork, and Inter fonts carried over. The same front
end runs in the browser from site/demo/ against the engine
compiled to WebAssembly (wasm/ crate, raw FFI, no wasm-bindgen),
deployed by .github/workflows/pages.yml.

Verification for the shipped build: cargo build --release
--workspace clean; cargo test --release --workspace reports the
same 167 passed, 0 failed; git diff on src/ (the library) is
empty. A Node smoke test drove the WASM module with the front
end's exact protocol: Example 3 gives RBD reliability 0.9214,
Example 1 fits to headline R(1000) = 0.9172 with a 121 point
series, dataset cards load from the embedded fallback, and
project_open_data round-trips a four item project. The Tauri
binary ran under Xvfb with its WebKitGTK webview and stayed up
with no panic. The screenshot pass was repeated on the Tauri app
(start page with the gradient hero and the Quickstart panel,
About view, Life Data fit, wizard, RBD placement and drag with
the inspector, MCMC, Markov, FRACAS), and the browser demo and
landing page were captured headlessly as 12-demo.png and
13-landing.png in the same set.

Behavior notes for the shipped build. Keyboard shortcuts are
Ctrl+S (save), Ctrl+O (open), and Delete for the selected RBD
block or table row; there is no Ctrl+Z undo shortcut, the RBD
inspector keeps its Undo button. In the browser demo, project
save downloads a file and open uses a file picker; controls that
need a local file path on desktop show a short note in the demo
instead. The Linux desktop build needs the system WebKitGTK
runtime (libwebkit2gtk-4.1); the macOS download is a raw binary,
not an .app bundle.

## v0.3.1: RBD drag fix, navigation rework, tutorial wizard

The library (src/) is untouched in this release; the changes are in
the front end (gui/src/, gui/styles/) and the app version strings.
The full test run for this version is the same 167 tests:
cargo build --release --workspace clean (exit 0; two pre-existing
warnings in the engine crate), cargo test --release --workspace
reports 167 passed, 0 failed.

RBD drag bug, reproduction on v0.3.0. The reproduction script
(repro_drag_before.py in the v031 GUI test set) drove the v0.3.0
browser demo with realistic multi-step mouse input: press on the
block body, move in steps, release, then the same starting on the
block name label. Both drags left the block unmoved: BODY DRAG
DELTA dx=0 dy=0 and LABEL DRAG DELTA dx=0 dy=0 (attempted +120,+80
each time). Result line: BUG REPRODUCED - drag did NOT move the
block.

Root cause. The v0.3.0 canvas dispatched an engine action on
pointerdown, and every dispatch re-rendered the whole app (the app
root innerHTML is rebuilt on each engine response). The re-render
destroyed the block element that held the pointer capture, so the
rest of the drag had no live element to track and the position
never changed.

Fix. The canvas now tracks the drag locally in gui/src/rbd.js: the
whole block is the drag handle (body and label alike), pointerdown
records the grab offset and calls setPointerCapture on the block,
pointermove past a 4 px threshold moves only that element by
setting its style left and top (no dispatch, no re-render, a
dragging CSS class while moving), and pointerup dispatches the
final position once (rbd-drag) followed by rbd-press so the block
ends selected with its inspector open. A press with no move stays a
plain click and only selects. Positions persist through the
existing diagram JSON pos field, so they save with the project and
restore on open.

Automated browser evidence (Playwright against the rebuilt browser
demo, scripts and logs in the v031 GUI test set):

- Drag test (test_drag_after.py): OVERALL: PASS - all drag,
  persistence and click-select checks passed. The power supply
  block in Example 3 moved by exactly (+120,+80); the saved project
  JSON holds pos [150.0, 170.0] for the block; after a page reload
  and reopening the saved project, the block renders at (150,170);
  a plain click on another block selects it and opens the Block
  inspector without moving it. Screenshots: drag_mid.png (live
  mid-drag position with the dragging class applied) and
  drag_after.png.
- Picker test (test_picker.py): OVERALL: PASS - picker, Markov
  header, and Ctrl+K FRACAS checks passed. The picker shows 18 kind
  cards under the 4 family headings; a Markov card click creates
  the item; the grouped tree shows family headers; the item header
  shows the name, kind label, and the exact one-line description;
  the Ctrl+K switcher filtered on "frac" creates a FRACAS item on
  Enter. Screenshots: picker_overlay.png and nav_header.png.
- Tours test (test_tours.py): OVERALL: PASS - life and RBD full
  plays, 16-tour selector smoke, and wizard path all passed. The
  Life Data tour plays all 6 steps and the tour item fit headline
  reads R(1000) = 0.9172; the RBD tour plays all 7 steps on the
  Example 3 item (headline reliability 0.9214) with its four blocks
  on screen. The other 16 tours run as a selector smoke: every step
  target found, 0 missing targets, 103 tour steps checked in total
  across all 18 tours. Each tour runs in its own new item named
  Tour: plus the kind name. The start page wizard card is present
  and its Analyze failure data path starts the Life Data tour in a
  new Tour: Life Data item. Screenshots: tour_life.png,
  tour_rbd.png, and tour_wizard.png.
- Canonical spot checks are covered by the tours test above: Life
  Data Example 1 headline R(1000) = 0.9172 and RBD Example 3
  headline 0.9214, the same values as the v0.3.0 gate.

RBD topology-faithful drawing (correctness fix). In v0.3.0 the
connector generator chained leaves in traversal order, so a
parallel group drew as a series chain: the picture misrepresented
the evaluated model (Example 3 drew its parallel controllers in
series). In v0.3.1, diagrams draw the topology that is evaluated:
parallel and k-out-of-n groups draw as branches between split and
merge rails, standby groups show their standby unit, and nested
groups are framed and labeled.

- Topology test (test_topology.py, Playwright against the rebuilt
  browser demo in the final pass): OVERALL: PASS - topology layout,
  rails, frames, ghosts and kofn checks passed. Example 3 final
  positions: power supply (30,90), pumps (190,90), controller A
  (350,90), controller B (350,174); the wires SVG contains two
  vertical rail bars of height 84 (the parallel split and merge
  rails); a frame labeled Parallel is present and a standby ghost
  is present for the pumps standby group; reliability headline
  still reads 0.9214. Nested diagram (loaded through Diagram JSON):
  a frame labeled Parallel exists, its two blocks are stacked, and
  a standby ghost is present. K-out-of-n diagram: a frame labeled
  "k-out-of-n (2 of 3)" is present with three stacked children
  sharing x and three distinct y values. Screenshots:
  topo_example3.png, topo_nested.png, and topo_kofn.png in the
  v031 GUI test set.
- Final-pass regression lines (all four browser tests re-run
  against the demo rebuilt from the final tree):
  Drag test (test_drag_after.py): OVERALL: PASS - all drag,
  persistence and click-select checks passed.
  Picker test (test_picker.py): OVERALL: PASS - picker, Markov
  header, and Ctrl+K FRACAS checks passed.
  Tours test (test_tours.py): OVERALL: PASS - life and RBD full
  plays, 16-tour selector smoke, and wizard path all passed.
  Topology test (test_topology.py): OVERALL: PASS - topology
  layout, rails, frames, ghosts and kofn checks passed.

## v0.3.2: correctness and validation

The library and engine changes in this release carry their own
test files. The browser suite is committed in the repo for the
first time. Counts below are as recorded by the workers who wrote
each file; the release totals line is finalized by the
coordinator after the full gate run.

| Area | Test file | Tests | Notes |
| --- | --- | --- | --- |
| Fit status taxonomy, adaptive Weibull bracketing, shifted exponential | tests/v032_fit_tests.rs | 13 | Includes the SciPy anchor for the clustered sample (beta about 80.8) and a scale invariance check at x3600 |
| Fault trees with repeated events, warranty cutoff and age convention | tests/v032_ft_war_tests.rs | 16 | Includes an independent brute force truth-table enumeration and per-cohort conservation checks |
| Validation fixtures (distributions, fitting, Fisher second path, RBD closed forms, CTMC and Monte Carlo availability, Arrhenius factor) | tests/validation_tests.rs | 17 | Fixture record with sources and tolerances: validation/fixtures.md |
| Engine dispatcher (precision, structured validation, fit status, outdated gate, duplicate, project round trips) | engine/tests/engine_tests.rs | 14 | Package reliakit-engine |
| Availability Monte Carlo standard error (mean bit-identical at 40 sims, SE shrinks with sim count, analytic anchor) | tests/v032_sim_tests.rs | 5 | Example 3 spec: mean 0.99875 unchanged, SE 0.00125 |
| Browser suite (import round trip, outdated badge, fit to RBD, save and reopen, keyboard-only workflow, all 18 tours and wizard paths, RBD viewport and drag at 1280x800) | browser-tests/ | 2 files, run by browser-tests/run_all.py | Playwright against the locally served demo build; see browser-tests/README.md |

TOTALS (finalized at the v0.3.2 gate, Nov 6, 2026): 232 tests
passed, 0 failed across the test-bearing crates (reliakit 218,
reliakit-engine 14; the reliakit-app shell crate carries no
tests of its own and is covered by the CI clippy and build
gates). The v0.3.1 baseline was 167; the 65 new tests are the
five v0.3.2 files in the table above. cargo fmt --all -- --check
and cargo clippy with -D warnings are clean on the same tree,
the CLI selftest passes, and the committed browser suite passed
in full at the gate (45 checks in test_v032_flows.py, 59 in
test_tours_v032.py), including all 18 tours and the three
first-run wizard paths.
