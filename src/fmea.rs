//! Failure modes and effects analysis (FMEA) worksheet.
//!
//! Rows carry severity, occurrence, and detection scores, an optional
//! set of revised scores after a recommended action, and helpers for
//! risk priority numbers (RPN) and CSV import and export.

/// One row of an FMEA worksheet.
///
/// Scores are plain integers as entered by the user. Revised scores
/// are optional and, when present, override the original scores for
/// the revised RPN calculation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FmeaRow {
    pub item: String,
    pub function: String,
    pub failure_mode: String,
    pub effect: String,
    pub cause: String,
    pub severity: u32,
    pub occurrence: u32,
    pub detection: u32,
    pub recommended_action: String,
    pub revised_severity: Option<u32>,
    pub revised_occurrence: Option<u32>,
    pub revised_detection: Option<u32>,
}

impl FmeaRow {
    /// Create a row with the item, failure mode, and the three scores.
    ///
    /// All other text fields start empty and all revised scores start
    /// as `None`.
    pub fn new(item: &str, failure_mode: &str, severity: u32, occurrence: u32, detection: u32) -> Self {
        FmeaRow {
            item: item.to_string(),
            function: String::new(),
            failure_mode: failure_mode.to_string(),
            effect: String::new(),
            cause: String::new(),
            severity,
            occurrence,
            detection,
            recommended_action: String::new(),
            revised_severity: None,
            revised_occurrence: None,
            revised_detection: None,
        }
    }

    /// Risk priority number: severity times occurrence times detection.
    pub fn rpn(&self) -> u32 {
        self.severity * self.occurrence * self.detection
    }

    /// Revised risk priority number.
    ///
    /// Each revised score falls back to the original score when it is
    /// not set. Returns `None` when all three revised scores are `None`.
    pub fn revised_rpn(&self) -> Option<u32> {
        if self.revised_severity.is_none()
            && self.revised_occurrence.is_none()
            && self.revised_detection.is_none()
        {
            return None;
        }
        let severity = self.revised_severity.unwrap_or(self.severity);
        let occurrence = self.revised_occurrence.unwrap_or(self.occurrence);
        let detection = self.revised_detection.unwrap_or(self.detection);
        Some(severity * occurrence * detection)
    }
}

/// Sort rows in place by descending RPN.
pub fn sort_by_rpn_desc(rows: &mut Vec<FmeaRow>) {
    rows.sort_by(|a, b| b.rpn().cmp(&a.rpn()));
}

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

/// Format an optional score for CSV output. `None` becomes an empty field.
fn optional_to_string(value: Option<u32>) -> String {
    match value {
        Some(v) => v.to_string(),
        None => String::new(),
    }
}

/// Serialize rows to CSV text with a header line.
///
/// Columns are: item, function, failure_mode, effect, cause, severity,
/// occurrence, detection, rpn, recommended_action, revised_severity,
/// revised_occurrence, revised_detection, revised_rpn.
pub fn rows_to_csv(rows: &[FmeaRow]) -> String {
    let mut out = String::new();
    out.push_str("item,function,failure_mode,effect,cause,severity,occurrence,detection,rpn,recommended_action,revised_severity,revised_occurrence,revised_detection,revised_rpn\n");
    for row in rows {
        let fields: Vec<String> = vec![
            escape_csv_field(&row.item),
            escape_csv_field(&row.function),
            escape_csv_field(&row.failure_mode),
            escape_csv_field(&row.effect),
            escape_csv_field(&row.cause),
            row.severity.to_string(),
            row.occurrence.to_string(),
            row.detection.to_string(),
            row.rpn().to_string(),
            escape_csv_field(&row.recommended_action),
            optional_to_string(row.revised_severity),
            optional_to_string(row.revised_occurrence),
            optional_to_string(row.revised_detection),
            optional_to_string(row.revised_rpn()),
        ];
        out.push_str(&fields.join(","));
        out.push('\n');
    }
    out
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

    // Flush a trailing record that has no final newline.
    if field_started || !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }

    Ok(records)
}

/// Parse a required unsigned integer field.
fn parse_u32_field(value: &str, column: &str) -> Result<u32, String> {
    value
        .trim()
        .parse::<u32>()
        .map_err(|_| format!("malformed numeric field for {}: {:?}", column, value))
}

