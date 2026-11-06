//! Parts-count reliability prediction.
//!
//! This framework computes system failure rates, mean time between
//! failures, and reliability from user-supplied base failure rates
//! and user-supplied adjustment factors only. It does not contain
//! proprietary handbook tables, and every rate used in a calculation
//! comes from the caller's own part entries.
//!
//! The clean-room methods section at the end of this module adds
//! SR-332 style calculators and a 62380-style mission profile
//! framework. Those methods were built only from publicly available
//! secondary sources: textbooks, vendor application notes and data
//! sheets, published papers, and university material. No generic
//! failure rate table and no base table from any standard is
//! reconstructed or bundled here. Generic rates and factor values
//! come only from the local library CSV the user loads, or from
//! the labeled synthetic example set.

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

// ---------------- reliability allocation ----------------

/// Equal reliability allocation for a series system.
///
/// For a system target reliability `target_r` split over `n`
/// subsystems in series, each subsystem gets target_r^(1/n), so the
/// product of the subsystem targets equals `target_r`. An `n` of
/// zero gives an empty list.
pub fn allocate_equal(target_r: f64, n: usize) -> Vec<f64> {
    if n == 0 {
        return Vec::new();
    }
    let each = target_r.powf(1.0 / n as f64);
    vec![each; n]
}

/// Weighted reliability allocation for a series system.
///
/// Each weight is the share of the system log-reliability budget
/// assigned to that subsystem: R_i = target_r^(w_i / sum(w)). A
/// larger weight means a harder, lower target for that subsystem,
/// because it takes a larger share of the budget. The product of
/// the results equals `target_r`. When all weights are zero or the
/// weight list is empty of usable weight, the allocation falls
/// back to equal shares.
pub fn allocate_weighted(target_r: f64, weights: &[f64]) -> Vec<f64> {
    if weights.is_empty() {
        return Vec::new();
    }
    let sum: f64 = weights.iter().sum();
    if !(sum > 0.0) {
        return allocate_equal(target_r, weights.len());
    }
    weights
        .iter()
        .map(|w| target_r.powf(w / sum))
        .collect()
}

// ---------------- derating ----------------

/// Derating ratio: operating stress divided by rated stress.
///
/// A ratio of 1.0 means the part runs at its full rated stress.
pub fn derating_ratio(operating_stress: f64, rated_stress: f64) -> f64 {
    operating_stress / rated_stress
}

/// Result of one derating check.
#[derive(Debug, Clone, PartialEq)]
pub struct DeratingResult {
    pub name: String,
    pub ratio: f64,
    pub pass: bool,
}

/// Check one part against a maximum allowed derating ratio.
///
/// The check passes when the derating ratio is at or below
/// `max_ratio`.
pub fn derating_check(
    name: &str,
    operating: f64,
    rated: f64,
    max_ratio: f64,
) -> DeratingResult {
    let ratio = derating_ratio(operating, rated);
    DeratingResult {
        name: name.to_string(),
        ratio,
        pass: ratio <= max_ratio,
    }
}

// ---------------- component library CSV ----------------
//
// A component library is the user's own part data, loaded at
// runtime from CSV text. It is not built into the crate. The
// format is one header line
//
//   name,base_rate_per_million_hours,quantity,stress_factor,quality_factor
//
// followed by one line per part. Lines whose first field starts
// with `#` are comment lines and are skipped, so a file may start
// with a comment such as
// `# Example component library - synthetic example data`.
//
// The stress_factor and quality_factor columns may be left out
// or left empty on a row; both default to 1.0, matching the
// defaults on PartEntry.

/// Escape one CSV field.
///
/// Fields containing a comma, a double quote, or a newline are wrapped
/// in double quotes, and any inner double quotes are doubled.
fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        let doubled = field.replace('"', "\"\"");
        format!("\"{}\"", doubled)
    } else {
        field.to_string()
    }
}

