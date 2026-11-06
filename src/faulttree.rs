//! Fault tree evaluation.
//!
//! A fault tree combines basic events with AND, OR, and voting gates.
//! This module computes the top event probability and the minimal
//! cut sets of a tree. Trees are assumed to be small, so cut sets are
//! built by direct combination rather than a reduced representation.

use std::collections::{BTreeMap, BTreeSet};

/// One node of a fault tree.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FtNode {
    /// A basic event with a fixed probability.
    Basic { name: String, prob: f64 },
    /// The gate fires when every child fires.
    And { name: String, children: Vec<FtNode> },
    /// The gate fires when at least one child fires.
    Or { name: String, children: Vec<FtNode> },
    /// The gate fires when at least `k` of its children fire.
    Voting {
        name: String,
        k: usize,
        children: Vec<FtNode>,
    },
}

/// Result of evaluating a fault tree with shared basic event identity.
///
/// Basic events are identified by name: two occurrences of the same
/// name are the same event, not two independent events.
#[derive(Debug, Clone)]
pub struct FtEval {
    pub top_probability: f64,
    pub cut_sets: Vec<std::collections::BTreeSet<String>>,
}

/// Collect the probability of every distinct basic event in the tree.
///
/// Returns the name to probability map and the total number of basic
/// event occurrences. An `Err` is returned when a probability lies
/// outside [0, 1] or is NaN, or when one name carries two different
/// probabilities.
fn collect_basic_events(
    node: &FtNode,
    probs: &mut BTreeMap<String, f64>,
    occurrences: &mut usize,
) -> Result<(), String> {
    match node {
        FtNode::Basic { name, prob } => {
            *occurrences += 1;
            if prob.is_nan() || *prob < 0.0 || *prob > 1.0 {
                return Err(format!(
                    "basic event '{name}' has probability {prob}, which is outside [0, 1]"
                ));
            }
            if let Some(existing) = probs.get(name) {
                if *existing != *prob {
                    return Err(format!(
                        "basic event '{name}' has conflicting probabilities {existing} and {prob}"
                    ));
                }
            } else {
                probs.insert(name.clone(), *prob);
            }
            Ok(())
        }
        FtNode::And { children, .. }
        | FtNode::Or { children, .. }
        | FtNode::Voting { children, .. } => {
            for child in children {
                collect_basic_events(child, probs, occurrences)?;
            }
            Ok(())
        }
    }
}

/// Boolean value of the tree under one assignment of basic events.
///
/// `index` maps each basic event name to its position in
/// `assignment` and `names`.
fn eval_assignment(node: &FtNode, index: &BTreeMap<String, usize>, assignment: &[bool]) -> bool {
    match node {
        FtNode::Basic { name, .. } => assignment[index[name]],
        FtNode::And { children, .. } => children
            .iter()
            .all(|child| eval_assignment(child, index, assignment)),
        FtNode::Or { children, .. } => children
            .iter()
            .any(|child| eval_assignment(child, index, assignment)),
        FtNode::Voting { k, children, .. } => {
            children
                .iter()
                .filter(|child| eval_assignment(child, index, assignment))
                .count()
                >= *k
        }
    }
}

/// Evaluate a fault tree, respecting shared basic event identity.
///
/// Basic events are identified by name, so repeated occurrences of
/// one name always take the same value. The whole tree is validated
/// first: a basic event probability outside [0, 1] or NaN is an
/// error naming the event and its value, and one name with two
/// different probabilities is an error naming the event.
///
/// The top probability is exact. When the tree has 20 or fewer
/// distinct basic events, every assignment of the distinct events is
/// enumerated, the boolean tree is evaluated per assignment, and the
/// assignments are weighted by the product of the per event
/// probabilities. The limit of 20 distinct events keeps that
/// enumeration at about one million assignments. When the distinct
/// count exceeds 20 and no event name repeats, the recursive
/// [`top_probability`] computation is used instead, which is exact
/// for independent events. When the distinct count exceeds 20 and
/// repeated event names exist, an error is returned rather than a
/// silently wrong probability.
///
/// Cut sets come from [`minimal_cut_sets`], which is already based
/// on event names and is consistent with this identity rule.
pub fn evaluate(node: &FtNode) -> Result<FtEval, String> {
    const EXACT_ENUMERATION_LIMIT: usize = 20;

    let mut probs: BTreeMap<String, f64> = BTreeMap::new();
    let mut occurrences = 0usize;
    collect_basic_events(node, &mut probs, &mut occurrences)?;

    let names: Vec<String> = probs.keys().cloned().collect();
    let distinct = names.len();

    let top_probability = if distinct <= EXACT_ENUMERATION_LIMIT {
        let index: BTreeMap<String, usize> = names
            .iter()
            .enumerate()
            .map(|(position, name)| (name.clone(), position))
            .collect();
        let ordered_probs: Vec<f64> = names.iter().map(|name| probs[name]).collect();
        let mut total = 0.0f64;
        for mask in 0..(1usize << distinct) {
            let mut assignment = vec![false; distinct];
            let mut weight = 1.0f64;
            for (position, prob) in ordered_probs.iter().enumerate() {
                if (mask >> position) & 1 == 1 {
                    assignment[position] = true;
                    weight *= prob;
                } else {
                    weight *= 1.0 - prob;
                }
            }
            if eval_assignment(node, &index, &assignment) {
                total += weight;
            }
        }
        total
    } else if occurrences == distinct {
        top_probability(node)
    } else {
        return Err(format!(
            "fault tree has {distinct} distinct basic events with repeated event \
             names, which exceeds the limit of {EXACT_ENUMERATION_LIMIT} distinct \
             events for exact repeated-event evaluation"
        ));
    };

    Ok(FtEval {
        top_probability,
        cut_sets: minimal_cut_sets(node),
    })
}

