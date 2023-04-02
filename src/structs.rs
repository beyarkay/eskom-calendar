use clap::Parser;
use regex::Regex;
use std::fmt::{Debug, Display};

use chrono::{DateTime, FixedOffset, NaiveTime};
use serde::{Deserialize, Serialize};

/// Represents a duration of time for which the power will be out for a particular area.
///
/// Requires specifying where the information came from (in `source`) as well as the stage of
/// loadshedding.
#[derive(PartialEq, Eq)]
pub struct PowerOutage {
    pub area_name: String,
    pub stage: u8,
    pub start: DateTime<FixedOffset>,
    pub finsh: DateTime<FixedOffset>,
    pub source: String,
}

impl PowerOutage {
    pub fn csv_header() -> String {
        "area_name,start,finsh,stage,source".to_owned()
    }
}

impl PartialOrd for PowerOutage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.area_name.partial_cmp(&other.area_name) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.stage.partial_cmp(&other.stage) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.start.partial_cmp(&other.start) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.finsh.partial_cmp(&other.finsh) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.source.partial_cmp(&other.source)
    }
}

impl Ord for PowerOutage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl Display for PowerOutage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{area_name},{start:?},{finsh:?},{stage},{source:?}",
            area_name = self.area_name,
            start = self.start,
            finsh = self.finsh,
            stage = self.stage,
            source = self.source,
        )
    }
}

/// Parse a number of CSV files and a manually_specified YAML file into various load shedding
/// outputs.
///
/// # Examples
///
/// By default, convert all files matching `generated/*.csv` into ICS calendar files (which are
/// written to `calendars/*.ics`), and also write the same data as a machine-friendly CSV file to
/// `calendars/machine_friendly.csv`:
/// ```
/// cargo run --release
/// ```
/// The program is silent by default. Use the `RUST_LOG` environment variable to choose your
/// desired logging level
/// ```
/// RUST_LOG=trace cargo run --release
/// RUST_LOG=info  cargo run --release
/// RUST_LOG=debug cargo run --release
/// RUST_LOG=warn  cargo run --release
/// RUST_LOG=error cargo run --release
/// ```
///
/// You can choose to only calculate loadshedding for files matching the provided regex:
///
/// ```
/// RUST_LOG=info cargo run --release -- --include-regex "city-of-cape-town-area-10"
/// RUST_LOG=info cargo run --release -- --include-regex "western-cape-stellenbosch"
/// RUST_LOG=info cargo run --release -- --include-regex "western-cape|eastern-cape"
/// RUST_LOG=info cargo run --release -- --include-regex "gauteng"
/// ```
/// You can choose whether or not you want ICS/CSV files to be calculated and written with the
/// `--output-ics-files` and `--output-csv-file` flags. These are true by default. Calculating and
/// writing the ICS files to disk takes a lot longer than the CSV file.
///
/// ```
/// RUST_LOG=info cargo run --release -- --output-ics-files=false
/// RUST_LOG=info cargo run --release -- --output-csv-file=false
/// ```
///
/// If you only want to check that `manually_specified.yaml` is valid, you can use the
/// `--only-check-for-overlaps` flag. This is a lot faster than actually creating the ICS/CSV files
///
/// ```
/// RUST_LOG=info cargo run --release -- --only-check-for-overlaps=true
/// ```
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// An optional regex, against which all input CSV schedules must match.
    #[arg(short, long)]
    pub include_regex: Option<Regex>,
    /// Whether or not to output human-friendly ICS files.
    #[arg(long, action=clap::ArgAction::Set, default_value_t = true)]
    pub output_ics_files: bool,
    /// Whether or not to output a machine-friendly CSV file.
    #[arg(long, action=clap::ArgAction::Set, default_value_t = true)]
    pub output_csv_file: bool,
    /// This option provides a fast check which ensures that the YAML is valid.
    #[arg(long, action=clap::ArgAction::Set, default_value_t = false)]
    pub only_check_for_overlaps: bool,
}

/// A multitude of load shedding
pub struct ManuallyInputSchedule {
    /// LoadShedding changes, usually in the future (but not always)
    pub changes: Vec<Shedding>,
    /// LoadShedding changes, always in the past
    pub historical_changes: Vec<Shedding>,
}

