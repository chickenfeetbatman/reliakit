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

// ---------------------------------------------------------------------
// Fractional factorial designs, defining relations and aliases.
//
// A word is a set of factor letters, stored sorted, for example
// "ABD". Word multiplication is symmetric difference of the letter
// sets: a letter appearing in both words cancels, which is the
// usual modulo-2 algebra of two-level factorial effects.
// ---------------------------------------------------------------------

use crate::optim;
use std::collections::BTreeSet;

fn letter_set(s: &str) -> BTreeSet<char> {
    s.chars()
        .map(|c| c.to_ascii_uppercase())
        .filter(|c| c.is_ascii_alphabetic())
        .collect()
}

fn set_to_word(set: &BTreeSet<char>) -> String {
    if set.is_empty() {
        "I".to_string()
    } else {
        set.iter().collect()
    }
}

fn word_xor(a: &BTreeSet<char>, b: &BTreeSet<char>) -> BTreeSet<char> {
    a.symmetric_difference(b).cloned().collect()
}

/// Defining word for one generator. A generator `D = AB` gives the
/// word ABD: moving AB to the left side, D * AB = I in effect
/// algebra, so I = ABD is in the defining relation.
fn generator_word(gen_letter: char, rhs: &str) -> BTreeSet<char> {
    let mut w = letter_set(rhs);
    w.insert(gen_letter.to_ascii_uppercase());
    w
}

/// All defining words generated by the generators, including the
/// products of two or more generator words, as sorted letter
/// strings sorted by word length then lexicographically. The
/// identity word I itself is not included.
pub fn defining_relation(generators: &[(char, &str)]) -> Vec<String> {
    let gen_words: Vec<BTreeSet<char>> = generators
        .iter()
        .map(|(letter, rhs)| generator_word(*letter, rhs))
        .collect();
    let p = gen_words.len();
    let mut words: Vec<BTreeSet<char>> = Vec::new();
    // Every non-empty subset of generator words, multiplied together.
    for mask in 1..(1usize << p) {
        let mut w: BTreeSet<char> = BTreeSet::new();
        for (i, gw) in gen_words.iter().enumerate() {
            if (mask >> i) & 1 == 1 {
                w = word_xor(&w, gw);
            }
        }
        if !w.is_empty() && !words.contains(&w) {
            words.push(w);
        }
    }
    let mut strings: Vec<String> = words.iter().map(set_to_word).collect();
    strings.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
    strings
}

/// Resolution of a fractional factorial defined by `generators`:
/// the length of the shortest defining word. Returns None when
/// there are no generators, where the design is a full factorial
/// and the resolution is not a finite number.
pub fn resolution(generators: &[(char, &str)]) -> Option<usize> {
    defining_relation(generators)
        .iter()
        .map(|w| w.chars().count())
        .min()
}

/// Alias set of one factor: every effect the factor is aliased
/// with, computed by multiplying the factor by each defining word.
/// The factor itself is excluded. Words are sorted by length then
/// lexicographically. For example with D = AB and E = AC, factor A
/// is aliased with BD, CE and ABCDE.
pub fn alias_set(generators: &[(char, &str)], factor: char) -> Vec<String> {
    let factor_set: BTreeSet<char> = [factor.to_ascii_uppercase()].into_iter().collect();
    let gen_words: Vec<BTreeSet<char>> = generators
        .iter()
        .map(|(letter, rhs)| generator_word(*letter, rhs))
        .collect();
    let p = gen_words.len();
    let mut aliases: Vec<BTreeSet<char>> = Vec::new();
    // Include the identity defining word (empty mask): it gives the
    // factor itself, which is then excluded below. Every other
    // subset product gives one alias.
    for mask in 0..(1usize << p) {
        let mut w: BTreeSet<char> = BTreeSet::new();
        for (i, gw) in gen_words.iter().enumerate() {
            if (mask >> i) & 1 == 1 {
                w = word_xor(&w, gw);
            }
        }
        let alias = word_xor(&factor_set, &w);
        if alias != factor_set && !aliases.contains(&alias) {
            aliases.push(alias);
        }
    }
    let mut strings: Vec<String> = aliases.iter().map(set_to_word).collect();
    strings.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
    strings
}

