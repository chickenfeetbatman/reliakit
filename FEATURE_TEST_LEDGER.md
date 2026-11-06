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
| GUI headless check | src/bin/reliakit_app.rs | reliakit-app --headless (constructs the Example 3 RBD, prints reliability 0.921419 and mean availability, exits 0; verified by the coordinator in the gate run) | PASS |

Note on the specfunc row: qa_specfunc_norm_ppf_cdf_roundtrip was
briefly ignored because norm_cdf was built on the Abramowitz and
Stegun erf approximation (error 1.28e-6 at x = 2.0 against a 1e-6
tolerance). The coordinator replaced erf and norm_cdf with
incomplete-gamma based versions accurate to about 1e-14; the test
now runs un-ignored with its original tolerance and passes.

Features with no test at all: none. The four gaps QA found
(lognormal, normal, degradation power path, IPL least squares fit)
are covered by tests/coverage_tests.rs.
