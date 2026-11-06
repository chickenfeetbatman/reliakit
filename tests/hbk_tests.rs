//! Tests for the v0.2.0 HBK-gap library features: Markov models,
//! prediction allocation, derating and library import, RCM task
//! selection, FRACAS, and the FMEA worksheet extensions.

use reliakit::fmea;
use reliakit::fracas;
use reliakit::markov::MarkovModel;
use reliakit::prediction;
use reliakit::rcm;

// ---------------- Markov ----------------

fn parallel_pair_model(with_repair_from_failed: bool) -> MarkovModel {
    let lambda = 0.01;
    let mu = 0.1;
    let mut transitions = vec![
        (0usize, 1usize, 2.0 * lambda),
        (1usize, 2usize, lambda),
        (1usize, 0usize, mu),
    ];
    if with_repair_from_failed {
        transitions.push((2usize, 1usize, mu));
    }
    MarkovModel::new(
        vec![
            "both up".to_string(),
            "one up".to_string(),
            "failed".to_string(),
        ],
        &transitions,
    )
    .expect("model should build")
}

#[test]
fn markov_steady_state_availability_matches_balance_equations() {
    let lambda = 0.01f64;
    let mu = 0.1f64;
    let model = parallel_pair_model(true);
    let pi = model.steady_state().expect("steady state");
    assert_eq!(pi.len(), 3);
    assert!((pi.iter().sum::<f64>() - 1.0).abs() < 1e-12);

    // Independent closed form from the balance equations:
    // pi0 * 2*lambda = pi1 * mu and pi2 * mu = pi1 * lambda.
    let ratio1 = 2.0 * lambda / mu;
    let ratio2 = ratio1 * lambda / mu;
    let pi0 = 1.0 / (1.0 + ratio1 + ratio2);
    let expected = [pi0, pi0 * ratio1, pi0 * ratio2];
    for i in 0..3 {
        assert!(
            (pi[i] - expected[i]).abs() < 1e-9,
            "pi[{}]={} expected {}",
            i,
            pi[i],
            expected[i]
        );
    }
    let availability: f64 = pi[0] + pi[1];
    let expected_availability = 1.0 - expected[2];
    assert!(
        (availability - expected_availability).abs() < 1e-9,
        "availability {} expected {}",
        availability,
        expected_availability
    );
}

#[test]
fn markov_mttf_matches_closed_form_650() {
    let model = parallel_pair_model(false);
    let absorbing = [false, false, true];
    let mttf = model
        .mean_time_to_absorption(&absorbing, 0)
        .expect("mean time to absorption");
    // Closed form (3*lambda + mu) / (2*lambda^2) with lambda=0.01, mu=0.1.
    let lambda = 0.01f64;
    let mu = 0.1f64;
    let closed = (3.0 * lambda + mu) / (2.0 * lambda * lambda);
    assert!((closed - 650.0).abs() < 1e-9, "closed form {}", closed);
    assert!((mttf - 650.0).abs() < 1e-6, "MTTF {} expected 650.0", mttf);

    // Starting from the one-up state the closed form gives 600.
    let from_one = model
        .mean_time_to_absorption(&absorbing, 1)
        .expect("mean time from state 1");
    assert!((from_one - 600.0).abs() < 1e-6, "from one {}", from_one);

    // Absorbing start gives zero.
    let from_failed = model
        .mean_time_to_absorption(&absorbing, 2)
        .expect("mean time from absorbing state");
    assert_eq!(from_failed, 0.0);
}

#[test]
fn markov_expected_time_in_state_sums_to_mttf() {
    let model = parallel_pair_model(false);
    let absorbing = [false, false, true];
    let times = model
        .expected_time_in_state(&absorbing, 0)
        .expect("expected times");
    assert_eq!(times.len(), 3);
    assert!(
        (times[0] - 550.0).abs() < 1e-6,
        "time in both up {}",
        times[0]
    );
    assert!(
        (times[1] - 100.0).abs() < 1e-6,
        "time in one up {}",
        times[1]
    );
    assert_eq!(times[2], 0.0);
    let sum: f64 = times.iter().sum();
    assert!((sum - 650.0).abs() < 1e-6, "sum {}", sum);
}

