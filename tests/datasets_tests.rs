//! Verification tests for the bundled sourced datasets in `datasets/`.
//!
//! Every CSV shipped in `datasets/` is transcribed from a documentation
//! page that was opened and read in full (see `datasets/README.md` for
//! the source URL of each file). These tests parse each file with the
//! simple rules the app importer uses (skip `#` comment lines, one
//! header row, comma separated f64 columns) and assert the row count
//! and column sums documented in `datasets/README.md`. A transcription
//! slip in any value changes a sum and fails the test.
//!
//! Three ReliaSoft examples also print the fitted 2-parameter Weibull
//! MLE parameters for the same data. Fit tests at the bottom assert
//! that `reliakit::distributions::fit_mle` reproduces those printed
//! values within the stated tolerance.

use reliakit::distributions::fit_mle;

/// Load a dataset CSV: skip `#` lines and the header, parse f64 columns.
fn load(path: &str) -> Vec<Vec<f64>> {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {}", path, e));
    let mut rows = Vec::new();
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        if l.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false)
            && l.contains(',')
            && l.split(',').next().unwrap().trim().parse::<f64>().is_err()
        {
            continue; // header row
        }
        let row: Vec<f64> = l
            .split(',')
            .map(|s| {
                s.trim()
                    .parse::<f64>()
                    .unwrap_or_else(|_| panic!("non-numeric value {:?} in {}", s, path))
            })
            .collect();
        rows.push(row);
    }
    rows
}

fn col_sum(rows: &[Vec<f64>], col: usize) -> f64 {
    rows.iter().map(|r| r[col]).sum()
}

fn check(rows: &[Vec<f64>], n_rows: usize, sums: &[(usize, f64, f64)], path: &str) {
    assert_eq!(
        rows.len(),
        n_rows,
        "{}: expected {} data rows, got {}",
        path,
        n_rows,
        rows.len()
    );
    for &(col, expected, tol) in sums {
        let got = col_sum(rows, col);
        assert!(
            (got - expected).abs() <= tol,
            "{}: column {} sum expected {} (+/- {}), got {}",
            path,
            col,
            expected,
            tol,
            got
        );
    }
}

// ---------------------------------------------------------------------
// Life data (time,failed)
// ---------------------------------------------------------------------

#[test]
fn life_data_example_rows_and_sums() {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/datasets/reliasoft_life_data_example.csv"
    );
    let rows = load(p);
    // 10 units: 6 failures (sum of failed = 6), times sum to 871 hours.
    check(&rows, 10, &[(0, 871.0, 1e-9), (1, 6.0, 1e-9)], p);
}

#[test]
fn unbiasing_complete_rows_and_sums() {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/datasets/reliasoft_unbiasing_complete.csv"
    );
    let rows = load(p);
    check(&rows, 10, &[(0, 920.0, 1e-9), (1, 10.0, 1e-9)], p);
}

#[test]
fn unbiasing_suspended_rows_and_sums() {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/datasets/reliasoft_unbiasing_suspended.csv"
    );
    let rows = load(p);
    // 7 failures and 3 suspensions at 120: times sum to 765.
    check(&rows, 10, &[(0, 765.0, 1e-9), (1, 7.0, 1e-9)], p);
}

#[test]
fn beta_shape_example_rows_and_sums() {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/datasets/reliasoft_beta_shape_example.csv"
    );
    let rows = load(p);
    // 20 items, 9 failures, times sum to 645. Transcription only: the
    // printed fit is not reproduced (see the fit note in README.md).
    check(&rows, 20, &[(0, 645.0, 1e-9), (1, 9.0, 1e-9)], p);
}

// ---------------------------------------------------------------------
// ALT (stress,time,failed)
// ---------------------------------------------------------------------

#[test]
fn alt_tensile_rows_and_sums() {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/datasets/reliasoft_alt_tensile.csv"
    );
    let rows = load(p);
    check(
        &rows,
        15,
        &[(0, 1480.0, 1e-9), (1, 17402.0, 1e-9), (2, 15.0, 1e-9)],
        p,
    );
}

#[test]
fn alt_circuit_boards_rows_and_sums() {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/datasets/reliasoft_alt_circuit_boards.csv"
    );
    let rows = load(p);
    // Grouped counts on the page expand to 29 units (19 at 463 K,
    // 10 at 488 K) with 15 failures. The page text says 27 boards;
    // the printed table sums to 29 and the table is what is shipped.
    check(
        &rows,
        29,
        &[(0, 13677.0, 1e-9), (1, 130133.0, 1e-9), (2, 15.0, 1e-9)],
        p,
    );
}

