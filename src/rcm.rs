//! Reliability centered maintenance (RCM) worksheet.
//!
//! Each failure mode is classified by consequence and assigned a
//! maintenance task by a fixed decision table. The table is applied
//! in this precedence order:
//!
//! 1. If the mode is condition monitorable and its consequence is
//!    not Hidden, the task is ConditionBased.
//! 2. If the consequence is Hidden, the task is FailureFinding.
//! 3. If the failure is age related, the task is
//!    ScheduledRestoration when the item is restorable and
//!    ScheduledDiscard otherwise.
//! 4. Otherwise the task is RunToFailure.
//!
//! The module also carries the FMECA style mode criticality number
//! Cm = alpha * beta * lambda * t, where alpha is the failure mode
//! ratio, beta is the probability that the mode causes the stated
//! effect, lambda is the item failure rate per hour, and t is the
//! mission time in hours.

/// Consequence class of a failure mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consequence {
    Hidden,
    Safety,
    Environmental,
    Operational,
    NonOperational,
}

impl Consequence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Consequence::Hidden => "Hidden",
            Consequence::Safety => "Safety",
            Consequence::Environmental => "Environmental",
            Consequence::Operational => "Operational",
            Consequence::NonOperational => "NonOperational",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        let key: String = s
            .trim()
            .chars()
            .filter(|c| !matches!(c, ' ' | '_' | '-'))
            .collect::<String>()
            .to_lowercase();
        match key.as_str() {
            "hidden" => Ok(Consequence::Hidden),
            "safety" => Ok(Consequence::Safety),
            "environmental" => Ok(Consequence::Environmental),
            "operational" => Ok(Consequence::Operational),
            "nonoperational" => Ok(Consequence::NonOperational),
            _ => Err(format!("unknown consequence: {:?}", s)),
        }
    }
}

/// Maintenance task selected for a failure mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcmTask {
    ConditionBased,
    ScheduledRestoration,
    ScheduledDiscard,
    FailureFinding,
    RunToFailure,
}

impl RcmTask {
    pub fn as_str(&self) -> &'static str {
        match self {
            RcmTask::ConditionBased => "ConditionBased",
            RcmTask::ScheduledRestoration => "ScheduledRestoration",
            RcmTask::ScheduledDiscard => "ScheduledDiscard",
            RcmTask::FailureFinding => "FailureFinding",
            RcmTask::RunToFailure => "RunToFailure",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        let key: String = s
            .trim()
            .chars()
            .filter(|c| !matches!(c, ' ' | '_' | '-'))
            .collect::<String>()
            .to_lowercase();
        match key.as_str() {
            "conditionbased" => Ok(RcmTask::ConditionBased),
            "scheduledrestoration" => Ok(RcmTask::ScheduledRestoration),
            "scheduleddiscard" => Ok(RcmTask::ScheduledDiscard),
            "failurefinding" => Ok(RcmTask::FailureFinding),
            "runtofailure" => Ok(RcmTask::RunToFailure),
            _ => Err(format!("unknown RCM task: {:?}", s)),
        }
    }
}

/// One failure mode on an RCM worksheet.
///
/// The four optional fields feed the criticality calculation: the
/// item failure rate per hour, the failure mode ratio alpha, the
/// effect probability beta, and the mission time in hours.
#[derive(Debug, Clone)]
pub struct RcmMode {
    pub asset: String,
    pub function: String,
    pub failure_mode: String,
    pub consequence: Consequence,
    pub condition_monitorable: bool,
    pub age_related: bool,
    pub restorable: bool,
    pub failure_rate_per_hour: Option<f64>,
    pub mode_ratio_alpha: Option<f64>,
    pub effect_probability_beta: Option<f64>,
    pub mission_hours: Option<f64>,
}

impl RcmMode {
    /// Mode criticality number for this mode.
    ///
    /// Returns `None` unless the failure rate, alpha, beta, and
    /// mission hours are all present.
    pub fn criticality(&self) -> Option<f64> {
        match (
            self.mode_ratio_alpha,
            self.effect_probability_beta,
            self.failure_rate_per_hour,
            self.mission_hours,
        ) {
            (Some(alpha), Some(beta), Some(lambda), Some(t)) => {
                Some(criticality_number(alpha, beta, lambda, t))
            }
            _ => None,
        }
    }
}

/// Select the maintenance task for a failure mode.
///
/// Applies the decision table in the module docs in precedence
/// order: condition based work first for monitorable non-hidden
/// modes, failure finding for hidden modes, scheduled restoration
/// or discard for age related modes, and run to failure otherwise.
pub fn select_task(m: &RcmMode) -> RcmTask {
    if m.condition_monitorable && !matches!(m.consequence, Consequence::Hidden) {
        return RcmTask::ConditionBased;
    }
    if matches!(m.consequence, Consequence::Hidden) {
        return RcmTask::FailureFinding;
    }
    if m.age_related {
        return if m.restorable {
            RcmTask::ScheduledRestoration
        } else {
            RcmTask::ScheduledDiscard
        };
    }
    RcmTask::RunToFailure
}

/// Mode criticality number: alpha * beta * lambda * t.
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