/// Fractional factorial design with `k_total` factors.
///
/// Each generator assigns one factor letter the product of other
/// letters, for example ('D', "AB") means column D equals column A
/// times column B in every run. The factors not named by a
/// generator are the basic factors; they run through a full
/// factorial in Yates order. Generated columns are evaluated from
/// their defining products, and a generator may use an earlier
/// generated factor. Columns are returned in letter order A, B, C
/// and so on, with entries -1.0 and 1.0, in the same run-matrix
/// style as `factorial_design`. The run count is
/// 2^(k_total - number of generators). An invalid request (a
/// generator count at or above k_total, or a product that refers
/// to an unknown or circular letter) gives an empty matrix.
pub fn fractional_factorial(
    k_total: usize,
    generators: &[(char, &str)],
) -> Vec<Vec<f64>> {
    if k_total == 0 || generators.len() >= k_total {
        return Vec::new();
    }
    let letters: Vec<char> = (0..k_total)
        .map(|i| (b'A' + i as u8) as char)
        .collect();
    let generated: Vec<char> = generators
        .iter()
        .map(|(c, _)| c.to_ascii_uppercase())
        .collect();
    // Every generated letter must be a column letter, used once.
    for (idx, g) in generated.iter().enumerate() {
        if !letters.contains(g) || generated[..idx].contains(g) {
            return Vec::new();
        }
    }
    let basic: Vec<char> = letters
        .iter()
        .cloned()
        .filter(|c| !generated.contains(c))
        .collect();
    let basic_design = factorial_design(basic.len());

    // Value of one letter in one run, resolving generated letters
    // recursively. `visiting` guards against circular products.
    fn letter_value(
        letter: char,
        run: &[f64],
        basic: &[char],
        generators: &[(char, &str)],
        visiting: &mut Vec<char>,
    ) -> Option<f64> {
        if let Some(pos) = basic.iter().position(|c| *c == letter) {
            return Some(run[pos]);
        }
        if visiting.contains(&letter) {
            return None;
        }
        let (_, rhs) = generators
            .iter()
            .find(|(c, _)| c.to_ascii_uppercase() == letter)?;
        visiting.push(letter);
        let mut value = 1.0;
        for rhs_letter in rhs.chars().map(|c| c.to_ascii_uppercase()) {
            value *= letter_value(rhs_letter, run, basic, generators, visiting)?;
        }
        visiting.pop();
        Some(value)
    }

    let mut design: Vec<Vec<f64>> = Vec::with_capacity(basic_design.len());
    for run in &basic_design {
        let mut row: Vec<f64> = Vec::with_capacity(k_total);
        for letter in &letters {
            let mut visiting: Vec<char> = Vec::new();
            match letter_value(*letter, run, &basic, generators, &mut visiting) {
                Some(v) => row.push(v),
                None => return Vec::new(),
            }
        }
        design.push(row);
    }
    design
}

// ---------------------------------------------------------------------
// Plackett-Burman screening designs.
// ---------------------------------------------------------------------

/// Plackett-Burman design with `n_runs` runs.
///
/// Supported run counts are 12, 20 and 24; any other count returns
/// None. The design has n_runs - 1 factor columns with entries
/// -1.0 and 1.0. It is built from the standard first row for the
/// run count by cyclic shifts, with a final row of all -1.0:
///
/// - 12 runs: + + - + + + - - - + -
/// - 20 runs: + + - - + + + + - + - + - - - - + + -
/// - 24 runs: + + + + + - + - + + - - + + - - + - + - - - -
pub fn plackett_burman(n_runs: usize) -> Option<Vec<Vec<f64>>> {
    let first: &[f64] = match n_runs {
        12 => &[
            1.0, 1.0, -1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0, 1.0, -1.0,
        ],
        20 => &[
            1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0,
            -1.0, -1.0, -1.0, 1.0, 1.0, -1.0,
        ],
        24 => &[
            1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0,
            1.0, -1.0, -1.0, 1.0, -1.0, 1.0, -1.0, -1.0, -1.0, -1.0,
        ],
        _ => return None,
    };
    let m = first.len();
    debug_assert_eq!(m + 1, n_runs);
    let mut design: Vec<Vec<f64>> = Vec::with_capacity(n_runs);
    // Cyclic rows: row r is the first row rotated left by r places.
    // The set of rotated rows is the same set a right rotation
    // gives, only in a different order.
    for r in 0..m {
        let row: Vec<f64> = (0..m).map(|c| first[(c + r) % m]).collect();
        design.push(row);
    }
    design.push(vec![-1.0; m]);
    Some(design)
}

// ---------------------------------------------------------------------
// Taguchi orthogonal arrays and signal-to-noise ratios.
//
// Arrays are returned in coded units: two-level arrays use -1.0
// and 1.0 for levels 1 and 2; the three-level L9 uses -1.0, 0.0
// and 1.0 for levels 1, 2 and 3. In coded units each column is
// balanced and distinct columns have dot product zero.
// ---------------------------------------------------------------------

