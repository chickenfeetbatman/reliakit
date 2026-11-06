//! Small numerical helpers: Nelder-Mead, golden-section scalar
//! minimisation, dense linear solve / inverse, least squares.

/// Outcome of a reporting minimisation: whether the search stopped
/// at an interior point, at a bracket bound, or at its iteration cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarOutcome {
    Interior,
    AtLowerBound,
    AtUpperBound,
    IterationCap,
}

/// Outcome of a reporting Nelder-Mead minimisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NelderMeadOutcome {
    Converged,
    IterationCap,
}

/// Nelder-Mead simplex minimisation. Returns (x_best, f_best).
pub fn nelder_mead<F: FnMut(&[f64]) -> f64>(
    f: &mut F,
    x0: &[f64],
    step: &[f64],
    max_iter: usize,
    tol: f64,
) -> (Vec<f64>, f64) {
    let (x, fx, _) = nelder_mead_report(f, x0, step, max_iter, tol);
    (x, fx)
}

/// Nelder-Mead minimisation that also reports whether it converged
/// (simplex spread below tol) or stopped at the iteration cap. The
/// search itself is identical to nelder_mead.
pub fn nelder_mead_report<F: FnMut(&[f64]) -> f64>(
    f: &mut F,
    x0: &[f64],
    step: &[f64],
    max_iter: usize,
    tol: f64,
) -> (Vec<f64>, f64, NelderMeadOutcome) {
    let n = x0.len();
    let mut simplex: Vec<Vec<f64>> = Vec::with_capacity(n + 1);
    simplex.push(x0.to_vec());
    for i in 0..n {
        let mut x = x0.to_vec();
        x[i] += step[i];
        simplex.push(x);
    }
    let mut vals: Vec<f64> = simplex.iter().map(|x| f(x)).collect();
    let (alpha, gamma, rho, sigma) = (1.0, 2.0, 0.5, 0.5);
    let mut converged = false;
    for _ in 0..max_iter {
        // sort by value
        let mut idx: Vec<usize> = (0..=n).collect();
        idx.sort_by(|&a, &b| vals[a].partial_cmp(&vals[b]).unwrap());
        let simplex_s: Vec<Vec<f64>> = idx.iter().map(|&i| simplex[i].clone()).collect();
        let vals_s: Vec<f64> = idx.iter().map(|&i| vals[i]).collect();
        simplex = simplex_s;
        vals = vals_s;
        let spread = (0..n)
            .map(|j| {
                let mx = simplex.iter().map(|x| x[j]).fold(f64::MIN, f64::max);
                let mn = simplex.iter().map(|x| x[j]).fold(f64::MAX, f64::min);
                mx - mn
            })
            .fold(0.0f64, f64::max);
        if spread < tol {
            converged = true;
            break;
        }
        // centroid of all but worst
        let mut centroid = vec![0.0; n];
        for point in simplex.iter().take(n) {
            for (c, v) in centroid.iter_mut().zip(point.iter()) {
                *c += v / n as f64;
            }
        }
        let worst = simplex[n].clone();
        let xr: Vec<f64> = centroid
            .iter()
            .zip(&worst)
            .map(|(c, w)| c + alpha * (c - w))
            .collect();
        let fr = f(&xr);
        if fr < vals[0] {
            let xe: Vec<f64> = centroid
                .iter()
                .zip(&worst)
                .map(|(c, w)| c + gamma * (c - w))
                .collect();
            let fe = f(&xe);
            if fe < fr {
                simplex[n] = xe;
                vals[n] = fe;
            } else {
                simplex[n] = xr;
                vals[n] = fr;
            }
        } else if fr < vals[n - 1] {
            simplex[n] = xr;
            vals[n] = fr;
        } else {
            let xc: Vec<f64> = centroid
                .iter()
                .zip(&worst)
                .map(|(c, w)| c + rho * (w - c))
                .collect();
            let fc = f(&xc);
            if fc < vals[n] {
                simplex[n] = xc;
                vals[n] = fc;
            } else {
                // shrink toward best
                let best = simplex[0].clone();
                for i in 1..=n {
                    for j in 0..n {
                        simplex[i][j] = best[j] + sigma * (simplex[i][j] - best[j]);
                    }
                    vals[i] = f(&simplex[i]);
                }
            }
        }
    }
    let mut best_i = 0;
    for i in 1..=n {
        if vals[i] < vals[best_i] {
            best_i = i;
        }
    }
    let outcome = if converged {
        NelderMeadOutcome::Converged
    } else {
        NelderMeadOutcome::IterationCap
    };
    (simplex[best_i].clone(), vals[best_i], outcome)
}