#[test]
fn alt_paper_clips_rows_and_sums() {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/datasets/reliasoft_alt_paper_clips.csv"
    );
    let rows = load(p);
    check(
        &rows,
        18,
        &[(0, 1890.0, 1e-9), (1, 572.0, 1e-9), (2, 18.0, 1e-9)],
        p,
    );
}

#[test]
fn alt_two_stress_rows_and_sums() {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/datasets/reliasoft_alt_two_stress.csv"
    );
    let rows = load(p);
    // stress = temperature K (sum 4416), stress2 = voltage V (sum 44).
    check(
        &rows,
        12,
        &[
            (0, 4416.0, 1e-9),
            (1, 5778.0, 1e-9),
            (2, 12.0, 1e-9),
            (3, 44.0, 1e-9),
        ],
        p,
    );
}

// ---------------------------------------------------------------------
// Degradation (unit,time,value)
// ---------------------------------------------------------------------

#[test]
fn degradation_turbine_blades_rows_and_sums() {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/datasets/reliasoft_degradation_turbine_blades.csv"
    );
    let rows = load(p);
    check(
        &rows,
        15,
        &[(0, 30.0, 1e-9), (1, 4500.0, 1e-9), (2, 331.0, 1e-9)],
        p,
    );
}

#[test]
fn degradation_mosfet_rows_and_sums() {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/datasets/reliasoft_degradation_mosfet.csv"
    );
    let rows = load(p);
    // 5 units x 35 inspection times. Value sum 878.11 percent.
    check(
        &rows,
        175,
        &[(0, 525.0, 1e-9), (1, 1430000.0, 1e-6), (2, 878.11, 0.01)],
        p,
    );
}

#[test]
fn alt_degradation_led_rows_and_sums() {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/datasets/reliasoft_alt_degradation_led.csv"
    );
    let rows = load(p);
    // 10 units x 5 inspections. Value sum 3813.6 mcd, stress sum 1875 mA.
    check(
        &rows,
        50,
        &[
            (0, 275.0, 1e-9),
            (1, 7500.0, 1e-9),
            (2, 3813.6, 0.01),
            (3, 1875.0, 1e-9),
        ],
        p,
    );
}

// ---------------------------------------------------------------------
// Reliability growth (time,failures): cumulative failure times
// ---------------------------------------------------------------------

#[test]
fn growth_crow_amsaa_rows_and_sums() {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/datasets/reliasoft_growth_crow_amsaa.csv"
    );
    let rows = load(p);
    // 27 failures in 300 hours. Times sum to 3006.9; the cumulative
    // count column runs 1..27 and sums to 378.
    check(&rows, 27, &[(0, 3006.9, 0.01), (1, 378.0, 1e-9)], p);
    assert_eq!(rows.last().unwrap()[1], 27.0);
}

#[test]
fn growth_test_fix_test_rows_and_sums() {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/datasets/reliasoft_growth_test_fix_test.csv"
    );
    let rows = load(p);
    // 23 failures, time terminated at 500 hours. Times sum to 3147.6;
    // cumulative count column sums to 276.
    check(&rows, 23, &[(0, 3147.6, 0.01), (1, 276.0, 1e-9)], p);
    assert_eq!(rows.last().unwrap()[1], 23.0);
}

// ---------------------------------------------------------------------
// Warranty
// ---------------------------------------------------------------------

#[test]
fn warranty_nevada_rows_and_sums() {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/datasets/reliasoft_warranty_nevada.csv"
    );
    let rows = load(p);
    // 3 shipments x their observed return lags = 6 rows. Returns sum
    // to 21. The sales column repeats each shipment size, so its raw
    // sum (730) is not a total; distinct shipments are 100, 140, 150.
    check(
        &rows,
        6,
        &[(0, 10.0, 1e-9), (2, 10.0, 1e-9), (3, 21.0, 1e-9)],
        p,
    );
    let mut sales: Vec<f64> = rows.iter().map(|r| r[1]).collect();
    sales.sort_by(|a, b| a.partial_cmp(b).unwrap());
    sales.dedup();
    assert_eq!(sales, vec![100.0, 140.0, 150.0]);
}

