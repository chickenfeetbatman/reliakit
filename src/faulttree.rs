//! Fault tree evaluation.
//!
//! A fault tree combines basic events with AND, OR, and voting gates.
//! This module computes the top event probability and the minimal
//! cut sets of a tree. Trees are assumed to be small, so cut sets are
//! built by direct combination rather than a reduced representation.

use std::collections::BTreeSet;

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
    Voting { name: String, k: usize, children: Vec<FtNode> },
}

/// Probability that the tree rooted at `node` fires.
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
            let none_fire: f64 = children.iter().map(|child| 1.0 - top_probability(child)).product();
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