/// A multitude of load shedding
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
#[derive(Debug, Clone)]
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
    pub start: String,
    /// The time when LoadShedding *should* end. Note that `finsh` is spelt without the second `i`,
    /// so that it lines up with `start`.
    pub finsh: String,
    /// The stage of loadshedding
    pub stage: u8,
    /// The source of information for this loadshedding event
    pub source: String,
    /// Optionally specify a rust-regex pattern which the area name must match in order for this
    /// shedding to be applied to it. For example, `include_regex: city-of-cape-town-area-\d{1,2}`
    /// will include all city of cape town areas. If `include_regex` and `exclude_regex` conflict
    /// with each other, the area *will* be included. If no include/exclude are specified,
    /// `include_regex: .*` is used by default (so the loadshedding is applied to all areas.
    pub include_regex: Option<String>,
    /// Optionally specify a rust-regex pattern which the area name must *not* match in order for this
    /// shedding to be applied to it. For example, `exclude_regex: city-of-cape-town-area-\d{1,2}`
    /// will exclude all city of cape town areas. If `include_regex` and `exclude_regex` conflict
    /// with each other, the area *will* be included. If no include/exclude are specified,
    /// `include_regex: .*` is used by default (so the loadshedding is applied to all areas.
    pub exclude_regex: Option<String>,
    /// A shorthand so you don't have to specify the full regex. `include: coct` is equivalent to
    /// `include_regex: city-of-cape-town-area-\d{1,2}`. If no include/exclude are specified,
    /// `include_regex: .*` is used by default (so the loadshedding is applied to all areas.
    pub include: Option<String>,
    /// A shorthand so you don't have to specify the full regex. `exclude: coct` is equivalent to
    /// `exclude_regex: city-of-cape-town-area-\d{1,2}`. If no include/exclude are specified,
    /// `include_regex: .*` is used by default (so the loadshedding is applied to all areas.
    pub exclude: Option<String>,
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
        let include_str = raw.include_regex.clone().unwrap_or_else(|| {
            raw.include.clone().map_or(r".*".to_string(), |shorthand| {
                shorthand_to_regex(shorthand, r".*".to_string())
            })
        });
        let include_regex = Regex::new(&include_str).unwrap_or_else(|_| Regex::new(r".*").unwrap());

        let exclude_str = raw.exclude_regex.clone().unwrap_or_else(|| {
            raw.exclude
                .clone()
                .map_or(r"matchnothing^".to_string(), |shorthand| {
                    shorthand_to_regex(shorthand, r"matchnothing^".to_string())
                })
        });
        let exclude_regex = Regex::new(&exclude_str).unwrap_or_else(|_| Regex::new(r".*").unwrap());

        Shedding {
            start: DateTime::parse_from_rfc3339(&format!("{}+02:00", raw.start)).unwrap_or_else(
                |_| {
                    panic!(
                        "Failed to parse start time 1970-01-01T{}:00+02:00 as RFC3339, {raw:?}",
                        raw.start
                    )
                },
            ),
            finsh: DateTime::parse_from_rfc3339(&format!("{}+02:00", raw.finsh)).unwrap_or_else(
                |_| {
                    panic!(
                        "Failed to parse finsh time 1970-01-01T{}:00+02:00 as RFC3339, {raw:?}",
                        raw.finsh
                    )
                },
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
#[derive(Debug, Clone, PartialEq)]
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
    pub start_time: String,
    /// The time when LoadShedding *should* finish (note the spelling).
    pub finsh_time: String,
    /// The stage of loadshedding.
    pub stage: u8,
    /// The date of the month which this event occurs on
    pub date_of_month: u8,
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
            .unwrap_or_else(|_| {
                panic!(
                    "Failed to parse start time 1970-01-01T{}:00+02:00 as RFC3339, {raw:?}",
                    raw.start_time
                )
            }),
            finsh_time: DateTime::parse_from_rfc3339(&format!(
                "1970-01-{date}T{}:00+02:00",
                raw.finsh_time
            ))
            .unwrap_or_else(|_| {
                panic!(
                    "Failed to parse start time 1970-01-01T{}:00+02:00 as RFC3339, {raw:?}",
                    raw.finsh_time
                )
            }),
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
