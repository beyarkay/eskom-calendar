use chrono::FixedOffset;
use chrono::{DateTime, Datelike, NaiveTime, Timelike};
use icalendar::Calendar;
use regex::Regex;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use structs::{Args, ManuallyInputSchedule, MonthlyShedding, PowerOutage, Shedding};
// TODO use rayon's par_iter to speed up lots of operatiosn
// TODO write docstrings
// TODO check that the formatting of the description is correct
// TODO include some sort of progress bar

extern crate pretty_env_logger;
use log::{info, trace};

use clap::Parser;
mod structs;

/// Download pdfs if the parsed CSVs don't already exist, and use them to create `ics` files.
fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    // Parse the command-line arguments
    let args = Args::parse();

    // Read in the CSV paths
    let paths = read::get_csv_paths("generated/")?;

    // Read in the manually_specified YAML file
    let manually_specified = read::read_manually_specified("manually_specified.yaml")?;

    // Ensure that none of the manually_specified areas conflict with one another
    err_if_overlaps(&manually_specified.changes, &paths)?;
    if args.only_check_for_overlaps {
        return Ok(());
    }

    // Only include those CSV paths permitted by the --include-regex CLI argument
    let mut filtered_paths = filter_paths_by_regex(args.include_regex, paths);
    filtered_paths.sort();

    // Convert the paths into lines which can be written to a CSV
    let mut csv_lines = filtered_paths
        .iter()
        // Convert the paths to (path, shedding) tuples
        .map(|path| (path, read::read_sheddings_from_csv_path(path)))
        // Exclude all sheddings which failed
        .filter_map(|(path, sheddings)| match sheddings {
            Ok(sheddings) => Some((path, sheddings)),
            Err(_) => None,
        })
        // Exclude all sheddings which can't be converted to CsvLines
        .filter_map(|(path, sheddings)| {
            let area_name = fmt::path_to_area_name(path).unwrap();
            match calculate_power_outages(&area_name, sheddings, &manually_specified) {
                Ok(lines) => Some((path, lines)),
                Err(_) => None,
            }
        })
        // Write the individual sheddings to ICS files
        .map(|(path, lines)| {
            if args.output_ics_files {
                write_sheddings_to_ics(path, &lines).unwrap();
            }
            (path, lines)
        })
        // Fold the multiple CSV vectors into one long vector
        .fold(vec![], |mut acc, (_path, lines)| {
            acc.extend(lines);
            acc
        });

    if args.output_csv_file {
        // Write the lines to a CSV
        overwrite_lines_to_csv(&mut csv_lines)?;
    }
    Ok(())
}

/// Checks the provided changes for illegal overlaps.
/// For example, specifying Stage 2 from 14h to 16h as well as Stage 3 from 13h to 16h is not
/// allowed.
fn err_if_overlaps(changes: &[Shedding], paths: &[PathBuf]) -> Result<(), Box<dyn Error>> {
    info!("Checking for overlaps...");
    for path in paths {
        let area_name = fmt::path_to_area_name(path)?;
        let regexed_changes = changes
            .iter()
            .filter(|c| {
                !c.exclude_regex.is_match(&area_name) && c.include_regex.is_match(&area_name)
            })
            .collect::<Vec<_>>();
        for change1 in &regexed_changes {
            for change2 in &regexed_changes {
                let not_same_item = change1 != change2;
                let c1_overlaps_c2 = change1.finsh > change2.start && change1.start < change2.finsh;
                let c2_overlaps_c1 = change2.finsh > change1.start && change2.start < change1.finsh;
                // If they're not the same item and they overlap, then return an Err()
                if not_same_item && (c1_overlaps_c2 || c2_overlaps_c1) {
                    return Err(Box::from(format!(
                        "Changes overlap:\nChange1: {change1:#?}\nChange2: {change2:#?}\nYou probably entered invalid information into `manually_specified.yaml`"
                    )));
                }
            }
        }
    }
    trace!("  No overlaps found");
    Ok(())
}

/// Using the `--include-regex` CLI argument, the user can specify that only certain paths be
/// included. Filter out all paths not explicitly included by `--include-regex`
fn filter_paths_by_regex(regex: Option<Regex>, paths: Vec<PathBuf>) -> Vec<PathBuf> {
    info!(
        "Filtering out {} paths based on regex {:?}",
        paths.len(),
        regex
    );
    let filtered_paths = paths
        .into_iter()
        .filter(|path| {
            regex
                .as_ref()
                .map_or(true, |re| re.is_match(path.to_str().unwrap()))
        })
        .collect::<Vec<_>>();
    trace!("  Resulted with {} paths", filtered_paths.len());
    filtered_paths
}

