// Engine dispatcher tests for v0.3.2 correctness items A, B, C, E, F, H and N.
use reliakit_engine::handle::{handle, new_work};
use reliakit_engine::work::{project_from_value, project_to_value};
use serde_json::{json, Value};

fn call(w: &mut reliakit_engine::work::Work, action: &str, fields: Value) -> Value {
    handle(w, &json!({"action": action, "fields": fields}))
}
fn select_first(w: &mut reliakit_engine::work::Work) -> Value {
    call(w, "select_item", json!({"index": 0}))
}
fn result_value(state: &Value, ui: &str, label_part: &str) -> String {
    let rows = state[ui]["results"].as_array().cloned().unwrap_or_default();
    for r in rows {
        if r["label"].as_str().unwrap_or("").contains(label_part) {
            return r["value"].as_str().unwrap_or("").to_string();
        }
    }
    String::new()
}

#[test]
fn csv_precision_and_fit_matches_direct() {
    let mut w = new_work();
    select_first(&mut w);
    let csv = "time,failed\n0.1,1\n0.2,1\n0.3,1\n0.4,0\n";
    let st = call(&mut w, "csv_load_text", json!({"text": csv}));
    assert_eq!(
        st["LifeUi"]["times"].as_str().unwrap(),
        "0.1, 0.2, 0.3, 0.4c"
    );
    let st = call(&mut w, "life-fit", json!({"LifeUi.t": "0.2"}));
    assert!(st["error"].is_null(), "unexpected error: {}", st["error"]);
    let fit = w.life_fit.clone().expect("fit stored");
    let direct = reliakit::distributions::fit_mle(
        "weibull2",
        &[0.1, 0.2, 0.3, 0.4],
        Some(&[true, true, true, false]),
    )
    .unwrap();
    assert!(
        (fit.params["beta"] - direct.params["beta"]).abs() < 1e-6,
        "beta {} vs {}",
        fit.params["beta"],
        direct.params["beta"]
    );
    assert!(
        (fit.params["eta"] - direct.params["eta"]).abs() < 1e-6,
        "eta {} vs {}",
        fit.params["eta"],
        direct.params["eta"]
    );
    // Project round trip leaves values unchanged
    let proj = project_to_value(&w.project_name, &w.items, w.selected);
    let (_, items, _) = project_from_value(&proj).expect("round trip");
    assert_eq!(
        items[0].state["life_times"].as_str().unwrap(),
        "0.1, 0.2, 0.3, 0.4c"
    );
    // Very small and very large times survive exactly (numerically)
    let csv2 = "time,failed\n0.0001,1\n10000000,1\n";
    let st = call(&mut w, "csv_load_text", json!({"text": csv2}));
    let times_text = st["LifeUi"]["times"].as_str().unwrap().to_string();
    let parsed: Vec<f64> = times_text
        .split(',')
        .map(|s| s.trim().parse::<f64>().unwrap())
        .collect();
    assert_eq!(parsed, vec![0.0001, 10000000.0]);
}

#[test]
fn validation_bad_token_no_fit() {
    let mut w = new_work();
    select_first(&mut w);
    let st = call(
        &mut w,
        "life-fit",
        json!({"LifeUi.times": "10, 20, bad, 30", "LifeUi.t": "1000"}),
    );
    assert!(!st["error"].is_null(), "expected error, got {}", st);
    assert_eq!(st["error"]["token"].as_str().unwrap(), "bad");
    assert!(w.life_fit.is_none());
    assert!(st["status"].as_str().unwrap().contains("bad"));
}

#[test]
fn validation_mission_time_bad() {
    let mut w = new_work();
    select_first(&mut w);
    let st = call(
        &mut w,
        "life-fit",
        json!({"LifeUi.times": "10, 20, 30, 40", "LifeUi.t": "BAD"}),
    );
    assert!(!st["error"].is_null());
    assert_eq!(st["error"]["token"].as_str().unwrap(), "BAD");
    assert!(w.life_fit.is_none());
}

