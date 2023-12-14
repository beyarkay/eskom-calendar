use chrono::{DateTime, Datelike, Duration, NaiveTime, Timelike};
use chrono::{Days, FixedOffset};
use icalendar::{Calendar, EventLike};
use rayon::prelude::*;
use regex::Regex;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::ops::Sub;
use std::path::{Path, PathBuf};
use std::process::Command;
use structs::{Args, Change, ManuallyInputSchedule, PowerOutage, Recurrence, RecurringShedding};

#[cfg(test)]
mod tests;

extern crate pretty_env_logger;
use log::{error, info, trace, warn};

use clap::Parser;
mod structs;

type BoxedError = Box<dyn Error + Sync + Send>;

/// Download pdfs if the parsed CSVs don't already exist, and use them to create `ics` files.
fn main() -> Result<(), BoxedError> {
    pretty_env_logger::init();

    // Some of the areas have expired and been replaced with outers. For now, it's just some
    // eThekwini suburbs so they can be dealt with in a bit of a hacky manner, although proper
    // methods will have to be implemented when more and more municipalities start updating their
    // schedules
    let expired = vec![
        "kwazulu-natal-ethekwini-block-1",
        "kwazulu-natal-ethekwini-block-10",
        "kwazulu-natal-ethekwini-block-11",
        "kwazulu-natal-ethekwini-block-12",
        "kwazulu-natal-ethekwini-block-13",
        "kwazulu-natal-ethekwini-block-14",
        "kwazulu-natal-ethekwini-block-15",
        "kwazulu-natal-ethekwini-block-16",
        "kwazulu-natal-ethekwini-block-2",
        "kwazulu-natal-ethekwini-block-3",
        "kwazulu-natal-ethekwini-block-4",
        "kwazulu-natal-ethekwini-block-5",
        "kwazulu-natal-ethekwini-block-6",
        "kwazulu-natal-ethekwini-block-7",
        "kwazulu-natal-ethekwini-block-8",
        "kwazulu-natal-ethekwini-block-9",
    ];
    let expired_at = DateTime::parse_from_rfc3339("2023-05-25T00:00:00.000000+02:00").unwrap();

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
    let paths_and_outages: Vec<(&PathBuf, Vec<PowerOutage>)> = filtered_paths
        .par_iter()
        // Convert the paths to (path, shedding) tuples
        .map(|path| (path, read::read_sheddings_from_csv_path(path)))
        // Exclude all sheddings which failed
        .filter_map(|(path, sheddings)| match sheddings {
            Ok(sheddings) => Some((path, sheddings)),
            Err(e) => {
                error!("Error while reading CSV {:?}: {}", path, e);
                None
            }
        })
        // Exclude all sheddings which can't be converted to CsvLines
        .filter_map(|(path, sheddings)| {
            let area_name = fmt::path_to_area_name(path).unwrap();
            match calculate_power_outages(&area_name, sheddings, &manually_specified) {
                Ok((outages, last_finsh)) => Some((path, outages, last_finsh)),
                Err(e) => {
                    error!(
                        "Error while calculating power outages for {}: {}",
                        area_name, e
                    );
                    None
                }
            }
        })
        // Some of the schedules are out of date. Exclude them.
        .map(|(path, outages, last_finsh)| {
            let new_outages: Vec<PowerOutage> = outages.into_iter().filter(|outage| {
                if expired.clone().contains(&outage.area_name.as_str()) && outage.start >= expired_at {
                    info!(
                        "Area {} expires at {} but outage starts at {:?}, not creating events for it",
                        outage.area_name, expired_at, outage.start
                        );
                    return false
                }
                true
            }).collect();
            (path, new_outages, last_finsh)
        })
        // Write the individual sheddings to ICS files
        .map(|(path, mut outages, last_finsh)| {
            if args.output_ics_files {
                write_sheddings_to_ics(path, &mut outages, last_finsh, expired.clone(), expired_at).unwrap();
            }
            (path, outages)
        })
        .collect();

    let mut csv_lines = paths_and_outages
        .into_par_iter()
        .flat_map(|(_p, o)| o)
        .collect();

    if args.output_csv_file {
        // Write the lines to a CSV
        overwrite_lines_to_csv(&mut csv_lines, "machine_friendly.csv".to_owned())?;
        // NOTE: Hacky solution for embedded platforms.
        // https://github.com/beyarkay/eskom-calendar/issues/341
        //
        // Microcontrollers don't have the memory/storage to handle a 1.42 MB file, so for now
        // (until an API can be set up), write out just cpt-9 and cpt-15 as their own CSV files so
        // that they can be downloaded individually.
        let custom_areas = vec!["city-of-cape-town-area-9", "city-of-cape-town-area-15"];
        for area in custom_areas {
            overwrite_lines_to_csv(
                &mut csv_lines
                    .iter()
                    .cloned()
                    .filter(|l| l.area_name == area)
                    .collect(),
                format!("{}.csv", area),
            )?;
        }
    }
    Ok(())
}