#[test]
fn markov_transient_rows_sum_to_one() {
    let model = parallel_pair_model(true);
    let rows = model
        .transient(&[1.0, 0.0, 0.0], &[0.0, 10.0, 100.0])
        .expect("transient");
    assert_eq!(rows.len(), 3);
    assert!((rows[0][0] - 1.0).abs() < 1e-9);
    for row in &rows {
        assert!((row.iter().sum::<f64>() - 1.0).abs() < 1e-9);
    }
    // Long run transient approaches the steady state.
    let pi = model.steady_state().expect("steady state");
    for i in 0..3 {
        assert!((rows[2][i] - pi[i]).abs() < 0.05, "row {:?}", rows[2]);
    }
}

// ---------------- prediction allocation, derating, library ----------------

#[test]
fn allocation_product_equals_target() {
    let target = 0.95f64;
    let equal = prediction::allocate_equal(target, 4);
    assert_eq!(equal.len(), 4);
    let product: f64 = equal.iter().product();
    assert!(
        (product - target).abs() < 1e-9,
        "equal product error {}",
        (product - target).abs()
    );

    let weighted = prediction::allocate_weighted(0.90, &[1.0, 2.0, 3.0]);
    assert_eq!(weighted.len(), 3);
    let wproduct: f64 = weighted.iter().product();
    assert!(
        (wproduct - 0.90).abs() < 1e-9,
        "weighted product error {}",
        (wproduct - 0.90).abs()
    );
}

#[test]
fn allocation_weighted_doubling_weight_lowers_that_target() {
    let target = 0.90f64;
    let base = prediction::allocate_weighted(target, &[1.0, 1.0, 1.0]);
    let doubled = prediction::allocate_weighted(target, &[2.0, 1.0, 1.0]);
    // The doubled subsystem takes a larger share of the budget, so
    // its target drops.
    assert!(
        doubled[0] < base[0],
        "doubled {} should be below base {}",
        doubled[0],
        base[0]
    );
    // The product is still the target, so the other subsystems are
    // not lowered; they rise to keep the product on target.
    assert!(doubled[1] >= base[1]);
    assert!(doubled[2] >= base[2]);
    let product: f64 = doubled.iter().product();
    assert!((product - target).abs() < 1e-9);
}

#[test]
fn derating_arithmetic_and_thresholds() {
    let ratio = prediction::derating_ratio(28.0, 50.0);
    assert!((ratio - 0.56).abs() < 1e-12, "ratio {}", ratio);

    let pass = prediction::derating_check("resistor", 28.0, 50.0, 0.7);
    assert_eq!(pass.name, "resistor");
    assert!((pass.ratio - 0.56).abs() < 1e-12);
    assert!(pass.pass);

    let fail = prediction::derating_check("resistor", 28.0, 50.0, 0.5);
    assert!((fail.ratio - 0.56).abs() < 1e-12);
    assert!(!fail.pass);
}

#[test]
fn library_csv_round_trip_and_hand_sum() {
    let text = "# Example comment line.\n\
name,base_rate_per_million_hours,quantity,stress_factor,quality_factor\n\
part A,2.0,3,1.0,1.0\n\
part B,1.5,2,2.0,0.5\n\
part C,4.0,1\n";
    let parts = prediction::library_from_csv(text).expect("library should load");
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[2].stress_factor, 1.0);
    assert_eq!(parts[2].quality_factor, 1.0);

    // Hand sum: A 2.0*3*1*1 = 6, B 1.5*2*2*0.5 = 3, C 4.0*1 = 4.
    let rate = prediction::system_failure_rate_per_million_hours(&parts);
    assert!((rate - 13.0).abs() < 1e-12, "system rate {}", rate);

    let csv = prediction::library_to_csv(&parts);
    let loaded = prediction::library_from_csv(&csv).expect("round trip should load");
    assert_eq!(loaded.len(), parts.len());
    for (a, b) in parts.iter().zip(loaded.iter()) {
        assert_eq!(a.name, b.name);
        assert!((a.base_rate_per_million_hours - b.base_rate_per_million_hours).abs() < 1e-12);
        assert_eq!(a.quantity, b.quantity);
        assert!((a.stress_factor - b.stress_factor).abs() < 1e-12);
        assert!((a.quality_factor - b.quality_factor).abs() < 1e-12);
    }
    let rate2 = prediction::system_failure_rate_per_million_hours(&loaded);
    assert!((rate2 - 13.0).abs() < 1e-12);
}

