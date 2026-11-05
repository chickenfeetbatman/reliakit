//! System reliability (BlockSim basics) plus standby redundancy,
//! Markov (CTMC) solvers and Monte Carlo availability simulation.
//!
//! Structure specs are JSON values with the same schema as the Python
//! suite, so diagrams are interchangeable:
//!   {"type": "block", "R": 0.95} or {"type": "block", "dist": ...,
//!    "params": {...}, "name": ..., "mtbf": ..., "mttr": ...}
//!   {"type": "series", "children": [...]}
//!   {"type": "parallel", "children": [...]}
//!   {"type": "kofn", "k": 2, "children": [...]}
//!   {"type": "standby_group", "n_units": 2, "dist": "exponential",
//!    "params": {"theta": 1000.0}, "mttr": 10.0, "dormancy": 0.0,
//!    "switch_reliability": 1.0, "swap_time": 0.0}

use crate::distributions::{self, Params};
use crate::optim;
use crate::rng::Rng64;
use serde_json::Value;
use std::collections::BTreeMap;

pub fn series_reliability(rs: &[f64]) -> f64 {
    rs.iter().product()
}

pub fn parallel_reliability(rs: &[f64]) -> f64 {
    1.0 - rs.iter().map(|r| 1.0 - r).product::<f64>()
}

/// k-out-of-n reliability for independent blocks (Poisson-binomial DP).
pub fn kofn_reliability(rs: &[f64], k: usize) -> f64 {
    let n = rs.len();
    let mut dp = vec![0.0; n + 1];
    dp[0] = 1.0;
    for &r in rs {
        let mut ndp = vec![0.0; n + 1];
        for j in 0..=n {
            ndp[j] += dp[j] * (1.0 - r);
            if j > 0 {
                ndp[j] += dp[j - 1] * r;
            }
        }
        // the loop above double counts the top cell; do it the direct way:
        ndp = vec![0.0; n + 1];
        for j in 0..=n {
            let stay = dp[j] * (1.0 - r);
            let up = if j > 0 { dp[j - 1] * r } else { 0.0 };
            ndp[j] = stay + up;
        }
        dp = ndp;
    }
    dp[k.min(n)..].iter().sum()
}

fn params_from_value(v: &Value) -> Params {
    let mut p = Params::new();
    if let Some(obj) = v.as_object() {
        for (k, val) in obj {
            if let Some(x) = val.as_f64() {
                p.insert(k.clone(), x);
            }
        }
    }
    p
}

pub fn block_reliability(block: &Value, t: Option<f64>) -> Result<f64, String> {
    if let Some(r) = block.get("R").and_then(|v| v.as_f64()) {
        return Ok(r);
    }
    let dist = block
        .get("dist")
        .and_then(|v| v.as_str())
        .ok_or("block needs R or dist")?;
    let params = params_from_value(block.get("params").unwrap_or(&Value::Null));
    let tt = t.ok_or("distribution block needs mission time t")?;
    Ok(distributions::reliability(dist, &params, tt))
}

/// Evaluate system reliability of a JSON RBD spec at mission time t.
pub fn evaluate_rbd(spec: &Value, t: Option<f64>) -> Result<f64, String> {
    let ty = spec
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or("spec missing type")?;
    match ty {
        "block" => block_reliability(spec, t),
        "standby_group" => {
            let dist = spec.get("dist").and_then(|v| v.as_str()).unwrap_or("exponential");
            let params = params_from_value(spec.get("params").unwrap_or(&Value::Null));
            let n_units = spec.get("n_units").and_then(|v| v.as_u64()).unwrap_or(2) as usize;
            let dormancy = spec.get("dormancy").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let sw = spec
                .get("switch_reliability")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0);
            let tt = t.ok_or("standby group needs mission time t")?;
            Ok(standby_reliability(tt, dist, &params, n_units, dormancy, sw))
        }
        "series" | "parallel" | "kofn" => {
            let children = spec
                .get("children")
                .and_then(|v| v.as_array())
                .ok_or("node needs children")?;
            let mut rs = Vec::new();
            for c in children {
                rs.push(evaluate_rbd(c, t)?);
            }
            match ty {
                "series" => Ok(series_reliability(&rs)),
                "parallel" => Ok(parallel_reliability(&rs)),
                _ => {
                    let k = spec.get("k").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
                    Ok(kofn_reliability(&rs, k))
                }
            }
        }
        _ => Err(format!("unknown RBD node type {}", ty)),
    }
}

