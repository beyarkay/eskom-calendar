use std::fmt::Debug;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

/// A multitude of load shedding for a particular suburb
pub struct ManuallyInputSchedule {
    /// LoadShedding changes, usually in the future (but not always)
    pub changes: Vec<Shedding>,
    /// LoadShedding changes, always in the past
    pub historical_changes: Vec<Shedding>,
}

/// A single duration of loadshedding that only has one stage.
pub struct Shedding {
    /// The time when LoadShedding *should* start
    pub start: DateTime<FixedOffset>,
    /// The time when LoadShedding *should* end
    pub finsh: DateTime<FixedOffset>,
    /// The stage of loadshedding
    pub stage: u8,
    /// The source of information for this loadshedding event
    pub source: String,
}

/// A multitude of load shedding for a particular suburb
#[derive(Serialize, Deserialize)]
pub struct RawManuallyInputSchedule {
    /// LoadShedding changes, usually in the future (but not always)
    changes: Vec<RawShedding>,
    /// LoadShedding changes, always in the past
    historical_changes: Vec<RawShedding>,
}

/// A single duration of loadshedding that only has one stage.
#[derive(Serialize, Deserialize, Debug)]
pub struct RawShedding {
    /// The time when LoadShedding *should* start
    start: String,
    /// The time when LoadShedding *should* end
    finsh: String,
    /// The stage of loadshedding
    stage: u8,
    /// The source of information for this loadshedding event
    source: String,
}

impl From<RawManuallyInputSchedule> for ManuallyInputSchedule {
    fn from(raw: RawManuallyInputSchedule) -> Self {
        ManuallyInputSchedule {
            changes: raw.changes.into_iter().map(|r| r.into()).collect(),
            historical_changes: raw
                .historical_changes
                .into_iter()
                .map(|r| r.into())
                .collect(),
        }
    }
}
impl From<RawShedding> for Shedding {
    fn from(raw: RawShedding) -> Self {
        Shedding {
            start: DateTime::parse_from_rfc3339(&format!("{}+02:00", raw.start)).expect(
                format!(
                    "Failed to parse start time 1970-01-01T{}:00+02:00 as RFC3339, {raw:?}",
                    raw.start
                )
                .as_str(),
            ),
            finsh: DateTime::parse_from_rfc3339(&format!("{}+02:00", raw.finsh)).expect(
                format!(
                    "Failed to parse finsh time 1970-01-01T{}:00+02:00 as RFC3339, {raw:?}",
                    raw.finsh
                )
                .as_str(),
            ),
            stage: raw.stage,
            source: raw.source,
        }
    }
}

/// A loadshedding event that repeats on the same day every month, not yet parsed. See
/// MonthlyShedding.
#[derive(Deserialize, Debug)]
pub struct RawMonthlyShedding {
    /// The time when LoadShedding *should* start.
    start_time: String,
    /// The time when LoadShedding *should* finish (note the spelling).
    finsh_time: String,
    /// The stage of loadshedding.
    stage: u8,
    /// The date of the month which this event occurs on
    date_of_month: u8,
}

/// A loadshedding event that repeats on the same day every month, parsed into datetimes.
/// Contains the date of the month, the start time (but the date is always 1 Jan 1970), the end
/// time (but the date is always 1 Jan 1970 or 2 Jan 1970), a boolean to indicate if the start time and the end
/// time imply the loadshedding goes over midnight (ie from 22:00 to 00:30) and the stage of the
/// loadshedding.
#[derive(Debug)]
pub struct MonthlyShedding {
    /// The time when LoadShedding *should* start. The date of this member will always be 1 Jan
    /// 1970.
    pub start_time: DateTime<FixedOffset>,
    /// The time when LoadShedding *should* finish (note the spelling). The date of this member
    /// will always be 1 Jan 1970, unless the loadshedding is from 22h00 to 00h30 in which case the
    /// finish date will be 2 Jan 1970.
    pub finsh_time: DateTime<FixedOffset>,
    /// The stage of loadshedding.
    pub stage: u8,
    /// The date of the month which this event occurs on
    pub date_of_month: u8,
    /// true iff start time is 22:00 and finsh time is 00:30
    pub goes_over_midnight: bool,
}

impl From<RawMonthlyShedding> for MonthlyShedding {
    fn from(raw: RawMonthlyShedding) -> Self {
        let date = if raw.start_time == "22:00" && raw.finsh_time == "00:30" {
            "01"
        } else {
            "02"
        };
        MonthlyShedding {
            start_time: DateTime::parse_from_rfc3339(&format!(
                "1970-01-01T{}:00+02:00",
                raw.start_time
            ))
            .expect(
                format!(
                    "Failed to parse start time 1970-01-01T{}:00+02:00 as RFC3339, {raw:?}",
                    raw.start_time
                )
                .as_str(),
            ),
            finsh_time: DateTime::parse_from_rfc3339(&format!(
                "1970-01-{date}T{}:00+02:00",
                raw.finsh_time
            ))
            .expect(
                format!(
                    "Failed to parse start time 1970-01-01T{}:00+02:00 as RFC3339, {raw:?}",
                    raw.finsh_time
                )
                .as_str(),
            ),
            stage: raw.stage,
            date_of_month: raw.date_of_month,
            goes_over_midnight: raw.start_time == "22:00" && raw.finsh_time == "00:30",
        }
    }
}
