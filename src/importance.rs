//! Reliability importance measures for an RBD spec.
//!
//! Leaves are nodes of type "block" and "standby_group". For each
//! leaf, the system reliability is recomputed with that leaf forced
//! perfect (R = 1) and forced failed (R = 0). From those two values:
//!
//! - Birnbaum importance: R1 - R0. It is the sensitivity of system
//!   reliability to the leaf reliability, evaluated exactly by the
//!   two endpoint evaluations because system reliability is linear
//!   in any one leaf reliability for a coherent structure.
//! - Criticality importance: Birnbaum * Ri / R_sys, where Ri is the
//!   leaf reliability itself. Defined as 0 when R_sys is 0, where
//!   the ratio has no meaning.
//! - Fussell-Vesely importance: (R1 - R_sys) / (1 - R_sys), the
//!   fraction of system unreliability removed if the leaf were made
//!   perfect. Defined as 0 when R_sys is 1, where system
//!   unreliability is already 0.

use crate::system;
use serde_json::Value;

/// Importance measures for one leaf of an RBD.
#[derive(Debug, Clone)]
pub struct BlockImportance {
    pub name: String,
    pub birnbaum: f64,
    pub fussell_vesely: f64,
    pub criticality: f64,
}

/// Collect leaf paths. A path is the list of child indices from the
/// root to the leaf. Leaves are recorded in walk order.
fn collect_leaves(node: &Value, path: &mut Vec<usize>, out: &mut Vec<(Vec<usize>, String, Value)>) {
    let ty = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if ty == "block" || ty == "standby_group" {
        let name = node
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("block {}", out.len()));
        out.push((path.clone(), name, node.clone()));
        return;
    }
    if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
        for (i, child) in children.iter().enumerate() {
            path.push(i);
            collect_leaves(child, path, out);
            path.pop();
        }
    }
}

/// Rebuild the tree with the node at `path` replaced by `replacement`.
fn replace_at(node: &Value, path: &[usize], replacement: &Value) -> Value {
    if path.is_empty() {
        return replacement.clone();
    }
    let mut rebuilt = node.clone();
    if let Some(children) = rebuilt.get_mut("children").and_then(|v| v.as_array_mut()) {
        let idx = path[0];
        if idx < children.len() {
            children[idx] = replace_at(&children[idx].clone(), &path[1..], replacement);
        }
    }
    rebuilt
}

/// Importance measures for every leaf of an RBD `spec` at time `t`.
///
/// All reliabilities are evaluated with `system::evaluate_rbd`.
/// Replacing a leaf works at any nesting depth: the JSON tree is
/// rebuilt along the leaf path with a plain block carrying a fixed
/// R value. A standby_group leaf is replaced as a whole group.
pub fn importance_measures(spec: &Value, t: f64) -> Result<Vec<BlockImportance>, String> {
    let r_sys = system::evaluate_rbd(spec, Some(t))?;
    let mut leaves: Vec<(Vec<usize>, String, Value)> = Vec::new();
    let mut path: Vec<usize> = Vec::new();
    collect_leaves(spec, &mut path, &mut leaves);
    if leaves.is_empty() {
        return Err("spec has no block or standby_group leaves".into());
    }
    let perfect = serde_json::json!({"type": "block", "R": 1.0});
    let failed = serde_json::json!({"type": "block", "R": 0.0});
    let mut out: Vec<BlockImportance> = Vec::with_capacity(leaves.len());
    for (leaf_path, name, leaf_node) in &leaves {
        let ri = system::evaluate_rbd(leaf_node, Some(t))?;
        let spec1 = replace_at(spec, leaf_path, &perfect);
        let spec0 = replace_at(spec, leaf_path, &failed);
        let r1 = system::evaluate_rbd(&spec1, Some(t))?;
        let r0 = system::evaluate_rbd(&spec0, Some(t))?;
        let birnbaum = r1 - r0;
        let criticality = if r_sys == 0.0 {
            0.0
        } else {
            birnbaum * ri / r_sys
        };
        let fussell_vesely = if r_sys == 1.0 {
            0.0
        } else {
            (r1 - r_sys) / (1.0 - r_sys)
        };
        out.push(BlockImportance {
            name: name.clone(),
            birnbaum,
            fussell_vesely,
            criticality,
        });
    }
    Ok(out)
}