/// Parse CSV text into a table of raw string fields.
///
/// Handles quoted fields, doubled quotes inside quoted fields, commas
/// and newlines inside quoted fields, and CRLF line endings.
fn parse_csv_records(text: &str) -> Result<Vec<Vec<String>>, String> {
    let mut records: Vec<Vec<String>> = Vec::new();
    let mut record: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut chars = text.chars().peekable();
    let mut in_quotes = false;
    let mut field_started = false;

    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(ch);
            }
        } else if ch == '"' && !field_started {
            in_quotes = true;
            field_started = true;
        } else if ch == ',' {
            record.push(std::mem::take(&mut field));
            field_started = false;
        } else if ch == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            record.push(std::mem::take(&mut field));
            field_started = false;
            records.push(std::mem::take(&mut record));
        } else if ch == '\n' {
            record.push(std::mem::take(&mut field));
            field_started = false;
            records.push(std::mem::take(&mut record));
        } else {
            field.push(ch);
            field_started = true;
        }
    }

    if in_quotes {
        return Err("unterminated quoted field in CSV input".to_string());
    }

    if field_started || !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }

    Ok(records)
}

/// Parse an optional factor field. Empty or missing means 1.0.
fn parse_factor_field(value: Option<&String>, column: &str) -> Result<f64, String> {
    match value {
        None => Ok(1.0),
        Some(v) => {
            let trimmed = v.trim();
            if trimmed.is_empty() {
                Ok(1.0)
            } else {
                trimmed
                    .parse::<f64>()
                    .map_err(|_| format!("malformed numeric field for {}: {:?}", column, v))
            }
        }
    }
}

/// Load a component library from CSV text.
///
/// See the format notes above. Comment lines starting with `#`
/// are skipped and the first remaining line is the header.
pub fn library_from_csv(text: &str) -> Result<Vec<PartEntry>, String> {
    let records = parse_csv_records(text)?;
    let mut parts: Vec<PartEntry> = Vec::new();
    let mut header_seen = false;

    for record in records.iter() {
        if record.len() == 1 && record[0].trim().is_empty() {
            continue;
        }
        if !record.is_empty() && record[0].trim_start().starts_with('#') {
            continue;
        }
        if !header_seen {
            header_seen = true;
            continue;
        }
        if record.len() < 3 || record.len() > 5 {
            return Err(format!(
                "expected 3 to 5 fields in library row, found {}: {:?}",
                record.len(),
                record
            ));
        }
        let base_rate = record[1]
            .trim()
            .parse::<f64>()
            .map_err(|_| {
                format!(
                    "malformed numeric field for base_rate_per_million_hours: {:?}",
                    record[1]
                )
            })?;
        let quantity = record[2]
            .trim()
            .parse::<u32>()
            .map_err(|_| format!("malformed numeric field for quantity: {:?}", record[2]))?;
        let stress_factor = parse_factor_field(record.get(3), "stress_factor")?;
        let quality_factor = parse_factor_field(record.get(4), "quality_factor")?;
        parts.push(PartEntry {
            name: record[0].clone(),
            base_rate_per_million_hours: base_rate,
            quantity,
            stress_factor,
            quality_factor,
        });
    }

    Ok(parts)
}

// ---------------- clean-room methods ----------------
//
// The calculators in this section are clean-room implementations
// of published method shapes, built only from publicly available
// secondary sources. They take every rate and factor from the
// caller. In particular, the SR-332 style calculators do not look
// up any generic failure rate: the generic rate argument is a
// value the user supplies, for example from the local library CSV
// loaded with library_from_csv or from the labeled synthetic
// example set. The SR-332 Method I structure here follows the
// vendor documents: a four-factor device rate, with the
// environment factor pi_e applied once at equipment level.

/// Boltzmann constant in eV per kelvin, as used by the Arrhenius
/// factors in this section.
pub const BOLTZMANN_EV_PER_K: f64 = 8.617e-5;

/// Default activation energy in eV for the SR-332 style
/// temperature factor.
///
/// The value 0.35 eV is vendor-sourced: it is the default cited
/// in vendor application notes, in two Broadcom/Avago SR-332
/// data sheets. Callers may pass any activation energy they have
/// a basis for instead.
pub const SR332_DEFAULT_ACTIVATION_ENERGY_EV: f64 = 0.35;