/// Golden-section minimisation on [a, b]. Returns (x, f(x)).
pub fn minimize_scalar<F: FnMut(f64) -> f64>(f: &mut F, a: f64, b: f64, tol: f64) -> (f64, f64) {
    let (x, fx, _) = minimize_scalar_report(f, a, b, tol);
    (x, fx)
}

/// Golden-section minimisation that also reports how it stopped.
/// The outcome is Interior when the converged point lies strictly
/// inside the original bracket, AtLowerBound or AtUpperBound when it
/// sits on a bracket end (within a small fraction of the bracket
/// width), and IterationCap when the iteration limit was reached
/// before the bracket shrank below tol.
pub fn minimize_scalar_report<F: FnMut(f64) -> f64>(
    f: &mut F,
    a: f64,
    b: f64,
    tol: f64,
) -> (f64, f64, ScalarOutcome) {
    let (a0, b0) = (a, b);
    let gr = (5.0f64.sqrt() - 1.0) / 2.0;
    let (mut a, mut b) = (a, b);
    let mut c = b - gr * (b - a);
    let mut d = a + gr * (b - a);
    let mut fc = f(c);
    let mut fd = f(d);
    let mut converged = false;
    for _ in 0..500 {
        if (b - a).abs() < tol {
            converged = true;
            break;
        }
        if fc < fd {
            b = d;
            d = c;
            fd = fc;
            c = b - gr * (b - a);
            fc = f(c);
        } else {
            a = c;
            c = d;
            fc = fd;
            d = a + gr * (b - a);
            fd = f(d);
        }
    }
    let x = 0.5 * (a + b);
    let fx = f(x);
    let outcome = if !converged {
        ScalarOutcome::IterationCap
    } else {
        let width = (b0 - a0).abs().max(1e-300);
        let near = 1e-6 * width;
        if (x - a0).abs() <= near.max(tol) {
            ScalarOutcome::AtLowerBound
        } else if (x - b0).abs() <= near.max(tol) {
            ScalarOutcome::AtUpperBound
        } else {
            ScalarOutcome::Interior
        }
    };
    (x, fx, outcome)
}

/// Solve A x = b by Gaussian elimination with partial pivoting.
pub fn solve_linear(a_in: &[Vec<f64>], b_in: &[f64]) -> Option<Vec<f64>> {
    let n = b_in.len();
    let mut a: Vec<Vec<f64>> = a_in.to_vec();
    let mut b = b_in.to_vec();
    for col in 0..n {
        let mut piv = col;
        for r in col + 1..n {
            if a[r][col].abs() > a[piv][col].abs() {
                piv = r;
            }
        }
        if a[piv][col].abs() < 1e-300 {
            return None;
        }
        if piv != col {
            a.swap(piv, col);
            b.swap(piv, col);
        }
        for r in col + 1..n {
            let factor = a[r][col] / a[col][col];
            if factor == 0.0 {
                continue;
            }
            let (top, bottom) = a.split_at_mut(r);
            for (cell, p) in bottom[0][col..].iter_mut().zip(top[col][col..].iter()) {
                *cell -= factor * p;
            }
            b[r] -= factor * b[col];
        }
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut s = b[i];
        for j in i + 1..n {
            s -= a[i][j] * x[j];
        }
        x[i] = s / a[i][i];
    }
    Some(x)
}

/// Matrix inverse via Gauss-Jordan. Returns None if singular.
pub fn matrix_inverse(a_in: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a_in.len();
    let mut cols: Vec<Vec<f64>> = Vec::with_capacity(n);
    for j in 0..n {
        let mut e = vec![0.0; n];
        e[j] = 1.0;
        cols.push(solve_linear(a_in, &e)?);
    }
    // cols[j] is column j of the inverse
    let mut inv = vec![vec![0.0; n]; n];
    for j in 0..n {
        for i in 0..n {
            inv[i][j] = cols[j][i];
        }
    }
    Some(inv)
}

/// Least squares via normal equations. X is row-major (rows x cols).
pub fn lstsq(x: &[Vec<f64>], y: &[f64]) -> Option<Vec<f64>> {
    let rows = x.len();
    let cols = if rows > 0 { x[0].len() } else { 0 };
    let mut ata = vec![vec![0.0; cols]; cols];
    let mut aty = vec![0.0; cols];
    for i in 0..cols {
        for j in 0..cols {
            let mut s = 0.0;
            for row in x.iter() {
                s += row[i] * row[j];
            }
            ata[i][j] = s;
        }
        let mut s = 0.0;
        for r in 0..rows {
            s += x[r][i] * y[r];
        }
        aty[i] = s;
    }
    solve_linear(&ata, &aty)
}