/// Parse an optional unsigned integer field. Empty means `None`.
fn parse_optional_u32_field(value: &str, column: &str) -> Result<Option<u32>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        trimmed
            .parse::<u32>()
            .map(Some)
            .map_err(|_| format!("malformed numeric field for {}: {:?}", column, value))
    }
}

/// Parse rows from CSV text in the format produced by `rows_to_csv`.
///
/// The first line is treated as a header and skipped. The derived rpn
/// and revised_rpn columns are read for format compatibility but their
/// values are recomputed from the score fields. Malformed numeric
/// fields produce an error.
pub fn rows_from_csv(text: &str) -> Result<Vec<FmeaRow>, String> {
    let records = parse_csv_records(text)?;
    let mut rows: Vec<FmeaRow> = Vec::new();

    for (index, record) in records.iter().enumerate() {
        // Skip blank lines.
        if record.len() == 1 && record[0].trim().is_empty() {
            continue;
        }
        // Skip the header line.
        if index == 0 {
            continue;
        }
        if record.len() != 14 {
            return Err(format!(
                "expected 14 fields on CSV record {}, found {}",
                index + 1,
                record.len()
            ));
        }

        let severity = parse_u32_field(&record[5], "severity")?;
        let occurrence = parse_u32_field(&record[6], "occurrence")?;
        let detection = parse_u32_field(&record[7], "detection")?;

        // The rpn column is derived, but a non-empty value must still
        // be numeric so malformed files are reported.
        if !record[8].trim().is_empty() {
            parse_u32_field(&record[8], "rpn")?;
        }

        let revised_severity = parse_optional_u32_field(&record[10], "revised_severity")?;
        let revised_occurrence = parse_optional_u32_field(&record[11], "revised_occurrence")?;
        let revised_detection = parse_optional_u32_field(&record[12], "revised_detection")?;

        if !record[13].trim().is_empty() {
            parse_u32_field(&record[13], "revised_rpn")?;
        }

        rows.push(FmeaRow {
            item: record[0].clone(),
            function: record[1].clone(),
            failure_mode: record[2].clone(),
            effect: record[3].clone(),
            cause: record[4].clone(),
            severity,
            occurrence,
            detection,
            recommended_action: record[9].clone(),
            revised_severity,
            revised_occurrence,
            revised_detection,
        });
    }

    Ok(rows)
}

// ---------------- worksheet type and FMECA helpers ----------------

/// Kind of FMEA worksheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FmeaType {
    Design,
    Process,
    System,
}

impl FmeaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FmeaType::Design => "Design",
            FmeaType::Process => "Process",
            FmeaType::System => "System",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.trim().to_lowercase().as_str() {
            "design" => Ok(FmeaType::Design),
            "process" => Ok(FmeaType::Process),
            "system" => Ok(FmeaType::System),
            _ => Err(format!("unknown FMEA type: {:?}", s)),
        }
    }
}

/// An FMEA worksheet: a type plus its rows.
///
/// The CSV form writes a first line `# fmea_type: Design` (with the
/// worksheet type) followed by the rows CSV produced by
/// `rows_to_csv`. Reading accepts that form and also accepts plain
/// rows CSV with no type line, which defaults to Design.
#[derive(Debug, Clone)]
pub struct FmeaWorksheet {
    pub fmea_type: FmeaType,
    pub rows: Vec<FmeaRow>,
}

impl FmeaWorksheet {
    /// Create an empty worksheet of the given type.
    pub fn new(fmea_type: FmeaType) -> Self {
        FmeaWorksheet {
            fmea_type,
            rows: Vec::new(),
        }
    }

    /// Serialize the worksheet to CSV text.
    ///
    /// The first line is `# fmea_type: <type>`, followed by the
    /// rows CSV from `rows_to_csv`.
    pub fn to_csv(&self) -> String {
        format!("# fmea_type: {}\n{}", self.fmea_type.as_str(), rows_to_csv(&self.rows))
    }

