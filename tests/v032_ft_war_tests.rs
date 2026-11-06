//! Tests for the v0.3.2 correctness fixes: fault trees with repeated
//! basic events, and the warranty forecast cutoff and age convention.

use std::collections::{BTreeMap, BTreeSet};

use reliakit::faulttree::{evaluate, minimal_cut_sets, FtNode};
use reliakit::warranty::{forecast_returns, WarrantyRecord};

fn approx(actual: f64, expected: f64, tol: f64) -> bool {
    (actual - expected).abs() <= tol
}

fn basic(name: &str, prob: f64) -> FtNode {
    FtNode::Basic {
        name: name.to_string(),
        prob,
    }
}

// ---- fault trees: repeated events share one identity ----

#[test]
fn ft_or_of_repeated_event_is_single_event_probability() {
    let tree = FtNode::Or {
        name: "top".to_string(),
        children: vec![basic("A", 0.1), basic("A", 0.1)],
    };
    let result = evaluate(&tree).expect("evaluate A OR A");
    assert!(
        approx(result.top_probability, 0.1, 1e-12),
        "A OR A gave {}",
        result.top_probability
    );
    // Cut sets stay name based: the only minimal cut set is {A}.
    let cut_sets: BTreeSet<BTreeSet<String>> = result.cut_sets.into_iter().collect();
    let mut expected: BTreeSet<BTreeSet<String>> = BTreeSet::new();
    expected.insert(["A".to_string()].into_iter().collect());
    assert_eq!(cut_sets, expected);
    assert_eq!(minimal_cut_sets(&tree).len(), 1);
}

#[test]
fn ft_and_of_repeated_event_is_single_event_probability() {
    let tree = FtNode::And {
        name: "top".to_string(),
        children: vec![basic("A", 0.1), basic("A", 0.1)],
    };
    let result = evaluate(&tree).expect("evaluate A AND A");
    assert!(
        approx(result.top_probability, 0.1, 1e-12),
        "A AND A gave {}",
        result.top_probability
    );
}

#[test]
fn ft_shared_event_across_and_branches() {
    // (A AND B) OR (A AND C) is A AND (B OR C):
    // 0.1 * (1 - 0.8 * 0.7) = 0.1 * 0.44 = 0.044.
    let tree = FtNode::Or {
        name: "top".to_string(),
        children: vec![
            FtNode::And {
                name: "left".to_string(),
                children: vec![basic("A", 0.1), basic("B", 0.2)],
            },
            FtNode::And {
                name: "right".to_string(),
                children: vec![basic("A", 0.1), basic("C", 0.3)],
            },
        ],
    };
    let result = evaluate(&tree).expect("evaluate shared A tree");
    assert!(
        approx(result.top_probability, 0.044, 1e-12),
        "shared A tree gave {}",
        result.top_probability
    );
}

#[test]
fn ft_voting_gate_over_repeated_event_equals_event_probability() {
    // k = 2 over [A, A, B]: the gate fires exactly when A fires,
    // because the two A children fire or fail together and B alone
    // can only supply one vote.
    let tree = FtNode::Voting {
        name: "top".to_string(),
        k: 2,
        children: vec![basic("A", 0.1), basic("A", 0.1), basic("B", 0.2)],
    };
    let result = evaluate(&tree).expect("evaluate voting over repeated A");
    assert!(
        approx(result.top_probability, 0.1, 1e-12),
        "voting over [A, A, B] gave {}",
        result.top_probability
    );
}

#[test]
fn ft_probability_above_one_is_error_naming_event() {
    let tree = FtNode::Or {
        name: "top".to_string(),
        children: vec![basic("pump", 1.5), basic("valve", 0.2)],
    };
    let err = evaluate(&tree).expect_err("probability 1.5 must be rejected");
    assert!(err.contains("pump"), "error should name the event: {err}");
    assert!(err.contains("1.5"), "error should give the value: {err}");
}

#[test]
fn ft_nan_probability_is_error_naming_event() {
    let tree = basic("sensor", f64::NAN);
    let err = evaluate(&tree).expect_err("NaN probability must be rejected");
    assert!(err.contains("sensor"), "error should name the event: {err}");
}