fn coded(rows: &[&[i32]]) -> Vec<Vec<f64>> {
    rows.iter()
        .map(|r| r.iter().map(|v| *v as f64).collect())
        .collect()
}

/// Taguchi L8 orthogonal array: 8 runs, 7 two-level factors.
/// Published level entries (1 and 2), coded here as -1.0 and 1.0.
pub fn taguchi_l8() -> Vec<Vec<f64>> {
    coded(&[
        &[-1, -1, -1, -1, -1, -1, -1],
        &[-1, -1, -1, 1, 1, 1, 1],
        &[-1, 1, 1, -1, -1, 1, 1],
        &[-1, 1, 1, 1, 1, -1, -1],
        &[1, -1, 1, -1, 1, -1, 1],
        &[1, -1, 1, 1, -1, 1, -1],
        &[1, 1, -1, -1, 1, 1, -1],
        &[1, 1, -1, 1, -1, -1, 1],
    ])
}

/// Taguchi L9 orthogonal array: 9 runs, 4 three-level factors.
/// Published level entries (1, 2 and 3), coded here as -1.0, 0.0
/// and 1.0.
pub fn taguchi_l9() -> Vec<Vec<f64>> {
    coded(&[
        &[-1, -1, -1, -1],
        &[-1, 0, 0, 0],
        &[-1, 1, 1, 1],
        &[0, -1, 0, 1],
        &[0, 0, 1, -1],
        &[0, 1, -1, 0],
        &[1, -1, 1, 0],
        &[1, 0, -1, 1],
        &[1, 1, 0, -1],
    ])
}

/// A Taguchi orthogonal array by name: "L8" or "L9".
/// Any other name returns None.
pub fn taguchi_orthogonal_array(name: &str) -> Option<Vec<Vec<f64>>> {
    match name {
        "L8" | "l8" => Some(taguchi_l8()),
        "L9" | "l9" => Some(taguchi_l9()),
        _ => None,
    }
}

/// Signal-to-noise ratio, smaller-is-better:
/// -10 * log10(mean of y^2).
pub fn sn_smaller_better(y: &[f64]) -> f64 {
    if y.is_empty() {
        return f64::NAN;
    }
    let ms: f64 = y.iter().map(|v| v * v).sum::<f64>() / y.len() as f64;
    -10.0 * ms.log10()
}

/// Signal-to-noise ratio, larger-is-better:
/// -10 * log10(mean of 1 / y^2).
pub fn sn_larger_better(y: &[f64]) -> f64 {
    if y.is_empty() {
        return f64::NAN;
    }
    let ms: f64 = y.iter().map(|v| 1.0 / (v * v)).sum::<f64>() / y.len() as f64;
    -10.0 * ms.log10()
}

/// Signal-to-noise ratio, nominal-is-best:
/// 10 * log10(mean^2 / variance).
///
/// The variance here is the population variance, with divisor n,
/// not the sample variance with divisor n - 1.
pub fn sn_nominal_best(y: &[f64]) -> f64 {
    if y.is_empty() {
        return f64::NAN;
    }
    let n = y.len() as f64;
    let mean: f64 = y.iter().sum::<f64>() / n;
    let var: f64 = y.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n;
    10.0 * (mean * mean / var).log10()
}

// ---------------------------------------------------------------------
// Response surface designs and quadratic fitting.
// ---------------------------------------------------------------------

/// Central composite design for `k` factors with axial distance
/// `alpha`.
///
/// Runs, in order: the full 2^k factorial points (coded -1 and 1),
/// then 2k axial points (one factor at -alpha then +alpha, the
/// rest at 0, factor by factor), then center points. This function
/// uses exactly 1 center point; use
/// `central_composite_with_center` for a different count.
pub fn central_composite(k: usize, alpha: f64) -> Vec<Vec<f64>> {
    central_composite_with_center(k, alpha, 1)
}

/// Central composite design with a stated number of center points.
pub fn central_composite_with_center(
    k: usize,
    alpha: f64,
    n_center: usize,
) -> Vec<Vec<f64>> {
    let mut runs = factorial_design(k);
    for factor in 0..k {
        for sign in [-1.0, 1.0] {
            let mut row = vec![0.0; k];
            row[factor] = sign * alpha;
            runs.push(row);
        }
    }
    for _ in 0..n_center {
        runs.push(vec![0.0; k]);
    }
    runs
}

