//! Failure reporting, analysis, and corrective action system
//! (FRACAS) log.
//!
//! An incident records a part, its failure mode, the unit operating
//! hours at the incident, a date string as entered by the user, a
//! workflow status, and the corrective action text. Helpers count
//! incidents by status and by failure mode, compute a simple mean
//! time between failures from a total operating hour figure, and
//! read and write the log as CSV.

/// Workflow status of a FRACAS incident.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncidentStatus {
    Open,
    Analysis,
    CorrectiveAction,
    Closed,
}

impl IncidentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            IncidentStatus::Open => "Open",
            IncidentStatus::Analysis => "Analysis",
            IncidentStatus::CorrectiveAction => "CorrectiveAction",
            IncidentStatus::Closed => "Closed",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, String> {
        let key: String = s
            .trim()
            .chars()
            .filter(|c| !matches!(c, ' ' | '_' | '-'))
            .collect::<String>()
            .to_lowercase();
        match key.as_str() {
            "open" => Ok(IncidentStatus::Open),
            "analysis" => Ok(IncidentStatus::Analysis),
            "correctiveaction" => Ok(IncidentStatus::CorrectiveAction),
            "closed" => Ok(IncidentStatus::Closed),
            _ => Err(format!("unknown incident status: {:?}", s)),
        }
    }

    /// All statuses in workflow order.
    pub fn workflow_order() -> [IncidentStatus; 4] {
        [
            IncidentStatus::Open,
            IncidentStatus::Analysis,
            IncidentStatus::CorrectiveAction,
            IncidentStatus::Closed,
        ]
    }
}

/// One FRACAS incident.
///
/// `hours` is the unit operating hours at the incident. `date` is
/// kept as the string the user entered.
#[derive(Debug, Clone, PartialEq)]
pub struct Incident {
    pub id: String,
    pub part: String,
    pub failure_mode: String,
    pub hours: f64,
    pub date: String,
    pub status: IncidentStatus,
    pub corrective_action: String,
}

/// Count incidents by status, in workflow order.
///
/// All four statuses are listed, including statuses with a count
/// of zero.
pub fn count_by_status(incidents: &[Incident]) -> Vec<(String, usize)> {
    IncidentStatus::workflow_order()
        .iter()
        .map(|status| {
            let count = incidents.iter().filter(|i| i.status == *status).count();
            (status.as_str().to_string(), count)
        })
        .collect()
}

/// Count incidents by failure mode.
///
/// Sorted by count descending, then by mode name ascending so the
/// order is stable for equal counts.
pub fn count_by_mode(incidents: &[Incident]) -> Vec<(String, usize)> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for incident in incidents {
        if let Some(entry) = counts
            .iter_mut()
            .find(|(name, _)| *name == incident.failure_mode)
        {
            entry.1 += 1;
        } else {
            counts.push((incident.failure_mode.clone(), 1));
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    counts
}

/// Mean time between failures in hours.
///
/// This is the total operating hours divided by the number of
/// incidents. Returns `None` when there are no incidents.
pub fn mtbf_hours(total_operating_hours: f64, incidents: &[Incident]) -> Option<f64> {
    if incidents.is_empty() {
        None
    } else {
        Some(total_operating_hours / incidents.len() as f64)
    }
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

/// Serialize incidents to CSV text with a header line.
///
/// Columns are: id, part, failure_mode, hours, date, status,
/// corrective_action.
pub fn incidents_to_csv(incidents: &[Incident]) -> String {
    let mut out = String::new();
    out.push_str("id,part,failure_mode,hours,date,status,corrective_action\n");
    for incident in incidents {
        let fields: Vec<String> = vec![
            escape_csv_field(&incident.id),
            escape_csv_field(&incident.part),
            escape_csv_field(&incident.failure_mode),
            incident.hours.to_string(),
            escape_csv_field(&incident.date),
            incident.status.as_str().to_string(),
            escape_csv_field(&incident.corrective_action),
        ];
        out.push_str(&fields.join(","));
        out.push('\n');
    }
    out
}

/// Parse incidents from CSV text in the format produced by
/// `incidents_to_csv`.
///
/// The first line is treated as a header and skipped.
pub fn incidents_from_csv(text: &str) -> Result<Vec<Incident>, String> {
    let records = parse_csv_records(text)?;
    let mut incidents: Vec<Incident> = Vec::new();

    for (index, record) in records.iter().enumerate() {
        if record.len() == 1 && record[0].trim().is_empty() {
            continue;
        }
        if index == 0 {
            continue;
        }
        if record.len() != 7 {
            return Err(format!(
                "expected 7 fields on CSV record {}, found {}",
                index + 1,
                record.len()
            ));
        }
        let hours = record[3]
            .trim()
            .parse::<f64>()
            .map_err(|_| format!("malformed numeric field for hours: {:?}", record[3]))?;
        let status = IncidentStatus::from_str(&record[5])?;
        incidents.push(Incident {
            id: record[0].clone(),
            part: record[1].clone(),
            failure_mode: record[2].clone(),
            hours,
            date: record[4].clone(),
            status,
            corrective_action: record[6].clone(),
        });
    }

    Ok(incidents)
}