#[test]
fn validation_overflow_and_empty() {
    let mut w = new_work();
    select_first(&mut w);
    let st = call(
        &mut w,
        "life-fit",
        json!({"LifeUi.times": "10, 1e999, 30", "LifeUi.t": "1000"}),
    );
    assert!(!st["error"].is_null());
    assert_eq!(st["error"]["token"].as_str().unwrap(), "1e999");
    let st = call(
        &mut w,
        "life-fit",
        json!({"LifeUi.times": "", "LifeUi.t": "1000"}),
    );
    assert!(!st["error"].is_null());
}

#[test]
fn fit_status_zero_data_not_success() {
    let mut w = new_work();
    select_first(&mut w);
    let st = call(
        &mut w,
        "life-fit",
        json!({"LifeUi.times": "0, 0, 0, 0", "LifeUi.t": "1000"}),
    );
    assert_ne!(
        st["LifeUi"]["fit-status"].as_str().unwrap_or(""),
        "success",
        "status was success: {}",
        st["LifeUi"]
    );
}

#[test]
fn fit_status_identical_times_boundary() {
    let mut w = new_work();
    select_first(&mut w);
    let st = call(
        &mut w,
        "life-fit",
        json!({"LifeUi.times": "1, 1, 1, 1", "LifeUi.t": "1"}),
    );
    assert_eq!(
        st["LifeUi"]["fit-status"].as_str().unwrap(),
        "boundary_or_degenerate",
        "state: {}",
        st["LifeUi"]
    );
}

