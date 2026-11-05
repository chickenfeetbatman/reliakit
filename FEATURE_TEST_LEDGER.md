# ReliaKit Rust: Feature Test Ledger

Every feature in the crate, the test file that covers it, and the
result in the final full test run: 73 passed, 0 failed. Totals are
in the README.

Status meanings:

- PASS: the named test exists and passes in the final gate run.
- NO TEST: no test for this feature was found in
  tests/ported_tests.rs, tests/parity_tests.rs, or
  tests/qa_tests.rs.
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
