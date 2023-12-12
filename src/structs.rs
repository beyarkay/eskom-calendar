use clap::Parser;
use regex::Regex;
use std::fmt::{Debug, Display};

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

/// Represents a duration of time for which the power will be out for a particular area.
///
/// Requires specifying where the information came from (in `source`) as well as the stage of
/// loadshedding.
#[derive(PartialEq, Eq, Clone)]
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
        match self.start.partial_cmp(&other.start) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.finsh.partial_cmp(&other.finsh) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.stage.partial_cmp(&other.stage) {
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
///
///     cargo run --release
///
/// The program is silent by default. Set the `RUST_LOG` environment variable to choose your
/// desired logging level to one of trace, info, debug, warn, error:
///
///     RUST_LOG=trace cargo run --release
///
///     RUST_LOG=error cargo run --release
///
/// You can choose to only calculate loadshedding for files matching the provided regex:
///
///     RUST_LOG=info cargo run --release -- --include-regex "city-of-cape-town-area-10"
///
///     RUST_LOG=info cargo run --release -- --include-regex "western-cape-stellenbosch"
///
///     RUST_LOG=info cargo run --release -- --include-regex "western-cape|eastern-cape"
///
///     RUST_LOG=info cargo run --release -- --include-regex "gauteng"
///
/// You can choose whether or not you want ICS/CSV files to be calculated and written with the
/// `--output-ics-files` and `--output-csv-file` flags. These are true by default. Calculating and
/// writing the ICS files to disk takes a lot longer than the CSV file.
///
///     RUST_LOG=info cargo run --release -- --output-ics-files=false
///
///     RUST_LOG=info cargo run --release -- --output-csv-file=false
///
/// If you only want to check that `manually_specified.yaml` is valid, you can use the
/// `--only-check-for-overlaps` flag. This is a lot faster than actually creating the ICS/CSV files
///
///     RUST_LOG=info cargo run --release -- --only-check-for-overlaps=true
#[derive(Parser, Debug)]
#[command(author, version, about)]
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
    pub changes: Vec<Change>,
    /// LoadShedding changes, always in the past
    pub historical_changes: Vec<Change>,
}

