use chrono::FixedOffset;
use chrono::{DateTime, Datelike, Duration, NaiveDate, Timelike, Utc};
use icalendar::{Calendar, Component, Event};
use scraper::Html;
use scraper::Selector;
use serde_yaml;
use std::fs::{read_dir, read_to_string, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use structs::{
    ManuallyInputSchedule, MonthlyShedding, RawManuallyInputSchedule, RawMonthlyShedding, Shedding,
};
#[macro_use]
extern crate colour;

mod structs;

/// Download pdfs if the parsed CSVs don't already exist, and use them to create `ics` files.
fn main() {
    // Download all the pdfs from the internet
    // let args: Vec<String> = env::args().collect();
    // Up to a limit that can be set via cmdline args (mainly used for testing)
    // let limit = if args.get(1).is_some() {
    //     Some(args[1].parse().expect("Failed to parse env::args()[0]"))
    // } else {
    //     None
    // };
    // Define the URLs which list the PDFs per-province.
    // let source_urls = vec![
    //     "https://www.eskom.co.za/distribution/customer-service/outages/municipal-loadshedding-schedules/eastern-cape/",
    //     "https://www.eskom.co.za/distribution/customer-service/outages/municipal-loadshedding-schedules/free-state/",
    //     "https://www.eskom.co.za/distribution/customer-service/outages/municipal-loadshedding-schedules/gauteng/",
    //     "https://www.eskom.co.za/distribution/customer-service/outages/municipal-loadshedding-schedules/kwazulu-natal/",
    //     "https://www.eskom.co.za/distribution/customer-service/outages/municipal-loadshedding-schedules/limpopo/",
    //     "https://www.eskom.co.za/distribution/customer-service/outages/municipal-loadshedding-schedules/mpumalanga/",
    //     "https://www.eskom.co.za/distribution/customer-service/outages/municipal-loadshedding-schedules/north-west/",
    //     "https://www.eskom.co.za/distribution/customer-service/outages/municipal-loadshedding-schedules/northern-cape/",
    //     "https://www.eskom.co.za/distribution/customer-service/outages/municipal-loadshedding-schedules/western-cape/",
    // ];

    // Download the PDFs for each province.
    // for url in source_urls {
    //     dl_pdfs(url, limit);
    // }

    // Get the paths of the csv files generated
    let csv_paths = read_dir("generated/")
        .expect("Failed to read_dir(\"generated/\")")
        .filter_map(|f| {
            if let Ok(entry) = f {
                if let Some(extension) = entry.path().extension() {
                    if extension == "csv" {
                        return Some(entry.path());
                    }
                }
            }
            None
        })
        .collect::<Vec<PathBuf>>();

    // Get the loadshedding schedule as defined in `manually_specified.yaml`
    let mis: ManuallyInputSchedule = serde_yaml::from_str::<RawManuallyInputSchedule>(
        read_to_string("manually_specified.yaml")
            .expect("Failed to read_to_string(manually_specified.yaml)")
            .as_str(),
    )
    .expect("Failed to convert string to yaml")
    .into();

    // Write the header line of the csv file
    let mut file = File::create("calendars/machine_friendly.csv").expect("Failed to create file ");
    writeln!(&mut file, "{}", "area_name,start,finsh,stage,source")
        .expect(format!("Failed to write to file {:?}", file).as_str());

    // Convert the csv files to ics files, taking the intersection of the load shedding events and
    // the manually input loadshedding schedules
    eprintln!("Creating {} calendars", csv_paths.len());
    for path in csv_paths {
        // if !path.to_str().unwrap().starts_with("generated/city-power-") {
        //     continue;
        // }
        create_calendar(
            path.to_str()
                .expect(format!("Failed to convert {:?} to str", path).as_str())
                .to_string(),
            &mis,
        );
    }
}

/// Given a url, download the pdfs from that url that match the css selector `div>div>p>span>a` and
/// convert them via a python script to csv file containing load shedding schedules.
fn dl_pdfs(url: &str, limit: Option<usize>) {
    blue_ln!(" Getting province PDFs from {}", url);
    let val = reqwest::blocking::get(url).expect(format!("Failed to get url {url}").as_str());
    let html = val.text().unwrap();
    let document = Html::parse_document(&html);

    let mut areas_hrefs = vec![];
    let selector1 = Selector::parse("div>div>p>span>a").unwrap();
    let selector2 = Selector::parse("div>div>p>a").unwrap();
    let limit = limit.unwrap_or(0)
        + document.select(&selector1).count()
        + document.select(&selector2).count();

    eprintln!(
        "  Parsing {} links ({} + {})",
        document.select(&selector2).count() + document.select(&selector1).count(),
        document.select(&selector1).count(),
        document.select(&selector2).count(),
    );
    // First go through the HTML elements which match selector1
    for element in document.select(&selector1) {
        let area = element.first_child().unwrap().value().as_text();
        let href = element.value().attr("href");
        areas_hrefs.push((&area.unwrap().text, href));
    }
    // Second go through the HTML elements which match selector2
    for element in document.select(&Selector::parse("div>div>p>a").unwrap()) {
        let mut area = element
            .first_child()
            .unwrap()
            .first_child()
            .and_then(|c| c.value().as_text());
        if area.is_none() {
            area = element.first_child().unwrap().value().as_text();
        }
        let href = element.value().attr("href");
        areas_hrefs.push((&area.unwrap().text, href));
    }
    let areas_hrefs = areas_hrefs
        .into_iter()
        .filter(|(_a, h)| {
            h.is_some()
                && h.unwrap()
                    .starts_with("https://www.eskom.co.za/distribution/wp-content/uploads")
        })
        .map(|(a, h)| (a, h.unwrap()))
        .collect::<Vec<_>>();

    let mut pdfs_scraped = 0;
    let mut handles = vec![];
    for (area, href) in areas_hrefs {
        if pdfs_scraped >= limit {
            eprintln!("Reached limit of {limit} URLs, stopping scraping");
            break;
        }
        let fname = area
            .replace(":", "")
            .replace(" ", "")
            .replace(|c: char| !c.is_ascii(), "");
        let province = url.split("/").collect::<Vec<_>>()[7];
        let savename = format!("{province}-{fname}").to_lowercase();
        // Don't bother downloading the pdf if the resultant CSV already exists
        if Path::new(format!("generated/{savename}.csv").as_str()).exists() {
            continue;
        }
        eprintln!("   $ python3 src/parse_eskom.py {href:<90} {savename:<20}");
        let handle = Command::new("python3")
            .args(["src/parse_eskom.py", href, &savename])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to execute command");
        handles.push(handle);
        if handles.len() > 50 {
            match handles.pop().unwrap().wait_with_output() {
                Ok(res) => {
                    if !res.status.success() {
                        red_ln!(
                            "   Error with async python process: \n   stdout: {}\n   stderr: {}",
                            String::from_utf8_lossy(&res.stdout),
                            String::from_utf8_lossy(&res.stderr).replace("\n", "\n    ")
                        );
                        grey_ln!("{:?}", res.stderr);
                    }
                }
                Err(e) => {
                    red_ln!("   Error with python process: {}", e);
                }
            }
        }
        pdfs_scraped += 1;
    }
    // Go through all the python processes and wait for them to finish
    for handle in handles {
        match handle.wait_with_output() {
            Ok(res) => {
                if !res.status.success() {
                    red_ln!(
                        "   Error with async python process: \n   stdout: {}\n   stderr: {}",
                        String::from_utf8_lossy(&res.stdout),
                        String::from_utf8_lossy(&res.stderr).replace("\n", "\n    ")
                    );
                }
            }
            Err(e) => {
                red_ln!("   Error waiting for python process: {}", e);
            }
        }
    }
}

/// Create a single loadshedding calendar given one area's csv data
fn create_calendar(csv_path: String, mis: &ManuallyInputSchedule) {
    let mut rdr = csv::Reader::from_path(csv_path.clone())
        .expect(format!("Failed to parse csv at {csv_path}").as_str());
    let mut local_sheddings: Vec<MonthlyShedding> = vec![];
    let mut csv_lines = vec![];
    let mut calendar = Calendar::new();
    for result in rdr.deserialize::<RawMonthlyShedding>() {
        let shedding: MonthlyShedding = result.unwrap().into();
        if shedding.stage == 0 {
            continue;
        }
        local_sheddings.push(MonthlyShedding {
            start_time: shedding.start_time.clone(),
            finsh_time: shedding.finsh_time.clone(),
            stage: shedding.stage,
            date_of_month: shedding.date_of_month,
            goes_over_midnight: shedding.goes_over_midnight,
        });
    }

    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    let emojis = vec!["😁", "😕", "☹️", "😟", "😣", "😭", "😫", "😤", "😡"];

    let area_name = csv_path
        .replace("generated/", "")
        .replace(".csv", "")
        .replace(|c: char| !c.is_ascii(), "");
    // eprintln!("{}", area_name);
    // for national in &mis.changes {
    //     eprintln!("National {:?}", national);
    // }
    let national_changes: Vec<&Shedding> = mis
        .changes
        .iter()
        .filter(|c| !c.exclude_regex.is_match(&area_name) && c.include_regex.is_match(&area_name))
        .collect();
    panic_if_changes_overlap(&national_changes);
    // for national in &national_changes {
    //     eprintln!("Without Overlap {:?}", national);
    // }

    let mut last_event: Option<DateTime<FixedOffset>> = None;
    for national in national_changes {
        // If the local loadshedding matches the include_regex and exclude_regex and the stage
        // is correct, then add the loadshedding

        blue!(
            "Creating calendar for {:?} from {:?} to {:?}",
            csv_path,
            national.start,
            national.finsh,
        );
        for local in &local_sheddings {
            let summary = format!(
                "🔌{area_name} Stage {stage} {emoji}",
                area_name = prettify_area_name(&area_name),
                stage = local.stage,
                emoji = emojis.get(local.stage as usize).unwrap_or(&"🫠"),
            );

            if national.stage == local.stage {
                let mut dt = national.start.clone();
                while national.finsh > dt {
                    let year = dt.year();
                    let month = dt.month();
                    // println!("Calculating loadshedding for the month {year}-{month:<02}");

                    // Get the number of days in the month by comparing this month's first to the next month's first
                    let days_in_month = if month == 12 {
                        NaiveDate::from_ymd(year + 1, 1, 1)
                    } else {
                        NaiveDate::from_ymd(year, month + 1, 1)
                    }
                    .signed_duration_since(NaiveDate::from_ymd(year, month, 1))
                    .num_days() as u8;
                    // Don't create events on the 31st of February
                    if local.date_of_month > days_in_month {
                        // yellow_ln!(
                        //     "Not creating event because date ({}) > days in month ({})",
                        //     local.date_of_month,
                        //     days_in_month
                        // );
                        let old_month = dt.month();
                        while old_month == dt.month() {
                            dt = dt + Duration::days(1);
                        }
                        continue;
                    }
                    let l_start = format!(
                        "{year}-{month:02}-{date:02}T{hour:02}:{minute:02}:00+02:00",
                        year = year,
                        month = month,
                        date = local.date_of_month,
                        hour = local.start_time.hour(),
                        minute = local.start_time.minute(),
                    );
                    let local_start = DateTime::parse_from_rfc3339(l_start.as_str())
                        .expect(format!("Failed to parse time {l_start} as RFC3339").as_str());

                    let l_finsh = format!(
                        "{year}-{month:02}-{date:02}T{hour:02}:{minute:02}:00+02:00",
                        year = year,
                        month = month,
                        date = local.date_of_month,
                        hour = local.finsh_time.hour(),
                        minute = local.finsh_time.minute(),
                    );
                    let mut local_finsh = DateTime::parse_from_rfc3339(l_finsh.as_str())
                        .expect(format!("Failed to parse time {l_finsh} as RFC3339").as_str());

                    // If the event goes over midnight, then add one day to the end date
                    if local.goes_over_midnight {
                        local_finsh = local_finsh + Duration::days(1);
                    }
                    if national.start < local_finsh && national.finsh > local_start {
                        let description = format!(
                            "{area_name} loadshedding: \n\
                            - from {local_start} \n\
                            - to {local_finsh} \n\
                            \n\
                            National loadshedding: \n\
                            - from {nat_start} \n\
                            - to {nat_finsh} \n\
                            \n\
                            Incorrect? Make a suggestion here: https://github.com/beyarkay/eskom-calendar/edit/main/manually_specified.yaml \n\
                            \n\
                            --- \n\
                            Generated by Boyd Kane's Eskom-Calendar: https://github.com/beyarkay/eskom-calendar/tree/{git_hash} \n\
                            \n\
                            National loadshedding information scraped from {nat_source}\n\
                            \n\
                            {area_name} loadshedding information scraped from https://www.eskom.co.za/distribution/customer-service/outages/municipal-loadshedding-schedules/ \n\
                            \n\
                            Calendar compiled at {compiletime:?}",
                            local_start=local_start,
                            local_finsh=local_finsh,
                            nat_start=national.start,
                            nat_finsh=national.finsh,
                            git_hash=git_hash,
                            nat_source=national.source,
                            compiletime=chrono::offset::Local::now(),
                        );
                        let event_start = national.start.max(local_start);
                        let event_finsh = national.finsh.min(local_finsh);
                        let evt = Event::new()
                            .summary(summary.as_str())
                            .description(description.as_str())
                            .starts(event_start.with_timezone(&Utc))
                            .ends(event_finsh.with_timezone(&Utc))
                            .done();
                        csv_lines.push(to_csv_line(
                            &area_name,
                            local.stage,
                            event_start,
                            event_finsh,
                            &national.source,
                        ));
                        calendar.push(evt);
                        // Loadshedding found for this national&local schedule
                        // eprintln!(" Overlap (National {} and Local {}):\n  national: {} to {},\n  local:    {} to {}",
                        //     national.stage, local.stage, national.start, national.finsh, local_start, local_finsh,
                        // );
                        // eprintln!("   Adding:  {} to {}", event_start, event_finsh);
                    } else {
                        // Loadshedding not found for this national&local schedule
                        // red_ln!("national.start({}) < local_finsh({}) && national.finsh({}) > local_start({})",
                        //     national.start , local_finsh , national.finsh , local_start
                        // );
                    }

                    let old_month = dt.month();
                    while old_month == dt.month() {
                        dt = dt + Duration::days(1);
                    }
                }
            }
        }
        if calendar.len() == 0 {
            yellow_ln!(" ({} events)", calendar.len());
        } else {
            blue_ln!(" ({} events)", calendar.len());
        }
        // Keep track of the very last event
        if let Some(event) = last_event {
            last_event = Some(event.max(national.finsh))
        } else {
            last_event = Some(national.finsh)
        }
    }
    // If there is any load shedding scheduled...
    if let Some(last_event) = last_event {
        // Add a final event to the calendar, indicating that there's no information past this
        // point
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
            Incorrect? Make a suggestion here: https://github.com/beyarkay/eskom-calendar/edit/main/manually_specified.yaml \n\
            \n\
            --- \n\
            Generated by Boyd Kane's Eskom-Calendar: https://github.com/beyarkay/eskom-calendar/tree/{git_hash} \n\
            Calendar compiled at {compiletime:?}",
            git_hash=git_hash,
            compiletime=chrono::offset::Local::now(),
        );
        calendar.push(
            Event::new()
                .summary("⚠️  End of schedule")
                .description(&description)
                .starts(last_event.with_timezone(&Utc))
                .ends(last_event.checked_add_signed(chrono::Duration::hours(1)).unwrap().with_timezone(&Utc))
                .done(),
        );
    }

    // Write out the calendar files
    let fname = csv_path
        .replace("csv", "ics")
        .replace("generated", "calendars")
        .replace(|c: char| !c.is_ascii(), "")
        .replace("&nbsp;", "");
    let mut file =
        File::create(fname.as_str()).expect(format!("Failed to create file {}", fname).as_str());
    writeln!(&mut file, "{}", calendar)
        .expect(format!("Failed to write to file {:?}", file).as_str());

    // Write out the CSV file
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open("calendars/machine_friendly.csv")
        .expect("Couldn't open `calendars/machine_friendly.csv` for appending");
    // Sort so we have some kind of consistency
    csv_lines.sort();
    for line in csv_lines {
        writeln!(&mut file, "{}", line)
            .expect(format!("Failed to write to file {:?}", file).as_str());
    }
}