#[test]
fn ft_conflicting_probabilities_for_one_name_is_error() {
    let tree = FtNode::And {
        name: "top".to_string(),
        children: vec![basic("A", 0.1), basic("A", 0.2)],
    };
    let err = evaluate(&tree).expect_err("conflicting probabilities must be rejected");
    assert!(err.contains("'A'"), "error should name the event: {err}");
}

// ---- fault trees: independent brute force truth table ----

fn brute_collect(node: &FtNode, probs: &mut BTreeMap<String, f64>) {
    match node {
        FtNode::Basic { name, prob } => {
            probs.insert(name.clone(), *prob);
        }
        FtNode::And { children, .. }
        | FtNode::Or { children, .. }
        | FtNode::Voting { children, .. } => {
            for child in children {
                brute_collect(child, probs);
            }
        }
    }
}

fn brute_eval(node: &FtNode, truth: &BTreeMap<String, bool>) -> bool {
    match node {
        FtNode::Basic { name, .. } => truth[name],
        FtNode::And { children, .. } => children.iter().all(|child| brute_eval(child, truth)),
        FtNode::Or { children, .. } => children.iter().any(|child| brute_eval(child, truth)),
        FtNode::Voting { k, children, .. } => {
            children
                .iter()
                .filter(|child| brute_eval(child, truth))
                .count()
                >= *k
        }
    }
}

/// Independent assignment enumeration, written for this test only:
/// enumerate every truth assignment of the distinct events, evaluate
/// the boolean tree directly, and weight by the event probabilities.
fn brute_force_top_probability(node: &FtNode) -> f64 {
    let mut probs: BTreeMap<String, f64> = BTreeMap::new();
    brute_collect(node, &mut probs);
    let names: Vec<String> = probs.keys().cloned().collect();
    let n = names.len();
    let mut total = 0.0f64;
    for mask in 0..(1usize << n) {
        let mut truth: BTreeMap<String, bool> = BTreeMap::new();
        let mut weight = 1.0f64;
        for (position, name) in names.iter().enumerate() {
            let fires = (mask >> position) & 1 == 1;
            truth.insert(name.clone(), fires);
            let p = probs[name];
            weight *= if fires { p } else { 1.0 - p };
        }
        if brute_eval(node, &truth) {
            total += weight;
        }
    }
    total
}

#[test]
fn ft_evaluate_matches_brute_force_truth_table() {
    // Mixed tree 1: a voting gate whose children are gates, with A
    // and B each repeated across different branches.
    let tree_one = FtNode::Voting {
        name: "top".to_string(),
        k: 2,
        children: vec![
            FtNode::Or {
                name: "g1".to_string(),
                children: vec![basic("A", 0.1), basic("B", 0.2)],
            },
            FtNode::And {
                name: "g2".to_string(),
                children: vec![basic("A", 0.1), basic("C", 0.3)],
            },
            basic("B", 0.2),
        ],
    };
    // Mixed tree 2: nested AND and OR feeding a voting gate, with A
    // and C repeated at different depths.
    let tree_two = FtNode::Or {
        name: "top".to_string(),
        children: vec![
            FtNode::And {
                name: "g1".to_string(),
                children: vec![
                    basic("A", 0.15),
                    FtNode::Or {
                        name: "g2".to_string(),
                        children: vec![basic("B", 0.25), basic("C", 0.05)],
                    },
                ],
            },
            FtNode::Voting {
                name: "g3".to_string(),
                k: 2,
                children: vec![basic("C", 0.05), basic("A", 0.15), basic("D", 0.4)],
            },
        ],
    };
    for tree in [&tree_one, &tree_two] {
        let expected = brute_force_top_probability(tree);
        let result = evaluate(tree).expect("evaluate mixed tree");
        assert!(
            approx(result.top_probability, expected, 1e-12),
            "evaluate gave {}, brute force gave {}",
            result.top_probability,
            expected
        );
        assert!(expected > 0.0 && expected < 1.0);
    }
}

