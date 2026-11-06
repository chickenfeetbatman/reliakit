use reliakit::{doe, faulttree, fmea, prediction, warranty};

use std::collections::BTreeSet;

fn approx(actual: f64, expected: f64, tolerance: f64) -> bool {
    (actual - expected).abs() <= tolerance
}

#[test]
fn fmea_rpn_and_revised_rpn() {
    let mut row = fmea::FmeaRow::new("widget", "cracks", 8, 5, 4);
    assert_eq!(row.rpn(), 160);
    assert_eq!(row.revised_rpn(), None);

    row.revised_occurrence = Some(2);
    assert_eq!(row.revised_rpn(), Some(64));
}

#[test]
fn fmea_csv_round_trip_preserves_row_with_comma() {
    let mut row = fmea::FmeaRow::new("pump, main", "leaks", 8, 5, 4);
    row.function = "move fluid".to_string();
    row.effect = "loss of flow".to_string();
    row.cause = "seal wear".to_string();
    row.recommended_action = "replace seal".to_string();
    row.revised_occurrence = Some(2);

    let csv = fmea::rows_to_csv(&[row]);
    let parsed = fmea::rows_from_csv(&csv).expect("CSV round trip should parse");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].item, "pump, main");
    assert_eq!(parsed[0].function, "move fluid");
    assert_eq!(parsed[0].failure_mode, "leaks");
    assert_eq!(parsed[0].effect, "loss of flow");
    assert_eq!(parsed[0].cause, "seal wear");
    assert_eq!(parsed[0].severity, 8);
    assert_eq!(parsed[0].occurrence, 5);
    assert_eq!(parsed[0].detection, 4);
    assert_eq!(parsed[0].rpn(), 160);
    assert_eq!(parsed[0].recommended_action, "replace seal");
    assert_eq!(parsed[0].revised_severity, None);
    assert_eq!(parsed[0].revised_occurrence, Some(2));
    assert_eq!(parsed[0].revised_detection, None);
    assert_eq!(parsed[0].revised_rpn(), Some(64));
}

#[test]
fn warranty_nevada_table_aggregation() {
    let records = vec![
        warranty::WarrantyRecord {
            sale_period: 0,
            return_period: 0,
        },
        warranty::WarrantyRecord {
            sale_period: 0,
            return_period: 1,
        },
        warranty::WarrantyRecord {
            sale_period: 0,
            return_period: 1,
        },
        warranty::WarrantyRecord {
            sale_period: 1,
            return_period: 2,
        },
        warranty::WarrantyRecord {
            sale_period: 2,
            return_period: 2,
        },
    ];

    let table = warranty::nevada_table(3, &records, 2);
    assert_eq!(table[0], vec![1, 2, 0]);
    assert_eq!(warranty::returns_by_age(&table), vec![2, 3, 0]);
    assert_eq!(warranty::total_returns(&table), 5);
}

#[test]
fn faulttree_gate_probabilities() {
    let or_tree = faulttree::FtNode::Or {
        name: "top".to_string(),
        children: vec![
            faulttree::FtNode::Basic {
                name: "a".to_string(),
                prob: 0.1,
            },
            faulttree::FtNode::Basic {
                name: "b".to_string(),
                prob: 0.2,
            },
        ],
    };
    assert!(approx(faulttree::top_probability(&or_tree), 0.28, 1e-12));

    let and_tree = faulttree::FtNode::And {
        name: "top".to_string(),
        children: vec![
            faulttree::FtNode::Basic {
                name: "a".to_string(),
                prob: 0.1,
            },
            faulttree::FtNode::Basic {
                name: "b".to_string(),
                prob: 0.2,
            },
        ],
    };
    assert!(approx(faulttree::top_probability(&and_tree), 0.02, 1e-12));

    let voting_tree = faulttree::FtNode::Voting {
        name: "top".to_string(),
        k: 2,
        children: vec![
            faulttree::FtNode::Basic {
                name: "a".to_string(),
                prob: 0.1,
            },
            faulttree::FtNode::Basic {
                name: "b".to_string(),
                prob: 0.1,
            },
            faulttree::FtNode::Basic {
                name: "c".to_string(),
                prob: 0.1,
            },
        ],
    };
    // 3 * 0.01 * 0.9 + 0.001 = 0.028
    assert!(approx(
        faulttree::top_probability(&voting_tree),
        0.028,
        1e-12
    ));
}

#[test]
fn faulttree_minimal_cut_sets() {
    let tree = faulttree::FtNode::And {
        name: "top".to_string(),
        children: vec![
            faulttree::FtNode::Or {
                name: "left".to_string(),
                children: vec![
                    faulttree::FtNode::Basic {
                        name: "a".to_string(),
                        prob: 0.1,
                    },
                    faulttree::FtNode::Basic {
                        name: "b".to_string(),
                        prob: 0.1,
                    },
                ],
            },
            faulttree::FtNode::Basic {
                name: "c".to_string(),
                prob: 0.1,
            },
        ],
    };

    let cut_sets: BTreeSet<BTreeSet<String>> =
        faulttree::minimal_cut_sets(&tree).into_iter().collect();

    let mut expected: BTreeSet<BTreeSet<String>> = BTreeSet::new();
    expected.insert(["a".to_string(), "c".to_string()].into_iter().collect());
    expected.insert(["b".to_string(), "c".to_string()].into_iter().collect());

    assert_eq!(cut_sets, expected);
}

#[test]
fn prediction_parts_count() {
    let parts = vec![prediction::PartEntry::new("resistor", 2.0, 3)];

    assert!(approx(
        prediction::part_failure_rate_per_million_hours(&parts[0]),
        6.0,
        1e-12
    ));
    assert!(approx(
        prediction::system_failure_rate_per_million_hours(&parts),
        6.0,
        1e-12
    ));

    let mtbf = prediction::system_mtbf_hours(&parts);
    assert!(approx(mtbf, 1e6 / 6.0, 1e-6));

    let reliability = prediction::system_reliability(&parts, mtbf);
    assert!(approx(reliability, (-1.0f64).exp(), 1e-9));
}

#[test]
fn doe_two_factor_effects() {
    let design = doe::factorial_design(2);
    assert_eq!(
        design,
        vec![
            vec![-1.0, -1.0],
            vec![1.0, -1.0],
            vec![-1.0, 1.0],
            vec![1.0, 1.0],
        ]
    );

    let y = vec![10.0, 14.0, 20.0, 24.0];
    let effects = doe::analyze_factorial(2, &y).expect("analysis should succeed");
    assert_eq!(effects.len(), 3);

    let effect_a = effects.iter().find(|e| e.name == "A").expect("A effect");
    let effect_b = effects.iter().find(|e| e.name == "B").expect("B effect");
    let effect_ab = effects.iter().find(|e| e.name == "AB").expect("AB effect");

    assert!(approx(effect_a.effect, 4.0, 1e-9));
    assert!(approx(effect_b.effect, 10.0, 1e-9));
    assert!(approx(effect_ab.effect, 0.0, 1e-9));
}