/// Validate a JSON RBD spec (the drag-and-drop app schema). Blocks must
/// carry a name. Returns the spec unchanged on success.
pub fn rbd_from_json(obj: Value) -> Result<Value, String> {
    fn check(node: &Value) -> Result<(), String> {
        let ty = node
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or("node missing type")?;
        if ty == "block" {
            if node.get("name").is_none() {
                return Err("block node needs a name".into());
            }
            return Ok(());
        }
        if ty == "standby_group" {
            return Ok(());
        }
        let children = node
            .get("children")
            .and_then(|v| v.as_array())
            .ok_or("node needs children")?;
        for c in children {
            check(c)?;
        }
        Ok(())
    }
    check(&obj)?;
    Ok(obj)
}

// ---------------- standby reliability ----------------

/// Cold standby reliability for identical exponential units.
/// For n_units = 2: R = e^{-lt} (1 + R_sw * lt). Otherwise the Poisson
/// sum with the switch reliability applied per term.
pub fn cold_standby_exponential(
    t: f64,
    lam: f64,
    n_units: usize,
    switch_reliability: f64,
) -> f64 {
    let lt = lam * t;
    let base = (-lt).exp();
    if n_units == 2 {
        return base * (1.0 + switch_reliability * lt);
    }
    let mut total = 0.0;
    let mut term = 1.0;
    for i in 0..n_units {
        if i > 0 {
            term *= switch_reliability * lt / i as f64;
        }
        total += term;
    }
    base * total
}

fn simpson<F: Fn(f64) -> f64>(f: F, a: f64, b: f64, n: usize) -> f64 {
    let n = if n % 2 == 0 { n } else { n + 1 };
    let h = (b - a) / n as f64;
    let mut sum = f(a) + f(b);
    for i in 1..n {
        let x = a + i as f64 * h;
        sum += if i % 2 == 0 { 2.0 } else { 4.0 } * f(x);
    }
    sum * h / 3.0
}

/// Reliability of 1 active + (n_units - 1) standby units at time t.
/// dormancy d in [0, 1]: standby units age at rate d while waiting
/// (0 = cold, 1 = hot). Switch reliability multiplies each switch.
pub fn standby_reliability(
    t: f64,
    dist: &str,
    params: &Params,
    n_units: usize,
    dormancy: f64,
    switch_reliability: f64,
) -> f64 {
    if n_units <= 1 {
        return distributions::reliability(dist, params, t);
    }
    if dormancy >= 1.0 {
        let r1 = distributions::reliability(dist, params, t);
        return 1.0 - (1.0 - r1).powi(n_units as i32);
    }
    if dist == "exponential" && dormancy == 0.0 {
        let lam = 1.0 / params["theta"];
        return cold_standby_exponential(t, lam, n_units, switch_reliability);
    }
    if n_units == 2 {
        let r1 = distributions::reliability(dist, params, t);
        let integrand = |u: f64| -> f64 {
            let f = distributions::pdf(dist, params, u);
            let va = dormancy * u;
            let denom = if va > 0.0 {
                distributions::reliability(dist, params, va)
            } else {
                1.0
            };
            let cond = distributions::reliability(dist, params, t - u + va) / denom.max(1e-300);
            f * cond
        };
        let val = simpson(integrand, 0.0, t, 4000);
        return (r1 + switch_reliability * val).clamp(0.0, 1.0);
    }
    if n_units <= 5 {
        // exact recursive convolution (virtual-age quadrature)
        return crate::standby::standby_reliability_quad(
            t,
            dist,
            params,
            n_units,
            dormancy,
            switch_reliability,
        )
        .clamp(0.0, 1.0);
    }
    standby_reliability_mc(t, dist, params, n_units, dormancy, switch_reliability, 40000, 99)
        .clamp(0.0, 1.0)
}

