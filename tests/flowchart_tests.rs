use reliakit::flowchart::simulate_flow;
use serde_json::json;

#[test]
fn deterministic_chain() {
    let spec = json!({
        "nodes": [
            {"id": "s", "type": "start", "next": "a"},
            {"id": "a", "type": "task", "name": "first",
             "time": {"dist": "fixed", "value": 5.0}, "next": "b"},
            {"id": "b", "type": "task", "name": "second",
             "time": {"dist": "fixed", "value": 7.0}, "next": "e"},
            {"id": "e", "type": "end"}
        ]
    });
    let r = simulate_flow(&spec, 200, 42).expect("deterministic chain should simulate");
    assert!(
        (r.mean_time - 12.0).abs() < 1e-9,
        "mean_time {}",
        r.mean_time
    );
    assert!(r.sd_time.abs() < 1e-9, "sd_time {}", r.sd_time);
    assert!((r.p50 - 12.0).abs() < 1e-9, "p50 {}", r.p50);
    assert_eq!(r.n_sim, 200);
}

#[test]
fn known_mixture() {
    let spec = json!({
        "nodes": [
            {"id": "s", "type": "start", "next": "d"},
            {"id": "d", "type": "decision", "branches": [
                {"p": 0.5, "next": "a"},
                {"p": 0.5, "next": "b"}
            ]},
            {"id": "a", "type": "task", "name": "short",
             "time": {"dist": "fixed", "value": 10.0}, "next": "e"},
            {"id": "b", "type": "task", "name": "long",
             "time": {"dist": "fixed", "value": 20.0}, "next": "e"},
            {"id": "e", "type": "end"}
        ]
    });
    let r = simulate_flow(&spec, 20000, 12345).expect("mixture should simulate");
    assert!(
        (r.mean_time - 15.0).abs() < 0.5,
        "mean_time {}",
        r.mean_time
    );
    assert!(
        (r.p50 - 10.0).abs() < 1e-9 || (r.p50 - 20.0).abs() < 1e-9,
        "p50 {}",
        r.p50
    );
    assert!((r.p10 - 10.0).abs() < 1e-9, "p10 {}", r.p10);
    assert!((r.p90 - 20.0).abs() < 1e-9, "p90 {}", r.p90);
}

#[test]
fn loop_semantics() {
    // The task runs exactly 3 times: loop arrivals 1 and 2 stay at or
    // below the limit of 2 and loop back, arrival 3 exceeds it and exits.
    let spec = json!({
        "nodes": [
            {"id": "s", "type": "start", "next": "t"},
            {"id": "t", "type": "task", "name": "attempt",
             "time": {"dist": "fixed", "value": 3.0}, "next": "l"},
            {"id": "l", "type": "loop", "counter": "r", "limit": 2,
             "next": "t", "exit": "e"},
            {"id": "e", "type": "end"}
        ]
    });
    let r = simulate_flow(&spec, 100, 7).expect("loop flow should simulate");
    assert!(
        (r.mean_time - 9.0).abs() < 1e-9,
        "mean_time {}",
        r.mean_time
    );
    assert!(r.sd_time.abs() < 1e-9, "sd_time {}", r.sd_time);
}

#[test]
fn cost_accumulates() {
    let spec = json!({
        "nodes": [
            {"id": "s", "type": "start", "next": "a"},
            {"id": "a", "type": "task", "name": "first",
             "time": {"dist": "fixed", "value": 1.0}, "cost": 10.0, "next": "b"},
            {"id": "b", "type": "task", "name": "second",
             "time": {"dist": "fixed", "value": 2.0}, "cost": 2.5, "next": "e"},
            {"id": "e", "type": "end"}
        ]
    });
    let r = simulate_flow(&spec, 100, 99).expect("cost flow should simulate");
    assert!(
        (r.mean_cost - 12.5).abs() < 1e-9,
        "mean_cost {}",
        r.mean_cost
    );
}

#[test]
fn unbounded_loop_errors() {
    // Limit 100000 is never reached within the 10000 step guard, so the
    // simulation must return an error instead of running forever.
    let spec = json!({
        "nodes": [
            {"id": "s", "type": "start", "next": "t"},
            {"id": "t", "type": "task", "name": "attempt",
             "time": {"dist": "fixed", "value": 1.0}, "next": "l"},
            {"id": "l", "type": "loop", "counter": "r", "limit": 100000,
             "next": "t", "exit": "e"},
            {"id": "e", "type": "end"}
        ]
    });
    let r = simulate_flow(&spec, 10, 3);
    assert!(r.is_err(), "expected Err for unbounded loop, got {:?}", r);
}