/// Go over the changes, and return true if any of them overlap. Note that the provided `changes`
/// should be from the same calendar.
fn panic_if_changes_overlap(changes: &Vec<&Shedding>) {
    for change1 in changes {
        for change2 in changes {
            // If they're not the same item, but they do overlap, then panic
            if (change1 != change2)
                && ((change1.finsh > change2.start && change1.start < change2.finsh)
                    || (change2.finsh > change1.start && change2.start < change1.finsh))
            {
                panic!("Changes overlap:\nChange1: {change1:#?}\nChange2: {change2:#?}\nYou probably entered invalid information into `manually_specified.yaml`");
            }
        }
    }
}

fn to_csv_line(
    area_name: &str,
    stage: u8,
    start: DateTime<FixedOffset>,
    finsh: DateTime<FixedOffset>,
    source: &str,
) -> String {
    format!("{area_name},{start:?},{finsh:?},{stage},{source:?}")
}

fn to_title_case(s: String) -> String {
    s.split(" ")
        .into_iter()
        .map(|si| {
            // Capitalise the first character
            si.chars().nth(0).unwrap().to_uppercase().to_string()
                // And just join the remaining characters together
                + si.chars().skip(1).map(|c| c.to_string()).reduce(|acc, curr| acc + &curr).unwrap_or("".to_string()).to_string().as_str()
        })
        .collect()
}