fn draw_lifetime(rng: &mut Rng64, dist: &str, params: &Params) -> f64 {
    match dist {
        "weibull2" => rng.weibull(params["beta"], params["eta"]),
        "weibull3" => params["gamma"] + rng.weibull(params["beta"], params["eta"]),
        "lognormal" => rng.lognormal(params["mu"], params["sigma"]),
        "exponential" => rng.exponential(params["theta"]),
        "normal" => (params["mu"] + params["sigma"] * rng.normal()).max(1e-9),
        "gamma" => rng.gamma(params["alpha"], params["beta"]),
        "gumbel" => params["mu"] - params["beta"] * (-rng.open01().ln()).ln(),
        _ => rng.exponential(1000.0),
    }
}

/// Monte Carlo standby reliability for general distributions. Each unit
/// draws a full lifetime; standby units accumulate virtual age at the
/// dormancy rate while waiting.
pub fn standby_reliability_mc(
    t: f64,
    dist: &str,
    params: &Params,
    n_units: usize,
    dormancy: f64,
    switch_reliability: f64,
    n_sim: usize,
    seed: u64,
) -> f64 {
    let mut rng = Rng64::new(seed);
    let mut success = 0usize;
    for _ in 0..n_sim {
        let lifetimes: Vec<f64> = (0..n_units).map(|_| draw_lifetime(&mut rng, dist, params)).collect();
        let mut clock = 0.0;
        let mut vage = vec![0.0; n_units];
        let mut used = vec![false; n_units];
        let mut active = 0usize;
        used[0] = true;
        loop {
            let remaining = (lifetimes[active] - vage[active]).max(0.0);
            if clock + remaining >= t {
                success += 1;
                break;
            }
            let phase = remaining;
            clock += phase;
            for v in vage.iter_mut() {
                *v += dormancy * phase;
            }
            vage[active] = lifetimes[active];
            let mut nxt: Option<usize> = None;
            for j in 0..n_units {
                if !used[j] && vage[j] < lifetimes[j] {
                    nxt = Some(j);
                    break;
                }
            }
            match nxt {
                None => break,
                Some(j) => {
                    if rng.next_f64() > switch_reliability {
                        break;
                    }
                    active = j;
                    used[j] = true;
                }
            }
        }
    }
    success as f64 / n_sim as f64
}

// ---------------- CTMC ----------------

/// Solve pi Q = 0, sum pi = 1 for a generator matrix Q.
pub fn ctmc_steady_state(q: &[Vec<f64>]) -> Vec<f64> {
    let n = q.len();
    let mut a: Vec<Vec<f64>> = (0..n).map(|i| (0..n).map(|j| q[j][i]).collect()).collect();
    for j in 0..n {
        a[n - 1][j] = 1.0;
    }
    let mut b = vec![0.0; n];
    b[n - 1] = 1.0;
    optim::solve_linear(&a, &b).unwrap_or_else(|| vec![1.0 / n as f64; n])
}

/// Transient state probabilities p(t) = p0 exp(Q t) by uniformisation.
pub fn ctmc_transient(q: &[Vec<f64>], p0: &[f64], times: &[f64]) -> Vec<Vec<f64>> {
    let n = q.len();
    let rate = (0..n).map(|i| -q[i][i]).fold(0.0f64, f64::max).max(1e-12);
    // P = I + Q / rate
    let p_mat: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| (if i == j { 1.0 } else { 0.0 }) + q[i][j] / rate)
                .collect()
        })
        .collect();
    times
        .iter()
        .map(|&tt| {
            let lam_t = rate * tt;
            let mut vec = p0.to_vec();
            let mut result = vec![0.0; n];
            let mut weight = (-lam_t).exp();
            let mut k = 0usize;
            loop {
                for i in 0..n {
                    result[i] += weight * vec[i];
                }
                if weight < 1e-14 && k as f64 > lam_t + 10.0 {
                    break;
                }
                // vec = vec * P
                let mut nv = vec![0.0; n];
                for i in 0..n {
                    for j in 0..n {
                        nv[j] += vec[i] * p_mat[i][j];
                    }
                }
                vec = nv;
                k += 1;
                weight *= lam_t / k as f64;
                if k > 100000 {
                    break;
                }
            }
            result
        })
        .collect()
}