/// Given some local monthly sheddings and some manually specified national sheddings, convert them
/// into a list of power outages.
fn calculate_power_outages(
    area_name: &str,
    monthly_sheddings: Vec<MonthlyShedding>,
    manually_specified: &ManuallyInputSchedule,
) -> Result<Vec<PowerOutage>, Box<dyn Error>> {
    let national_changes: Vec<Shedding> = manually_specified
        .changes
        .clone()
        .into_iter()
        .filter(|c| !c.exclude_regex.is_match(area_name) && c.include_regex.is_match(area_name))
        .collect();
    let combos = make_combinations_from_sheddings(&monthly_sheddings, &national_changes);
    trace!(
        "Checking {} combinations for possible load shedding in {area_name}",
        combos.len(),
    );

    let mut lines = vec![];

    for (local, natnl) in combos {
        let datetimes = gen_datetimes(
            natnl.start,
            natnl.finsh,
            local.date_of_month,
            local.start_time.time(),
            local.finsh_time.time(),
        );
        for dt in datetimes {
            lines.push(PowerOutage {
                area_name: area_name.to_owned(),
                stage: local.stage,
                start: dt.0,
                finsh: dt.1,
                source: natnl.source.clone(),
            })
        }
    }
    Ok(lines)
}

/// Attempt to write some PowerOutages to the specified path as a ICS file.
fn write_sheddings_to_ics(
    path: &Path,
    power_outages: &[PowerOutage],
) -> Result<(), Box<dyn Error>> {
    // Get the correct filename
    let fname = path
        .to_str()
        .ok_or("Couldn't convert path to string")?
        .replace("csv", "ics")
        .replace("generated", "calendars")
        .replace(|c: char| !c.is_ascii(), "")
        .replace("&nbsp;", "");

    info!("Writing {} events to {:?}", power_outages.len(), fname);

    let mut calendar = Calendar::new();
    let mut last_finsh: Option<DateTime<FixedOffset>> = None;

    // Add all the events to the calendar
    for outage in power_outages {
        // Keep track of the last finished event, so that we can add one more event immediately
        // after it
        last_finsh = match last_finsh {
            Some(le) => Some(le.max(outage.finsh)),
            None => Some(outage.finsh),
        };
        // Convert the outage to an event, and add it to the calendar
        calendar.push(fmt::power_outage_to_event(outage)?);
    }

    // If we have >0 events, add one final event specifying that there's no more loadshedding
    // information after here.
    if let Some(last_finsh) = last_finsh {
        calendar.push(fmt::end_of_schedule_event(last_finsh)?);
    }

    // Write all the data to disk
    let mut file = File::create(fname.as_str())?;
    writeln!(&mut file, "{}", calendar)?;

    Ok(())
}

/// Given a list of power outages, convert them to a single CSV file for machine consumption.
fn overwrite_lines_to_csv(power_outages: &mut Vec<PowerOutage>) -> Result<(), Box<dyn Error>> {
    info!(
        "Writing {}+1 lines to machine_friendly.csv",
        power_outages.len()
    );
    // Create the file (overwriting if it exists)
    let mut file = File::create("calendars/machine_friendly.csv")?;
    // Write the header line of the csv file
    writeln!(&mut file, "{}", PowerOutage::csv_header())?;
    // Sort the lines so we have some kind of consistency of the output
    power_outages.sort();
    for line in power_outages {
        writeln!(&mut file, "{}", line)?;
    }
    Ok(())
}

/// Returns the git hash of the current repository.
///
/// # Errors
///
/// - The `git` command is not found on the system.
/// - The current directory is not a valid git repository.
fn get_git_hash() -> Result<String, Box<dyn Error>> {
    Ok(String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()?
            .stdout,
    )?)
}

/// Given some monthly shedding data and some national shedding data, calculate all the ways they
/// could be combined while ensuring the stages are equal.
fn make_combinations_from_sheddings(
    monthly_sheddings: &[MonthlyShedding],
    national_sheddings: &[Shedding],
) -> Vec<(MonthlyShedding, Shedding)> {
    let mut combos = vec![];
    for monthly in monthly_sheddings {
        for national in national_sheddings {
            if monthly.stage == national.stage {
                combos.push((monthly.clone(), national.clone()));
            }
        }
    }
    combos.sort_by_key(|(_monthly, natnl)| natnl.start);
    combos
}

