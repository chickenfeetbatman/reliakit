//! RENO-style flowchart simulation.
//!
//! A flow is a JSON object with a `nodes` array:
//!
//! ```json
//! {"nodes": [
//!   {"id": "s", "type": "start", "next": "a"},
//!   {"id": "a", "type": "task", "name": "inspect",
//!    "time": {"dist": "fixed", "value": 5.0}, "cost": 12.5, "next": "e"},
//!   {"id": "e", "type": "end"}
//! ]}
//! ```
//!
//! Node schema, by `type`:
//!
//! - `start`: `{"type": "start", "next": "<id>"}`. Exactly one start node
//!   must be present in the flow.
//! - `task`: `{"type": "task", "name": "...", "time": <time spec>,
//!   "next": "<id>"}` with an optional `"cost"` number (default 0). A time
//!   spec is one of:
//!   - `{"dist": "fixed", "value": 5.0}`
//!   - `{"dist": "exponential", "theta": 100.0}` (mean `theta`)
//!   - `{"dist": "weibull2", "beta": .., "eta": ..}`
//!   - `{"dist": "lognormal", "mu": .., "sigma": ..}`
//!   - `{"dist": "normal", "mu": .., "sigma": ..}` (sampled times are
//!     clamped at 0, since a task cannot take negative time)
//! - `decision`: `{"type": "decision", "branches": [{"p": 0.3,
//!   "next": "<id>"}, ...]}`. Branch probabilities need not sum to 1; they
//!   are normalised by their sum. A sum at or below 0 is an error.
//! - `loop`: `{"type": "loop", "counter": "retries", "limit": 2,
//!   "next": "<id>", "exit": "<id>"}`. Each arrival at the node increments
//!   the named counter for the current run. If the new count is at most
//!   `limit` the walk goes to `next`, otherwise it goes to `exit`.
//!   Counters are reset at the start of every run.
//! - `end`: `{"type": "end"}`. Ends the current run.
//!
//! Each run walks from the start node, accumulating task times and task
//! costs, until it reaches an end node. A run is aborted with an error
//! after 10000 steps, which catches unbounded loops. An unknown node id,
//! an unknown node type, and a missing (or duplicated) start node are
//! also errors.

use crate::rng::Rng64;
use serde_json::Value;
use std::collections::HashMap;

/// Maximum number of node visits allowed in a single run.
const MAX_STEPS: usize = 10000;

/// Summary of a flowchart simulation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FlowResult {
    pub mean_time: f64,
    pub sd_time: f64,
    pub p10: f64,
    pub p50: f64,
    pub p90: f64,
    pub min_time: f64,
    pub max_time: f64,
    pub mean_cost: f64,
    pub n_sim: usize,
}

fn get_str<'a>(node: &'a Value, key: &str, id: &str) -> Result<&'a str, String> {
    node.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("node '{}': missing or non-string field '{}'", id, key))
}

fn get_f64(node: &Value, key: &str, context: &str) -> Result<f64, String> {
    node.get(key)
        .and_then(|v| v.as_f64())
        .ok_or_else(|| format!("{}: missing or non-numeric field '{}'", context, key))
}

fn get_limit(node: &Value, id: &str) -> Result<i64, String> {
    let v = node
        .get("limit")
        .ok_or_else(|| format!("node '{}': missing field 'limit'", id))?;
    if let Some(x) = v.as_i64() {
        return Ok(x);
    }
    if let Some(x) = v.as_u64() {
        if x <= i64::MAX as u64 {
            return Ok(x as i64);
        }
    }
    if let Some(x) = v.as_f64() {
        if x.fract() == 0.0 && x.abs() < 9.0e15 {
            return Ok(x as i64);
        }
    }
    Err(format!("node '{}': field 'limit' must be an integer", id))
}

fn sample_time(spec: &Value, rng: &mut Rng64, id: &str) -> Result<f64, String> {
    let context = format!("node '{}' time spec", id);
    let dist = spec
        .get("dist")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("{}: missing or non-string field 'dist'", context))?;
    let t = match dist {
        "fixed" => get_f64(spec, "value", &context)?,
        "exponential" => {
            let theta = get_f64(spec, "theta", &context)?;
            rng.exponential(theta)
        }
        "weibull2" => {
            let beta = get_f64(spec, "beta", &context)?;
            let eta = get_f64(spec, "eta", &context)?;
            rng.weibull(beta, eta)
        }
        "lognormal" => {
            let mu = get_f64(spec, "mu", &context)?;
            let sigma = get_f64(spec, "sigma", &context)?;
            rng.lognormal(mu, sigma)
        }
        "normal" => {
            let mu = get_f64(spec, "mu", &context)?;
            let sigma = get_f64(spec, "sigma", &context)?;
            (mu + sigma * rng.normal()).max(0.0)
        }
        other => return Err(format!("node '{}': unknown time dist '{}'", id, other)),
    };
    Ok(t)
}

fn percentile_sorted(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let idx = q * (sorted.len() - 1) as f64;
    let lo = idx.floor() as usize;
    let hi = (lo + 1).min(sorted.len() - 1);
    let frac = idx - lo as f64;
    sorted[lo] * (1.0 - frac) + sorted[hi] * frac
}