#[test]
fn example_library_file_loads() {
    let text = include_str!("../libraries/example_library.csv");
    assert!(text.starts_with(
        "# Example component library - synthetic example data, not from any standard."
    ));
    let parts = prediction::library_from_csv(text).expect("example library should load");
    assert!(parts.len() >= 4 && parts.len() <= 6, "rows {}", parts.len());
    assert!(parts.iter().all(|p| p.name.contains("EXAMPLE")));
    assert!(prediction::system_failure_rate_per_million_hours(&parts) > 0.0);
}

// ---------------- RCM ----------------

fn rcm_mode(
    consequence: rcm::Consequence,
    condition_monitorable: bool,
    age_related: bool,
    restorable: bool,
) -> rcm::RcmMode {
    rcm::RcmMode {
        asset: "pump".to_string(),
        function: "move fluid".to_string(),
        failure_mode: "seal leak".to_string(),
        consequence,
        condition_monitorable,
        age_related,
        restorable,
        failure_rate_per_hour: None,
        mode_ratio_alpha: None,
        effect_probability_beta: None,
        mission_hours: None,
    }
}

#[test]
fn rcm_select_task_decision_table() {
    // Hidden plain -> FailureFinding.
    let hidden = rcm_mode(rcm::Consequence::Hidden, false, false, false);
    assert_eq!(rcm::select_task(&hidden), rcm::RcmTask::FailureFinding);

    // Hidden stays FailureFinding even when monitorable, because
    // the condition based rule excludes Hidden.
    let hidden_monitorable = rcm_mode(rcm::Consequence::Hidden, true, false, false);
    assert_eq!(
        rcm::select_task(&hidden_monitorable),
        rcm::RcmTask::FailureFinding
    );

    // Safety age related restorable, not monitorable -> ScheduledRestoration.
    let safety = rcm_mode(rcm::Consequence::Safety, false, true, true);
    assert_eq!(
        rcm::select_task(&safety),
        rcm::RcmTask::ScheduledRestoration
    );

    // Environmental age related not restorable -> ScheduledDiscard.
    let environmental = rcm_mode(rcm::Consequence::Environmental, false, true, false);
    assert_eq!(
        rcm::select_task(&environmental),
        rcm::RcmTask::ScheduledDiscard
    );

    // Operational plain -> RunToFailure.
    let operational = rcm_mode(rcm::Consequence::Operational, false, false, false);
    assert_eq!(rcm::select_task(&operational), rcm::RcmTask::RunToFailure);

    // NonOperational monitorable -> ConditionBased.
    let non_operational = rcm_mode(rcm::Consequence::NonOperational, true, false, false);
    assert_eq!(
        rcm::select_task(&non_operational),
        rcm::RcmTask::ConditionBased
    );
}

#[test]
fn rcm_criticality_arithmetic() {
    let value = rcm::criticality_number(0.3, 1.0, 2e-5, 1000.0);
    assert!((value - 0.006).abs() < 1e-12, "criticality {}", value);

    let mut mode = rcm_mode(rcm::Consequence::Operational, false, false, false);
    assert_eq!(mode.criticality(), None);
    mode.mode_ratio_alpha = Some(0.3);
    mode.effect_probability_beta = Some(1.0);
    mode.failure_rate_per_hour = Some(2e-5);
    mode.mission_hours = Some(1000.0);
    let mode_value = mode.criticality().expect("criticality should be present");
    assert!((mode_value - 0.006).abs() < 1e-12);
}

// ---------------- FRACAS ----------------

