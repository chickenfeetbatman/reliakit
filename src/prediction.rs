//! Parts-count reliability prediction.
//!
//! This framework computes system failure rates, mean time between
//! failures, and reliability from user-supplied base failure rates
//! and user-supplied adjustment factors only. It does not contain
//! proprietary handbook tables, and every rate used in a calculation
//! comes from the caller's own part entries.

/// One part entry in a parts-count prediction.
///
/// The base failure rate is supplied by the user in failures per
/// million hours. The stress and quality factors are also supplied
/// by the user and default to 1.0, meaning no adjustment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PartEntry {
    pub name: String,
    pub base_rate_per_million_hours: f64,
    pub quantity: u32,
    pub stress_factor: f64,
    pub quality_factor: f64,
}

impl PartEntry {
    /// Create a part entry with stress and quality factors of 1.0.
    pub fn new(name: &str, base_rate_per_million_hours: f64, quantity: u32) -> Self {
        PartEntry {
            name: name.to_string(),
            base_rate_per_million_hours,
            quantity,
            stress_factor: 1.0,
            quality_factor: 1.0,
        }
    }
}

/// Failure rate contribution of one part entry, in failures per
/// million hours: base rate times quantity times stress factor times
/// quality factor.
pub fn part_failure_rate_per_million_hours(p: &PartEntry) -> f64 {
    p.base_rate_per_million_hours * p.quantity as f64 * p.stress_factor * p.quality_factor
}

/// System failure rate in failures per million hours.
///
/// This is the series model sum of the part contributions, which
/// assumes any part failure fails the system.
pub fn system_failure_rate_per_million_hours(parts: &[PartEntry]) -> f64 {
    parts.iter().map(part_failure_rate_per_million_hours).sum()
}

/// System mean time between failures in hours.
///
/// Returns positive infinity when the system failure rate is zero.
pub fn system_mtbf_hours(parts: &[PartEntry]) -> f64 {
    let rate = system_failure_rate_per_million_hours(parts);
    if rate == 0.0 {
        f64::INFINITY
    } else {
        1e6 / rate
    }
}

/// System reliability at `t_hours` under a constant failure rate.
///
/// Reliability is exp(-rate * t / 1e6) with the rate in failures per
/// million hours.
pub fn system_reliability(parts: &[PartEntry], t_hours: f64) -> f64 {
    let rate = system_failure_rate_per_million_hours(parts);
    (-rate * t_hours / 1e6).exp()
}