#[test]
fn ft_large_tree_without_repeats_uses_recursive_fallback() {
    // 21 distinct events, no repeats: exact enumeration is over the
    // limit, so evaluate falls back to the recursive computation.
    let children: Vec<FtNode> = (0..21).map(|i| basic(&format!("E{i}"), 0.05)).collect();
    let tree = FtNode::Or {
        name: "top".to_string(),
        children,
    };
    let result = evaluate(&tree).expect("large tree without repeats should evaluate");
    let expected = 1.0 - 0.95f64.powi(21);
    assert!(
        approx(result.top_probability, expected, 1e-12),
        "fallback gave {}, expected {}",
        result.top_probability,
        expected
    );
}

#[test]
fn ft_large_tree_with_repeats_beyond_limit_is_error() {
    // 21 distinct events with one repeat: too large for exact
    // repeated event evaluation, so it must error, not approximate.
    let mut children: Vec<FtNode> = (0..21).map(|i| basic(&format!("E{i}"), 0.05)).collect();
    children.push(basic("E0", 0.05));
    let tree = FtNode::Or {
        name: "top".to_string(),
        children,
    };
    let err = evaluate(&tree).expect_err("large repeated tree must be rejected");
    assert!(
        err.contains("repeated") && err.contains("20"),
        "error should explain the exact evaluation limit: {err}"
    );
}

// ---- warranty: cutoff, excess returns, and age convention ----

fn rec(sale_period: usize, return_period: usize) -> WarrantyRecord {
    WarrantyRecord {
        sale_period,
        return_period,
    }
}

fn base_sales() -> Vec<u32> {
    vec![100, 100, 100]
}

fn base_records() -> Vec<WarrantyRecord> {
    vec![
        rec(0, 0),
        rec(0, 0),
        rec(0, 1),
        rec(0, 2),
        rec(1, 1),
        rec(1, 2),
        rec(1, 2),
        rec(2, 2),
    ]
}

#[test]
fn war_future_return_does_not_change_forecast() {
    let sales = base_sales();
    let before =
        forecast_returns(&sales, &base_records(), 2, 3).expect("baseline warranty forecast");
    assert_eq!(before.excluded_future_returns, 0);
    assert_eq!(before.units_in_service, 292.0);

    let mut records = base_records();
    records.push(rec(0, 20));
    let after = forecast_returns(&sales, &records, 2, 3).expect("forecast with a future return");

    assert_eq!(after.excluded_future_returns, 1);
    // The Weibull fit inputs are unaffected, so the fit and the
    // forecast come out identical.
    assert!(approx(after.beta, before.beta, 1e-12), "beta changed");
    assert!(approx(after.eta, before.eta, 1e-12), "eta changed");
    assert!(
        approx(after.expected_returns, before.expected_returns, 1e-12),
        "expected returns changed: {} vs {}",
        after.expected_returns,
        before.expected_returns
    );
    assert_eq!(after.units_in_service, before.units_in_service);
    // The result reports the exclusion and the cutoff period.
    assert!(
        after
            .notes
            .iter()
            .any(|note| note.contains("Excluded 1 future return")
                && note.contains("cutoff period 2")),
        "notes should report the exclusion and cutoff: {:?}",
        after.notes
    );
    assert!(
        after
            .notes
            .iter()
            .any(|note| note.contains("r - s + 1") && note.contains("c - s + 1")),
        "notes should state the age convention: {:?}",
        after.notes
    );
}

