//! MIL-HDBK-217F Notice 2 part stress models, backed by the transcribed
//! handbook tables in `libraries/mil217/`.
//!
//! Every number used here comes from the JSON data files in
//! `libraries/mil217/`, which were transcribed from the MIL-HDBK-217F
//! Notice 2 PDF (28 February 1995) and, for sections Notice 2 did not
//! revise, from the base MIL-HDBK-217F pages those sections keep. Each
//! JSON file carries its own provenance: handbook section, the PDF page
//! numbers it was transcribed from, and the handbook page numbers.
//!
//! The generic model in the handbook is
//! `lambda_p = lambda_b * (product of pi factors)`, in failures per
//! 10^6 hours. [`part_failure_rate`] evaluates that product from named
//! inputs. Section 16.1 (plated through hole interconnection
//! assemblies) has a different printed equation and is evaluated by
//! [`pth_failure_rate`] instead.
//!
//! Input conventions for [`part_failure_rate`]:
//! - `"style"` or `"type"`: a [`FactorInput::Named`] key into the
//!   category's `lambda_b` by-style or by-type table. The style entry
//!   also selects table columns (for example `pi_T_column`) and any
//!   fixed factor values the handbook prints for that style (for
//!   example `pi_T = 1` for resistor style RD).
//! - `"environment"`: a [`FactorInput::Named`] environment symbol as
//!   the handbook prints it, for example `"GB"`, `"GF"`, `"AIC"`, `"CL"`.
//! - `"quality"`: a [`FactorInput::Named`] quality level as the
//!   handbook prints it for that category, for example `"S"`, `"R"`,
//!   `"MIL-SPEC"`, `"Lower"`, `"Non-Established Reliability"`.
//! - Table variables: a [`FactorInput::Num`] under the variable name
//!   the JSON table declares (for example `"temperature_c"`,
//!   `"power_stress"`, `"capacitance_uf"`, `"voltage_stress"`,
//!   `"hot_spot_temperature_c"`, `"frame_temperature_c"`,
//!   `"frequency_mhz"`, `"active_pins"`, `"load_stress"` together with
//!   `"load_type"`). Series and column tables are interpolated
//!   linearly and are exact at printed grid points.
//! - A factor itself: a [`FactorInput::Num`] under the factor name
//!   (for example `"pi_CYC"`) uses that value directly. This is how
//!   formula factors printed in the handbook are supplied, using the
//!   formula helpers in this module (for example [`relay_pi_cyc`],
//!   [`capacitor_pi_v`], [`load_stress_pi_l`]).
//!
//! A pi factor whose inputs were not supplied is skipped (treated as
//! 1.0), except that a supplied input that lands on a blank handbook
//! entry (for example an inductive load stress above the printed
//! table) makes the result NaN rather than a silently wrong number.

use serde_json::Value;
use std::collections::HashMap;

/// One named or numeric input to a part failure rate calculation.
#[derive(Debug, Clone, PartialEq)]
pub enum FactorInput {
    /// A numeric value: a table variable, a factor value, or lambda_b.
    Num(f64),
    /// A named value: a style, type, environment, quality, load type,
    /// contact form, or similar handbook label.
    Named(String),
}