fn prettify_area_name(area_name: &str) -> String {
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
        let mut prettified = to_title_case(area_name.replace("-", " "));
        for (prefix, replacement) in prefixes {
            if area_name.starts_with(prefix) {
                prettified = format!(
                    "{} ({})",
                    to_title_case(area_name.replace(prefix, "").replace("-", " ")),
                    replacement
                );
                break;
            }
        }
        prettified
    }
}

#[cfg(test)]
mod tests {
    mod panic_if_changes_overlap {
        use chrono::DateTime;
        use regex::Regex;

        use crate::{panic_if_changes_overlap, structs::Shedding};

        fn make_shedding(start: &str, finsh: &str) -> Shedding {
            Shedding {
                start: DateTime::parse_from_rfc3339(start).unwrap(),
                finsh: DateTime::parse_from_rfc3339(finsh).unwrap(),
                stage: 8,
                source: "No source".to_string(),
                include_regex: Regex::new(".*").unwrap(),
                exclude_regex: Regex::new("matchnothing^").unwrap(),
            }
        }

        #[test]
        #[should_panic]
        fn start_lt_finsh_is_overlap() {
            let change1 = make_shedding("2022-01-01T09:00:00+02:00", "2022-01-01T10:00:00+02:00");
            let change2 = make_shedding("2022-01-01T08:00:00+02:00", "2022-01-01T09:30:00+02:00");
            let changes = vec![&change1, &change2];
            panic_if_changes_overlap(&changes);
        }