#[test]
fn warranty_time_to_failure_rows_and_sums() {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/datasets/reliasoft_warranty_time_to_failure.csv"
    );
    let rows = load(p);
    // Grouped table: total units 1510, of which 10 failed.
    check(&rows, 4, &[(0, 600.0, 1e-9), (2, 1510.0, 1e-9)], p);
    let failures: f64 = rows.iter().map(|r| r[1] * r[2]).sum();
    assert_eq!(failures, 10.0);
}

// ---------------------------------------------------------------------
// SMRD textbook data (grouped life data with counts)
// ---------------------------------------------------------------------

#[test]
fn smrd_bearingcage_rows_and_sums() {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/datasets/smrd_bearingcage.csv");
    let rows = load(p);
    // 25 grouped rows covering 1703 assemblies: 6 failures, 1697
    // censored, matching the SMRD documentation totals.
    check(&rows, 25, &[(0, 22846.0, 1e-9), (2, 1703.0, 1e-9)], p);
    let failures: f64 = rows.iter().map(|r| r[1] * r[2]).sum();
    assert_eq!(failures, 6.0);
}

// ---------------------------------------------------------------------
// Fit verification against parameters printed on the source pages.
//
// Tolerance: the pages print beta and eta to 4 decimal places.
// ReliaKit MLE agrees to within 0.0001 on beta and 0.0016 on eta in
// the worst case below (the eta difference on the life data example
// is optimizer convergence, not rounding of the last printed digit).
// The asserted tolerances are beta +/- 0.002 and eta +/- 0.005, about
// an order of magnitude above the observed differences and far below
// any difference a transcription slip would cause.
// ---------------------------------------------------------------------

fn life_fit(path: &str) -> (f64, f64) {
    let rows = load(path);
    let times: Vec<f64> = rows.iter().map(|r| r[0]).collect();
    let failed: Vec<bool> = rows.iter().map(|r| r[1] > 0.5).collect();
    let fit = fit_mle("weibull2", &times, Some(&failed)).expect("weibull2 MLE fit");
    (fit.params["beta"], fit.params["eta"])
}

#[test]
fn fit_life_data_example_matches_printed_mle() {
    // Source: https://help.reliasoft.com/weibull22/content/life_data_analysis_example.htm
    // Printed (2P Weibull, MLE): beta = 1.5070, eta = 128.6414.
    // ReliaKit MLE on the shipped transcription: beta = 1.5069, eta = 128.6430.
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/datasets/reliasoft_life_data_example.csv"
    );
    let (beta, eta) = life_fit(p);
    assert!(
        (beta - 1.5070).abs() <= 0.002,
        "beta {} vs printed 1.5070",
        beta
    );
    assert!(
        (eta - 128.6414).abs() <= 0.005,
        "eta {} vs printed 128.6414",
        eta
    );
}

#[test]
fn fit_unbiasing_complete_matches_printed_mle() {
    // Source: https://help.reliasoft.com/articles/content/hotwire/issue109/relbasics109.htm
    // Printed (2P Weibull, MLE): beta = 1.7255, eta = 103.8609.
    // ReliaKit MLE on the shipped transcription: beta = 1.7255, eta = 103.8606.
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/datasets/reliasoft_unbiasing_complete.csv"
    );
    let (beta, eta) = life_fit(p);
    assert!(
        (beta - 1.7255).abs() <= 0.002,
        "beta {} vs printed 1.7255",
        beta
    );
    assert!(
        (eta - 103.8609).abs() <= 0.005,
        "eta {} vs printed 103.8609",
        eta
    );
}

#[test]
fn fit_unbiasing_suspended_matches_printed_mle() {
    // Source: https://help.reliasoft.com/articles/content/hotwire/issue109/relbasics109.htm
    // Printed (2P Weibull, MLE): beta = 1.7670, eta = 100.2278.
    // ReliaKit MLE on the shipped transcription: beta = 1.7669, eta = 100.2279.
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/datasets/reliasoft_unbiasing_suspended.csv"
    );
    let (beta, eta) = life_fit(p);
    assert!(
        (beta - 1.7670).abs() <= 0.002,
        "beta {} vs printed 1.7670",
        beta
    );
    assert!(
        (eta - 100.2278).abs() <= 0.005,
        "eta {} vs printed 100.2278",
        eta
    );
}