/// Box-Behnken design for `k` factors (k at least 3).
///
/// For each pair of factors, the four runs with those two factors
/// at -1 and 1 in all combinations and every other factor at 0,
/// followed by exactly 1 center point. Run counts: 13 for k = 3
/// and 25 for k = 4. Returns an empty matrix for k below 3.
pub fn box_behnken(k: usize) -> Vec<Vec<f64>> {
    if k < 3 {
        return Vec::new();
    }
    let mut runs: Vec<Vec<f64>> = Vec::new();
    for i in 0..k {
        for j in (i + 1)..k {
            for a in [-1.0, 1.0] {
                for b in [-1.0, 1.0] {
                    let mut row = vec![0.0; k];
                    row[i] = a;
                    row[j] = b;
                    runs.push(row);
                }
            }
        }
    }
    runs.push(vec![0.0; k]);
    runs
}

/// Design matrix for a full quadratic model in the run factors.
/// Column order: intercept, linear terms x_i in factor order,
/// squared terms x_i^2 in factor order, then interaction terms
/// x_i * x_j for i < j in lexicographic order.
pub fn quadratic_design(runs: &[Vec<f64>]) -> Vec<Vec<f64>> {
    runs.iter()
        .map(|run| {
            let k = run.len();
            let mut row: Vec<f64> = Vec::new();
            row.push(1.0);
            row.extend_from_slice(run);
            for v in run {
                row.push(v * v);
            }
            for i in 0..k {
                for j in (i + 1)..k {
                    row.push(run[i] * run[j]);
                }
            }
            row
        })
        .collect()
}

/// Fit a full quadratic response surface by least squares.
///
/// The returned coefficients follow the `quadratic_design` column
/// order: [1, x_i, x_i^2, x_i * x_j].
pub fn fit_quadratic(runs: &[Vec<f64>], y: &[f64]) -> Result<Vec<f64>, String> {
    if runs.is_empty() || runs.len() != y.len() {
        return Err("runs and y must be non-empty and the same length".into());
    }
    let design = quadratic_design(runs);
    optim::lstsq(&design, y).ok_or_else(|| "quadratic least squares failed".to_string())
}

/// Predict responses from quadratic coefficients following the
/// `quadratic_design` column order.
pub fn predict_quadratic(coef: &[f64], runs: &[Vec<f64>]) -> Vec<f64> {
    quadratic_design(runs)
        .iter()
        .map(|row| row.iter().zip(coef).map(|(a, b)| a * b).sum())
        .collect()
}

/// ANOVA summary for a fitted quadratic response surface.
#[derive(Debug, Clone)]
pub struct QuadraticAnova {
    pub ss_regression: f64,
    pub ss_residual: f64,
    pub ss_total: f64,
    /// Degrees of freedom for regression: coefficient count - 1.
    pub df_regression: usize,
    /// Degrees of freedom for the residual: runs - coefficient count.
    pub df_residual: usize,
    pub df_total: usize,
    /// F statistic: (SS_reg / df_reg) / (SS_res / df_res). Infinite
    /// when the residual sum of squares is zero and the regression
    /// sum of squares is positive.
    pub f_statistic: f64,
    pub r_squared: f64,
}

/// ANOVA for a full quadratic model fitted to `runs` and `y`.
pub fn quadratic_anova(
    runs: &[Vec<f64>],
    y: &[f64],
) -> Result<QuadraticAnova, String> {
    let coef = fit_quadratic(runs, y)?;
    let pred = predict_quadratic(&coef, runs);
    let n = y.len();
    let mean: f64 = y.iter().sum::<f64>() / n as f64;
    let ss_total: f64 = y.iter().map(|v| (v - mean) * (v - mean)).sum();
    let ss_residual: f64 = y
        .iter()
        .zip(&pred)
        .map(|(v, p)| (v - p) * (v - p))
        .sum();
    let ss_regression: f64 = pred.iter().map(|p| (p - mean) * (p - mean)).sum();
    let p = coef.len();
    let df_regression = p.saturating_sub(1);
    let df_residual = n.saturating_sub(p);
    let df_total = n.saturating_sub(1);
    let f_statistic = if ss_residual == 0.0 {
        if ss_regression > 0.0 {
            f64::INFINITY
        } else {
            f64::NAN
        }
    } else if df_regression == 0 || df_residual == 0 {
        f64::NAN
    } else {
        (ss_regression / df_regression as f64) / (ss_residual / df_residual as f64)
    };
    let r_squared = if ss_total > 0.0 {
        ss_regression / ss_total
    } else {
        f64::NAN
    };
    Ok(QuadraticAnova {
        ss_regression,
        ss_residual,
        ss_total,
        df_regression,
        df_residual,
        df_total,
        f_statistic,
        r_squared,
    })
}