#[test]
fn fit_status_tight_data() {
    let mut w = new_work();
    select_first(&mut w);
    let st = call(
        &mut w,
        "life-fit",
        json!({"LifeUi.times": "100, 101, 102, 103, 104", "LifeUi.t": "100"}),
    );
    let status = st["LifeUi"]["fit-status"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if status == "success" || status == "success_with_warnings" {
        let beta = w.life_fit.as_ref().unwrap().params["beta"];
        assert!((beta - 80.8).abs() < 20.0, "beta {} not near 80.8", beta);
        assert!(
            (beta - 20.0855).abs() > 1.0,
            "bare boundary beta reported as success"
        );
    } else {
        assert_eq!(status, "boundary_or_degenerate");
        assert!(!st["LifeUi"]["fit-status-note"]
            .as_str()
            .unwrap_or("")
            .is_empty());
    }
}

fn add_ft_item(w: &mut reliakit_engine::work::Work) {
    call(
        w,
        "add_item",
        json!({"new_kind": "Fault Tree", "new-name": "FT test"}),
    );
}

#[test]
fn ft_repeated_event_probability() {
    let mut w = new_work();
    add_ft_item(&mut w);
    let ft = r#"{"Or": {"name": "top", "children": [{"Basic": {"name": "A", "prob": 0.1}}, {"Basic": {"name": "A", "prob": 0.1}}]}}"#;
    let st = call(&mut w, "ft-run", json!({"FtUi.json": ft}));
    assert!(st["error"].is_null(), "error: {}", st["error"]);
    let v = result_value(&st, "FtUi", "Top probability");
    let p: f64 = v.parse().unwrap();
    assert!((p - 0.1).abs() < 1e-9, "top prob {}", p);
}

#[test]
fn ft_prob_out_of_range_errors() {
    let mut w = new_work();
    add_ft_item(&mut w);
    let ft = r#"{"Basic": {"name": "A", "prob": 1.5}}"#;
    let st = call(&mut w, "ft-run", json!({"FtUi.json": ft}));
    assert!(!st["error"].is_null(), "expected error: {}", st);
}

fn add_warranty_item(w: &mut reliakit_engine::work::Work) -> Value {
    call(
        w,
        "add_item",
        json!({"new_kind": "Warranty", "new-name": "War test"}),
    )
}

#[test]
fn warranty_future_return_excluded() {
    let mut w = new_work();
    add_warranty_item(&mut w);
    let base_fields = json!({"WarUi.sales": "120, 150, 140, 160", "WarUi.returns": "0,1\n0,2\n1,2\n1,3\n2,3\n0,3\n2,4\n1,4", "WarUi.current": "3", "WarUi.horizon": "6"});
    let st1 = call(&mut w, "war-run", base_fields.clone());
    let exp1 = result_value(&st1, "WarUi", "Expected returns");
    assert!(!exp1.is_empty(), "no expected returns: {}", st1["WarUi"]);
    let mut fields2 = base_fields.clone();
    fields2["WarUi.returns"] = json!("0,1\n0,2\n1,2\n1,3\n2,3\n0,3\n2,4\n1,4\n0,10");
    let base_excl = st1["WarUi"]["excluded-future-returns"].as_u64().unwrap();
    let st2 = call(&mut w, "war-run", fields2);
    let exp2 = result_value(&st2, "WarUi", "Expected returns");
    assert_eq!(exp1, exp2, "future return changed forecast");
    assert_eq!(
        st2["WarUi"]["excluded-future-returns"].as_u64().unwrap(),
        base_excl + 1
    );
    let excl_row = result_value(&st2, "WarUi", "Excluded future");
    assert_eq!(excl_row, (base_excl + 1).to_string());
}

#[test]
fn outdated_and_flow_gate() {
    let mut w = new_work();
    select_first(&mut w);
    let st = call(&mut w, "life-fit", json!({}));
    assert!(st["error"].is_null());
    assert!(!st["LifeUi"]["outdated"].as_bool().unwrap_or(true));
    let n_items = w.items.len();
    // Change times without recomputing
    let st = call(
        &mut w,
        "state",
        json!({"LifeUi.times": "500, 900, 1400, 2000, 2600"}),
    );
    assert!(
        st["LifeUi"]["outdated"].as_bool().unwrap_or(false),
        "not outdated: {}",
        st["LifeUi"]["outdated"]
    );
    assert!(st["outdated"].as_bool().unwrap_or(false));
    // Flow without confirm: warning, no QCP item created
    let st = call(&mut w, "flow-qcp", json!({}));
    assert_eq!(
        st["warning"]["kind"].as_str().unwrap_or(""),
        "outdated",
        "warning: {}",
        st["warning"]
    );
    assert_eq!(w.items.len(), n_items);
    // With confirm it proceeds
    let st = call(&mut w, "flow-qcp", json!({"confirm_outdated": true}));
    assert!(st["warning"].is_null());
    assert_eq!(w.items.len(), n_items + 1);
}

#[test]
fn duplicate_item_copies_and_is_independent() {
    let mut w = new_work();
    let st0 = select_first(&mut w);
    let times0 = st0["LifeUi"]["times"].as_str().unwrap().to_string();
    let n0 = w.items.len();
    let st = call(&mut w, "duplicate_item", json!({}));
    assert_eq!(w.items.len(), n0 + 1);
    let names: Vec<String> = st["AppState"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap().to_string())
        .collect();
    assert!(
        names.iter().any(|n| n.ends_with(" copy")),
        "names: {:?}",
        names
    );
    assert_eq!(st["LifeUi"]["times"].as_str().unwrap(), times0);
    // Editing the copy does not change the original
    call(&mut w, "state", json!({"LifeUi.times": "11, 22, 33"}));
    let st = call(&mut w, "select_item", json!({"index": 0}));
    assert_eq!(st["LifeUi"]["times"].as_str().unwrap(), times0);
}

#[test]
fn example_load_has_meaningful_name() {
    let mut w = new_work();
    let st = call(&mut w, "card_action", json!({"id": "example:3"}));
    let items = st["AppState"]["items"].as_array().unwrap();
    let last = items.last().unwrap();
    assert_eq!(
        last["name"].as_str().unwrap(),
        "Example: RBD with cold standby"
    );
    let st = call(&mut w, "card_action", json!({"id": "example:1"}));
    let items = st["AppState"]["items"].as_array().unwrap();
    assert_eq!(
        items.last().unwrap()["name"].as_str().unwrap(),
        "Example: Bearing life data"
    );
}

#[test]
fn probability_chart_title_renamed() {
    let mut w = new_work();
    select_first(&mut w);
    let st = call(&mut w, "life-fit", json!({}));
    assert_eq!(st["chart"]["title"].as_str().unwrap(), "Reliability curve");
    let st = call(&mut w, "state", json!({"LifeUi.show-prob": true}));
    assert_eq!(
        st["chart"]["title"].as_str().unwrap(),
        "Empirical vs fitted probability"
    );
    assert_eq!(
        st["LifeUi"]["chart-title"].as_str().unwrap(),
        "Empirical vs fitted probability"
    );
}