/// Probability that the tree rooted at `node` fires.
///
/// This computation treats every occurrence of a basic event as an
/// independent event. Use [`evaluate`] when a tree can repeat a basic
/// event name and the occurrences must share one identity.
///
/// Basic events return their own probability. AND gates multiply
/// child probabilities, OR gates return one minus the product of the
/// child non-occurrence probabilities, and voting gates use a
/// Poisson-binomial dynamic program over the child probabilities.
pub fn top_probability(node: &FtNode) -> f64 {
    match node {
        FtNode::Basic { prob, .. } => *prob,
        FtNode::And { children, .. } => children.iter().map(top_probability).product(),
        FtNode::Or { children, .. } => {
            let none_fire: f64 = children
                .iter()
                .map(|child| 1.0 - top_probability(child))
                .product();
            1.0 - none_fire
        }
        FtNode::Voting { k, children, .. } => {
            let probs: Vec<f64> = children.iter().map(top_probability).collect();
            let dp = poisson_binomial_distribution(&probs);
            dp.iter().skip(*k).sum()
        }
    }
}

/// Full Poisson-binomial distribution for independent events.
///
/// Entry `j` of the result is the probability that exactly `j` of the
/// events occur.
fn poisson_binomial_distribution(probs: &[f64]) -> Vec<f64> {
    let mut dp = vec![0.0f64; probs.len() + 1];
    dp[0] = 1.0;
    for (count, p) in probs.iter().enumerate() {
        // Update in descending order so each event is used once.
        for j in (0..=count).rev() {
            let occurs = dp[j] * p;
            dp[j] *= 1.0 - p;
            dp[j + 1] += occurs;
        }
    }
    dp
}

/// Cartesian union of cut sets.
///
/// Combines one cut set from each child list and unions them, which
/// is the cut set construction for an AND combination of subtrees.
fn cartesian_union(child_cut_sets: &[Vec<BTreeSet<String>>]) -> Vec<BTreeSet<String>> {
    let mut combined: Vec<BTreeSet<String>> = vec![BTreeSet::new()];
    for child_sets in child_cut_sets {
        let mut next: Vec<BTreeSet<String>> = Vec::new();
        for base in &combined {
            for extra in child_sets {
                let mut merged = base.clone();
                merged.extend(extra.iter().cloned());
                next.push(merged);
            }
        }
        combined = next;
    }
    combined
}

/// All index combinations of size `k` drawn from `0..n`, in
/// lexicographic order.
fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut result: Vec<Vec<usize>> = Vec::new();
    if k > n {
        return result;
    }
    let mut current: Vec<usize> = Vec::with_capacity(k);
    collect_combinations(n, k, 0, &mut current, &mut result);
    result
}

fn collect_combinations(
    n: usize,
    k: usize,
    start: usize,
    current: &mut Vec<usize>,
    result: &mut Vec<Vec<usize>>,
) {
    if current.len() == k {
        result.push(current.clone());
        return;
    }
    for index in start..n {
        current.push(index);
        collect_combinations(n, k, index + 1, current, result);
        current.pop();
    }
}

/// Remove duplicate cut sets and any set that is a strict superset
/// of another set in the list.
fn keep_minimal(sets: Vec<BTreeSet<String>>) -> Vec<BTreeSet<String>> {
    let mut unique: Vec<BTreeSet<String>> = Vec::new();
    for set in sets {
        if !unique.contains(&set) {
            unique.push(set);
        }
    }
    let mut minimal: Vec<BTreeSet<String>> = Vec::new();
    for candidate in &unique {
        let has_strict_subset = unique
            .iter()
            .any(|other| other != candidate && other.is_subset(candidate));
        if !has_strict_subset {
            minimal.push(candidate.clone());
        }
    }
    minimal
}

/// Minimal cut sets of the tree rooted at `node`.
///
/// A cut set is a set of basic event names whose joint occurrence
/// fires the top event. Only minimal sets are returned: a set that
/// strictly contains another returned set is removed, and duplicates
/// are dropped.
pub fn minimal_cut_sets(node: &FtNode) -> Vec<BTreeSet<String>> {
    let raw: Vec<BTreeSet<String>> = match node {
        FtNode::Basic { name, .. } => {
            let mut set = BTreeSet::new();
            set.insert(name.clone());
            vec![set]
        }
        FtNode::Or { children, .. } => {
            let mut sets: Vec<BTreeSet<String>> = Vec::new();
            for child in children {
                sets.extend(minimal_cut_sets(child));
            }
            sets
        }
        FtNode::And { children, .. } => {
            let child_cut_sets: Vec<Vec<BTreeSet<String>>> =
                children.iter().map(minimal_cut_sets).collect();
            cartesian_union(&child_cut_sets)
        }
        FtNode::Voting { k, children, .. } => {
            let mut sets: Vec<BTreeSet<String>> = Vec::new();
            for combo in combinations(children.len(), *k) {
                let child_cut_sets: Vec<Vec<BTreeSet<String>>> = combo
                    .iter()
                    .map(|index| minimal_cut_sets(&children[*index]))
                    .collect();
                sets.extend(cartesian_union(&child_cut_sets));
            }
            sets
        }
    };
    keep_minimal(raw)
}