#[test]
fn war_cohort_conservation_returns_plus_in_service_equals_sales() {
    // Counted returns per cohort under the cutoff rule: cohort 0 has
    // 4, cohort 1 has 2, cohort 2 has 1. The return at period 20 is
    // a future return and the record returned before its sale is a
    // data error, so neither is counted.
    let sales = vec![50, 40, 30];
    let current = 2usize;
    let records = vec![
        rec(0, 0),
        rec(0, 1),
        rec(0, 2),
        rec(0, 2),
        rec(1, 1),
        rec(1, 2),
        rec(2, 2),
        rec(2, 20),
        rec(2, 1),
    ];
    let counted_returns = [4.0f64, 2.0, 1.0];

    let full = forecast_returns(&sales, &records, current, 2).expect("mixed forecast");
    assert_eq!(full.excluded_future_returns, 1);
    let total_counted: f64 = counted_returns.iter().sum();
    assert!(
        approx(
            total_counted + full.units_in_service,
            sales.iter().map(|s| *s as f64).sum::<f64>(),
            1e-9
        ),
        "total conservation failed: {} counted + {} in service",
        total_counted,
        full.units_in_service
    );

    // Per cohort: removing one cohort (sales zeroed, its records
    // dropped) lowers units in service by exactly that cohort's in
    // service count, and returns plus in service must equal sales.
    for cohort in 0..sales.len() {
        let mut reduced_sales = sales.clone();
        reduced_sales[cohort] = 0;
        let reduced_records: Vec<WarrantyRecord> = records
            .iter()
            .copied()
            .filter(|record| record.sale_period != cohort)
            .collect();
        let reduced = forecast_returns(&reduced_sales, &reduced_records, current, 2)
            .expect("reduced forecast");
        let cohort_in_service = full.units_in_service - reduced.units_in_service;
        assert!(
            approx(
                counted_returns[cohort] + cohort_in_service,
                sales[cohort] as f64,
                1e-9
            ),
            "cohort {cohort}: {} returns + {cohort_in_service} in service != {} sales",
            counted_returns[cohort],
            sales[cohort]
        );
    }
}

#[test]
fn war_excess_returns_is_error_naming_cohort_and_counts() {
    // Cohort 1 sold 4 units but has 5 counted returns.
    let sales = vec![10, 4];
    let mut records = vec![rec(0, 0), rec(0, 1)];
    for _ in 0..5 {
        records.push(rec(1, 1));
    }
    let err =
        forecast_returns(&sales, &records, 1, 2).expect_err("excess returns must be an error");
    assert!(
        err.contains("sale period 1"),
        "error should name the cohort: {err}"
    );
    assert!(
        err.contains("5 returns"),
        "error should give the return count: {err}"
    );
    assert!(
        err.contains("sales of 4"),
        "error should give the sales count: {err}"
    );
}

#[test]
fn war_same_period_return_is_failure_at_age_one() {
    // Four returns in the sale period itself (r == s) are failures
    // at age 1, so cohort 1 keeps 50 - 4 = 46 units in service.
    let sales = vec![100, 50];
    let mut records = vec![rec(0, 0), rec(0, 0), rec(0, 1), rec(0, 1), rec(0, 1)];
    for _ in 0..4 {
        records.push(rec(1, 1));
    }
    let forecast = forecast_returns(&sales, &records, 1, 2).expect("same period forecast");
    assert_eq!(forecast.units_in_service, 95.0 + 46.0);
    assert_eq!(forecast.excluded_future_returns, 0);
    assert!(forecast.expected_returns.is_finite() && forecast.expected_returns >= 0.0);
}

#[test]
fn war_cohort_sold_in_current_period_contributes_units_in_service() {
    // Cohort 1 is sold in the current period (s == c == 1) with no
    // returns: its 50 units are in service (censored at age 1)
    // rather than skipped.
    let sales = vec![100, 50];
    let records = vec![rec(0, 0), rec(0, 0), rec(0, 1), rec(0, 1), rec(0, 1)];
    let forecast = forecast_returns(&sales, &records, 1, 2).expect("current cohort forecast");
    assert_eq!(forecast.units_in_service, 95.0 + 50.0);
    assert!(forecast.beta.is_finite() && forecast.beta > 0.0);
    assert!(forecast.eta.is_finite() && forecast.eta > 0.0);
}

#[test]
fn war_return_before_sale_does_not_enter_fit() {
    let sales = base_sales();
    let before = forecast_returns(&sales, &base_records(), 2, 3).expect("baseline forecast");
    let mut records = base_records();
    records.push(rec(2, 1));
    let after = forecast_returns(&sales, &records, 2, 3).expect("forecast with bad record");
    assert!(approx(after.beta, before.beta, 1e-12));
    assert!(approx(after.eta, before.eta, 1e-12));
    assert!(approx(
        after.expected_returns,
        before.expected_returns,
        1e-12
    ));
    assert_eq!(after.units_in_service, before.units_in_service);
    assert_eq!(after.excluded_future_returns, 0);
    assert!(
        after
            .notes
            .iter()
            .any(|note| note.contains("before the sale period")),
        "notes should count the data error record: {:?}",
        after.notes
    );
}
