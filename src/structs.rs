use std::fmt::Debug;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
#[allow(dead_code)]
pub enum Province {
    EasternCape,
    FreeState,
    Gauteng,
    KwaZuluNatal,
    Limpopo,
    Mpumalanga,
    NorthWest,
    NorthernCape,
    WesternCape,
}

/// Looks something like:
/// {
///     "Disabled":false,
///     "Group":null,
///     "Selected":false,
///     "Text":"Beaufort West",
///     "Value":"336"
/// }
#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct RawMunicipalityInfo {
    /// The usage of this is unknown
    Disabled: bool,
    /// The usage of this is unknown, it's often just `null`
    Group: Option<String>,
    /// The usage of this is unknown
    Selected: bool,
    /// The name of this municipality
    Text: String,
    /// The ID of this municipality
    Value: String,
}

/// The information about a municipality.
#[derive(Debug)]
pub struct MunicipalityInfo {
    /// The usage of this is unknown
    _is_disabled: bool,
    // group: Option<String>,
    /// The usage of this is unknown
    _is_selected: bool,
    /// The name of this municipality
    pub name: String,
    /// The ID of this municipality
    pub id: u32,
}

impl From<RawMunicipalityInfo> for MunicipalityInfo {
    fn from(raw: RawMunicipalityInfo) -> Self {
        MunicipalityInfo {
            _is_disabled: raw.Disabled,
            // group: raw.Group,
            _is_selected: raw.Selected,
            name: raw.Text,
            id: raw.Value.parse().expect("Couldn't parse raw.Value"),
        }
    }
}

/// A struct representing the results of a `GetSurburbData`
/// request. Contains the results and the total number of entries
#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct GetSuburbDataResult {
    pub Results: Vec<RawSuburbInfo>,
    pub Total: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct RawSuburbInfo {
    id: String,
    text: String,
    Tot: u32,
}

/// Information about an eskom suburb
#[derive(Debug)]
pub struct SuburbInfo {
    /// The ID number
    pub id: u32,
    /// The name of the suburb
    pub name: String,
    /// Not sure what this is
    _total: u32,
    /// If true, then this suburb will have a schedule associated with it
    pub has_schedule: bool,
}
impl From<RawSuburbInfo> for SuburbInfo {
    fn from(raw: RawSuburbInfo) -> Self {
        SuburbInfo {
            id: raw.id.parse().expect("Couldn't parse suburb id to u32"),
            name: raw.text,
            _total: raw.Tot,
            has_schedule: raw.Tot > 0,
        }
    }
}

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
#[derive(Serialize, Deserialize)]
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
                    "Failed to parse time 1970-01-01T{}:00+02:00 as RFC3339",
                    raw.start
                )
                .as_str(),
            ),
            finsh: DateTime::parse_from_rfc3339(&format!("{}+02:00", raw.finsh)).expect(
                format!(
                    "Failed to parse time 1970-01-01T{}:00+02:00 as RFC3339",
                    raw.finsh
                )
                .as_str(),
            ),
            stage: raw.stage,
            source: raw.source,
        }
    }
}

/// A loadshedding event that repeats on the same day every month
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
/// A loadshedding event that repeats on the same day every month
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
                    "Failed to parse time 1970-01-01T{}:00+02:00 as RFC3339",
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
                    "Failed to parse time 1970-01-01T{}:00+02:00 as RFC3339",
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