/// Simulate a flowchart spec (see the module docs for the node schema)
/// `n_sim` times with a fixed seed and summarise the run times and costs.
pub fn simulate_flow(spec: &Value, n_sim: usize, seed: u64) -> Result<FlowResult, String> {
    if n_sim == 0 {
        return Err("n_sim must be at least 1".to_string());
    }
    let nodes = spec
        .get("nodes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "flow spec: missing or non-array field 'nodes'".to_string())?;
    if nodes.is_empty() {
        return Err("flow spec: 'nodes' is empty".to_string());
    }

    // Index nodes by id and find the start node.
    let mut index: HashMap<&str, &Value> = HashMap::new();
    let mut start_id: Option<&str> = None;
    let mut n_start = 0usize;
    for node in nodes {
        let id = node
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "flow spec: every node needs a string 'id'".to_string())?;
        if index.insert(id, node).is_some() {
            return Err(format!("flow spec: duplicate node id '{}'", id));
        }
        let ty = node
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("node '{}': missing or non-string field 'type'", id))?;
        match ty {
            "start" | "task" | "decision" | "loop" | "end" => {}
            other => return Err(format!("node '{}': unknown type '{}'", id, other)),
        }
        if ty == "start" {
            n_start += 1;
            start_id = Some(id);
        }
    }
    if n_start == 0 {
        return Err("flow spec: missing start node".to_string());
    }
    if n_start > 1 {
        return Err(format!(
            "flow spec: expected exactly one start node, found {}",
            n_start
        ));
    }
    let start_id = start_id.unwrap();

    let mut rng = Rng64::new(seed);
    let mut times: Vec<f64> = Vec::with_capacity(n_sim);
    let mut total_cost_sum = 0.0f64;

    for _ in 0..n_sim {
        let mut counters: HashMap<String, i64> = HashMap::new();
        let mut current: &str = start_id;
        let mut total_time = 0.0f64;
        let mut total_cost = 0.0f64;
        let mut steps = 0usize;
        loop {
            steps += 1;
            if steps > MAX_STEPS {
                return Err(format!(
                    "flow exceeded {} steps in one run (possible unbounded loop)",
                    MAX_STEPS
                ));
            }
            let node = index
                .get(current)
                .ok_or_else(|| format!("unknown node id: '{}'", current))?;
            let id = current;
            let ty = node
                .get("type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("node '{}': missing or non-string field 'type'", id))?;
            match ty {
                "start" => {
                    current = get_str(node, "next", id)?;
                }
                "task" => {
                    let time_spec = node
                        .get("time")
                        .ok_or_else(|| format!("node '{}': missing field 'time'", id))?;
                    total_time += sample_time(time_spec, &mut rng, id)?;
                    if let Some(c) = node.get("cost") {
                        total_cost += c.as_f64().ok_or_else(|| {
                            format!("node '{}': field 'cost' must be a number", id)
                        })?;
                    }
                    current = get_str(node, "next", id)?;
                }
                "decision" => {
                    let branches =
                        node.get("branches")
                            .and_then(|v| v.as_array())
                            .ok_or_else(|| {
                                format!("node '{}': missing or non-array field 'branches'", id)
                            })?;
                    if branches.is_empty() {
                        return Err(format!("node '{}': 'branches' is empty", id));
                    }
                    let mut ps: Vec<f64> = Vec::with_capacity(branches.len());
                    let mut sum = 0.0f64;
                    for b in branches {
                        let p = b.get("p").and_then(|v| v.as_f64()).ok_or_else(|| {
                            format!("node '{}': branch missing or non-numeric field 'p'", id)
                        })?;
                        ps.push(p);
                        sum += p;
                    }
                    if sum <= 0.0 || !sum.is_finite() {
                        return Err(format!(
                            "node '{}': branch probabilities must sum to a positive value",
                            id
                        ));
                    }
                    let draw = rng.next_f64() * sum;
                    let mut cumulative = 0.0f64;
                    let mut chosen = branches.len() - 1;
                    for (i, p) in ps.iter().enumerate() {
                        cumulative += *p;
                        if draw < cumulative {
                            chosen = i;
                            break;
                        }
                    }
                    current = branches[chosen]
                        .get("next")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            format!("node '{}': branch missing or non-string field 'next'", id)
                        })?;
                }
                "loop" => {
                    let counter = get_str(node, "counter", id)?;
                    let limit = get_limit(node, id)?;
                    let entry = counters.entry(counter.to_string()).or_insert(0);
                    *entry += 1;
                    let count = *entry;
                    if count <= limit {
                        current = get_str(node, "next", id)?;
                    } else {
                        current = get_str(node, "exit", id)?;
                    }
                }
                "end" => break,
                other => return Err(format!("node '{}': unknown type '{}'", id, other)),
            }
        }
        times.push(total_time);
        total_cost_sum += total_cost;
    }

    let n = times.len() as f64;
    let mean_time = times.iter().sum::<f64>() / n;
    let sd_time = (times.iter().map(|x| (x - mean_time).powi(2)).sum::<f64>() / n).sqrt();
    let min_time = times.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_time = times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mean_cost = total_cost_sum / n;

    let mut sorted = times.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    Ok(FlowResult {
        mean_time,
        sd_time,
        p10: percentile_sorted(&sorted, 0.10),
        p50: percentile_sorted(&sorted, 0.50),
        p90: percentile_sorted(&sorted, 0.90),
        min_time,
        max_time,
        mean_cost,
        n_sim,
    })
}
