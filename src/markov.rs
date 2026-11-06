//! Markov chain analysis for diagrams with constant transition rates.
//!
//! A model is a continuous-time Markov chain described by state names
//! and a list of transitions, each with a constant rate. The generator
//! matrix Q has the transition rates off the diagonal and, on the
//! diagonal, the negative row sum, so each row sums to zero.
//!
//! Steady-state and transient probabilities reuse the CTMC solvers in
//! the system module. Mean time to absorption and the expected time
//! spent in each transient state come from the fundamental matrix of
//! the transient subchain: with Q_tt the subgenerator over transient
//! states, the mean absorption times tau satisfy -Q_tt * tau = 1, and
//! the per-state expected times are the matching row of
//! N = (-Q_tt)^-1.

use crate::optim;
use crate::system;

/// A continuous-time Markov chain with named states.
///
/// `q` is the generator matrix: `q[i][j]` for i != j is the rate of
/// moving from state i to state j, and `q[i][i]` is the negative sum
/// of the other entries in row i.
#[derive(Debug, Clone)]
pub struct MarkovModel {
    pub names: Vec<String>,
    pub q: Vec<Vec<f64>>,
}

impl MarkovModel {
    /// Build a model from state names and transitions.
    ///
    /// Each transition is `(from, to, rate)` with state indices into
    /// `names` and a non-negative rate. Repeated transitions between
    /// the same pair of states are added together. The diagonal of
    /// the generator is set to the negative row sum.
    pub fn new(names: Vec<String>, transitions: &[(usize, usize, f64)]) -> Result<Self, String> {
        let n = names.len();
        if n == 0 {
            return Err("markov model needs at least one state".to_string());
        }
        let mut q = vec![vec![0.0; n]; n];
        for &(from, to, rate) in transitions {
            if from >= n || to >= n {
                return Err(format!(
                    "transition index out of range: {} -> {} for {} states",
                    from, to, n
                ));
            }
            if from == to {
                return Err(format!(
                    "self transition is not allowed for state {}",
                    from
                ));
            }
            if !rate.is_finite() || rate < 0.0 {
                return Err(format!(
                    "transition rate must be finite and non-negative: {}",
                    rate
                ));
            }
            q[from][to] += rate;
        }
        for i in 0..n {
            let row_sum: f64 = (0..n).filter(|&j| j != i).map(|j| q[i][j]).sum();
            q[i][i] = -row_sum;
        }
        Ok(MarkovModel { names, q })
    }

    /// Number of states in the model.
    pub fn n_states(&self) -> usize {
        self.names.len()
    }

    /// Steady-state state probabilities pi with pi Q = 0 and sum 1.
    pub fn steady_state(&self) -> Result<Vec<f64>, String> {
        if self.q.len() != self.names.len() {
            return Err("generator size does not match state names".to_string());
        }
        Ok(system::ctmc_steady_state(&self.q))
    }

    /// Transient state probabilities at each time in `times`.
    ///
    /// `p0` is the initial probability vector. The result has one row
    /// per requested time, each row a probability vector over states.
    pub fn transient(&self, p0: &[f64], times: &[f64]) -> Result<Vec<Vec<f64>>, String> {
        if p0.len() != self.names.len() {
            return Err(format!(
                "initial vector has {} entries for {} states",
                p0.len(),
                self.names.len()
            ));
        }
        Ok(system::ctmc_transient(&self.q, p0, times))
    }

    /// Indices of the transient states, in state order.
    fn transient_indices(&self, absorbing: &[bool]) -> Result<Vec<usize>, String> {
        if absorbing.len() != self.names.len() {
            return Err(format!(
                "absorbing flags have {} entries for {} states",
                absorbing.len(),
                self.names.len()
            ));
        }
        Ok((0..self.names.len())
            .filter(|&i| !absorbing[i])
            .collect())
    }

    /// Build -Q_tt over the transient states listed in `transient`.
    fn neg_transient_subgenerator(&self, transient: &[usize]) -> Vec<Vec<f64>> {
        transient
            .iter()
            .map(|&i| transient.iter().map(|&j| -self.q[i][j]).collect())
            .collect()
    }

    /// Mean time to absorption starting from state `start`.
    ///
    /// `absorbing` marks the absorbing states. The mean absorption
    /// times over the transient states solve -Q_tt * tau = 1, where
    /// Q_tt is the subgenerator over transient states. Starting in an
    /// absorbing state gives 0.
    pub fn mean_time_to_absorption(
        &self,
        absorbing: &[bool],
        start: usize,
    ) -> Result<f64, String> {
        if start >= self.names.len() {
            return Err(format!("start state {} out of range", start));
        }
        let transient = self.transient_indices(absorbing)?;
        if absorbing[start] {
            return Ok(0.0);
        }
        if transient.is_empty() {
            return Err("no transient states".to_string());
        }
        let a = self.neg_transient_subgenerator(&transient);
        let b = vec![1.0; transient.len()];
        let tau = optim::solve_linear(&a, &b)
            .ok_or_else(|| "transient subgenerator is singular".to_string())?;
        let pos = transient
            .iter()
            .position(|&i| i == start)
            .ok_or_else(|| "start state is not transient".to_string())?;
        Ok(tau[pos])
    }

    /// Expected total time in each state before absorption.
    ///
    /// The result has one entry per state, in state order. Absorbing
    /// states get 0. For a transient state j, the entry is the
    /// expected total time spent in j before absorption when starting
    /// from `start`, which is entry (start, j) of the fundamental
    /// matrix N = (-Q_tt)^-1. The entries sum to the mean time to
    /// absorption from `start`.
    pub fn expected_time_in_state(
        &self,
        absorbing: &[bool],
        start: usize,
    ) -> Result<Vec<f64>, String> {
        if start >= self.names.len() {
            return Err(format!("start state {} out of range", start));
        }
        let transient = self.transient_indices(absorbing)?;
        let mut out = vec![0.0; self.names.len()];
        if absorbing[start] {
            return Ok(out);
        }
        if transient.is_empty() {
            return Err("no transient states".to_string());
        }
        // The per-state times are the row of N for the start state.
        // That row r satisfies r^T = e_start^T * A^-1 with
        // A = -Q_tt, so r solves A^T * r = e_start.
        let a = self.neg_transient_subgenerator(&transient);
        let m = transient.len();
        let a_t: Vec<Vec<f64>> = (0..m)
            .map(|i| (0..m).map(|j| a[j][i]).collect())
            .collect();
        let pos = transient
            .iter()
            .position(|&i| i == start)
            .ok_or_else(|| "start state is not transient".to_string())?;
        let mut e = vec![0.0; m];
        e[pos] = 1.0;
        let r = optim::solve_linear(&a_t, &e)
            .ok_or_else(|| "transient subgenerator is singular".to_string())?;
        for (k, &state) in transient.iter().enumerate() {
            out[state] = r[k];
        }
        Ok(out)
    }
}