/// CTMC for 1 active + standby units with repair. States 0..n_units:
/// number of failed units (system down when all units failed). Returns
/// (Q, down_states).
pub fn standby_ctmc(
    lam: f64,
    mu: f64,
    n_units: usize,
    dormancy: f64,
    repairmen: usize,
) -> (Vec<Vec<f64>>, Vec<usize>) {
    let n = n_units;
    let mut q = vec![vec![0.0; n + 1]; n + 1];
    for i in 0..=n {
        let working = n - i;
        if working >= 1 {
            let lam_tot = lam + (working - 1).max(0) as f64 * dormancy * lam;
            // note: working-1 standby units; when working>=1 there are working-1 standbys
            q[i][i + 1] = lam_tot;
        }
        if i >= 1 {
            q[i][i - 1] = (i.min(repairmen)) as f64 * mu;
        }
    }
    for i in 0..=n {
        let row_sum: f64 = (0..=n).filter(|&j| j != i).map(|j| q[i][j]).sum();
        q[i][i] = -row_sum;
    }
    (q, vec![n])
}

pub fn single_block_availability(mtbf: f64, mttr: f64) -> f64 {
    mtbf / (mtbf + mttr)
}

// ---------------- Monte Carlo availability ----------------

#[derive(Debug, Clone)]
pub struct AvailabilityResult {
    pub mean_availability: f64,
    pub point_availability: Vec<f64>,
    pub grid: Vec<f64>,
    pub expected_downtime: f64,
    pub mean_failures: f64,
    pub mean_repairs: f64,
}

fn collect_leaves<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
    let ty = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match ty {
        "block" | "standby_group" => out.push(node),
        _ => {
            if let Some(ch) = node.get("children").and_then(|v| v.as_array()) {
                for c in ch {
                    collect_leaves(c, out);
                }
            }
        }
    }
}

fn leaf_index<'a>(leaves: &[&'a Value], node: &'a Value) -> usize {
    leaves
        .iter()
        .position(|l| std::ptr::eq(*l, node))
        .unwrap_or(0)
}

fn leaf_block_timeline(leaf: &Value, horizon: f64, rng: &mut Rng64) -> Vec<(f64, bool)> {
    let mtbf = leaf.get("mtbf").and_then(|v| v.as_f64()).unwrap_or(1000.0);
    let mttr = leaf.get("mttr").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let mut events = vec![(0.0, true)];
    let mut t = 0.0;
    let mut up = true;
    loop {
        let dt = if up {
            rng.exponential(mtbf)
        } else {
            if mttr > 0.0 {
                rng.exponential(mttr)
            } else {
                0.0
            }
        };
        t += dt;
        if t >= horizon {
            break;
        }
        up = !up;
        events.push((t, up));
    }
    events
}

