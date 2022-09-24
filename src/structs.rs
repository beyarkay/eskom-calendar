use regex::Regex;
use std::fmt::Debug;

use chrono::{DateTime, FixedOffset, NaiveTime};
use serde::{Deserialize, Serialize};

/// A multitude of load shedding for a particular suburb
pub struct ManuallyInputSchedule {
    /// LoadShedding changes, usually in the future (but not always)
    pub changes: Vec<Shedding>,
    /// LoadShedding changes, always in the past
    pub historical_changes: Vec<Shedding>,
}

/// A multitude of load shedding for a particular suburb
#[derive(Serialize, Deserialize)]
pub struct RawManuallyInputSchedule {
    /// LoadShedding changes, usually in the future (but not always)
    changes: Vec<RawShedding>,
    /// LoadShedding changes, always in the past
    historical_changes: Vec<RawShedding>,
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

/// A single duration of loadshedding that only has one stage.
#[derive(Debug)]
pub struct Shedding {
    /// The time when LoadShedding *should* start
    pub start: DateTime<FixedOffset>,
    /// The time when LoadShedding *should* end
    pub finsh: DateTime<FixedOffset>,
    /// The stage of loadshedding
    pub stage: u8,
    /// The source of information for this loadshedding event
    pub source: String,
    /// Optionally specify a rust-regex pattern which the area name must match in order for this
    /// shedding to be applied to it. For example, `include_regex: city-of-cape-town-area-\d{1,2}`
    /// will include all city of cape town areas. If `include_regex` and `exclude_regex` conflict
    /// with each other, the area *will* be included. If no include/exclude are specified,
    /// `include_regex: .*` is used by default (so the loadshedding is applied to all areas.
    pub include_regex: Regex,
    /// Optionally specify a rust-regex pattern which the area name must *not* match in order for this
    /// shedding to be applied to it. For example, `exclude_regex: city-of-cape-town-area-\d{1,2}`
    /// will exclude all city of cape town areas. If `include_regex` and `exclude_regex` conflict
    /// with each other, the area *will* be included. If no include/exclude are specified,
    /// `include_regex: .*` is used by default (so the loadshedding is applied to all areas.
    pub exclude_regex: Regex,
}

/// A single duration of loadshedding that only has one stage.
#[derive(Serialize, Deserialize, Debug)]
pub struct RawShedding {
    /// The time when LoadShedding *should* start
    start: String,
    /// The time when LoadShedding *should* end. Note that `finsh` is spelt without the second `i`,
    /// so that it lines up with `start`.
    finsh: String,
    /// The stage of loadshedding
    stage: u8,
    /// The source of information for this loadshedding event
    source: String,
    /// Optionally specify a rust-regex pattern which the area name must match in order for this
    /// shedding to be applied to it. For example, `include_regex: city-of-cape-town-area-\d{1,2}`
    /// will include all city of cape town areas. If `include_regex` and `exclude_regex` conflict
    /// with each other, the area *will* be included. If no include/exclude are specified,
    /// `include_regex: .*` is used by default (so the loadshedding is applied to all areas.
    include_regex: Option<String>,
    /// Optionally specify a rust-regex pattern which the area name must *not* match in order for this
    /// shedding to be applied to it. For example, `exclude_regex: city-of-cape-town-area-\d{1,2}`
    /// will exclude all city of cape town areas. If `include_regex` and `exclude_regex` conflict
    /// with each other, the area *will* be included. If no include/exclude are specified,
    /// `include_regex: .*` is used by default (so the loadshedding is applied to all areas.
    exclude_regex: Option<String>,
    /// A shorthand so you don't have to specify the full regex. `include: coct` is equivalent to
    /// `include_regex: city-of-cape-town-area-\d{1,2}`. If no include/exclude are specified,
    /// `include_regex: .*` is used by default (so the loadshedding is applied to all areas.
    include: Option<String>,
    /// A shorthand so you don't have to specify the full regex. `exclude: coct` is equivalent to
    /// `exclude_regex: city-of-cape-town-area-\d{1,2}`. If no include/exclude are specified,
    /// `include_regex: .*` is used by default (so the loadshedding is applied to all areas.
    exclude: Option<String>,
}

impl From<RawShedding> for Shedding {
    fn from(raw: RawShedding) -> Self {
        let shorthand_to_regex = |shorthand: String, default: String| {
            match shorthand.to_lowercase().as_str() {
            "citypower" | "cp" => r"city-power-\d{1,2}".to_string(),
            "capetown" | "cpt" | "coct" => r"city-of-cape-town-area-\d{1,2}".to_string(),
            "ekurhuleni" => r"gauteng-ekurhuleni-block-\d{1,2}".to_string(),
            "eskom" => r"^(eastern-cape-)|(free-state-)|(kwazulu-natal-)|(limpopo-)|(mpumalanga-)|(north-west-)|(northern-cape-)|(western-cape-)".to_string(),
            "tshwane" => r"gauteng-tshwane-group-\d{1,2}".to_string(),
            _ => default,
        }
        };
        // This will first try to use the explicit regex. If there is no explicit regex, then try
        // to parse the shorthand. If there is no shorthand or if the shorthand is unknown, then
        // use the explicit ".*" match everything regex
        let include_str = raw.include_regex.clone().unwrap_or(
            raw.include.clone().map_or(r".*".to_string(), |shorthand| {
                shorthand_to_regex(shorthand, r".*".to_string())
            }),
        );
        let include_regex = Regex::new(&include_str).unwrap_or(Regex::new(r".*").unwrap());

        let exclude_str = raw.exclude_regex.clone().unwrap_or(
            raw.exclude
                .clone()
                .map_or(r"matchnothing^".to_string(), |shorthand| {
                    shorthand_to_regex(shorthand, r"matchnothing^".to_string())
                }),
        );
        let exclude_regex = Regex::new(&exclude_str).unwrap_or(Regex::new(r".*").unwrap());

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
            exclude_regex,
            include_regex,
        }
    }
}