fn fracas_fixture() -> Vec<fracas::Incident> {
    vec![
        fracas::Incident {
            id: "F-1".to_string(),
            part: "pump".to_string(),
            failure_mode: "bearing wear".to_string(),
            hours: 1200.0,
            date: "2026-01-10".to_string(),
            status: fracas::IncidentStatus::Open,
            corrective_action: String::new(),
        },
        fracas::Incident {
            id: "F-2".to_string(),
            part: "pump".to_string(),
            failure_mode: "bearing wear".to_string(),
            hours: 2400.0,
            date: "2026-02-11".to_string(),
            status: fracas::IncidentStatus::Open,
            corrective_action: String::new(),
        },
        fracas::Incident {
            id: "F-3".to_string(),
            part: "valve".to_string(),
            failure_mode: "seal leak".to_string(),
            hours: 800.0,
            date: "2026-03-01".to_string(),
            status: fracas::IncidentStatus::Analysis,
            corrective_action: "inspect seal".to_string(),
        },
        fracas::Incident {
            id: "F-4".to_string(),
            part: "motor".to_string(),
            failure_mode: "bearing wear".to_string(),
            hours: 3000.0,
            date: "2026-03-15".to_string(),
            status: fracas::IncidentStatus::CorrectiveAction,
            corrective_action: "replace bearing, check alignment".to_string(),
        },
        fracas::Incident {
            id: "F-5".to_string(),
            part: "valve".to_string(),
            failure_mode: "seal leak".to_string(),
            hours: 1500.0,
            date: "2026-04-02".to_string(),
            status: fracas::IncidentStatus::Closed,
            corrective_action: "replaced seal".to_string(),
        },
        fracas::Incident {
            id: "F-6".to_string(),
            part: "motor".to_string(),
            failure_mode: "overheat".to_string(),
            hours: 4200.0,
            date: "2026-05-20".to_string(),
            status: fracas::IncidentStatus::Closed,
            corrective_action: "cleaned vents".to_string(),
        },
    ]
}

#[test]
fn fracas_aggregation_counts() {
    let incidents = fracas_fixture();
    let by_status = fracas::count_by_status(&incidents);
    assert_eq!(
        by_status,
        vec![
            ("Open".to_string(), 2usize),
            ("Analysis".to_string(), 1usize),
            ("CorrectiveAction".to_string(), 1usize),
            ("Closed".to_string(), 2usize),
        ]
    );

    let by_mode = fracas::count_by_mode(&incidents);
    assert_eq!(
        by_mode,
        vec![
            ("bearing wear".to_string(), 3usize),
            ("seal leak".to_string(), 2usize),
            ("overheat".to_string(), 1usize),
        ]
    );
}

#[test]
fn fracas_mtbf_arithmetic() {
    let incidents = fracas_fixture();
    let mtbf = fracas::mtbf_hours(10000.0, &incidents[..4]).expect("mtbf");
    assert!((mtbf - 2500.0).abs() < 1e-12, "MTBF {}", mtbf);
    assert_eq!(fracas::mtbf_hours(10000.0, &[]), None);
}

#[test]
fn fracas_csv_round_trip() {
    let incidents = fracas_fixture();
    let csv = fracas::incidents_to_csv(&incidents);
    assert!(csv.starts_with("id,part,failure_mode,hours,date,status,corrective_action\n"));
    let parsed = fracas::incidents_from_csv(&csv).expect("round trip should parse");
    assert_eq!(parsed, incidents);
    // Status strings round trip through as_str and from_str.
    for status in fracas::IncidentStatus::workflow_order() {
        assert_eq!(
            fracas::IncidentStatus::from_str(status.as_str()).expect("from_str"),
            status
        );
    }
}

// ---------------- FMEA extensions ----------------

fn fmea_rows_fixture() -> Vec<fmea::FmeaRow> {
    let mut r1 = fmea::FmeaRow::new("pump", "leaks", 8, 5, 4);
    r1.recommended_action = "replace seal".to_string();
    let r2 = fmea::FmeaRow::new("valve", "sticks", 6, 3, 3);
    let mut r3 = fmea::FmeaRow::new("motor", "overheats", 9, 2, 5);
    r3.recommended_action = "add temperature sensor".to_string();
    vec![r1, r2, r3]
}

