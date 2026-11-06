//! Tests for the MIL-HDBK-217F Notice 2 library in
//! `libraries/mil217/`, against values read directly from the
//! handbook PDF (MIL-HDBK-217F Notice 2, 28 February 1995; the
//! Notice 2 change pages were downloaded from Everyspec, and the
//! sections Notice 2 did not revise were taken from the base
//! MIL-HDBK-217F of 2 December 1991 via Quanterion).

use reliakit::standards217::{
    capacitor_pi_c, capacitor_pi_v, category_lambda_b_table, load_category, part_failure_rate,
    pth_failure_rate, FactorInput, Inputs,
};
use serde_json::Value;
use std::path::PathBuf;

fn category_file(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("libraries/mil217")
        .join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", path.display(), e));
    load_category(&text).expect("category JSON parses")
}

fn inputs(pairs: Vec<(&str, FactorInput)>) -> Inputs {
    pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

fn approx(actual: f64, expected: f64, tol: f64) -> bool {
    (actual - expected).abs() <= tol
}

// ---------------------------------------------------------------------
// (a) Exact grid point lookups.
//
// Mechanical relays, Section 13.1, handbook page 13-1 (Notice 2 PDF
// page 50). The lambda_b table is indexed by ambient temperature TA
// (rows, 25 to 125 C) and rated temperature (columns, 85 C and
// 125 C). Printed values checked against the PDF page image:
//   TA = 25,  rated 85 C:  0.0059
//   TA = 50,  rated 125 C: 0.0098
//   TA = 85,  rated 85 C:  0.021
//   TA = 125, rated 125 C: 0.031
#[test]
fn relay_lambda_b_exact_grid_points() {
    let cat = category_file("relays_mechanical.json");
    let table = category_lambda_b_table(&cat).expect("relay lambda_b grid");
    assert_eq!(table.lambda_b(&[25.0, 85.0]), 0.0059);
    assert_eq!(table.lambda_b(&[50.0, 125.0]), 0.0098);
    assert_eq!(table.lambda_b(&[85.0, 85.0]), 0.021);
    assert_eq!(table.lambda_b(&[125.0, 125.0]), 0.031);
}

// (a) One axis series exact point. Synchros and resolvers,
// Section 12.2, handbook page 12-4 (Notice 2 PDF page 48): at frame
// temperature TF = 50 C the printed lambda_b is 0.011.
#[test]
fn synchro_lambda_b_series_exact_point() {
    let cat = category_file("synchros_and_resolvers.json");
    let table = category_lambda_b_table(&cat).expect("synchro lambda_b series");
    assert_eq!(table.lambda_b(&[50.0]), 0.011);
    // TF = 80 C prints 0.027 on the same table.
    assert_eq!(table.lambda_b(&[80.0]), 0.027);
}

// ---------------------------------------------------------------------
// (b) Bilinear interpolation midpoint, computed by hand.
//
// Same relay table as above. Take TA = 52.5 C (halfway between the
// 50 and 55 C rows) and rated temperature 105 C (halfway between
// the 85 and 125 C columns). The four surrounding printed values
// (handbook page 13-1) are:
//   (TA 50, 85 C) = 0.010    (TA 50, 125 C) = 0.0098
//   (TA 55, 85 C) = 0.012    (TA 55, 125 C) = 0.011
// Bilinear interpolation at the center of a cell is the average of
// the four corners:
//   (0.010 + 0.0098 + 0.012 + 0.011) / 4 = 0.0428 / 4 = 0.0107.
#[test]
fn relay_lambda_b_bilinear_midpoint() {
    let cat = category_file("relays_mechanical.json");
    let table = category_lambda_b_table(&cat).expect("relay lambda_b grid");
    let v = table.lambda_b(&[52.5, 105.0]);
    assert!(approx(v, 0.0107, 1e-9), "got {}", v);
}

// ---------------------------------------------------------------------
// (c) End to end lambda_p, derived step by step from the tables.
//
// Resistors, Section 9.1, handbook pages 9-1 to 9-3 (Notice 2 PDF
// pages 32 to 34). Part: film resistor, style RL, at 60 C ambient,
// dissipating 1.0 W, at power stress S = 0.5, quality
// Non-Established Reliability, ground fixed environment.
//   lambda_b (style RL, page 9-1)            = 0.0037
//   pi_T: style RL uses pi_T column 2;
//         at 60 C the column 2 value (page 9-2) = 1.4
//   pi_P: at 1.0 W dissipation (page 9-2)     = 1.0
//   pi_S: style RL uses pi_S column 1;
//         at S = 0.5 the column 1 value (page 9-3) = 1.2
//   pi_Q: Non-Established Reliability (page 9-3)   = 3.0
//   pi_E: GF (page 9-3)                            = 4.0
// lambda_p = 0.0037 * 1.4 * 1.0 * 1.2 * 3.0 * 4.0
//          = 0.074592 failures/10^6 hours.
#[test]
fn resistor_end_to_end_handbook_tables() {
    let cat = category_file("resistors.json");
    let inp = inputs(vec![
        ("style", FactorInput::named("RL")),
        ("temperature_c", FactorInput::num(60.0)),
        ("power_dissipation_w", FactorInput::num(1.0)),
        ("power_stress", FactorInput::num(0.5)),
        ("quality", FactorInput::named("Non-Established Reliability")),
        ("environment", FactorInput::named("GF")),
    ]);
    let v = part_failure_rate(&cat, &inp);
    assert!(approx(v, 0.074592, 1e-9), "got {}", v);
}

// (c) End to end lambda_p: the handbook's own worked example.
//
// Capacitors, Section 10.2, handbook page 10-6 (Notice 2 PDF
// page 40). The example part is a CQ capacitor (MIL-C-83421) at
// 50 C, 15000 pF = 0.015 uF, applied voltage 200 V DC plus 50 V AC
// peak on a 400 V DC rating, so S = (200 + sqrt(2) * 50) / 400
// = 0.68, in ground fixed use, non-established reliability quality.
// The handbook reads (pages 10-1 to 10-5):
//   lambda_b (style CQ)                       = 0.00051
//   pi_T at 50 C, column 1                    = 1.6
//   pi_C from the equation C^0.09 = 0.69 (rounded from 0.685)
//   pi_V from the equation (S/0.6)^5 + 1 = 2.9 (rounded from 2.87)
//   pi_SR                                     = 1 (not applicable)
//   pi_Q, non-ER (Non-Established Reliability) = 3.0
//   pi_E, GF                                  = 10
// and gives lambda_p = 0.049 failures/10^6 hours:
//   0.00051 * 1.6 * 0.69 * 2.9 * 1 * 3.0 * 10 = 0.04898, rounds to 0.049.
#[test]
fn capacitor_end_to_end_handbook_worked_example() {
    let cat = category_file("capacitors.json");
    // Variant 1: the factor values as printed in the worked example.
    let inp = inputs(vec![
        ("style", FactorInput::named("CQ, CQR")),
        ("temperature_c", FactorInput::num(50.0)),
        ("pi_C", FactorInput::num(0.69)),
        ("pi_V", FactorInput::num(2.9)),
        ("quality", FactorInput::named("Non-Established Reliability")),
        ("environment", FactorInput::named("GF")),
    ]);
    let v = part_failure_rate(&cat, &inp);
    assert!(approx(v, 0.049, 0.0005), "got {}", v);

    // Variant 2: pi_C and pi_V from the handbook equations instead
    // of the rounded printed factors. pi_C = 0.015^0.09 = 0.6853 and
    // pi_V = (0.68/0.6)^5 + 1 = 2.8698, giving
    // 0.00051 * 1.6 * 0.6853 * 2.8698 * 3.0 * 10 = 0.0481, within
    // 2 percent of the handbook's rounded 0.049.
    let inp2 = inputs(vec![
        ("style", FactorInput::named("CQ, CQR")),
        ("temperature_c", FactorInput::num(50.0)),
        ("pi_C", FactorInput::num(capacitor_pi_c(1, 0.015))),
        ("pi_V", FactorInput::num(capacitor_pi_v(1, 0.68))),
        ("quality", FactorInput::named("Non-Established Reliability")),
        ("environment", FactorInput::named("GF")),
    ]);
    let v2 = part_failure_rate(&cat, &inp2);
    assert!(approx(v2, 0.049, 0.001), "got {}", v2);
}

// End to end with the Section 16.1 dedicated equation, handbook
// page 16-1 (Notice 2 PDF page 59):
// lambda_p = lambda_b * [N1 * pi_C + N2 * (pi_C + 13)] * pi_Q * pi_E.
// Printed wiring assembly, N1 = 100 automated soldered PTHs,
// N2 = 10 hand soldered PTHs, P = 4 circuit planes, MIL-SPEC
// quality, ground benign:
//   lambda_b = 0.000017, pi_C at P = 4 is 1.6, pi_Q = 1, pi_E = 1.
//   lambda_p = 0.000017 * (100 * 1.6 + 10 * (1.6 + 13))
//            = 0.000017 * (160 + 146) = 0.000017 * 306 = 0.005202.
#[test]
fn pth_end_to_end_section_16_1() {
    let cat = category_file("interconnection_assemblies_pth.json");
    let v = pth_failure_rate(
        &cat,
        "Printed Wiring Assembly/Printed Circuit Boards with PTHs",
        100.0,
        10.0,
        4.0,
        "MIL-SPEC or IPC Level 3",
        "GB",
    );
    assert!(approx(v, 0.005202, 1e-9), "got {}", v);
}

// Quartz crystals end to end, Section 19.1, handbook page 19-1
// (base handbook PDF page 170): lambda_b at f = 10 MHz is 0.022,
// pi_Q for MIL-SPEC is 1.0, pi_E for GF is 3.0, so
// lambda_p = 0.022 * 1.0 * 3.0 = 0.066.
#[test]
fn quartz_end_to_end() {
    let cat = category_file("quartz_crystals.json");
    let inp = inputs(vec![
        ("frequency_mhz", FactorInput::num(10.0)),
        ("quality", FactorInput::named("MIL-SPEC")),
        ("environment", FactorInput::named("GF")),
    ]);
    let v = part_failure_rate(&cat, &inp);
    assert!(approx(v, 0.066, 1e-9), "got {}", v);
}

// ---------------------------------------------------------------------
// (d) Every JSON file in libraries/mil217 parses and carries the
// provenance fields: handbook, section, and pdf_pages, all
// non-empty, plus category and model.
#[test]
fn all_category_files_have_provenance() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("libraries/mil217");
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).expect("libraries/mil217 exists") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        count += 1;
        let text = std::fs::read_to_string(&path).expect("readable");
        let v: Value = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("{} does not parse: {}", path.display(), e));
        for field in ["category", "handbook", "section", "model"] {
            let s = v
                .get(field)
                .and_then(|x| x.as_str())
                .unwrap_or_else(|| panic!("{} missing string field {}", path.display(), field));
            assert!(!s.is_empty(), "{} field {} is empty", path.display(), field);
        }
        assert_eq!(
            v.get("handbook").and_then(|x| x.as_str()),
            Some("MIL-HDBK-217F Notice 2"),
            "{} handbook field",
            path.display()
        );
        let pages = v
            .get("pdf_pages")
            .and_then(|x| x.as_array())
            .unwrap_or_else(|| panic!("{} missing pdf_pages array", path.display()));
        assert!(!pages.is_empty(), "{} pdf_pages is empty", path.display());
        assert!(
            v.get("lambda_b").is_some(),
            "{} missing lambda_b",
            path.display()
        );
        assert!(
            v.get("coverage").and_then(|x| x.as_str()).is_some(),
            "{} missing coverage",
            path.display()
        );
    }
    assert!(
        count >= 18,
        "expected at least 18 category files, found {}",
        count
    );
}