    /// Parse a worksheet from CSV text.
    ///
    /// An optional leading `# fmea_type: <type>` line sets the
    /// worksheet type. When the line is absent the type defaults
    /// to Design and the text is read as plain rows CSV.
    pub fn from_csv(text: &str) -> Result<Self, String> {
        let mut fmea_type = FmeaType::Design;
        let mut body: &str = text;

        let first_end = text.find('\n').unwrap_or(text.len());
        let first_line = text[..first_end].trim_end_matches('\r').trim();
        if let Some(rest) = first_line
            .strip_prefix("# fmea_type:")
            .or_else(|| first_line.strip_prefix("#fmea_type:"))
        {
            fmea_type = FmeaType::from_str(rest.trim())?;
            body = if first_end < text.len() {
                &text[first_end + 1..]
            } else {
                ""
            };
        }

        let rows = rows_from_csv(body)?;
        Ok(FmeaWorksheet { fmea_type, rows })
    }
}

/// FMECA mode criticality number: alpha * beta * lambda * t.
///
/// `alpha` is the failure mode ratio, `beta` is the probability
/// that the mode causes the stated effect, `lambda_per_hour` is
/// the item failure rate per hour, and `mission_hours` is the
/// mission time in hours.
pub fn criticality_number(
    alpha: f64,
    beta: f64,
    lambda_per_hour: f64,
    mission_hours: f64,
) -> f64 {
    alpha * beta * lambda_per_hour * mission_hours
}

/// Build a control plan CSV from FMEA rows.
///
/// The header is item, failure_mode, control_method, source_rpn.
/// One line is written for each row whose recommended action is
/// non-empty, with the recommended action as the control method
/// and the row RPN as the source RPN.
pub fn control_plan_csv(rows: &[FmeaRow]) -> String {
    let mut out = String::new();
    out.push_str("item,failure_mode,control_method,source_rpn\n");
    for row in rows {
        if row.recommended_action.trim().is_empty() {
            continue;
        }
        let fields: Vec<String> = vec![
            escape_csv_field(&row.item),
            escape_csv_field(&row.failure_mode),
            escape_csv_field(&row.recommended_action),
            row.rpn().to_string(),
        ];
        out.push_str(&fields.join(","));
        out.push('\n');
    }
    out
}

// ---------------- DVP&R ----------------

/// One entry in a design verification plan and report (DVP&R).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DvprEntry {
    pub id: String,
    pub failure_mode_ref: String,
    pub test: String,
    pub sample_size: u32,
    pub acceptance_criteria: String,
}

/// Serialize DVP&R entries to CSV text with a header line.
///
/// Columns are: id, failure_mode_ref, test, sample_size,
/// acceptance_criteria.
pub fn dvpr_to_csv(entries: &[DvprEntry]) -> String {
    let mut out = String::new();
    out.push_str("id,failure_mode_ref,test,sample_size,acceptance_criteria\n");
    for entry in entries {
        let fields: Vec<String> = vec![
            escape_csv_field(&entry.id),
            escape_csv_field(&entry.failure_mode_ref),
            escape_csv_field(&entry.test),
            entry.sample_size.to_string(),
            escape_csv_field(&entry.acceptance_criteria),
        ];
        out.push_str(&fields.join(","));
        out.push('\n');
    }
    out
}

/// Parse DVP&R entries from CSV text in the format produced by
/// `dvpr_to_csv`.
///
/// The first line is treated as a header and skipped.
pub fn dvpr_from_csv(text: &str) -> Result<Vec<DvprEntry>, String> {
    let records = parse_csv_records(text)?;
    let mut entries: Vec<DvprEntry> = Vec::new();

    for (index, record) in records.iter().enumerate() {
        if record.len() == 1 && record[0].trim().is_empty() {
            continue;
        }
        if index == 0 {
            continue;
        }
        if record.len() != 5 {
            return Err(format!(
                "expected 5 fields on CSV record {}, found {}",
                index + 1,
                record.len()
            ));
        }
        let sample_size = parse_u32_field(&record[3], "sample_size")?;
        entries.push(DvprEntry {
            id: record[0].clone(),
            failure_mode_ref: record[1].clone(),
            test: record[2].clone(),
            sample_size,
            acceptance_criteria: record[4].clone(),
        });
    }

    Ok(entries)
}