/// Find all (start, finsh) datetimes in a certain date range, during a certain time range, on a
/// certain day.
///
/// Specifically, the returned (start, finish) datetimes are:
/// 1. Not before `natnl_start`
/// 2. Not after `natnl_finsh`
/// 3. On no other day of the month other than `local_dom`
/// 4. Never before `local_start` on any particular day.
/// 5. Never after `local_finsh` on any particular day.
///
/// Note that if local_start is 23:30 and local_finsh is 00:30, then the (start, finsh) tuple will
/// cross over midnight.
fn gen_datetimes(
    nat_start_dt: DateTime<FixedOffset>,
    nat_finsh_dt: DateTime<FixedOffset>,
    lcl_dom: u8,
    lcl_start_t: NaiveTime,
    lcl_finsh_t: NaiveTime,
) -> Vec<(DateTime<FixedOffset>, DateTime<FixedOffset>)> {
    nat_start_dt
        // subtract a day to ensure we handle the midnight boundary condition properly
        .checked_sub_days(chrono::Days::new(1))
        .unwrap()
        // Convert datetime to date to make things simpler. We'll add back the time later.
        .date_naive()
        // create an unbounded iterator, starting from one day before nat_start_dt
        .iter_days()
        .take_while(|d| d <= &nat_finsh_dt.date_naive())
        // Get all possible datetime ranges with start and finsh times specified by `lcl_start_t`
        // and `lcl_finsh_t`
        .map(|d| {
            // Create the starting datetime
            let start_dt = d
                .and_hms_opt(
                    lcl_start_t.hour(),
                    lcl_start_t.minute(),
                    lcl_start_t.second(),
                )
                .unwrap();
            // Create the finishing datetime
            let mut finsh_dt = d
                .and_hms_opt(
                    lcl_finsh_t.hour(),
                    lcl_finsh_t.minute(),
                    lcl_finsh_t.second(),
                )
                .unwrap();
            // its possible that finsh_t is 00:30 and start_t is 22:00, in which case make sure
            // finsh_dt is on the next day.
            if finsh_dt < start_dt {
                finsh_dt = finsh_dt.checked_add_days(chrono::Days::new(1)).unwrap();
            }

            let lcl_range: (DateTime<FixedOffset>, DateTime<FixedOffset>) = (
                chrono::DateTime::from_local(start_dt, *nat_start_dt.offset()),
                chrono::DateTime::from_local(finsh_dt, *nat_finsh_dt.offset()),
            );
            lcl_range
        })
        // Ensure each range starts on the correct date of the month
        .filter(|(start, _finsh)| start.day() == lcl_dom as u32)
        // Truncate each local range so that it's actually within the specified national range
        .map(|(start, finsh)| (nat_start_dt.max(start), nat_finsh_dt.min(finsh)))
        // Ensure each range is before the finish
        .filter(|(start, finsh)| finsh <= &nat_finsh_dt && start < &nat_finsh_dt)
        // Ensure each range is after the start
        .filter(|(start, finsh)| start >= &nat_start_dt && finsh > &nat_start_dt)
        .collect()
}

/// Contains some formatting functions, including some event-creation functions.
mod fmt {
    use crate::get_git_hash;
    use chrono::FixedOffset;
    use chrono::{DateTime, Utc};
    use icalendar::{Component, Event};
    use std::error::Error;
    use std::path::Path;

    use crate::structs::PowerOutage;

    /// Format a path as an area name: remove the extension and the `generated/` directory. This fails
    /// if the path isn't valid.
    pub fn path_to_area_name(path: &Path) -> Result<String, Box<dyn Error>> {
        Ok(path
            .to_str()
            .ok_or("Path is not valid unicode")?
            .replace("generated/", "")
            .replace(".csv", "")
            .replace(|c: char| !c.is_ascii(), ""))
    }

    /// Convert a power outage to a ICS calendar event, with a nicely formatted description.
    pub fn power_outage_to_event(power_outage: &PowerOutage) -> Result<Event, Box<dyn Error>> {
        // TODO can a default alarm be added to this?
        let description = format!(
            "This event shows that there will be loadshedding on {} to {} in the load \
            shedding area {}.\n\
            \n\
            When new loadshedding schedules are announced, your calendar will be \
            automatically updated to show when your power will be off. \n\
            \n\
            While these new schedules are calculated immediately, it can sometimes take a \
            bit of time for your calendar app (ie Google Calendar, Apple iCalendar, or \
            Outlook) to fetch the updated schedules. Often you can set the update \n\
            frequency in the settings of your calendar app. \n\
            \n\
            --- \n\
            \n\
            Incorrect? Open an issue here: https://github.com/beyarkay/eskom-calendar/issues/new\n\
            \n\
            Generated by Boyd Kane's Eskom-Calendar: https://eskomcalendar.co.za/ec?calendar={}.ics.\n\
            \n\
            National loadshedding information scraped from {}.\n\
            \n\
            Calendar compiled at {}.\n\
            \n\
            Eskom-calendar version: https://github.com/beyarkay/eskom-calendar/tree/{}",
            power_outage.start.format("%A from %H:%M"),
            power_outage.finsh.format("%H:%M"),
            power_outage.area_name,
            power_outage.area_name,
            power_outage.source,
            chrono::offset::Local::now(),
            get_git_hash()?,
        );
        let emojis = vec!["😁", "😕", "☹️", "😟", "😣", "😭", "😫", "😤", "😡"];
        let summary = format!(
            "🔌{area_name} Stage {stage} {emoji}",
            area_name = prettify_area_name(&power_outage.area_name),
            stage = power_outage.stage,
            emoji = emojis.get(power_outage.stage as usize).unwrap_or(&"🫠"),
        );
        let evt = Event::new()
            .summary(summary.as_str())
            .description(description.as_str())
            .starts(power_outage.start.with_timezone(&Utc))
            .ends(power_outage.finsh.with_timezone(&Utc))
            .done();
        Ok(evt)
    }