/// Arrhenius temperature factor between a use temperature and a
/// reference temperature, both in degrees Celsius:
///
/// factor = exp((Ea / k) * (1 / T_ref_K - 1 / T_use_K))
///
/// with k = 8.617e-5 eV/K and temperatures converted to kelvin.
/// The factor is exactly 1.0 when the two temperatures are equal,
/// above 1.0 when the use temperature is hotter than the
/// reference, and below 1.0 when it is cooler.
fn arrhenius_factor(activation_energy_ev: f64, temp_c: f64, ref_temp_c: f64) -> f64 {
    let t_use_k = temp_c + 273.15;
    let t_ref_k = ref_temp_c + 273.15;
    ((activation_energy_ev / BOLTZMANN_EV_PER_K) * (1.0 / t_ref_k - 1.0 / t_use_k)).exp()
}

/// SR-332 Method I style device prediction, black-box form.
///
/// Per the vendor documents, the device rate has four factors:
///
/// device_rate = generic_rate * pi_q * pi_s * pi_t
///
/// where pi_q is the quality factor, pi_s is the stress factor,
/// and pi_t is the temperature factor (see
/// `sr332_temperature_factor`). The environment factor pi_e is
/// not part of the device rate: it is applied once at equipment
/// level, in `sr332_equipment_rate`. No generic rate or factor
/// value is looked up anywhere; all inputs come from the caller.
pub fn sr332_method1(generic_rate: f64, pi_q: f64, pi_s: f64, pi_t: f64) -> f64 {
    generic_rate * pi_q * pi_s * pi_t
}

/// SR-332 style equipment rate.
///
/// Per the vendor documents, the environment factor pi_e is
/// applied once at equipment level:
///
/// equipment_rate = pi_e * sum over parts of (quantity * device_rate)
///
/// Each entry in `parts` is a `(quantity, device_rate)` pair,
/// with the device rate as computed by `sr332_method1`.
pub fn sr332_equipment_rate(parts: &[(u32, f64)], pi_e: f64) -> f64 {
    let sum: f64 = parts.iter().map(|(quantity, rate)| *quantity as f64 * rate).sum();
    pi_e * sum
}

/// SR-332 style temperature factor pi_T.
///
/// The closed form is the standard Arrhenius relation,
/// pi_T = exp((Ea / k) * (1 / T_ref_K - 1 / T_use_K)) with
/// k = 8.617e-5 eV/K, the same relation this crate's ALT module
/// already uses; no SR-332 source is claimed for the formula
/// itself. `temp_c` is the use temperature and `ref_temp_c` is
/// the reference temperature, both in degrees Celsius. The
/// 0.35 eV default activation energy is vendor-cited (two
/// Broadcom/Avago SR-332 data sheets; default cited in vendor
/// application notes); see
/// `SR332_DEFAULT_ACTIVATION_ENERGY_EV`.
pub fn sr332_temperature_factor(
    activation_energy_ev: f64,
    temp_c: f64,
    ref_temp_c: f64,
) -> f64 {
    arrhenius_factor(activation_energy_ev, temp_c, ref_temp_c)
}

/// SR-332 Method II style extrapolation from observed conditions.
///
/// Translates a failure rate observed on the user's own data at
/// observed temperature and stress factors to target conditions
/// by the factor ratios:
///
/// rate_target = observed_rate
///     * (pi_t_target / pi_t_observed)
///     * (pi_s_target / pi_s_observed)
pub fn sr332_method2_extrapolate(
    observed_rate: f64,
    pi_t_observed: f64,
    pi_t_target: f64,
    pi_s_observed: f64,
    pi_s_target: f64,
) -> f64 {
    observed_rate * (pi_t_target / pi_t_observed) * (pi_s_target / pi_s_observed)
}

