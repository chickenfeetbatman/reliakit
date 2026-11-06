//! Warranty returns analysis using a Nevada table layout.
//!
//! A Nevada table counts returns by the period in which a unit was
//! sold and the age of the unit when it was returned. This module
//! also fits a two parameter Weibull model to return ages and uses
//! it to forecast returns from units still in service.
//!
//! Age convention for the forecast, applied identically to failures
//! and to censored units: a unit sold in period s is at risk
//! starting in period s, a return in period r is a failure at age
//! r - s + 1, and a unit still in service at the end of the current
//! period c has censoring age c - s + 1. A return in the same period
//! as the sale is a failure at age 1, and a cohort sold in the
//! current period contributes censored units at age 1.
//!
//! The forecast is cut off at the current period. A record with a
//! return period after the current period is a future return: it is
//! excluded from the fit ages, from the cohort return counts, and
//! from the forecast, and the exclusion is counted and reported in
//! the result.

use crate::distributions;

/// One warranty return record.
///
/// `sale_period` is the period in which the unit was sold and
/// `return_period` is the period in which it was returned. Periods
/// are integer indices in whatever period unit the caller uses.
#[derive(Debug, Clone, Copy)]
pub struct WarrantyRecord {
    pub sale_period: usize,
    pub return_period: usize,
}

/// Build a Nevada table of return counts.
///
/// The result is a matrix indexed by `[sale_period][age]`, where
/// age is `return_period - sale_period`. There are `sales_periods`
/// rows and `max_age + 1` columns. Records with a return period
/// before the sale period, an age greater than `max_age`, or a sale
/// period outside the table are ignored.
pub fn nevada_table(
    sales_periods: usize,
    records: &[WarrantyRecord],
    max_age: usize,
) -> Vec<Vec<u32>> {
    let mut table = vec![vec![0u32; max_age + 1]; sales_periods];
    for record in records {
        if record.return_period < record.sale_period {
            continue;
        }
        let age = record.return_period - record.sale_period;
        if age > max_age {
            continue;
        }
        if record.sale_period >= sales_periods {
            continue;
        }
        table[record.sale_period][age] += 1;
    }
    table
}

/// Sum a Nevada table by age, returning one total per age column.
pub fn returns_by_age(table: &[Vec<u32>]) -> Vec<u32> {
    let width = table.iter().map(|row| row.len()).max().unwrap_or(0);
    let mut totals = vec![0u32; width];
    for row in table {
        for (age, count) in row.iter().enumerate() {
            totals[age] += count;
        }
    }
    totals
}

/// Total number of returns recorded in a Nevada table.
pub fn total_returns(table: &[Vec<u32>]) -> u32 {
    table.iter().flat_map(|row| row.iter()).sum()
}

/// Result of a warranty returns forecast.
#[derive(Debug, Clone)]
pub struct WarrantyForecast {
    /// Weibull shape parameter from the fit to return ages.
    pub beta: f64,
    /// Weibull scale parameter from the fit to return ages.
    pub eta: f64,
    /// Expected number of returns over the forecast horizon.
    pub expected_returns: f64,
    /// Number of sold units that have not been returned.
    pub units_in_service: f64,
    /// Number of return records excluded because their return
    /// period is after the current period.
    pub excluded_future_returns: usize,
    /// Notes on the analysis, including the cutoff and the number
    /// of excluded future returns, and the age convention used.
    pub notes: Vec<String>,
}

/// A record counts toward a cohort, the fit, and the forecast only
/// when its sale period names a cohort in `sales`, its return
/// period is not before its sale period, and its return period is
/// not after `current_period`.
fn record_counts(record: &WarrantyRecord, sales_len: usize, current_period: usize) -> bool {
    record.sale_period < sales_len
        && record.return_period >= record.sale_period
        && record.return_period <= current_period
}

/// Count valid returns per sale cohort.
///
/// Only records accepted by `record_counts` are counted, so future
/// returns with a return period after `current_period` never reduce
/// a cohort's units in service.
fn returns_per_cohort(
    sales: &[u32],
    records: &[WarrantyRecord],
    current_period: usize,
) -> Vec<u32> {
    let mut counts = vec![0u32; sales.len()];
    for record in records {
        if !record_counts(record, sales.len(), current_period) {
            continue;
        }
        counts[record.sale_period] += 1;
    }
    counts
}