    /// Create an event that signals the end of known loadshedding data
    pub fn end_of_schedule_event(
        last_event: DateTime<FixedOffset>,
    ) -> Result<Event, Box<dyn Error>> {
        let description = format!(
            "This is the end of the known loadshedding schedule.\n\
            \n\
            Unfortunately only a few days worth of loadshedding schedules are released at a time. \
            An update to the loadshedding schedule is usually announced a day or two before the \
            previous schedule runs out, so this event will move in your calendar as new schedules \
            are announced.\n\
            \n\
            You don't have to do anything, but just know that there might be loadshedding after \
            this point, but there also might not. It's impossible to say for sure.\n\
            \n\
            Incorrect? Open an issue here: https://github.com/beyarkay/eskom-calendar/issues/new\n\
            \n\
            --- \n\
            Generated by Boyd Kane's Eskom-Calendar: https://github.com/beyarkay/eskom-calendar/tree/{git_hash} \n\
            Calendar compiled at {compiletime:?}",
            git_hash=get_git_hash()?,
            compiletime=chrono::offset::Local::now(),
        );

        let start = last_event.with_timezone(&Utc);
        let end = last_event
            .checked_add_signed(chrono::Duration::hours(1))
            .unwrap()
            .with_timezone(&Utc);

        Ok(Event::new()
            .summary("⚠️  End of schedule")
            .description(&description)
            .starts(start)
            .ends(end)
            .done())
    }

    /// Convert a string to title case.
    /// Rust makes this harder than I ever believed possible.
    pub fn to_title_case(s: String) -> String {
        s.split(' ')
        .into_iter()
        .map(|si| {
            // Capitalise the first character
            si.chars().next().unwrap().to_uppercase().to_string()
                // And just join the remaining characters together
                + si.chars().skip(1).map(|c| c.to_string()).reduce(|acc, curr| acc + &curr).unwrap_or_default().as_str()
        })
        .collect()
    }

    /// Take an area name like `western-cape-stellenbosch` and prettify it, ditching the dashes and
    /// making it title case.
    pub fn prettify_area_name(area_name: &str) -> String {
        let prefixes = vec![
            ("eastern-cape-", "EC"),
            ("free-state-", "FS"),
            ("kwazulu-natal-", "KZN"),
            ("limpopo-", "LP"),
            ("mpumalanga-", "MP"),
            ("north-west-", "NC"),
            ("northern-cape-", "NW"),
            ("western-cape-", "WC"),
        ];

        if area_name.starts_with("city-of-cape-town-area-") {
            area_name.replace("city-of-cape-town-area-", "Cape Town ")
        } else if area_name.starts_with("city-power") {
            area_name.replace("city-power", "City Power ")
        } else if area_name.starts_with("gauteng-ekurhuleni-block-") {
            area_name.replace("gauteng-ekurhuleni-block-", "Ekurhuleni ")
        } else if area_name.starts_with("gauteng-tshwane-group-") {
            area_name.replace("gauteng-tshwane-group-", "Tshwane ")
        } else {
            // Convert areas of the form `{province}-{area}` into `{area} ({province acronym})`
            let mut prettified = to_title_case(area_name.replace('-', " "));
            for (prefix, replacement) in prefixes {
                if area_name.starts_with(prefix) {
                    prettified = format!(
                        "{} ({})",
                        to_title_case(area_name.replace(prefix, "").replace('-', " ")),
                        replacement
                    );
                    break;
                }
            }
            prettified
        }
    }
}

/// Contains some read-based functions, such as `get_csv_paths` and `read_manually_specified`.
mod read {
    use crate::structs::{
        ManuallyInputSchedule, MonthlyShedding, RawManuallyInputSchedule, RawMonthlyShedding,
    };
    use std::error::Error;
    use std::fs::read_to_string;
    use std::path::PathBuf;

    extern crate pretty_env_logger;
    use log::{info, trace};