/// Unprefixed alias for `sr332_method2_extrapolate`.
pub fn method2_extrapolate(
    observed_rate: f64,
    pi_t_observed: f64,
    pi_t_target: f64,
    pi_s_observed: f64,
    pi_s_target: f64,
) -> f64 {
    sr332_method2_extrapolate(
        observed_rate,
        pi_t_observed,
        pi_t_target,
        pi_s_observed,
        pi_s_target,
    )
}

/// Chi-square quantile (percent point function).
///
/// A chi-square distribution with nu degrees of freedom is a
/// gamma distribution with shape nu / 2 and scale 2, so this is
/// `specfunc::gamma_ppf(df / 2, 2.0, p)`, using the verified
/// convention that `gamma_ppf(a, scale, p)` is the gamma quantile
/// for shape `a` and the given scale at probability `p`.
pub fn chi2_ppf(p: f64, df: f64) -> f64 {
    crate::specfunc::gamma_ppf(df / 2.0, 2.0, p)
}

/// Two-sided chi-square confidence bounds on a constant failure
/// rate.
///
/// For `r` failures in `T` exposure hours at confidence level
/// `confidence` (with alpha = 1 - confidence), the bounds are
///
/// lower = chi2_ppf(alpha / 2, 2r) / (2T)
/// upper = chi2_ppf(1 - alpha / 2, 2r + 2) / (2T)
///
/// The lower bound is 0 when r is 0. Rates are in failures per
/// hour when the exposure is in hours.
pub fn failure_rate_ci(
    failures: u64,
    exposure_hours: f64,
    confidence: f64,
) -> (f64, f64) {
    let r = failures as f64;
    let alpha = 1.0 - confidence;
    let lower = if failures == 0 {
        0.0
    } else {
        chi2_ppf(alpha / 2.0, 2.0 * r) / (2.0 * exposure_hours)
    };
    let upper = chi2_ppf(1.0 - alpha / 2.0, 2.0 * r + 2.0) / (2.0 * exposure_hours);
    (lower, upper)
}

/// SR-332 Method III style unit-level combination, weighted
/// average core.
///
/// Combines a generic steady-state rate (a Method I or Method II
/// style prediction) with a field rate (unit-level test or field
/// data) as the weights-normalized weighted average
///
/// result = (predicted_rate * predicted_weight
///     + observed_rate * observed_weight)
///     / (predicted_weight + observed_weight)
///
/// The weighted-average structure is the part confirmed from
/// public secondary sources. The weights are user-set, because
/// public sources do not publish the case thresholds that would
/// pin them down. When both weights are zero the result falls
/// back to the plain average of the two rates. This is only the
/// weighted-average core of the method; no further weighting
/// detail is claimed.
pub fn sr332_method3_combine(
    predicted_rate: f64,
    predicted_weight: f64,
    observed_rate: f64,
    observed_weight: f64,
) -> f64 {
    let total = predicted_weight + observed_weight;
    if total == 0.0 {
        return 0.5 * (predicted_rate + observed_rate);
    }
    (predicted_rate * predicted_weight + observed_rate * observed_weight) / total
}

/// Unprefixed alias for `sr332_method3_combine`.
pub fn method3_combine(
    predicted_rate: f64,
    predicted_weight: f64,
    observed_rate: f64,
    observed_weight: f64,
) -> f64 {
    sr332_method3_combine(predicted_rate, predicted_weight, observed_rate, observed_weight)
}

// ---------------- 62380-style mission profile ----------------
//
// This is a 62380-style mission profile framework only. It is not
// IEC 62380 data, it contains no table from that standard, and
// every base value is supplied by the user.
//
// Exact formula implemented by
// `iec62380_style_mission_profile_rate`:
//
//   total_hours = sum of phase duration_hours
//   phase_rate = base_rate * arrhenius_factor(phase temp)
//       for an operating phase, with the Arrhenius form of
//       `sr332_temperature_factor` (same k and formula), or
//   phase_rate = base_rate * dormant_factor
//       for a dormant phase (the dormant multiplier replaces the
//       temperature factor for that phase)
//   thermal_part = sum(duration_hours * phase_rate) / total_hours
//   cycling_part = (sum of phase cycles)
//       * cycling_factor_per_cycle * base_rate / total_hours
//   result = thermal_part + cycling_part
//
// An empty profile, or one with zero total duration, gives 0.0.