#[test]
fn fmea_worksheet_round_trip_keeps_type_and_rows() {
    let rows = fmea_rows_fixture();
    let mut worksheet = fmea::FmeaWorksheet::new(fmea::FmeaType::Process);
    worksheet.rows = rows.clone();
    let csv = worksheet.to_csv();
    assert!(
        csv.starts_with("# fmea_type: Process\n"),
        "csv head {:?}",
        &csv[..30]
    );

    let parsed = fmea::FmeaWorksheet::from_csv(&csv).expect("worksheet should parse");
    assert_eq!(parsed.fmea_type, fmea::FmeaType::Process);
    assert_eq!(parsed.fmea_type.as_str(), "Process");
    assert_eq!(parsed.rows.len(), 3);
    assert_eq!(parsed.rows[0].item, "pump");
    assert_eq!(parsed.rows[0].failure_mode, "leaks");
    assert_eq!(parsed.rows[0].rpn(), 160);
    assert_eq!(parsed.rows[1].item, "valve");
    assert_eq!(parsed.rows[2].recommended_action, "add temperature sensor");

    // Plain rows CSV with no type line defaults to Design.
    let plain = fmea::rows_to_csv(&rows);
    let parsed_plain = fmea::FmeaWorksheet::from_csv(&plain).expect("plain rows should parse");
    assert_eq!(parsed_plain.fmea_type, fmea::FmeaType::Design);
    assert_eq!(parsed_plain.rows.len(), 3);
    assert_eq!(
        fmea::FmeaType::from_str("System").expect("from_str"),
        fmea::FmeaType::System
    );
}

#[test]
fn fmea_control_plan_only_rows_with_actions() {
    let rows = fmea_rows_fixture();
    let csv = fmea::control_plan_csv(&rows);
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines[0], "item,failure_mode,control_method,source_rpn");
    assert_eq!(lines.len(), 3, "header plus two action rows: {:?}", lines);
    assert!(csv.contains("pump,leaks,replace seal,160"), "csv {}", csv);
    assert!(
        csv.contains("motor,overheats,add temperature sensor,90"),
        "csv {}",
        csv
    );
    assert!(!csv.contains("valve"), "csv {}", csv);
}

#[test]
fn fmea_dvpr_csv_round_trip() {
    let entries = vec![
        fmea::DvprEntry {
            id: "DV-1".to_string(),
            failure_mode_ref: "leaks".to_string(),
            test: "pressure hold, 2 hours".to_string(),
            sample_size: 5,
            acceptance_criteria: "no leak at 1.5x pressure".to_string(),
        },
        fmea::DvprEntry {
            id: "DV-2".to_string(),
            failure_mode_ref: "overheats".to_string(),
            test: "thermal soak".to_string(),
            sample_size: 3,
            acceptance_criteria: "case temp below 85 C".to_string(),
        },
    ];
    let csv = fmea::dvpr_to_csv(&entries);
    let parsed = fmea::dvpr_from_csv(&csv).expect("dvpr round trip should parse");
    assert_eq!(parsed, entries);
}

#[test]
fn fmea_criticality_arithmetic() {
    let value = fmea::criticality_number(0.3, 1.0, 2e-5, 1000.0);
    assert!((value - 0.006).abs() < 1e-12, "criticality {}", value);
}

// ---------------- clean-room methods: SR-332 style ----------------

#[test]
fn sr332_temperature_factor_equal_temps_and_arrhenius_value() {
    // Equal temperatures give exactly 1.0.
    let at_equal = prediction::sr332_temperature_factor(0.35, 40.0, 40.0);
    assert_eq!(at_equal, 1.0);

    // Independent Arrhenius computation with the same constants:
    // pi_T = exp((Ea/k) * (1/T_ref_K - 1/T_use_K)), k = 8.617e-5.
    let k = 8.617e-5f64;
    let ea = 0.35f64;
    let t_use_k = 55.0 + 273.15;
    let t_ref_k = 40.0 + 273.15;
    let expected = ((ea / k) * (1.0 / t_ref_k - 1.0 / t_use_k)).exp();
    assert!(
        expected > 1.0,
        "hotter use should raise the factor: {}",
        expected
    );
    let actual = prediction::sr332_temperature_factor(ea, 55.0, 40.0);
    let rel = (actual - expected).abs() / expected;
    assert!(
        rel < 1e-12,
        "pi_T {} expected {} rel {}",
        actual,
        expected,
        rel
    );

    // The documented default constant is 0.35 eV.
    assert_eq!(prediction::SR332_DEFAULT_ACTIVATION_ENERGY_EV, 0.35);
    assert_eq!(prediction::BOLTZMANN_EV_PER_K, 8.617e-5);
}