    /// Get the paths of all CSVs in `dir`.
    pub fn get_csv_paths(dir: &str) -> Result<Vec<PathBuf>, Box<dyn Error>> {
        info!("Looking for CSV paths in {}", dir);
        let paths = std::fs::read_dir(dir)?
            // Filter out all those directory entries which couldn't be read
            .filter_map(|res| res.ok())
            // Map the directory entries to paths
            .map(|dir_entry| dir_entry.path())
            // Filter out all paths with extensions other than `csv`
            .filter_map(|path| {
                if path.extension().map_or(false, |ext| ext == "csv") {
                    Some(path)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        trace!("  Found {} CSVs in {dir}", paths.len());
        Ok(paths)
    }

    /// Read in `manually_specified.yaml` from YAML to an in-memory struct
    pub fn read_manually_specified(path: &str) -> Result<ManuallyInputSchedule, Box<dyn Error>> {
        Ok(
            serde_yaml::from_str::<RawManuallyInputSchedule>(read_to_string(path)?.as_str())?
                .into(),
        )
    }

    /// Read in the load shedding information from the provided path.
    pub fn read_sheddings_from_csv_path(
        path: &PathBuf,
    ) -> Result<Vec<MonthlyShedding>, Box<dyn Error>> {
        let local_shedding = csv::Reader::from_path(path)?
            .deserialize::<RawMonthlyShedding>()
            .into_iter()
            .map(|res| Into::<MonthlyShedding>::into(res.unwrap()))
            .filter(|shedding| shedding.stage != 0)
            .map(|shedding| MonthlyShedding {
                start_time: shedding.start_time,
                finsh_time: shedding.finsh_time,
                stage: shedding.stage,
                date_of_month: shedding.date_of_month,
                goes_over_midnight: shedding.goes_over_midnight,
            })
            .collect::<Vec<MonthlyShedding>>();
        Ok(local_shedding)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, FixedOffset};

    fn rfc3339(s: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(s).unwrap()
    }

    mod power_outage_to_event {
        use chrono::Utc;
        use icalendar::Component;

        use crate::{fmt::power_outage_to_event, structs::PowerOutage, tests::rfc3339};

        #[test]
        fn description_contains() {
            let e = power_outage_to_event(&PowerOutage {
                area_name: "test-name".to_owned(),
                stage: 2,
                start: rfc3339("2022-01-02T13:00:00+02:00"),
                finsh: rfc3339("2022-01-02T15:00:00+02:00"),
                source: "test-source".to_owned(),
            })
            .unwrap();
            let desc = e.get_description().unwrap();

            let should_contain_all = vec![
                "This event shows that there will be loadshedding on Sunday from 13:00 to 15:00",
                "Generated by Boyd Kane's Eskom-Calendar: https://eskomcalendar.co.za/ec?calendar=test-name.ics.",
                "in the load shedding area test-name.",
                "National loadshedding information scraped from test-source.",
            ];
            for should_contain in should_contain_all {
                assert!(
                    desc.contains(should_contain),
                    "Description should contain:\n\"{should_contain}\"\nbut is:\n{desc}"
                );
            }
        }

        #[test]
        fn start_and_finsh_correct() {
            let start = rfc3339("2022-01-02T13:00:00+02:00");
            let finsh = rfc3339("2022-01-02T15:00:00+02:00");
            let e = power_outage_to_event(&PowerOutage {
                area_name: "test-name".to_owned(),
                stage: 2,
                start,
                finsh,
                source: "test-source".to_owned(),
            })
            .unwrap();
            assert_eq!(e.get_start().unwrap(), start.with_timezone(&Utc).into());
            assert_eq!(e.get_end().unwrap(), finsh.with_timezone(&Utc).into());
        }
    }

    mod check_for_overlaps {
        mod err_if {
            use std::path::PathBuf;
            use std::str::FromStr;

            use crate::structs::RawShedding;
            use crate::{err_if_overlaps, structs::Shedding};

            #[test]
            fn first_is_subset_of_second() {
                let changes: Vec<Shedding> = vec![
                    RawShedding {
                        start: "2022-01-01T10:00:00".to_string(),
                        finsh: "2022-01-01T13:00:00".to_string(),
                        stage: 1,
                        source: "test_source".to_string(),
                        include_regex: None,
                        exclude_regex: None,
                        include: Some("coct".to_string()),
                        exclude: None,
                    }
                    .into(),
                    RawShedding {
                        start: "2022-01-01T11:00:00".to_string(),
                        finsh: "2022-01-01T12:00:00".to_string(),
                        stage: 1,
                        source: "test_source".to_string(),
                        include_regex: None,
                        exclude_regex: None,
                        include: Some("coct".to_string()),
                        exclude: None,
                    }
                    .into(),
                ];
                let paths = vec![
                    PathBuf::from_str("generated/city-of-cape-town-area-1.csv").unwrap(),
                    PathBuf::from_str("generated/city-of-cape-town-area-10.csv").unwrap(),
                    PathBuf::from_str("generated/western-cape-stellenbosch.csv").unwrap(),
                    PathBuf::from_str("generated/western-cape-darling.csv.csv").unwrap(),
                ];
                assert!(err_if_overlaps(&changes, &paths).is_err())
            }

            #[test]
            fn start2_lt_finsh1() {
                let changes: Vec<Shedding> = vec![
                    RawShedding {
                        start: "2022-01-01T10:00:00".to_string(),
                        finsh: "2022-01-01T12:00:00".to_string(),
                        stage: 1,
                        source: "test_source".to_string(),
                        include_regex: None,
                        exclude_regex: None,
                        include: Some("coct".to_string()),
                        exclude: None,
                    }
                    .into(),
                    RawShedding {
                        start: "2022-01-01T11:00:00".to_string(),
                        finsh: "2022-01-01T13:00:00".to_string(),
                        stage: 1,
                        source: "test_source".to_string(),
                        include_regex: None,
                        exclude_regex: None,
                        include: Some("coct".to_string()),
                        exclude: None,
                    }
                    .into(),
                ];
                let paths = vec![
                    PathBuf::from_str("generated/city-of-cape-town-area-1.csv").unwrap(),
                    PathBuf::from_str("generated/city-of-cape-town-area-10.csv").unwrap(),
                    PathBuf::from_str("generated/western-cape-stellenbosch.csv").unwrap(),
                    PathBuf::from_str("generated/western-cape-darling.csv.csv").unwrap(),
                ];
                assert!(err_if_overlaps(&changes, &paths).is_err())
            }
        }

        mod ok_if {
            use std::path::PathBuf;
            use std::str::FromStr;

            use crate::structs::RawShedding;
            use crate::{err_if_overlaps, structs::Shedding};
            #[test]
            fn start2_lt_finsh1_but_different_regex() {
                let changes: Vec<Shedding> = vec![
                    RawShedding {
                        start: "2022-01-01T10:00:00".to_string(),
                        finsh: "2022-01-01T12:00:00".to_string(),
                        stage: 1,
                        source: "test_source".to_string(),
                        include_regex: None,
                        exclude_regex: None,
                        include: Some("coct".to_string()),
                        exclude: None,
                    }
                    .into(),
                    RawShedding {
                        start: "2022-01-01T11:00:00".to_string(),
                        finsh: "2022-01-01T13:00:00".to_string(),
                        stage: 1,
                        source: "test_source".to_string(),
                        include_regex: None,
                        exclude_regex: None,
                        include: None,
                        exclude: Some("coct".to_string()),
                    }
                    .into(),
                ];
                let paths = vec![
                    PathBuf::from_str("generated/city-of-cape-town-area-1.csv").unwrap(),
                    PathBuf::from_str("generated/city-of-cape-town-area-10.csv").unwrap(),
                    PathBuf::from_str("generated/western-cape-stellenbosch.csv").unwrap(),
                    PathBuf::from_str("generated/western-cape-darling.csv.csv").unwrap(),
                ];
                assert!(err_if_overlaps(&changes, &paths).is_ok())
            }
            #[test]
            fn ok_if_start_eq_finsh() {
                let changes: Vec<Shedding> = vec![
                    RawShedding {
                        start: "2022-01-01T10:00:00".to_string(),
                        finsh: "2022-01-01T12:30:00".to_string(),
                        stage: 1,
                        source: "test_source".to_string(),
                        include_regex: None,
                        exclude_regex: None,
                        include: Some("coct".to_string()),
                        exclude: None,
                    }
                    .into(),
                    RawShedding {
                        start: "2022-01-01T12:30:00".to_string(),
                        finsh: "2022-01-01T14:30:00".to_string(),
                        stage: 1,
                        source: "test_source".to_string(),
                        include_regex: None,
                        exclude_regex: None,
                        exclude: Some("coct".to_string()),
                        include: None,
                    }
                    .into(),
                ];
                let paths = vec![
                    PathBuf::from_str("generated/city-of-cape-town-area-1.csv").unwrap(),
                    PathBuf::from_str("generated/city-of-cape-town-area-10.csv").unwrap(),
                    PathBuf::from_str("generated/western-cape-stellenbosch.csv").unwrap(),
                    PathBuf::from_str("generated/western-cape-darling.csv.csv").unwrap(),
                ];
                assert!(err_if_overlaps(&changes, &paths).is_ok())
            }

            #[test]
            fn ok_if_mutually_exclusive() {
                let changes: Vec<Shedding> = vec![
                    RawShedding {
                        start: "2022-01-01T10:00:00".to_string(),
                        finsh: "2022-01-01T12:30:00".to_string(),
                        stage: 1,
                        source: "test_source".to_string(),
                        include_regex: None,
                        exclude_regex: None,
                        include: Some("coct".to_string()),
                        exclude: None,
                    }
                    .into(),
                    RawShedding {
                        start: "2022-01-01T10:00:00".to_string(),
                        finsh: "2022-01-01T12:30:00".to_string(),
                        stage: 1,
                        source: "test_source".to_string(),
                        include_regex: None,
                        exclude_regex: None,
                        exclude: Some("coct".to_string()),
                        include: None,
                    }
                    .into(),
                ];
                let paths = vec![
                    PathBuf::from_str("generated/city-of-cape-town-area-1.csv").unwrap(),
                    PathBuf::from_str("generated/city-of-cape-town-area-10.csv").unwrap(),
                    PathBuf::from_str("generated/western-cape-stellenbosch.csv").unwrap(),
                    PathBuf::from_str("generated/western-cape-darling.csv.csv").unwrap(),
                ];
                assert!(err_if_overlaps(&changes, &paths).is_ok())
            }

            #[test]
            fn ok_if_empty_vecs() {
                let changes = vec![];
                let paths = vec![];
                assert!(err_if_overlaps(&changes, &paths).is_ok())
            }
        }
    }

    mod gen_datetimes {
        use crate::tests::rfc3339;
        use chrono::NaiveTime;

        use crate::gen_datetimes;

        #[test]
        fn dom_different_to_start_date() {
            let start_dt = rfc3339("2022-01-02T00:00:00+02:00");
            let finsh_dt = rfc3339("2022-01-02T01:00:00+02:00");
            let dom = 1;
            let start_time = NaiveTime::from_hms_opt(23, 30, 0).unwrap();
            let finsh_time = NaiveTime::from_hms_opt(0, 30, 0).unwrap();
            let datetimes = gen_datetimes(start_dt, finsh_dt, dom, start_time, finsh_time);
            assert_eq!(
                datetimes,
                vec![(
                    rfc3339("2022-01-02T00:00:00+02:00"),
                    rfc3339("2022-01-02T00:30:00+02:00")
                )]
            );
        }

        #[test]
        fn crosses_finsh_boundary() {
            let start_dt = rfc3339("2022-01-01T10:00:00+02:00");
            let finsh_dt = rfc3339("2022-01-01T20:00:00+02:00");
            let dom = 1;
            let start_time = NaiveTime::from_hms_opt(19, 0, 0).unwrap();
            let finsh_time = NaiveTime::from_hms_opt(21, 0, 0).unwrap();
            let datetimes = gen_datetimes(start_dt, finsh_dt, dom, start_time, finsh_time);
            assert_eq!(
                datetimes,
                vec![(
                    rfc3339("2022-01-01T19:00:00+02:00"),
                    rfc3339("2022-01-01T20:00:00+02:00")
                )]
            );
        }

        #[test]
        fn crosses_start_boundary() {
            let start_dt = rfc3339("2022-01-01T10:00:00+02:00");
            let finsh_dt = rfc3339("2022-01-01T20:00:00+02:00");
            let dom = 1;
            let start_time = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
            let finsh_time = NaiveTime::from_hms_opt(11, 0, 0).unwrap();
            let datetimes = gen_datetimes(start_dt, finsh_dt, dom, start_time, finsh_time);
            assert_eq!(
                datetimes,
                vec![(
                    rfc3339("2022-01-01T10:00:00+02:00"),
                    rfc3339("2022-01-01T11:00:00+02:00")
                )]
            );
        }

        #[test]
        fn thirty_first() {
            let start_dt = rfc3339("2022-01-01T00:00:00+02:00");
            let finsh_dt = rfc3339("2023-01-01T00:00:00+02:00");
            let dom = 31;
            let start_time = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
            let finsh_time = NaiveTime::from_hms_opt(14, 0, 0).unwrap();
            let datetimes = gen_datetimes(start_dt, finsh_dt, dom, start_time, finsh_time);
            assert_eq!(
                datetimes,
                vec![
                    (
                        rfc3339("2022-01-31T12:00:00+02:00"),
                        rfc3339("2022-01-31T14:00:00+02:00")
                    ),
                    (
                        rfc3339("2022-03-31T12:00:00+02:00"),
                        rfc3339("2022-03-31T14:00:00+02:00")
                    ),
                    (
                        rfc3339("2022-05-31T12:00:00+02:00"),
                        rfc3339("2022-05-31T14:00:00+02:00")
                    ),
                    (
                        rfc3339("2022-07-31T12:00:00+02:00"),
                        rfc3339("2022-07-31T14:00:00+02:00")
                    ),
                    (
                        rfc3339("2022-08-31T12:00:00+02:00"),
                        rfc3339("2022-08-31T14:00:00+02:00")
                    ),
                    (
                        rfc3339("2022-10-31T12:00:00+02:00"),
                        rfc3339("2022-10-31T14:00:00+02:00")
                    ),
                    (
                        rfc3339("2022-12-31T12:00:00+02:00"),
                        rfc3339("2022-12-31T14:00:00+02:00")
                    )
                ]
            );
        }

        #[test]
        fn multiple_months() {
            let start_dt = rfc3339("2022-01-01T00:00:00+02:00");
            let finsh_dt = rfc3339("2023-01-01T00:00:00+02:00");
            let dom = 1;
            let start_time = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
            let finsh_time = NaiveTime::from_hms_opt(14, 0, 0).unwrap();
            let datetimes = gen_datetimes(start_dt, finsh_dt, dom, start_time, finsh_time);
            assert_eq!(
                datetimes,
                vec![
                    (
                        rfc3339("2022-01-01T12:00:00+02:00"),
                        rfc3339("2022-01-01T14:00:00+02:00")
                    ),
                    (
                        rfc3339("2022-02-01T12:00:00+02:00"),
                        rfc3339("2022-02-01T14:00:00+02:00")
                    ),
                    (
                        rfc3339("2022-03-01T12:00:00+02:00"),
                        rfc3339("2022-03-01T14:00:00+02:00")
                    ),
                    (
                        rfc3339("2022-04-01T12:00:00+02:00"),
                        rfc3339("2022-04-01T14:00:00+02:00")
                    ),
                    (
                        rfc3339("2022-05-01T12:00:00+02:00"),
                        rfc3339("2022-05-01T14:00:00+02:00")
                    ),
                    (
                        rfc3339("2022-06-01T12:00:00+02:00"),
                        rfc3339("2022-06-01T14:00:00+02:00")
                    ),
                    (
                        rfc3339("2022-07-01T12:00:00+02:00"),
                        rfc3339("2022-07-01T14:00:00+02:00")
                    ),
                    (
                        rfc3339("2022-08-01T12:00:00+02:00"),
                        rfc3339("2022-08-01T14:00:00+02:00")
                    ),
                    (
                        rfc3339("2022-09-01T12:00:00+02:00"),
                        rfc3339("2022-09-01T14:00:00+02:00")
                    ),
                    (
                        rfc3339("2022-10-01T12:00:00+02:00"),
                        rfc3339("2022-10-01T14:00:00+02:00")
                    ),
                    (
                        rfc3339("2022-11-01T12:00:00+02:00"),
                        rfc3339("2022-11-01T14:00:00+02:00")
                    ),
                    (
                        rfc3339("2022-12-01T12:00:00+02:00"),
                        rfc3339("2022-12-01T14:00:00+02:00")
                    )
                ]
            );
        }

        #[test]
        fn crosses_year_boundary() {
            let start_dt = rfc3339("2022-12-31T00:00:00+02:00");
            let finsh_dt = rfc3339("2023-01-02T00:00:00+02:00");
            let dom = 1;
            let start_time = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
            let finsh_time = NaiveTime::from_hms_opt(14, 0, 0).unwrap();
            let datetimes = gen_datetimes(start_dt, finsh_dt, dom, start_time, finsh_time);
            assert_eq!(
                datetimes,
                vec![(
                    rfc3339("2023-01-01T12:00:00+02:00"),
                    rfc3339("2023-01-01T14:00:00+02:00")
                ),]
            );
        }

        #[test]
        fn crosses_month_boundary() {
            let start_dt = rfc3339("2022-01-31T00:00:00+02:00");
            let finsh_dt = rfc3339("2022-02-02T00:00:00+02:00");
            let dom = 1;
            let start_time = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
            let finsh_time = NaiveTime::from_hms_opt(14, 0, 0).unwrap();
            let datetimes = gen_datetimes(start_dt, finsh_dt, dom, start_time, finsh_time);
            assert_eq!(
                datetimes,
                vec![(
                    rfc3339("2022-02-01T12:00:00+02:00"),
                    rfc3339("2022-02-01T14:00:00+02:00")
                ),]
            );
        }

        #[test]
        fn crosses_midnight() {
            let start_dt = rfc3339("2022-01-01T00:00:00+02:00");
            let finsh_dt = rfc3339("2022-04-01T00:00:00+02:00");
            let dom = 1;
            let start_time = NaiveTime::from_hms_opt(22, 0, 0).unwrap();
            let finsh_time = NaiveTime::from_hms_opt(0, 30, 0).unwrap();
            let datetimes = gen_datetimes(start_dt, finsh_dt, dom, start_time, finsh_time);
            assert_eq!(
                datetimes,
                vec![
                    (
                        rfc3339("2022-01-01T22:00:00+02:00"),
                        rfc3339("2022-01-02T00:30:00+02:00")
                    ),
                    (
                        rfc3339("2022-02-01T22:00:00+02:00"),
                        rfc3339("2022-02-02T00:30:00+02:00")
                    ),
                    (
                        rfc3339("2022-03-01T22:00:00+02:00"),
                        rfc3339("2022-03-02T00:30:00+02:00")
                    )
                ]
            );
        }
    }
}