/// Forecast warranty returns over the next `horizon_periods` periods.
///
/// Records with a return period after `current_period` are future
/// returns. They are excluded from the fit ages, from the cohort
/// return counts, and from the forecast. The result counts them in
/// `excluded_future_returns` and states the count and the cutoff
/// period in `notes`. Records with a return period before the sale
/// period are data errors: they never enter the fit, and they are
/// counted in `notes` when any are present.
///
/// One age convention applies to failures and censored units
/// alike. A unit sold in period s is at risk starting in period s.
/// A return in period r is a failure at age `r - s + 1`. A unit
/// still in service at the end of the current period c is censored
/// at age `c - s + 1`. A cohort sold in the current period
/// therefore contributes censored units at age 1, and a return in
/// the sale period is a failure at age 1. Cohorts sold after
/// `current_period` contribute nothing.
///
/// After future returns are excluded, a cohort whose return count
/// exceeds its sales is an error naming the cohort and both counts.
///
/// A two parameter Weibull distribution is fit by maximum likelihood
/// through `crate::distributions::fit_mle`. The expected future
/// returns for a cohort are its in-service count times the
/// conditional probability of failing between its censoring age
/// `current_period - s + 1` and that age plus the horizon.
pub fn forecast_returns(
    sales: &[u32],
    records: &[WarrantyRecord],
    current_period: usize,
    horizon_periods: usize,
) -> Result<WarrantyForecast, String> {
    let excluded_future_returns = records
        .iter()
        .filter(|record| record.return_period > current_period)
        .count();
    let excluded_before_sale = records
        .iter()
        .filter(|record| {
            record.return_period <= current_period && record.return_period < record.sale_period
        })
        .count();

    let mut notes: Vec<String> = vec![
        format!(
            "Excluded {excluded_future_returns} future return record(s) with a return \
             period after the analysis cutoff period {current_period}; excluded records \
             were not used in the fit, the cohort return counts, or the forecast."
        ),
        "Age convention: a unit sold in period s is at risk starting in period s, \
         a return in period r is a failure at age r - s + 1, and a unit still in \
         service at the end of the current period c has censoring age c - s + 1."
            .to_string(),
    ];
    if excluded_before_sale > 0 {
        notes.push(format!(
            "Excluded {excluded_before_sale} return record(s) with a return period \
             before the sale period as data errors; they were not used in the fit."
        ));
    }

    let cohort_returns = returns_per_cohort(sales, records, current_period);

    // Excess returns are an error, not a silent saturation.
    for (s, sold) in sales.iter().enumerate() {
        if cohort_returns[s] > *sold {
            return Err(format!(
                "warranty cohort for sale period {s} has {} returns, which exceeds \
                 its sales of {sold}",
                cohort_returns[s]
            ));
        }
    }

    let mut ages: Vec<f64> = Vec::new();
    let mut failed_flags: Vec<bool> = Vec::new();

    // Failures: one entry per counted returned unit, at age
    // return_period - sale_period + 1.
    for record in records {
        if !record_counts(record, sales.len(), current_period) {
            continue;
        }
        let age = (record.return_period - record.sale_period + 1) as f64;
        ages.push(age);
        failed_flags.push(true);
    }

    // Censored units: sold but not returned, one entry per unit, at
    // age current_period - s + 1. The excess check above makes the
    // subtraction exact, and a cohort sold in the current period
    // contributes units at age 1.
    for (s, sold) in sales.iter().enumerate() {
        if s > current_period {
            continue;
        }
        let censor_age = current_period - s + 1;
        let in_service = sold - cohort_returns[s];
        for _ in 0..in_service {
            ages.push(censor_age as f64);
            failed_flags.push(false);
        }
    }

    if ages.is_empty() {
        return Err("no failure or censoring data for warranty forecast".to_string());
    }
    if !failed_flags.iter().any(|flag| *flag) {
        return Err("no returned units for warranty forecast".to_string());
    }

    let fit = distributions::fit_mle("weibull2", &ages, Some(&failed_flags))?;

    let beta = fit
        .params
        .get("beta")
        .copied()
        .ok_or_else(|| "weibull2 fit did not return a beta parameter".to_string())?;
    let eta = fit
        .params
        .get("eta")
        .copied()
        .ok_or_else(|| "weibull2 fit did not return an eta parameter".to_string())?;

    let eps = 1e-12f64;
    let horizon = horizon_periods as f64;
    let mut expected_returns = 0.0f64;
    let mut units_in_service = 0.0f64;

    for (s, sold) in sales.iter().enumerate() {
        if s > current_period {
            continue;
        }
        let in_service = (sold - cohort_returns[s]) as f64;
        units_in_service += in_service;
        if in_service <= 0.0 {
            continue;
        }
        let current_age = (current_period - s + 1) as f64;
        let f_now = distributions::cdf("weibull2", &fit.params, current_age);
        let f_later = distributions::cdf("weibull2", &fit.params, current_age + horizon);
        let survival_now = (1.0 - f_now).max(eps);
        expected_returns += in_service * (f_later - f_now) / survival_now;
    }

    Ok(WarrantyForecast {
        beta,
        eta,
        expected_returns,
        units_in_service,
        excluded_future_returns,
        notes,
    })
}