/// Seeds-based deterministic order used for standby-group generator:
/// operational_unit_draws mirrors the Python generator loosely; the
/// exponential clocks make standby repair overlaps approximate in both.
fn leaf_standby_timeline(leaf: &Value, horizon: f64, rng: &mut Rng64) -> Vec<(f64, bool)> {
    let n_units = leaf.get("n_units").and_then(|v| v.as_u64()).unwrap_or(2) as usize;
    let mut mtbf = leaf.get("mtbf").and_then(|v| v.as_f64());
    if mtbf.is_none() {
        if let (Some(d), Some(p)) = (
            leaf.get("dist").and_then(|v| v.as_str()),
            leaf.get("params"),
        ) {
            let params = params_from_value(p);
            if d == "exponential" {
                mtbf = Some(params["theta"]);
            }
        }
    }
    let mtbf = mtbf.unwrap_or(1000.0);
    let mttr = leaf.get("mttr").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let dormancy = leaf.get("dormancy").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let swap = leaf.get("swap_time").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let sw_r = leaf
        .get("switch_reliability")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);
    let lam = 1.0 / mtbf;
    let mut events = vec![(0.0, true)];
    let mut t = 0.0;
    let mut spares: i64 = n_units as i64 - 1;
    let mut repairs: Vec<f64> = Vec::new(); // completion times of failed units
    'outer: loop {
        // clocks for this phase (exponential draws are memoryless, so
        // redrawing after a repair event is exact for this model)
        let life_active = rng.exponential(1.0 / lam);
        let life_standby = if dormancy > 0.0 && spares > 0 {
            rng.exponential(1.0 / (dormancy * lam * spares as f64))
        } else {
            f64::INFINITY
        };
        let next_repair = repairs.iter().cloned().fold(f64::INFINITY, f64::min);
        let phase = life_active.min(life_standby).min(next_repair - t);
        if t + phase >= horizon {
            break;
        }
        if phase == next_repair - t && next_repair.is_finite() {
            // a repaired unit returns to the spare pool
            t = next_repair;
            if let Some(pos) = repairs.iter().position(|x| *x == next_repair) {
                repairs.remove(pos);
            }
            spares += 1;
            continue;
        }
        t += phase;
        if life_standby < life_active {
            // a standby unit failed silently; it goes for repair
            spares -= 1;
            if mttr > 0.0 {
                repairs.push(t + rng.exponential(mttr));
            } else {
                repairs.push(t);
            }
            continue;
        }
        // the active unit failed: outage
        events.push((t, false));
        if mttr > 0.0 {
            repairs.push(t + rng.exponential(mttr));
        } else {
            repairs.push(t);
        }
        // try spares one at a time
        loop {
            if spares <= 0 {
                // wait for the earliest repair to finish
                let tr = repairs.iter().cloned().fold(f64::INFINITY, f64::min);
                if !tr.is_finite() || tr >= horizon {
                    break 'outer;
                }
                t = tr.max(t + swap);
                if let Some(pos) = repairs.iter().position(|x| (*x - tr).abs() < 1e-12) {
                    repairs.remove(pos);
                }
                if t >= horizon {
                    break 'outer;
                }
                events.push((t, true)); // repaired unit becomes active
                break;
            }
            spares -= 1;
            if rng.next_f64() <= sw_r {
                t += swap;
                if t >= horizon {
                    break 'outer;
                }
                events.push((t, true));
                break;
            }
            // switch failure consumes that spare attempt; try another
        }
    }
    events
}

fn system_up_at(
    node: &Value,
    leaves: &[&Value],
    timelines: &[Vec<(f64, bool)>],
    t: f64,
) -> bool {
    let ty = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match ty {
        "block" | "standby_group" => {
            let idx = leaf_index(leaves, node);
            let tl = &timelines[idx];
            let mut up = tl[0].1;
            for (et, u) in tl {
                if *et <= t {
                    up = *u;
                } else {
                    break;
                }
            }
            up
        }
        "series" => node
            .get("children")
            .and_then(|v| v.as_array())
            .map(|ch| ch.iter().all(|c| system_up_at(c, leaves, timelines, t)))
            .unwrap_or(false),
        "parallel" => node
            .get("children")
            .and_then(|v| v.as_array())
            .map(|ch| ch.iter().any(|c| system_up_at(c, leaves, timelines, t)))
            .unwrap_or(false),
        "kofn" => {
            let k = node.get("k").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
            let count = node
                .get("children")
                .and_then(|v| v.as_array())
                .map(|ch| {
                    ch.iter()
                        .filter(|c| system_up_at(c, leaves, timelines, t))
                        .count()
                })
                .unwrap_or(0);
            count >= k
        }
        _ => false,
    }
}

