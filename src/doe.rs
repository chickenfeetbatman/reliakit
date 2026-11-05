//! Two-level full factorial design of experiments (DOE).
//!
//! Designs use factor levels coded as -1.0 and 1.0 in standard
//! (Yates) order, with the first factor varying fastest. Analysis
//! reports the effect and sum of squares for every main effect and
//! interaction.

/// Build a full 2^k two-level factorial design in Yates order.
///
/// The result has 2^k runs and `k` columns. Entry values are -1.0
/// and 1.0, and the first factor varies fastest across runs.
pub fn factorial_design(k: usize) -> Vec<Vec<f64>> {
    let runs = 1usize << k;
    let mut design: Vec<Vec<f64>> = Vec::with_capacity(runs);
    for run in 0..runs {
        let mut row: Vec<f64> = Vec::with_capacity(k);
        for factor in 0..k {
            if (run >> factor) & 1 == 1 {
                row.push(1.0);
            } else {
                row.push(-1.0);
            }
        }
        design.push(row);
    }
    design
}

/// One estimated effect from a factorial analysis.
#[derive(Debug, Clone)]
pub struct DoeEffect {
    /// Term name such as "A", "B", or "AB". Factor index 0 is "A".
    pub name: String,
    /// Factor indices that make up the term, in increasing order.
    pub terms: Vec<usize>,
    /// Effect estimate: twice the contrast divided by the run count.
    pub effect: f64,
    /// Sum of squares: the squared contrast divided by the run count.
    pub sum_squares: f64,
}

/// Letter name for a factor index. Index 0 is "A", 1 is "B", and so
/// on. Indices past 25 fall back to a numbered label.
fn factor_letter(index: usize) -> String {
    if index < 26 {
        let ch = (b'A' + index as u8) as char;
        ch.to_string()
    } else {
        format!("F{}", index + 1)
    }
}

/// All subsets of `0..k` of the given size, in lexicographic order.
fn subsets_of_size(k: usize, size: usize) -> Vec<Vec<usize>> {
    let mut result: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = Vec::with_capacity(size);
    collect_subsets(k, size, 0, &mut current, &mut result);
    result
}

fn collect_subsets(
    k: usize,
    size: usize,
    start: usize,
    current: &mut Vec<usize>,
    result: &mut Vec<Vec<usize>>,
) {
    if current.len() == size {
        result.push(current.clone());
        return;
    }
    for index in start..k {
        current.push(index);
        collect_subsets(k, size, index + 1, current, result);
        current.pop();
    }
}

/// Analyze a full 2^k factorial experiment.
///
/// `y` holds one response per run in Yates order, so its length must
/// be exactly 2^k. Effects are returned for every non-empty subset of
/// factors, ordered first by subset size and then lexicographically.
/// For each subset, the contrast column is the product of the design
/// columns in the subset, the contrast is the dot product of that
/// column with `y`, the effect is 2 * contrast / N, and the sum of
/// squares is contrast^2 / N, with N equal to 2^k.
pub fn analyze_factorial(k: usize, y: &[f64]) -> Result<Vec<DoeEffect>, String> {
    let n = 1usize << k;
    if y.len() != n {
        return Err(format!(
            "expected {} responses for a 2^{} design, found {}",
            n,
            k,
            y.len()
        ));
    }

    let design = factorial_design(k);
    let n_f = n as f64;
    let mut effects: Vec<DoeEffect> = Vec::new();

    for size in 1..=k {
        for subset in subsets_of_size(k, size) {
            let mut contrast = 0.0f64;
            for (run, response) in y.iter().enumerate() {
                let mut column = 1.0f64;
                for factor in &subset {
                    column *= design[run][*factor];
                }
                contrast += column * response;
            }
            let name: String = subset.iter().map(|index| factor_letter(*index)).collect();
            effects.push(DoeEffect {
                name,
                terms: subset,
                effect: 2.0 * contrast / n_f,
                sum_squares: contrast * contrast / n_f,
            });
        }
    }

    Ok(effects)
}