/// One phase of a 62380-style mission profile.
///
/// `cycles` is the number of temperature or power cycles counted
/// in this phase. `dormant_factor` is the user-supplied rate
/// multiplier used instead of the temperature factor when
/// `dormant` is true; it is ignored for operating phases.
#[derive(Debug, Clone, PartialEq)]
pub struct MissionPhase {
    pub name: String,
    pub duration_hours: f64,
    pub temp_c: f64,
    pub cycles: f64,
    pub dormant: bool,
    pub dormant_factor: f64,
}

/// Prefixed name for `MissionPhase`, the 62380-style phase type.
pub type Iec62380StyleMissionPhase = MissionPhase;

impl MissionPhase {
    /// Create an operating 62380-style phase with a dormant
    /// factor of 1.0.
    pub fn new(name: &str, duration_hours: f64, temp_c: f64, cycles: f64) -> Self {
        MissionPhase {
            name: name.to_string(),
            duration_hours,
            temp_c,
            cycles,
            dormant: false,
            dormant_factor: 1.0,
        }
    }
}

/// 62380-style mission profile failure rate.
///
/// Computes the time-weighted rate described in the section notes
/// above: each operating phase contributes base_rate times its
/// Arrhenius temperature factor, each dormant phase contributes
/// base_rate times its user-supplied dormant_factor, and the
/// cycling contribution, total cycles times
/// `cycling_factor_per_cycle` times base_rate, is spread over the
/// total profile hours. All base values are user-supplied; this
/// framework holds no standard data.
pub fn iec62380_style_mission_profile_rate(
    base_rate: f64,
    phases: &[MissionPhase],
    activation_energy_ev: f64,
    ref_temp_c: f64,
    cycling_factor_per_cycle: f64,
) -> f64 {
    let total_hours: f64 = phases.iter().map(|p| p.duration_hours).sum();
    if total_hours <= 0.0 {
        return 0.0;
    }
    let mut weighted = 0.0;
    let mut total_cycles = 0.0;
    for phase in phases {
        let phase_rate = if phase.dormant {
            base_rate * phase.dormant_factor
        } else {
            base_rate * arrhenius_factor(activation_energy_ev, phase.temp_c, ref_temp_c)
        };
        weighted += phase.duration_hours * phase_rate;
        total_cycles += phase.cycles;
    }
    let thermal_part = weighted / total_hours;
    let cycling_part = total_cycles * cycling_factor_per_cycle * base_rate / total_hours;
    thermal_part + cycling_part
}

/// Unprefixed alias for `iec62380_style_mission_profile_rate`,
/// the 62380-style mission profile rate.
pub fn mission_profile_rate(
    base_rate: f64,
    phases: &[MissionPhase],
    activation_energy_ev: f64,
    ref_temp_c: f64,
    cycling_factor_per_cycle: f64,
) -> f64 {
    iec62380_style_mission_profile_rate(
        base_rate,
        phases,
        activation_energy_ev,
        ref_temp_c,
        cycling_factor_per_cycle,
    )
}

/// Serialize a component library to CSV text.
///
/// Writes the header line followed by one line per part, in the
/// format read by `library_from_csv`.
pub fn library_to_csv(parts: &[PartEntry]) -> String {
    let mut out = String::new();
    out.push_str("name,base_rate_per_million_hours,quantity,stress_factor,quality_factor\n");
    for part in parts {
        let fields: Vec<String> = vec![
            escape_csv_field(&part.name),
            part.base_rate_per_million_hours.to_string(),
            part.quantity.to_string(),
            part.stress_factor.to_string(),
            part.quality_factor.to_string(),
        ];
        out.push_str(&fields.join(","));
        out.push('\n');
    }
    out
}