impl FactorInput {
    pub fn num(v: f64) -> Self {
        FactorInput::Num(v)
    }
    pub fn named(s: &str) -> Self {
        FactorInput::Named(s.to_string())
    }
    fn as_num(&self) -> Option<f64> {
        match self {
            FactorInput::Num(v) => Some(*v),
            _ => None,
        }
    }
    fn as_named(&self) -> Option<&str> {
        match self {
            FactorInput::Named(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

/// The set of inputs to [`part_failure_rate`], keyed by input name.
pub type Inputs = HashMap<String, FactorInput>;

/// One axis of a [`LambdaBTable`]: a name and its printed grid values.
#[derive(Debug, Clone, PartialEq)]
pub struct Axis {
    pub name: String,
    pub values: Vec<f64>,
}

/// A base failure rate table on a regular grid, transcribed from a
/// handbook lambda_b table. The grid is stored row major with the
/// first axis as the slow axis. A `None` entry is a blank in the
/// printed table (that combination is outside the rated range).
#[derive(Debug, Clone, PartialEq)]
pub struct LambdaBTable {
    pub axes: Vec<Axis>,
    pub grid: Vec<Option<f64>>,
}

impl LambdaBTable {
    /// Build a table from the `lambda_b` object of a category JSON
    /// file. Returns `None` when the object is not a grid or series
    /// table (for example a by-style table).
    pub fn from_json(lambda_b: &Value) -> Option<LambdaBTable> {
        let axes_v = lambda_b.get("axes")?.as_array()?;
        let mut axes = Vec::new();
        for a in axes_v {
            let name = a.get("name")?.as_str()?.to_string();
            let values = a
                .get("values")?
                .as_array()?
                .iter()
                .filter_map(|v| v.as_f64())
                .collect::<Vec<f64>>();
            if values.is_empty() {
                return None;
            }
            axes.push(Axis { name, values });
        }
        let grid = parse_grid(lambda_b.get("grid")?)?;
        let expect: usize = axes.iter().map(|a| a.values.len()).product();
        if grid.len() != expect {
            return None;
        }
        Some(LambdaBTable { axes, grid })
    }

    fn at(&self, idx: &[usize]) -> Option<f64> {
        let mut flat = 0usize;
        for (axis, &i) in self.axes.iter().zip(idx.iter()) {
            flat = flat * axis.values.len() + i;
        }
        self.grid[flat]
    }

    /// Look up lambda_b at the given axis values. With one axis this
    /// is linear interpolation; with two axes it is bilinear
    /// interpolation. Exact grid points return the exact printed
    /// table values. Values outside the printed range are clamped to
    /// the nearest printed row or column. If any corner needed for a
    /// bilinear interpolation is a blank table entry, the result is
    /// NaN.
    pub fn lambda_b(&self, axis_values: &[f64]) -> f64 {
        match self.axes.len() {
            0 => self.grid.first().copied().flatten().unwrap_or(f64::NAN),
            1 => {
                let x = axis_values.first().copied().unwrap_or(f64::NAN);
                if x.is_nan() {
                    return f64::NAN;
                }
                let (i, frac) = bracket(&self.axes[0].values, x);
                let a = self.at(&[i]);
                if frac == 0.0 {
                    return a.unwrap_or(f64::NAN);
                }
                let b = self.at(&[i + 1]);
                match (a, b) {
                    (Some(a), Some(b)) => a + frac * (b - a),
                    _ => f64::NAN,
                }
            }
            2 => {
                let x = axis_values.first().copied().unwrap_or(f64::NAN);
                let y = axis_values.get(1).copied().unwrap_or(f64::NAN);
                if x.is_nan() || y.is_nan() {
                    return f64::NAN;
                }
                let (i, fx) = bracket(&self.axes[0].values, x);
                let (j, fy) = bracket(&self.axes[1].values, y);
                let i2 = if fx == 0.0 { i } else { i + 1 };
                let j2 = if fy == 0.0 { j } else { j + 1 };
                let c00 = self.at(&[i, j]);
                let c01 = self.at(&[i, j2]);
                let c10 = self.at(&[i2, j]);
                let c11 = self.at(&[i2, j2]);
                match (c00, c01, c10, c11) {
                    (Some(a), Some(b), Some(c), Some(d)) => {
                        let top = a + fy * (b - a);
                        let bottom = c + fy * (d - c);
                        top + fx * (bottom - top)
                    }
                    _ => f64::NAN,
                }
            }
            _ => f64::NAN,
        }
    }
}

/// Parse a JSON grid (a flat array, or an array of rows) of numbers
/// and nulls into a flat row major vector.
fn parse_grid(v: &Value) -> Option<Vec<Option<f64>>> {
    let arr = v.as_array()?;
    let mut out = Vec::new();
    for item in arr {
        if let Some(row) = item.as_array() {
            for cell in row {
                out.push(if cell.is_null() {
                    None
                } else {
                    Some(cell.as_f64()?)
                });
            }
        } else if item.is_null() {
            out.push(None);
        } else {
            out.push(Some(item.as_f64()?));
        }
    }
    Some(out)
}

/// Find the bracket for `x` in a sorted axis: the index of the grid
/// point at or below `x`, and the fraction toward the next point.
/// Values below the first or above the last point are clamped, and
/// an exact grid point gives fraction 0.
fn bracket(axis: &[f64], x: f64) -> (usize, f64) {
    if axis.len() < 2 || x <= axis[0] {
        return (0, 0.0);
    }
    if x >= axis[axis.len() - 1] {
        return (axis.len() - 1, 0.0);
    }
    for i in 0..axis.len() - 1 {
        if x >= axis[i] && x <= axis[i + 1] {
            if x == axis[i] {
                return (i, 0.0);
            }
            if x == axis[i + 1] {
                return (i + 1, 0.0);
            }
            return (i, (x - axis[i]) / (axis[i + 1] - axis[i]));
        }
    }
    (axis.len() - 1, 0.0)
}

/// Linear interpolation on a one dimensional series, exact at grid
/// points and clamped outside the printed range. A `None` value at a
/// needed point gives NaN.
fn interp_series(axis: &[f64], values: &[Option<f64>], x: f64) -> Option<f64> {
    if axis.is_empty() || axis.len() != values.len() {
        return None;
    }
    if axis.len() == 1 {
        return values[0];
    }
    let (i, frac) = bracket(axis, x);
    if frac == 0.0 {
        return values[i];
    }
    match (values[i], values[i + 1]) {
        (Some(a), Some(b)) => Some(a + frac * (b - a)),
        _ => None,
    }
}

/// Load one category JSON file's contents.
pub fn load_category(json_text: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(json_text)
}

/// The lambda_b grid or series table of a category, if it has one.
pub fn category_lambda_b_table(category: &Value) -> Option<LambdaBTable> {
    LambdaBTable::from_json(category.get("lambda_b")?)
}

/// Look up a named value in a named pi table, for example
/// `pi_table_value(cat, "pi_E", "GF")`.
pub fn pi_table_value(category: &Value, factor: &str, name: &str) -> Option<f64> {
    category
        .get("pi_tables")?
        .get(factor)?
        .get("values")?
        .get(name)?
        .as_f64()
}

/// The by-style or by-type entry selected by `name`, if the category
/// lambda_b table has one.
fn style_entry<'a>(category: &'a Value, name: &str) -> Option<&'a Value> {
    let lb = category.get("lambda_b")?;
    for key in ["by_style", "by_type"] {
        if let Some(e) = lb.get(key).and_then(|m| m.get(name)) {
            return Some(e);
        }
    }
    None
}

/// Resolve the base failure rate for a calculation: an explicit
/// `"lambda_b"` numeric input wins, then a style or type entry, then
/// the category's grid or series lambda_b table evaluated at the
/// axis variables named in the table.
pub fn lambda_b_for(category: &Value, inputs: &Inputs) -> Option<f64> {
    if let Some(v) = inputs.get("lambda_b").and_then(|i| i.as_num()) {
        return Some(v);
    }
    for key in ["style", "type"] {
        if let Some(name) = inputs.get(key).and_then(|i| i.as_named()) {
            if let Some(e) = style_entry(category, name) {
                if let Some(v) = e.get("lambda_b").and_then(|v| v.as_f64()) {
                    return Some(v);
                }
            }
        }
    }
    if let Some(table) = category_lambda_b_table(category) {
        let mut vals = Vec::new();
        for axis in &table.axes {
            vals.push(inputs.get(&axis.name).and_then(|i| i.as_num())?);
        }
        return Some(table.lambda_b(&vals));
    }
    category
        .get("lambda_b")
        .and_then(|lb| lb.get("value"))
        .and_then(|v| v.as_f64())
}

enum Resolved {
    Value(f64),
    NotProvided,
    Failed,
}

/// Resolve one pi factor from its JSON table and the inputs.
fn resolve_factor(
    _category: &Value,
    factor: &str,
    table: &Value,
    inputs: &Inputs,
    style: Option<&Value>,
) -> Resolved {
    // A direct numeric or named input under the factor name wins.
    if let Some(inp) = inputs.get(factor) {
        match inp {
            FactorInput::Num(v) => return Resolved::Value(*v),
            FactorInput::Named(name) => {
                if let Some(v) = table
                    .get("values")
                    .and_then(|m| m.get(name.as_str()))
                    .and_then(|v| v.as_f64())
                {
                    return Resolved::Value(v);
                }
                return Resolved::Failed;
            }
        }
    }
    // A fixed value printed on the selected style entry, for example
    // pi_T = 1 or pi_SR = 1.
    if let Some(s) = style {
        if let Some(v) = s.get(factor).and_then(|v| v.as_f64()) {
            return Resolved::Value(v);
        }
    }
    let kind = table.get("kind").and_then(|k| k.as_str()).unwrap_or("");
    let input_key = table.get("input").and_then(|k| k.as_str());
    match kind {
        "named" | "named_grid" => {
            let key = match input_key {
                Some(k) => k,
                None => return Resolved::NotProvided,
            };
            match inputs.get(key) {
                Some(FactorInput::Named(name)) => {
                    match table
                        .get("values")
                        .and_then(|m| m.get(name.as_str()))
                        .and_then(|v| v.as_f64())
                    {
                        Some(v) => Resolved::Value(v),
                        None => Resolved::Failed,
                    }
                }
                Some(FactorInput::Num(v)) => {
                    // Small integer like tables, for example the
                    // number of brushes factor with keys "<= 2", "3",
                    // "4".
                    let values = match table.get("values") {
                        Some(m) => m,
                        None => return Resolved::Failed,
                    };
                    if *v <= 2.0 {
                        if let Some(x) = values.get("<= 2").and_then(|x| x.as_f64()) {
                            return Resolved::Value(x);
                        }
                    }
                    let key = format!("{}", *v as i64);
                    match values.get(key.as_str()).and_then(|x| x.as_f64()) {
                        Some(x) => Resolved::Value(x),
                        None => Resolved::Failed,
                    }
                }
                None => Resolved::NotProvided,
            }
        }
        "series" => {
            let key = match input_key {
                Some(k) => k,
                None => return Resolved::NotProvided,
            };
            let x = match inputs.get(key).and_then(|i| i.as_num()) {
                Some(x) => x,
                None => return Resolved::NotProvided,
            };
            let axis = match table.get("axis").and_then(|a| a.as_array()) {
                Some(a) => a.iter().filter_map(|v| v.as_f64()).collect::<Vec<f64>>(),
                None => return Resolved::Failed,
            };
            let values = match table.get("values").and_then(|a| a.as_array()) {
                Some(a) => a
                    .iter()
                    .map(|v| if v.is_null() { None } else { v.as_f64() })
                    .collect::<Vec<Option<f64>>>(),
                None => return Resolved::Failed,
            };
            match interp_series(&axis, &values, x) {
                Some(v) => Resolved::Value(v),
                None => Resolved::Failed,
            }
        }
        "columns" => {
            // Column selection: by a named input (load type), or by
            // the column field of the selected style entry.
            let column: Option<String> = if let Some(src) =
                table.get("column_from_input").and_then(|k| k.as_str())
            {
                inputs.get(src).and_then(|i| i.as_named()).map(|s| s.to_string())
            } else if let Some(field) =
                table.get("column_from_style_field").and_then(|k| k.as_str())
            {
                style
                    .and_then(|s| s.get(field))
                    .and_then(|v| v.as_f64())
                    .map(|v| format!("{}", v as i64))
            } else {
                None
            };
            let column = match column {
                Some(c) => c,
                None => return Resolved::NotProvided,
            };
            let key = match input_key {
                Some(k) => k,
                None => return Resolved::NotProvided,
            };
            let x = match inputs.get(key).and_then(|i| i.as_num()) {
                Some(x) => x,
                None => return Resolved::NotProvided,
            };
            let axis = match table.get("axis").and_then(|a| a.as_array()) {
                Some(a) => a.iter().filter_map(|v| v.as_f64()).collect::<Vec<f64>>(),
                None => return Resolved::Failed,
            };
            let col = match table
                .get("columns")
                .and_then(|c| c.get(column.as_str()))
                .and_then(|a| a.as_array())
            {
                Some(a) => a
                    .iter()
                    .map(|v| if v.is_null() { None } else { v.as_f64() })
                    .collect::<Vec<Option<f64>>>(),
                None => return Resolved::Failed,
            };
            match interp_series(&axis, &col, x) {
                Some(v) => Resolved::Value(v),
                None => Resolved::Failed,
            }
        }
        "bands" => {
            let key = match input_key {
                Some(k) => k,
                None => return Resolved::NotProvided,
            };
            let x = match inputs.get(key).and_then(|i| i.as_num()) {
                Some(x) => x,
                None => return Resolved::NotProvided,
            };
            let bands = match table.get("bands").and_then(|b| b.as_array()) {
                Some(b) => b,
                None => return Resolved::Failed,
            };
            for band in bands {
                let above_min = match (
                    band.get("min_exclusive").and_then(|v| v.as_f64()),
                    band.get("min_inclusive").and_then(|v| v.as_f64()),
                ) {
                    (Some(m), _) => x > m,
                    (_, Some(m)) => x >= m,
                    _ => true,
                };
                let below_max = match (
                    band.get("max_exclusive").and_then(|v| v.as_f64()),
                    band.get("max_inclusive").and_then(|v| v.as_f64()),
                ) {
                    (Some(m), _) => x < m,
                    (_, Some(m)) => x <= m,
                    _ => true,
                };
                if above_min && below_max {
                    if let Some(v) = band.get("value").and_then(|v| v.as_f64()) {
                        return Resolved::Value(v);
                    }
                }
            }
            Resolved::Failed
        }
        // Formula factors (for example pi_CYC) are supplied as a
        // numeric input under the factor name, computed with the
        // formula helpers in this module.
        _ => Resolved::NotProvided,
    }
}

/// Compute `lambda_p = lambda_b * (product of pi factors)` for one
/// category, in failures per 10^6 hours. See the module docs for the
/// input conventions. Returns NaN when lambda_b cannot be resolved
/// or when a supplied input lands on a blank handbook entry.
pub fn part_failure_rate(category: &Value, inputs: &Inputs) -> f64 {
    let lb = match lambda_b_for(category, inputs) {
        Some(v) => v,
        None => return f64::NAN,
    };
    let style = inputs
        .get("style")
        .and_then(|i| i.as_named())
        .and_then(|n| style_entry(category, n))
        .or_else(|| {
            inputs
                .get("type")
                .and_then(|i| i.as_named())
                .and_then(|n| style_entry(category, n))
        });
    let mut result = lb;
    let mut handled: Vec<String> = Vec::new();
    if let Some(tables) = category.get("pi_tables").and_then(|t| t.as_object()) {
        let mut names: Vec<&String> = tables.keys().collect();
        names.sort();
        for name in names {
            handled.push(name.clone());
            match resolve_factor(category, name, &tables[name], inputs, style) {
                Resolved::Value(v) => result *= v,
                Resolved::NotProvided => {}
                Resolved::Failed => return f64::NAN,
            }
        }
    }
    // Caller supplied pi factors that are not tables in this
    // category (for example a factor from a formula printed in the
    // handbook) are multiplied in as given.
    for (key, inp) in inputs.iter() {
        if key.starts_with("pi_") && !handled.contains(key) {
            if let Some(v) = inp.as_num() {
                result *= v;
            }
        }
    }
    result
}

/// The Arrhenius temperature factor printed in several sections:
/// `pi_T = exp((-Ea/(8.617e-5)) * (1/(T+273) - 1/298))`, with the
/// activation energy Ea in electron volts and T in degrees C.
pub fn arrhenius_pi_t(ea_ev: f64, t_c: f64) -> f64 {
    ((-ea_ev / 8.617e-5) * (1.0 / (t_c + 273.0) - 1.0 / 298.0)).exp()
}

/// Resistor power factor, Section 9.1: `pi_P = (power dissipation
/// in watts)^0.39`.
pub fn resistor_pi_p(power_w: f64) -> f64 {
    power_w.powf(0.39)
}

/// Resistor power stress factor, Section 9.1: column 1 is
/// `pi_S = 0.71 * exp(1.1 * S)`, column 2 is
/// `pi_S = 0.54 * exp(2.04 * S)`, with S the power stress ratio.
pub fn resistor_pi_s(column: u32, s: f64) -> f64 {
    match column {
        1 => 0.71 * (1.1 * s).exp(),
        _ => 0.54 * (2.04 * s).exp(),
    }
}

/// Capacitor capacitance factor, Section 10.1: column 1 is
/// `pi_C = C^0.09`, column 2 is `pi_C = C^0.23`, with C in
/// microfarads.
pub fn capacitor_pi_c(column: u32, c_uf: f64) -> f64 {
    match column {
        1 => c_uf.powf(0.09),
        _ => c_uf.powf(0.23),
    }
}

/// Capacitor voltage stress factor, Section 10.1, by printed
/// column: 1: `(S/0.6)^5 + 1`, 2: `(S/0.6)^10 + 1`,
/// 3: `(S/0.6)^3 + 1`, 4: `(S/0.6)^17 + 1`, 5: `(S/0.5)^3 + 1`,
/// with S the voltage stress ratio.
pub fn capacitor_pi_v(column: u32, s: f64) -> f64 {
    match column {
        1 => (s / 0.6).powf(5.0) + 1.0,
        2 => (s / 0.6).powf(10.0) + 1.0,
        3 => (s / 0.6).powf(3.0) + 1.0,
        4 => (s / 0.6).powf(17.0) + 1.0,
        _ => (s / 0.5).powf(3.0) + 1.0,
    }
}

/// Load stress factor printed in Sections 13.1 and 14.1:
/// resistive `pi_L = exp((S/0.8)^2)`, inductive
/// `pi_L = exp((S/0.4)^2)`, lamp `pi_L = exp((S/0.2)^2)`, with
/// S = operating load current / rated resistive load current.
pub fn load_stress_pi_l(load_type: &str, s: f64) -> f64 {
    let denom = match load_type {
        "Inductive" => 0.4,
        "Lamp" => 0.2,
        _ => 0.8,
    };
    (s / denom).powi(2).exp()
}

/// Switch contact configuration factor, Section 14.1:
/// `pi_C = (NC)^0.33`, with NC the number of contacts. Applies to
/// toggle and pushbutton switches only.
pub fn switch_pi_c(num_contacts: f64) -> f64 {
    num_contacts.powf(0.33)
}

/// Socket active pins factor, Section 15.2:
/// `pi_P = exp(((N-1)/10)^0.39)`, with N the number of active pins.
pub fn socket_pi_p(active_pins: f64) -> f64 {
    (((active_pins - 1.0) / 10.0).powf(0.39)).exp()
}

/// Plated through hole complexity factor, Section 16.1:
/// `pi_C = 0.65 * P^0.63` for 2 to 18 circuit planes.
pub fn pth_pi_c(circuit_planes: f64) -> f64 {
    0.65 * circuit_planes.powf(0.63)
}

/// Quartz crystal base failure rate, Section 19.1:
/// `lambda_b = 0.013 * f^0.23`, with f in MHz.
pub fn quartz_lambda_b(f_mhz: f64) -> f64 {
    0.013 * f_mhz.powf(0.23)
}

/// Mechanical relay base failure rate formulas, Section 13.1:
/// `lambda_b = 0.0059 * exp((-Ea/(8.617e-5)) * (1/(T+273) - 1/298))`
/// with Ea = 0.19 for 85 C rated relays and Ea = 0.17 for 125 C
/// rated relays, and T the ambient temperature in degrees C.
pub fn relay_lambda_b_formula(rated_temperature_c: f64, t_c: f64) -> f64 {
    let ea = if rated_temperature_c >= 125.0 { 0.17 } else { 0.19 };
    0.0059 * arrhenius_pi_t(ea, t_c)
}

/// Mechanical relay cycling factor, Section 13.1. For MIL-SPEC
/// quality (`mil_spec = true`): cycles per hour divided by 10 at
/// rates of 1.0 per hour or more, and 0.1 below that. For
/// commercial quality: `((cycles per hour)/100)^2` above 1000 per
/// hour, cycles per hour divided by 10 from 10 to 1000 per hour,
/// and 1.0 below 10 per hour.
pub fn relay_pi_cyc(mil_spec: bool, cycles_per_hour: f64) -> f64 {
    if mil_spec {
        if cycles_per_hour >= 1.0 {
            cycles_per_hour / 10.0
        } else {
            0.1
        }
    } else if cycles_per_hour > 1000.0 {
        (cycles_per_hour / 100.0).powi(2)
    } else if cycles_per_hour >= 10.0 {
        cycles_per_hour / 10.0
    } else {
        1.0
    }
}

/// Connector insert temperature rise, Section 15.1:
/// `delta T = coefficient * i^1.85`, with i in amperes per contact
/// and the coefficient set by contact gauge. Returns NaN for a
/// gauge the handbook does not list.
pub fn connector_delta_t(contact_gauge: u32, amperes_per_contact: f64) -> f64 {
    let coeff = match contact_gauge {
        32 => 3.256,
        30 => 2.856,
        28 => 2.286,
        24 => 1.345,
        22 => 0.989,
        20 => 0.640,
        18 => 0.429,
        16 => 0.274,
        12 => 0.100,
        _ => return f64::NAN,
    };
    coeff * amperes_per_contact.powf(1.85)
}

/// Section 16.1 plated through hole interconnection assembly
/// failure rate:
/// `lambda_p = lambda_b * [N1 * pi_C + N2 * (pi_C + 13)] * pi_Q * pi_E`.
/// `technology` is a by-type key of the category, `n1` the quantity
/// of automated (wave, infrared or vapor phase) soldered functional
/// PTHs, `n2` the quantity of hand soldered PTHs, and
/// `circuit_planes` the number of circuit planes (pi_C = 1 for
/// discrete wiring, selected automatically for that technology).
pub fn pth_failure_rate(
    category: &Value,
    technology: &str,
    n1: f64,
    n2: f64,
    circuit_planes: f64,
    quality: &str,
    environment: &str,
) -> f64 {
    let lb = match style_entry(category, technology)
        .and_then(|e| e.get("lambda_b"))
        .and_then(|v| v.as_f64())
    {
        Some(v) => v,
        None => return f64::NAN,
    };
    let pi_c = if technology.starts_with("Discrete") {
        1.0
    } else {
        let table = match category.get("pi_tables").and_then(|t| t.get("pi_C")) {
            Some(t) => t,
            None => return f64::NAN,
        };
        let axis: Vec<f64> = table
            .get("axis")
            .and_then(|a| a.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
            .unwrap_or_default();
        let values: Vec<Option<f64>> = table
            .get("values")
            .and_then(|a| a.as_array())
            .map(|a| {
                a.iter()
                    .map(|v| if v.is_null() { None } else { v.as_f64() })
                    .collect()
            })
            .unwrap_or_default();
        match interp_series(&axis, &values, circuit_planes) {
            Some(v) => v,
            None => return f64::NAN,
        }
    };
    let pi_q = match pi_table_value(category, "pi_Q", quality) {
        Some(v) => v,
        None => return f64::NAN,
    };
    let pi_e = match pi_table_value(category, "pi_E", environment) {
        Some(v) => v,
        None => return f64::NAN,
    };
    lb * (n1 * pi_c + n2 * (pi_c + 13.0)) * pi_q * pi_e
}