#[test]
fn sr332_method1_product_arithmetic() {
    // Four-factor device form: 0.5 * 2.0 * 1.5 * 1.2 = 1.8
    let rate = prediction::sr332_method1(0.5, 2.0, 1.5, 1.2);
    assert!((rate - 1.8).abs() < 1e-12, "method1 rate {}", rate);
    // All factors at 1.0 pass the generic rate through unchanged.
    assert_eq!(prediction::sr332_method1(3.25, 1.0, 1.0, 1.0), 3.25);
}

#[test]
fn sr332_equipment_rate_applies_pi_e_once() {
    // Two part types, hand computed: 3 * 1.8 = 5.4 and
    // 4 * 0.25 = 1.0, sum 6.4, times pi_E 1.5 gives 9.6.
    let parts: [(u32, f64); 2] = [(3, 1.8), (4, 0.25)];
    let rate = prediction::sr332_equipment_rate(&parts, 1.5);
    assert!((rate - 9.6).abs() < 1e-12, "equipment rate {}", rate);
    // pi_E of 1.0 returns the plain quantity weighted sum.
    let plain = prediction::sr332_equipment_rate(&parts, 1.0);
    assert!((plain - 6.4).abs() < 1e-12, "plain sum {}", plain);
    // Empty parts give zero.
    assert_eq!(prediction::sr332_equipment_rate(&[], 2.0), 0.0);
}

#[test]
fn sr332_method2_extrapolate_factor_ratios() {
    // Temperature ratio 2.0/1.0 = 2, stress ratio 1.5/0.5 = 3,
    // so the observed rate scales by 6: 2.0 * 6 = 12.0.
    let rate = prediction::sr332_method2_extrapolate(2.0, 1.0, 2.0, 0.5, 1.5);
    assert!((rate - 12.0).abs() < 1e-12, "method2 rate {}", rate);
    // The unprefixed alias agrees.
    let alias = prediction::method2_extrapolate(2.0, 1.0, 2.0, 0.5, 1.5);
    assert_eq!(alias, rate);
    // Same conditions in and out leave the rate unchanged.
    assert_eq!(
        prediction::sr332_method2_extrapolate(7.5, 1.3, 1.3, 0.9, 0.9),
        7.5
    );
}

#[test]
fn chi2_ppf_known_values() {
    let at_10 = prediction::chi2_ppf(0.95, 10.0);
    assert!(
        (at_10 - 18.307).abs() < 1e-3,
        "chi2 0.95 at 10 df: {}",
        at_10
    );
    let at_2 = prediction::chi2_ppf(0.95, 2.0);
    assert!((at_2 - 5.991).abs() < 1e-3, "chi2 0.95 at 2 df: {}", at_2);
}

#[test]
fn failure_rate_ci_zero_failures_uses_known_two_df_value() {
    // Two-sided 95 percent, r = 0, T = 1000 hours.
    // Lower is 0. Upper is chi2_ppf(0.975, 2) / (2T), and the 2 df
    // chi-square has CDF 1 - exp(-x/2), so the 0.975 quantile is
    // -2 * ln(0.025) = 7.3778.
    let quantile = -2.0f64 * 0.025f64.ln();
    assert!((quantile - 7.3778).abs() < 1e-3, "quantile {}", quantile);
    let (lower, upper) = prediction::failure_rate_ci(0, 1000.0, 0.95);
    assert_eq!(lower, 0.0);
    let expected_upper = quantile / 2000.0;
    assert!(
        (upper - expected_upper).abs() < 1e-9,
        "upper {} expected {}",
        upper,
        expected_upper
    );

    // With failures, the point rate sits inside the interval.
    let (lo, hi) = prediction::failure_rate_ci(5, 10000.0, 0.95);
    assert!(
        lo > 0.0 && lo < 5.0 / 10000.0 && 5.0 / 10000.0 < hi,
        "lo {} hi {}",
        lo,
        hi
    );
}