/// A multitude of load shedding
#[derive(Serialize, Deserialize)]
pub struct RawManuallyInputSchedule {
    /// LoadShedding changes, usually in the future (but not always)
    changes: Vec<RawChange>,
    /// LoadShedding changes, always in the past
    historical_changes: Vec<RawChange>,
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
pub struct Change {
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
pub struct RawChange {
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

impl From<RawChange> for Change {
    fn from(raw: RawChange) -> Self {
        let shorthand_to_regex = |shorthand: String, default: String| {
            match shorthand.to_lowercase().as_str() {
            "citypower" | "cp" => r"city-power-\d{1,2}".to_string(),
            "capetown" | "cpt" | "coct" => r"city-of-cape-town(-area-\d{1,2})?".to_string(),
            "ekurhuleni" => r"gauteng-ekurhuleni-block-\d{1,2}".to_string(),
            "eskom" => r"^(eskom)|(eastern-cape-)|(free-state-)|(kwazulu-natal-)|(limpopo-)|(mpumalanga-)|(north-west-)|(northern-cape-)|(western-cape-)".to_string(),
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

        // TODO remove this string parsing nonsense
        Change {
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

impl PartialEq for Change {
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
pub struct RecurringShedding {
    /// The time when LoadShedding *should* start. The date of this member will always be 1 Jan
    /// 1970. TODO Convert this to a `chrono::NaiveTime`
    pub start_time: DateTime<FixedOffset>,
    /// The time when LoadShedding *should* finish (note the spelling). The date of this member
    /// will always be 1 Jan 1970, unless the loadshedding is from 22h00 to 00h30 in which case the
    /// finish date will be 2 Jan 1970. TODO Convert this to a `chrono::NaiveTime`
    pub finsh_time: DateTime<FixedOffset>,
    /// The stage of loadshedding.
    pub stage: u8,
    /// How frequently this recurring loadshedding schedule occurs
    pub recurrence: Recurrence,
    /// The day of this particular schedule, which is 1-indexed (the first day is 1, the second is
    /// 2, etc)
    pub day_of_recurrence: u8,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Recurrence {
    /// A weekly recurrence. The week starts on Monday as day 1, and ends on Sunday as day 7.
    Weekly,
    /// A monthly recurrence.
    Monthly,
    /// A recurrence with an arbitrary period measured in days, and an offset specified by `start_dt`.
    Periodic { offset: NaiveDate, period: u8 },
}

#[derive(Deserialize, Debug)]
pub struct RawPeriodicShedding {
    /// The time when LoadShedding *should* start.
    pub start_time: String,
    /// The time when LoadShedding *should* finish (note the spelling).
    pub finsh_time: String,
    /// The stage of loadshedding.
    pub stage: u8,
    /// The day of the 20 day cycle, with the first day being 1, the second day being 2, etc
    pub day_of_cycle: u8,
    pub period_of_cycle: u8,
    pub start_of_cycle: String,
}

// TODO this needs to be tested
impl From<RawPeriodicShedding> for RecurringShedding {
    fn from(raw: RawPeriodicShedding) -> Self {
        assert!(
            raw.day_of_cycle <= raw.period_of_cycle,
            "Day of the cycle {} must be <= period of the cycle {}",
            raw.day_of_cycle,
            raw.period_of_cycle
        );
        let timezone_sast = FixedOffset::east_opt(2 * 60 * 60).unwrap();

        let start_t = NaiveTime::parse_from_str(&raw.start_time, "%H:%M").unwrap();
        let finsh_t = NaiveTime::parse_from_str(&raw.finsh_time, "%H:%M").unwrap();

        let start_datetime = NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_time(start_t);
        let finsh_datetime = NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_time(finsh_t);

        let offset = NaiveDate::parse_from_str(&raw.start_of_cycle, "%Y-%m-%d").unwrap();

        RecurringShedding {
            start_time: DateTime::<FixedOffset>::from_local(start_datetime, timezone_sast),
            finsh_time: DateTime::<FixedOffset>::from_local(finsh_datetime, timezone_sast),
            stage: raw.stage,
            recurrence: Recurrence::Periodic {
                offset,
                period: raw.period_of_cycle,
            },
            day_of_recurrence: raw.day_of_cycle,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct RawWeeklyShedding {
    /// The time when LoadShedding *should* start.
    pub start_time: String,
    /// The time when LoadShedding *should* finish (note the spelling).
    pub finsh_time: String,
    /// The stage of loadshedding.
    pub stage: u8,
    /// The day of the week, with Monday being 1, Tuesday being 2, etc
    pub day_of_week: u8,
}

impl From<RawWeeklyShedding> for RecurringShedding {
    fn from(raw: RawWeeklyShedding) -> Self {
        assert!(
            0 < raw.day_of_week && raw.day_of_week < 8,
            "Day of the week must be one of 1, 2, 3, 4, 5, 6, 7"
        );
        let timezone_sast = FixedOffset::east_opt(2 * 60 * 60).unwrap();

        let start_t = NaiveTime::parse_from_str(&raw.start_time, "%H:%M").unwrap();
        let finsh_t = NaiveTime::parse_from_str(&raw.finsh_time, "%H:%M").unwrap();

        let start_datetime = NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_time(start_t);
        let finsh_datetime = NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_time(finsh_t);

        RecurringShedding {
            start_time: DateTime::<FixedOffset>::from_local(start_datetime, timezone_sast),
            finsh_time: DateTime::<FixedOffset>::from_local(finsh_datetime, timezone_sast),
            stage: raw.stage,
            recurrence: Recurrence::Weekly,
            day_of_recurrence: raw.day_of_week,
        }
    }
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

impl From<RawMonthlyShedding> for RecurringShedding {
    fn from(raw: RawMonthlyShedding) -> Self {
        assert!(
            0 < raw.date_of_month && raw.date_of_month <= 31,
            "Date of month must be in the range (0, 31]"
        );
        let timezone_sast = FixedOffset::east_opt(2 * 60 * 60).unwrap();

        let start_t = NaiveTime::parse_from_str(&raw.start_time, "%H:%M").unwrap();
        let finsh_t = NaiveTime::parse_from_str(&raw.finsh_time, "%H:%M").unwrap();

        let start_datetime = NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_time(start_t);
        let finsh_datetime = NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_time(finsh_t);

        RecurringShedding {
            start_time: DateTime::<FixedOffset>::from_local(start_datetime, timezone_sast),
            finsh_time: DateTime::<FixedOffset>::from_local(finsh_datetime, timezone_sast),
            stage: raw.stage,
            recurrence: Recurrence::Monthly,
            day_of_recurrence: raw.date_of_month,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, FixedOffset};

    fn rfc3339(s: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(s).unwrap()
    }

    mod raw_shedding_to_shedding {
        use chrono::NaiveDate;

        use crate::structs::{
            tests::rfc3339, RawMonthlyShedding, RawPeriodicShedding, RawWeeklyShedding, Recurrence,
            RecurringShedding,
        };

        #[should_panic]
        #[test]
        fn test_periodic_too_high() {
            let raw_too_high = RawPeriodicShedding {
                start_time: "12:00".to_owned(),
                finsh_time: "14:30".to_owned(),
                stage: 1,
                day_of_cycle: 100,
                period_of_cycle: 19,
                start_of_cycle: "2023-01-01".to_owned(),
            };
            let _ = Into::<RecurringShedding>::into(raw_too_high);
        }

        #[test]
        fn test_periodic() {
            let raw = RawPeriodicShedding {
                start_time: "12:00".to_owned(),
                finsh_time: "14:30".to_owned(),
                stage: 1,
                day_of_cycle: 1,
                period_of_cycle: 19,
                start_of_cycle: "2023-01-01".to_owned(),
            };
            let cooked = RecurringShedding {
                start_time: rfc3339("1970-01-01T12:00:00+02:00"),
                finsh_time: rfc3339("1970-01-01T14:30:00+02:00"),
                stage: 1,
                recurrence: Recurrence::Periodic {
                    offset: NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
                    period: 19,
                },
                day_of_recurrence: 1,
            };
            assert_eq!(Into::<RecurringShedding>::into(raw), cooked);
        }

        #[should_panic]
        #[test]
        fn test_weekly_oob_too_high() {
            let raw_too_high = RawWeeklyShedding {
                start_time: "12:00".to_owned(),
                finsh_time: "14:30".to_owned(),
                stage: 1,
                day_of_week: 8,
            };
            let _ = Into::<RecurringShedding>::into(raw_too_high);
        }

        #[should_panic]
        #[test]
        fn test_weekly_oob_too_low() {
            let raw_too_low = RawWeeklyShedding {
                start_time: "12:00".to_owned(),
                finsh_time: "14:30".to_owned(),
                stage: 1,
                day_of_week: 0,
            };
            let _ = Into::<RecurringShedding>::into(raw_too_low);
        }

        #[test]
        fn test_weekly() {
            let raw = RawWeeklyShedding {
                start_time: "12:00".to_owned(),
                finsh_time: "14:30".to_owned(),
                stage: 1,
                day_of_week: 1,
            };
            let cooked = RecurringShedding {
                start_time: rfc3339("1970-01-01T12:00:00+02:00"),
                finsh_time: rfc3339("1970-01-01T14:30:00+02:00"),
                stage: 1,
                recurrence: Recurrence::Weekly,
                day_of_recurrence: 1,
            };
            assert_eq!(Into::<RecurringShedding>::into(raw), cooked);
        }

        #[should_panic]
        #[test]
        fn test_monthly_oob_too_high() {
            let raw = RawMonthlyShedding {
                start_time: "12:00".to_owned(),
                finsh_time: "14:30".to_owned(),
                stage: 1,
                date_of_month: 32,
            };
            let _ = Into::<RecurringShedding>::into(raw);
        }

        #[test]
        fn test_monthly() {
            let raw = RawMonthlyShedding {
                start_time: "12:00".to_owned(),
                finsh_time: "14:30".to_owned(),
                stage: 1,
                date_of_month: 1,
            };
            let cooked = RecurringShedding {
                start_time: rfc3339("1970-01-01T12:00:00+02:00"),
                finsh_time: rfc3339("1970-01-01T14:30:00+02:00"),
                stage: 1,
                recurrence: Recurrence::Monthly,
                day_of_recurrence: 1,
            };
            assert_eq!(Into::<RecurringShedding>::into(raw), cooked);
        }
    }
    mod raw_change_to_change {
        use crate::structs::{Change, RawChange};
        use regex::Regex;

        fn cooked_with_regex(include_regex: &str, exclude_regex: &str) -> Change {
            Change {
                start: chrono::DateTime::parse_from_rfc3339("2022-01-01T08:00:00+02:00").unwrap(),
                finsh: chrono::DateTime::parse_from_rfc3339("2022-01-02T08:00:00+02:00").unwrap(),
                stage: 1,
                source: "Test source".to_string(),
                include_regex: Regex::new(include_regex).unwrap(),
                exclude_regex: Regex::new(exclude_regex).unwrap(),
            }
        }

        fn raw_with_regex(include: Option<String>, exclude: Option<String>) -> RawChange {
            RawChange {
                start: "2022-01-01T08:00:00".to_string(),
                finsh: "2022-01-02T08:00:00".to_string(),
                stage: 1,
                source: "Test source".to_string(),
                include_regex: None,
                exclude_regex: None,
                include,
                exclude,
            }
        }

        #[test]
        fn test_regex() {
            let shorthand_to_longhand = vec![
                // City power
                ("citypower", r"city-power-\d{1,2}"),
                // Cape Town
                ("coct", r"city-of-cape-town(-area-\d{1,2})?"),
                ("capetown", r"city-of-cape-town(-area-\d{1,2})?"),
                ("CapEtOwN", r"city-of-cape-town(-area-\d{1,2})?"),
                ("cpt", r"city-of-cape-town(-area-\d{1,2})?"),
                ("coct", r"city-of-cape-town(-area-\d{1,2})?"),
                // Ekurhuleni
                ("ekurhuleni", r"gauteng-ekurhuleni-block-\d{1,2}"),
                // Eskom
                (
                    "eskom",
                    r"^(eskom)|(eastern-cape-)|(free-state-)|(kwazulu-natal-)|(limpopo-)|(mpumalanga-)|(north-west-)|(northern-cape-)|(western-cape-)",
                ),
                // Tshwane
                ("tshwane", r"gauteng-tshwane-group-\d{1,2}"),
            ];
            for incl in &shorthand_to_longhand {
                assert_eq!(
                    Into::<Change>::into(raw_with_regex(Some(incl.0.to_string()), None)),
                    cooked_with_regex(incl.1, "matchnothing^")
                );
                let is_first = false;
                for excl in &shorthand_to_longhand {
                    if is_first {
                        assert_eq!(
                            Into::<Change>::into(raw_with_regex(None, Some(excl.0.to_string()))),
                            cooked_with_regex(".*", excl.1)
                        );
                    }
                    assert_eq!(
                        Into::<Change>::into(raw_with_regex(
                            Some(incl.0.to_string()),
                            Some(excl.0.to_string())
                        )),
                        cooked_with_regex(incl.1, excl.1)
                    );
                }
            }
        }
    }
}