/// Checks the provided changes for illegal overlaps.
/// For example, specifying Stage 2 from 14h to 16h as well as Stage 3 from 13h to 16h is not
/// allowed.
fn err_if_overlaps(changes: &[Change], paths: &[PathBuf]) -> Result<(), BoxedError> {
    info!("Checking for overlaps...");
    for path in paths {
        let area_name = fmt::path_to_area_name(path)?;
        let regexed_changes = changes
            .par_iter()
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
        .into_par_iter()
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
/// into a list of power outages. Filters out all non-regex matching areas.
fn calculate_power_outages(
    area_name: &str,
    monthly_sheddings: Vec<RecurringShedding>,
    manually_specified: &ManuallyInputSchedule,
) -> Result<(Vec<PowerOutage>, Option<DateTime<FixedOffset>>), BoxedError> {
    let national_changes: Vec<Change> = manually_specified
        .changes
        .clone()
        .into_par_iter()
        .filter(|c| !c.exclude_regex.is_match(area_name) && c.include_regex.is_match(area_name))
        .collect();
    let combos = make_combinations_from_sheddings(&monthly_sheddings, &national_changes);
    if combos.is_empty() {
        warn!("No combinations for possible load shedding in {area_name}\nmonthly sheddings: {}, national changes: {}", monthly_sheddings.len(), national_changes.len());
    } else {
        trace!(
            "Checking {} combinations for possible load shedding in {area_name}",
            combos.len(),
        );
    }
    let mut last_finsh: Option<DateTime<FixedOffset>> = None;
    let mut outages = vec![];

    for (local, natnl) in combos {
        let datetimes = gen_datetimes(
            natnl.start,
            natnl.finsh,
            local.day_of_recurrence,
            local.recurrence,
            local.start_time.time(),
            local.finsh_time.time(),
        );
        for dt in datetimes {
            // Keep track of the last finished event, so that we can add one more event immediately
            // after it
            last_finsh = last_finsh.map_or(Some(dt.1), |le| Some(le.max(dt.1)));
            // Also create a power outage struct and push it onto the vector
            outages.push(PowerOutage {
                area_name: area_name.to_owned(),
                stage: local.stage,
                start: dt.0,
                finsh: dt.1,
                source: natnl.source.clone(),
            })
        }
    }
    Ok((outages, last_finsh))
}

/// Attempt to write some PowerOutages to the specified path as a ICS file.
fn write_sheddings_to_ics(
    path: &Path,
    power_outages: &mut [PowerOutage],
    last_finsh: Option<DateTime<FixedOffset>>,
    expired: Vec<&str>,
    expired_at: DateTime<FixedOffset>,
) -> Result<Calendar, BoxedError> {
    // Get the correct filename
    let fname = path
        .to_str()
        .ok_or("Couldn't convert path to string")?
        .replace("csv", "ics")
        .replace("generated", "calendars")
        .replace(|c: char| !c.is_ascii(), "")
        .replace("&nbsp;", "");

    info!("Writing {} events to {:?}", power_outages.len(), fname);

    // Any shedding duration <= min_duration is not included.
    //
    // NOTE: these events *are* include in the CSV file, but are *not* included in the ICS file.
    // There's no formal notice from any municipality that they won't turn off the power if a
    // loadshedding outage is <=30m long, however this is the widely observed truth. So for the
    // user-facing ICS files, we remove the 30m loadshedding item, but for the machine-facing CSV
    // file, we keep it in.
    let min_duration = Duration::minutes(30);

    let mut calendar = Calendar::new();

    power_outages.sort_by_key(|outage| outage.start);

    // Filter out all the outages which are under 30 minutes long
    let long_enough_outages = power_outages.iter().enumerate().filter(|(i, outage)| {
        let curr_event_long_enough = outage.finsh - outage.start > min_duration;

        // If i == 0, then there's no previous event so they can't collide
        let prev_event_collides = i != &0
            && power_outages
                .get(i - 1)
                .map_or(false, |o| outage.start.sub(o.finsh) <= Duration::minutes(1));

        // If i+1 == power_outages.len, then there's no next event so they can't collide
        let next_event_does_collide = i + 1 != power_outages.len()
            && power_outages
                .get(i + 1)
                .map_or(false, |o| o.start.sub(outage.finsh) <= Duration::minutes(1));

        // trace!("{curr_event_long_enough} || ({prev_event_collides} || {next_event_does_collide})");
        (curr_event_long_enough) || ((prev_event_collides) || (next_event_does_collide))
    });

    // Add all the long enough events to the calendar
    for (_i, outage) in long_enough_outages {
        // Convert the outage to an event, and add it to the calendar
        calendar.push(fmt::power_outage_to_event(outage)?);
    }

    let mut is_expired_ics = false;
    if let Ok(area_name) = fmt::path_to_area_name(path) {
        if expired.contains(&area_name.as_str()) {
            calendar.push(fmt::expired_schedule_event(&area_name, expired_at)?);
            is_expired_ics = true;
            // If the calendar has expired in the past, it's possible a new user might not see the
            // warning. So add another warning on the same day as compilation, to make sure.
            if expired_at.checked_add_days(Days::new(1)).unwrap() < chrono::offset::Local::now() {
                info!(
                    "Writing expired event because {area_name} in expired && {expired_at:?} < {:?}",
                    chrono::offset::Local::now()
                );
                let event = fmt::expired_schedule_event(&area_name, expired_at)?
                    .all_day(chrono::offset::Local::now().date_naive())
                    .done();
                calendar.push(event);
            }
        }
    }
    // TODO There are no tests to check that an end-of-schedule event is being added properly
    // If we have >0 events, add one final event specifying that there's no more loadshedding
    // information after here.
    if let Some(last_finsh) = last_finsh {
        if !is_expired_ics {
            calendar.push(fmt::end_of_schedule_event(
                last_finsh,
                fmt::path_to_area_name(path).ok(),
            )?);
        }
    }

    // Write all the data to disk
    let mut file = File::create(fname.as_str())?;
    writeln!(&mut file, "{}", calendar)?;

    Ok(calendar)
}

/// Given a list of power outages, convert them to a single CSV file for machine consumption.
fn overwrite_lines_to_csv(
    power_outages: &mut Vec<PowerOutage>,
    path: String,
) -> Result<(), BoxedError> {
    // Create the file (overwriting if it exists)
    let mut file = File::create(format!("calendars/{path}"))?;
    info!("Writing {}+1 lines to {file:?}", power_outages.len());
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
fn get_git_hash() -> Result<String, BoxedError> {
    Ok(String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()?
            .stdout,
    )?)
}

/// Given some monthly shedding data and some national shedding data, calculate all the ways they
/// could be combined while ensuring the stages are equal. Does not check that the regex matches
fn make_combinations_from_sheddings(
    monthly_sheddings: &[RecurringShedding],
    national_sheddings: &[Change],
) -> Vec<(RecurringShedding, Change)> {
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
    lcl_dor: u8,
    lcl_recurrence: Recurrence,
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
        // Ensure each range starts on the correct date of the recurrence
        .filter(|(start, _finsh)| match lcl_recurrence {
            Recurrence::Weekly => start.weekday().number_from_monday() == lcl_dor as u32,
            Recurrence::Monthly => start.day() == lcl_dor as u32,
            Recurrence::Periodic { offset, period } => {
                assert!(lcl_dor <= period);
                let duration = start.date_naive().sub(offset);
                duration.num_days() % period as i64 == (lcl_dor - 1) as i64
            }
        })
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
    use crate::BoxedError;
    use chrono::Duration;
    use chrono::FixedOffset;
    use chrono::{DateTime, Utc};
    use icalendar::Alarm;
    use icalendar::EventLike;
    use icalendar::{Component, Event};
    use std::path::Path;

    use crate::structs::PowerOutage;

    /// Format a path as an area name: remove the extension and the `generated/` directory. This fails
    /// if the path isn't valid.
    pub fn path_to_area_name(path: &Path) -> Result<String, BoxedError> {
        Ok(path
            .to_str()
            .ok_or("Path is not valid unicode")?
            .replace("generated/", "")
            .replace(".csv", "")
            .replace(|c: char| !c.is_ascii(), ""))
    }

    /// Convert a power outage to a ICS calendar event, with a nicely formatted description.
    pub fn power_outage_to_event(power_outage: &PowerOutage) -> Result<Event, BoxedError> {
        let timezone_sast = FixedOffset::east_opt(2 * 60 * 60).unwrap();
        let now = DateTime::<FixedOffset>::from_local(
            chrono::offset::Local::now().naive_local(),
            timezone_sast,
        );

        // Get a nice URL link to the exact run which created this calendar (if the run even
        // exists)
        let github_run_url = if let Ok(run_id) = std::env::var("GITHUB_RUN_ID") {
            // And infer the repo name from the ENV variables, because sometimes this code is run
            // on the development repository `beyarkay/eskom-calendar-dev`
            let owner_repo = std::env::var("GITHUB_REPOSITORY")
                .unwrap_or_else(|_| "beyarkay/eskom-calendar".to_owned());
            format!(" by run https://github.com/{owner_repo}/actions/runs/{run_id}")
        } else {
            "".to_string()
        };

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
            Outlook) to fetch the updated schedules. Often you can set the update \
            frequency in the settings of your calendar app. \n\
            \n\
            --- \n\
            \n\
            Incorrect? Open an issue here: https://github.com/beyarkay/eskom-calendar/issues/new\n\
            \n\
            Generated by Boyd Kane's eskom-calendar: https://eskomcalendar.co.za/ec?calendar={}.ics.\n\
            \n\
            National loadshedding information scraped from {}.\n\
            \n\
            Calendar compiled {}{}.\n\
            \n\
            eskom-calendar version: https://github.com/beyarkay/eskom-calendar/tree/{}",
            power_outage.start.format("%A from %H:%M"),
            power_outage.finsh.format("%A at %H:%M"),
            power_outage.area_name,
            power_outage.area_name,
            power_outage.source,
            now.format("on %A %d %h %Y at %H:%M:%S (UTC+02:00)"),
            github_run_url,
            get_git_hash()?,
        );
        // These emojis are for stages:
        //                  0     1     2    3     4     5     6     7     8
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
            .alarm(Alarm::display(
                &format!("In 1 hour: {}", summary),
                -Duration::hours(1),
            ))
            .done();
        Ok(evt)
    }

    /// Create an event that signals that the schedule has expired
    pub fn expired_schedule_event(
        area_name: &str,
        expired_at: DateTime<FixedOffset>,
    ) -> Result<Event, BoxedError> {
        let description = format!(
            "The schedule for {area_name} has expired, and is not valid after {fmt_date}.\n\
            \n\
            A schedule expires when the municipality in charge of your area updates their \
            loadshedding schedules. This might mean that you end up in a different loadshedding \
            block/group, or you might get loadshedding at different times than you used to. \n\
            \n\
            Once the expiry date has passed, this calendar feed will stop being updated. You need \
            to subscribe to the new calendar feed if you want to keep up-to-date.\n\
            \n\
            You can find your new schedule by going to https://eskomcalendar.co.za and searching \
            for either '{area_name}', or for the suburb you're in.\n\
            \n\
            Often the new blocks are similar to the old blocks. For example, try:\n\
            - https://eskomcalendar.co.za/ec/?calendar={area_name}a.ics \n\
            - https://eskomcalendar.co.za/ec/?calendar={area_name}b.ics \n\
            \n\
            --- \n\
            Generated by Boyd Kane's eskom-calendar: https://github.com/beyarkay/eskom-calendar/tree/{git_hash} \n\
            Calendar compiled at {compiletime:?}",
            fmt_date=expired_at.format("%-d %B %Y"),
            git_hash=get_git_hash()?,
            compiletime=chrono::offset::Local::now(),
        );

        Ok(Event::new()
            .all_day(expired_at.date_naive())
            .summary("❌ Schedule expired")
            .description(&description)
            .done())
    }

    /// Create an event that signals the end of known loadshedding data
    pub fn end_of_schedule_event(
        last_finsh: DateTime<FixedOffset>,
        area_name: Option<String>,
    ) -> Result<Event, BoxedError> {
        let website_link = if let Some(area_name) = area_name {
            format!("To check for the most up-to-date schedule, go to the website: https://eskomcalendar.co.za/ec?calendar={area_name}.ics. ")
        } else {
            String::new()
        };
        let description = format!(
            "This is the end of the known loadshedding schedule.\n\
            \n\
            Only a few days worth of loadshedding schedules are released at a time \
            and most calendar apps (Google Calendar, Outlook, Apple Calendar) don't update \
            immediately. {website_link}This event will automatically get moved into the future when \
            your calendar app updates.\n\
            \n\
            If you're comfortable with Google Apps Script, there is an alternative \
            solution which will force Google Calendar to update: https://github.com/derekantrican/GAS-ICS-Sync\n\
            \n\
            --- \n\
            Generated by Boyd Kane's eskom-calendar: https://github.com/beyarkay/eskom-calendar/tree/{git_hash} \n\
            Calendar compiled at {compiletime:?}",
            git_hash=get_git_hash()?,
            compiletime=chrono::offset::Local::now(),
        );

        let start = last_finsh.with_timezone(&Utc);
        let end = last_finsh
            .checked_add_signed(chrono::Duration::minutes(1))
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
        ManuallyInputSchedule, RawManuallyInputSchedule, RawMonthlyShedding, RawPeriodicShedding,
        RawWeeklyShedding, RecurringShedding,
    };
    use crate::BoxedError;
    use std::fs::read_to_string;
    use std::path::PathBuf;

    extern crate pretty_env_logger;

    use log::{info, trace};

    /// Get the paths of all CSVs in `dir`.
    pub fn get_csv_paths(dir: &str) -> Result<Vec<PathBuf>, BoxedError> {
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
    pub fn read_manually_specified(path: &str) -> Result<ManuallyInputSchedule, BoxedError> {
        Ok(
            serde_yaml::from_str::<RawManuallyInputSchedule>(read_to_string(path)?.as_str())?
                .into(),
        )
    }

    /// Read in the load shedding information from the provided path.
    pub fn read_sheddings_from_csv_path(
        path: &PathBuf,
    ) -> Result<Vec<RecurringShedding>, BoxedError> {
        let mut reader = csv::Reader::from_path(path)?;
        let headers = reader.headers()?;
        let raw: Vec<RecurringShedding>;

        // Parse the CSV file in a manner that depends on the headers
        if headers.iter().any(|h| h == "date_of_month") {
            raw = reader
                .deserialize::<RawMonthlyShedding>()
                .map(|res| Into::<RecurringShedding>::into(res.unwrap()))
                .collect::<Vec<_>>();
        } else if headers.iter().any(|h| h == "day_of_week") {
            raw = reader
                .deserialize::<RawWeeklyShedding>()
                .map(|res| Into::<RecurringShedding>::into(res.unwrap()))
                .collect::<Vec<_>>();
        } else if headers.iter().any(|h| h == "day_of_20_day_cycle") {
            raw = reader
                .deserialize::<RawPeriodicShedding>()
                .map(|res| Into::<RecurringShedding>::into(res.unwrap()))
                .collect::<Vec<_>>();
        } else {
            return Err(Box::from(format!("Could not parse headers {:?}", headers)));
        }

        let local_shedding = raw
            .into_iter()
            .filter(|shedding| shedding.stage != 0)
            .collect::<Vec<RecurringShedding>>();
        Ok(local_shedding)
    }
}