#[test]
fn sr332_method3_combine_weighted_average() {
    // Equal weights give the midpoint.
    let mid = prediction::sr332_method3_combine(10.0, 1.0, 20.0, 1.0);
    assert!((mid - 15.0).abs() < 1e-12, "midpoint {}", mid);
    // Zero observed weight returns the prediction.
    let pred_only = prediction::sr332_method3_combine(10.0, 1.0, 20.0, 0.0);
    assert_eq!(pred_only, 10.0);
    // Weights 3 to 1: (10*3 + 20*1) / 4 = 12.5. Alias agrees.
    let weighted = prediction::method3_combine(10.0, 3.0, 20.0, 1.0);
    assert!((weighted - 12.5).abs() < 1e-12, "weighted {}", weighted);
}

// ---------------- clean-room methods: 62380-style mission profile ----------------

#[test]
fn iec62380_style_single_phase_at_ref_temp_returns_base_rate() {
    let phases = vec![prediction::MissionPhase::new("cruise", 100.0, 40.0, 0.0)];
    let rate = prediction::iec62380_style_mission_profile_rate(10.0, &phases, 0.35, 40.0, 0.001);
    assert_eq!(rate, 10.0);
    // The unprefixed alias agrees.
    assert_eq!(
        prediction::mission_profile_rate(10.0, &phases, 0.35, 40.0, 0.001),
        10.0
    );
}

#[test]
fn iec62380_style_hotter_phase_raises_rate() {
    let base_phases = vec![prediction::MissionPhase::new("cruise", 100.0, 40.0, 0.0)];
    let hot_phases = vec![
        prediction::MissionPhase::new("cruise", 50.0, 40.0, 0.0),
        prediction::MissionPhase::new("climb", 50.0, 85.0, 0.0),
    ];
    let base = prediction::iec62380_style_mission_profile_rate(10.0, &base_phases, 0.35, 40.0, 0.0);
    let hot = prediction::iec62380_style_mission_profile_rate(10.0, &hot_phases, 0.35, 40.0, 0.0);
    assert_eq!(base, 10.0);
    assert!(hot > base, "hot {} should exceed base {}", hot, base);
}

#[test]
fn iec62380_style_two_phase_hand_arithmetic() {
    // Base rate 10, Ea 0.35, ref 40 C, cycling factor 0.001.
    // Phase A: 600 h operating at 40 C, 0 cycles, rate 10.
    // Phase B: 400 h operating at 55 C, 10 cycles.
    // Phase B rate = 10 * pi_T(55 C vs 40 C), pi_T from the
    // Arrhenius formula with k = 8.617e-5, computed here by hand.
    // Cycling part = 10 cycles * 0.001 * 10 / 1000 h total.
    let k = 8.617e-5f64;
    let pi_t = ((0.35 / k) * (1.0 / (40.0 + 273.15) - 1.0 / (55.0 + 273.15))).exp();
    let expected = (600.0 * 10.0 + 400.0 * 10.0 * pi_t) / 1000.0 + 10.0 * 0.001 * 10.0 / 1000.0;

    let phases = vec![
        prediction::MissionPhase::new("ground", 600.0, 40.0, 0.0),
        prediction::MissionPhase::new("flight", 400.0, 55.0, 10.0),
    ];
    let actual = prediction::iec62380_style_mission_profile_rate(10.0, &phases, 0.35, 40.0, 0.001);
    assert!(
        (actual - expected).abs() < 1e-9,
        "mission rate {} expected {}",
        actual,
        expected
    );
}

#[test]
fn iec62380_style_dormant_phase_uses_dormant_factor() {
    // Half the time operating at the reference temperature (rate
    // 10), half dormant with a user-supplied factor of 0.1 (rate
    // 1), no cycles: the time-weighted rate is 5.5 regardless of
    // the dormant phase temperature.
    let mut dormant = prediction::MissionPhase::new("storage", 100.0, 85.0, 0.0);
    dormant.dormant = true;
    dormant.dormant_factor = 0.1;
    let phases = vec![
        prediction::MissionPhase::new("run", 100.0, 40.0, 0.0),
        dormant,
    ];
    let rate = prediction::iec62380_style_mission_profile_rate(10.0, &phases, 0.35, 40.0, 0.0);
    assert!((rate - 5.5).abs() < 1e-12, "dormant blended rate {}", rate);
    // Empty profile gives 0.
    assert_eq!(
        prediction::iec62380_style_mission_profile_rate(10.0, &[], 0.35, 40.0, 0.0),
        0.0
    );
}