impl PartialEq for Shedding {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start
            && self.finsh == other.finsh
            && self.stage == other.stage
            && self.source == other.source
            && self.include_regex.clone().as_str() == other.include_regex.clone().as_str()
            && self.exclude_regex.clone().as_str() == other.exclude_regex.clone().as_str()
    }
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
    /// true iff finish time < start time
    pub goes_over_midnight: bool,
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

impl From<RawMonthlyShedding> for MonthlyShedding {
    fn from(raw: RawMonthlyShedding) -> Self {
        let start = NaiveTime::parse_from_str(&raw.start_time, "%H:%M").unwrap();
        let finsh = NaiveTime::parse_from_str(&raw.finsh_time, "%H:%M").unwrap();
        let goes_over_midnight = finsh < start;

        let date = if goes_over_midnight { "01" } else { "02" };

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
            goes_over_midnight,
        }
    }
}

#[cfg(test)]
mod tests {
    mod raw_shedding_to_shedding {
        use crate::structs::{RawShedding, Shedding};
        use chrono::DateTime;
        use regex::Regex;

        fn _get_shedding() -> Shedding {
            Shedding {
                start: DateTime::parse_from_rfc3339("2022-01-01T08:00:00+02:00").unwrap(),
                finsh: DateTime::parse_from_rfc3339("2022-01-02T08:00:00+02:00").unwrap(),
                stage: 1,
                source: "Test source".to_string(),
                include_regex: Regex::new(".*").unwrap(),
                exclude_regex: Regex::new(".*").unwrap(),
            }
        }
        fn _get_raw_shedding() -> RawShedding {
            RawShedding {
                start: "2022-01-01T08:00:00".to_string(),
                finsh: "2022-01-02T08:00:00".to_string(),
                stage: 1,
                source: "Test source".to_string(),
                include_regex: None,
                exclude_regex: None,
                include: None,
                exclude: None,
            }
        }
    }
}