        #[test]
        #[should_panic]
        fn finsh_gt_start_is_overlap() {
            let change1 = make_shedding("2022-01-01T08:00:00+02:00", "2022-01-01T09:00:00+02:00");
            let change2 = make_shedding("2022-01-01T08:30:00+02:00", "2022-01-01T10:00:00+02:00");
            let changes = vec![&change1, &change2];
            panic_if_changes_overlap(&changes);
        }

        #[test]
        #[should_panic]
        fn finsh_eq_start_is_overlap() {
            let change1 = make_shedding("2022-01-01T08:00:00+02:00", "2022-01-01T09:00:00+02:00");
            let change2 = make_shedding("2022-01-01T09:00:00+02:00", "2022-01-01T10:00:00+02:00");
            let changes = vec![&change1, &change2];
            panic_if_changes_overlap(&changes);
        }

        #[test]
        #[should_panic]
        fn start_lt_start_and_finsh_gt_finsh_is_overlap() {
            let change1 = make_shedding("2022-01-01T08:00:00+02:00", "2022-01-01T09:00:00+02:00");
            let change2 = make_shedding("2022-01-01T08:15:00+02:00", "2022-01-01T08:45:00+02:00");
            let changes = vec![&change1, &change2];
            panic_if_changes_overlap(&changes);
        }

        #[test]
        fn eq_sheddings_dont_overlap() {
            let change1 = make_shedding("2022-01-01T08:00:00+02:00", "2022-01-01T08:30:00+02:00");
            let change2 = make_shedding("2022-01-01T08:00:00+02:00", "2022-01-01T08:30:00+02:00");
            let changes = vec![&change1, &change2];
            panic_if_changes_overlap(&changes);
        }

        #[test]
        fn start_gt_finsh_no_overlap() {
            let change1 = make_shedding("2022-01-01T09:00:00+02:00", "2022-01-01T09:30:00+02:00");
            let change2 = make_shedding("2022-01-01T08:00:00+02:00", "2022-01-01T08:30:00+02:00");
            let changes = vec![&change1, &change2];
            panic_if_changes_overlap(&changes);
        }

        #[test]
        fn finsh_lt_start_no_overlap() {
            let change1 = make_shedding("2022-01-01T08:00:00+02:00", "2022-01-01T08:30:00+02:00");
            let change2 = make_shedding("2022-01-01T09:00:00+02:00", "2022-01-01T09:30:00+02:00");
            let changes = vec![&change1, &change2];
            panic_if_changes_overlap(&changes);
        }
    }
}