/// Discrete-event Monte Carlo availability simulation over the RBD.
pub fn simulate_availability(
    spec: &Value,
    horizon: f64,
    n_sim: usize,
    seed: u64,
    n_grid: usize,
) -> AvailabilityResult {
    let grid: Vec<f64> = (0..n_grid)
        .map(|i| horizon * i as f64 / (n_grid - 1).max(1) as f64)
        .collect();
    let mut leaves: Vec<&Value> = Vec::new();
    collect_leaves(spec, &mut leaves);
    let mut master = Rng64::new(seed);
    let mut up_counts = vec![0usize; n_grid];
    let mut downtime_total = 0.0;
    let mut failures_total = 0usize;
    let mut repairs_total = 0usize;
    for _ in 0..n_sim {
        let mut timelines = Vec::new();
        for leaf in &leaves {
            let mut rng = master.fork();
            let ty = leaf.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if ty == "standby_group" {
                timelines.push(leaf_standby_timeline(leaf, horizon, &mut rng));
            } else {
                timelines.push(leaf_block_timeline(leaf, horizon, &mut rng));
            }
        }
        let mut change_times: Vec<f64> = vec![0.0];
        for tl in &timelines {
            for (et, _) in tl {
                change_times.push(*et);
            }
        }
        change_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        change_times.dedup();
        let mut run_up = |t: f64| {
            for (gi, g) in grid.iter().enumerate() {
                if (*g - t).abs() < 1e-12 {
                    up_counts[gi] += 1;
                }
            }
        };
        let _ = &mut run_up;
        for (gi, g) in grid.iter().enumerate() {
            if system_up_at(spec, &leaves, &timelines, *g) {
                up_counts[gi] += 1;
            }
        }
        let mut prev_t = 0.0;
        let mut prev_up = system_up_at(spec, &leaves, &timelines, 0.0);
        let mut downtime = 0.0;
        let mut fails = 0usize;
        let mut reps = 0usize;
        for ct in change_times.iter().skip(1) {
            if *ct > horizon {
                break;
            }
            if !prev_up {
                downtime += ct - prev_t;
            }
            let now_up = system_up_at(spec, &leaves, &timelines, *ct + 1e-9);
            if prev_up && !now_up {
                fails += 1;
            }
            if !prev_up && now_up {
                reps += 1;
            }
            prev_t = *ct;
            prev_up = now_up;
        }
        if !prev_up {
            downtime += horizon - prev_t;
        }
        downtime_total += downtime;
        failures_total += fails;
        repairs_total += reps;
    }
    let point: Vec<f64> = up_counts
        .iter()
        .map(|c| *c as f64 / n_sim as f64)
        .collect();
    let mean_a = point.iter().sum::<f64>() / point.len().max(1) as f64;
    AvailabilityResult {
        mean_availability: mean_a,
        point_availability: point,
        grid,
        expected_downtime: downtime_total / n_sim as f64,
        mean_failures: failures_total as f64 / n_sim as f64,
        mean_repairs: repairs_total as f64 / n_sim as f64,
    }
}

/// Propagate per-block reliability draws through the structure.
pub fn posterior_system_reliability(
    spec: &Value,
    block_draws: &BTreeMap<String, Vec<f64>>,
    t: f64,
) -> Result<Vec<f64>, String> {
    let _ = t;
    fn eval_draw(
        node: &Value,
        draws: &BTreeMap<String, Vec<f64>>,
        i: usize,
    ) -> Result<f64, String> {
        let ty = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match ty {
            "block" => {
                if let Some(arr) = node.get("draws") {
                    let a = arr.as_array().ok_or("draws must be an array")?;
                    return Ok(a[i].as_f64().ok_or("draw not a number")?);
                }
                let name = node
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or("block needs a name for posterior draws")?;
                let d = draws.get(name).ok_or(format!("no draws for block {}", name))?;
                Ok(d[i])
            }
            "series" | "parallel" | "kofn" => {
                let ch = node
                    .get("children")
                    .and_then(|v| v.as_array())
                    .ok_or("node needs children")?;
                let mut rs = Vec::new();
                for c in ch {
                    rs.push(eval_draw(c, draws, i)?);
                }
                match ty {
                    "series" => Ok(series_reliability(&rs)),
                    "parallel" => Ok(parallel_reliability(&rs)),
                    _ => {
                        let k = node.get("k").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
                        Ok(kofn_reliability(&rs, k))
                    }
                }
            }
            _ => Err(format!("posterior propagation unsupported for {}", ty)),
        }
    }
    let n = block_draws
        .values()
        .next()
        .map(|v| v.len())
        .unwrap_or(0);
    let mut out = Vec::new();
    for i in 0..n {
        out.push(eval_draw(spec, block_draws, i)?);
    }
    Ok(out)
}
