use chrono::{DateTime, Datelike, Duration, NaiveDate, Timelike, Utc};
use icalendar::{Calendar, Component, Event};
use scraper::Html;
use scraper::Selector;
use serde_yaml;
use std::env;
use std::fs::{read_dir, read_to_string, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use structs::{
    ManuallyInputSchedule, MonthlyShedding, RawManuallyInputSchedule, RawMonthlyShedding,
};

mod structs;

/// Download pdfs if the parsed CSVs don't already exist, and use them to create `ics` files.
fn main() {
    // Download all the pdfs from the internet
    let args: Vec<String> = env::args().collect();
    // Up to a limit that can be set via cmdline args (mainly used for testing)
    let limit = if args.get(1).is_some() {
        Some(args[1].parse().expect("Failed to parse env::args()[0]"))
    } else {
        None
    };
    // Define the URLs which list the PDFs per-province.
    let source_urls = vec![
        "https://www.eskom.co.za/distribution/customer-service/outages/municipal-loadshedding-schedules/western-cape/",
        "https://www.eskom.co.za/distribution/customer-service/outages/municipal-loadshedding-schedules/gauteng/",
        "https://www.eskom.co.za/distribution/customer-service/outages/municipal-loadshedding-schedules/eastern-cape/",
        "https://www.eskom.co.za/distribution/customer-service/outages/municipal-loadshedding-schedules/free-state/",
        "https://www.eskom.co.za/distribution/customer-service/outages/municipal-loadshedding-schedules/limpopo/",
        "https://www.eskom.co.za/distribution/customer-service/outages/municipal-loadshedding-schedules/mpumalanga/",
        "https://www.eskom.co.za/distribution/customer-service/outages/municipal-loadshedding-schedules/north-west/",
        "https://www.eskom.co.za/distribution/customer-service/outages/municipal-loadshedding-schedules/northern-cape/",
    ];

    // Download the PDFs for each province.
    for url in source_urls {
        dl_pdfs(url, limit);
    }

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

    // Convert the csv files to ics files, taking the intersection of the load shedding events and
    // the manually input loadshedding schedules
    eprintln!("Creating {} calendars", csv_paths.len());
    for path in csv_paths {
        // if !path.to_str().unwrap().starts_with("generated/city-power-") {
        //     continue;
        // }
        eprintln!("Creating calendar from {:?}", path);
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
    eprintln!(" Getting links from {url}");
    let val = reqwest::blocking::get(url).expect(format!("Failed to get url {url}").as_str());
    let html = val.text().unwrap();
    let document = Html::parse_document(&html);
    let link_selector = Selector::parse("div>div>p>span>a").unwrap();
    let limit = limit.unwrap_or(document.select(&link_selector).count());
    eprintln!("  Parsing {} links", limit);
    let mut pdfs_scraped = 0;
    let mut handles = vec![];
    for element in document.select(&link_selector) {
        if pdfs_scraped >= limit {
            eprintln!("Reached limit of {limit}, stopping scraping");
            break;
        }
        // If the href exists and starts with eskom.co.za/..../uploads then download and parse it
        // via python

        if let Some(href) = element.value().attr("href") {
            if href.starts_with("https://www.eskom.co.za/distribution/wp-content/uploads") {
                let fname = element
                    .inner_html()
                    .replace(":", "")
                    .replace(" ", "")
                    .replace(|c: char| !c.is_ascii(), "");
                let province = url.split("/").collect::<Vec<_>>()[7];
                let savename = format!("{province}-{fname}").to_lowercase();
                // Don't bother downloading the pdf if the resultant CSV already exists
                if Path::new(format!("generated/{savename}.csv").as_str()).exists() {
                    continue;
                }
                eprintln!("   $ python3 parse_pdf.py {href:<90} {savename:<20}");
                let handle = Command::new("python3")
                    .args(["parse_pdf.py", href, &savename])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("Failed to execute command");
                handles.push(handle);
                if handles.len() > 50 {
                    match handles.pop().unwrap().wait_with_output() {
                        Ok(res) => {
                            if !res.status.success() {
                                eprintln!(
                                    "!  Error with python process: \nstdout: {:?}\nstderr:{:?}",
                                    String::from_utf8_lossy(&res.stdout),
                                    String::from_utf8_lossy(&res.stderr)
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!("!  Error with python process: {e}");
                        }
                    }
                }
                pdfs_scraped += 1;
            } else {
                eprintln!(
                    "!  Cannot parse: {:?} {:?}",
                    element.value().attr("href"),
                    element.inner_html()
                );
            }
        }
    }
    for handle in handles {
        match handle.wait_with_output() {
            Ok(res) => {
                if !res.status.success() {
                    eprintln!(
                        "!  Error with python process: \nstdout: {:?}\nstderr:{:?}",
                        String::from_utf8_lossy(&res.stdout),
                        String::from_utf8_lossy(&res.stderr)
                    );
                }
            }
            Err(e) => {
                eprintln!("!  Error with python process: {e}");
            }
        }
    }
}

/// Create a single loadshedding calendar given one area's csv data
fn create_calendar(csv_path: String, mis: &ManuallyInputSchedule) {
    let mut rdr = csv::Reader::from_path(csv_path.clone())
        .expect(format!("Failed to parse csv at {csv_path}").as_str());
    let mut local_sheddings: Vec<MonthlyShedding> = vec![];
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
    let emojis = vec!["😕", "☹️", "😖", "😤", "😡", "🤬", "🔪", "☠️"];

    // for national in &mis.changes {
    //     eprintln!("{:?}", national);
    // }

    for local in local_sheddings.into_iter() {
        let curr_year = Utc::now().year();
        let curr_month = Utc::now().date();
        // Get the number of days in the month by comparing this month's first to the next month's first
        let days_in_month = if curr_month.month() == 12 {
            NaiveDate::from_ymd(curr_year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd(curr_year, curr_month.month() + 1, 1)
        }
        .signed_duration_since(NaiveDate::from_ymd(curr_year, curr_month.month(), 1))
        .num_days() as u8;
        // Don't create events on the 31st of February
        if local.date_of_month > days_in_month {
            continue;
        }
        let l_start = format!(
            "{year}-{month:02}-{date:02}T{hour:02}:{minute:02}:00+02:00",
            year = curr_year,
            month = curr_month.month(),
            date = local.date_of_month,
            hour = local.start_time.hour(),
            minute = local.start_time.minute(),
        );
        let local_start = DateTime::parse_from_rfc3339(l_start.as_str())
            .expect(format!("Failed to parse time {l_start} as RFC3339").as_str());

        let l_finsh = format!(
            "{year}-{month:02}-{date:02}T{hour:02}:{minute:02}:00+02:00",
            year = curr_year,
            month = curr_month.month(),
            date = local.date_of_month,
            hour = local.finsh_time.hour(),
            minute = local.finsh_time.minute(),
        );
        let mut local_finsh = DateTime::parse_from_rfc3339(l_finsh.as_str())
            .expect(format!("Failed to parse time {l_finsh} as RFC3339").as_str());

        // If the event is from 22:00 to 00:30, then add one day to the end date
        if local.goes_over_midnight {
            local_finsh = local_finsh + Duration::days(1);
        }

        let summary = format!(
            "Stage {} Loadshedding {}",
            local.stage,
            emojis.get(local.stage as usize).unwrap_or(&"🫠")
        );
        for national in &mis.changes {
            if national.stage == local.stage {
                if national.start < local_finsh && national.finsh > local_start {
                    eprintln!("Overlap (National {} and Local {}):\n  national: {} to {},\n  local:    {} to {}",
                        national.stage, local.stage, national.start, national.finsh, local_start, local_finsh,
                    );
                    let area_name = csv_path
                        .replace("generated/", "")
                        .replace(".csv", "")
                        .replace(|c: char| !c.is_ascii(), "");
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
                    calendar.push(evt);
                    eprintln!("    Adding: {} to {}", event_start, event_finsh);
                }
            }
        }
    }

    let fname = csv_path
        .replace("csv", "ics")
        .replace("generated", "calendars")
        .replace(|c: char| !c.is_ascii(), "")
        .replace("&nbsp;", "");
    let mut file =
        File::create(fname.as_str()).expect(format!("Failed to create file {}", fname).as_str());

    writeln!(&mut file, "{}", calendar)
        .expect(format!("Failed to write to file {:?}", file).as_str());
}
