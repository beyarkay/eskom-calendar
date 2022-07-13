use chrono::{DateTime, Datelike, Duration, NaiveDate, Timelike, Utc};
use icalendar::{Calendar, Component, Event};
use serde_yaml;
use std::fs::{read_dir, read_to_string, File};
use std::io::Write;
use std::path::PathBuf;
use structs::{
    ManuallyInputSchedule, MonthlyShedding, RawManuallyInputSchedule, RawMonthlyShedding,
};
use structs::{MunicipalityInfo, Province, RawMunicipalityInfo, SuburbInfo};

use crate::structs::GetSuburbDataResult;
use scraper::Html;
use scraper::Selector;
use std::process::Command;

mod structs;
fn main() {
    // Download all the pdfs from the internet
    // dl_pdfs("https://www.eskom.co.za/distribution/customer-service/outages/municipal-loadshedding-schedules/western-cape/".to_string());

    // Get the paths of the csv files generated
    let csv_paths = read_dir("generated/")
        .unwrap()
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

    let mis: ManuallyInputSchedule = serde_yaml::from_str::<RawManuallyInputSchedule>(
        read_to_string("manually_specified.yaml").unwrap().as_str(),
    )
    .unwrap()
    .into();

    // Convert the csv files to ics files, taking the intersection of the load shedding events and
    // the manually input loadshedding schedules
    for path in csv_paths {
        eprintln!("Creating calendar from {:?}", path);
        create_calendar(path.to_str().unwrap().to_string(), &mis);
    }
}

/// Given a url, download the pdfs from that url that match the css selector `div>div>p>span>a` and
/// convert them via a python script to csv file containing load shedding schedules.
fn dl_pdfs(url: String) {
    eprintln!("Getting links");
    let val = reqwest::blocking::get(url).unwrap();
    let html = val.text().unwrap();
    let document = Html::parse_document(&html);
    let link_selector = Selector::parse("div>div>p>span>a").unwrap();
    eprintln!("Parsing {} links", document.select(&link_selector).count());
    for element in document.select(&link_selector) {
        // If the href exists and starts with eskom.co.za/..../uploads then download and parse it
        // via python
        if let Some(href) = element.value().attr("href") {
            if href.starts_with("https://www.eskom.co.za/distribution/wp-content/uploads") {
                let fname = element.inner_html().replace(":", "").replace(" ", "");
                eprintln!("$ python3 parse_pdf.py {href:<90} {fname:<20}");
                let res = Command::new("python3")
                    .args(["parse_pdf.py", href, fname.as_str()])
                    .output()
                    .expect("Failed to execute command");
                if !res.status.success() {
                    eprintln!("stdout: {:?}\nstderr:{:?}", res.stdout, res.stderr);
                }
            } else {
                eprintln!(
                    "Cannot parse: {:?} {:?}",
                    element.value().attr("href"),
                    element.inner_html()
                );
            }
        }
    }
}

/// Create a single loadshedding calendar given one area's csv data
fn create_calendar(csv_path: String, mis: &ManuallyInputSchedule) {
    let mut rdr = csv::Reader::from_path(csv_path.clone()).unwrap();
    let mut events: Vec<MonthlyShedding> = vec![];
    let mut calendar = Calendar::new();
    for result in rdr.deserialize::<RawMonthlyShedding>() {
        let shedding: MonthlyShedding = result.unwrap().into();
        if shedding.stage == 0 {
            continue;
        }
        // Each load shedding stage also implies load shedding for all stages greater than it.
        for stage in shedding.stage..=8 {
            events.push(MonthlyShedding {
                start_time: shedding.start_time.clone(),
                finsh_time: shedding.finsh_time.clone(),
                stage,
                date_of_month: shedding.date_of_month,
                goes_over_midnight: shedding.goes_over_midnight,
            });
        }
    }
    // eprintln!("{} events", events.len());
    for event in events.into_iter() {
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
        if event.date_of_month > days_in_month {
            continue;
        }
        let local_start = DateTime::parse_from_rfc3339(
            format!(
                "{year}-{month:02}-{date:02}T{hour:02}:{minute:02}:00+02:00",
                year = curr_year,
                month = curr_month.month(),
                date = event.date_of_month,
                hour = event.start_time.hour(),
                minute = event.start_time.minute(),
            )
            .as_str(),
        )
        .unwrap();

        let mut local_finsh = DateTime::parse_from_rfc3339(
            format!(
                "{year}-{month:02}-{date:02}T{hour:02}:{minute:02}:00+02:00",
                year = curr_year,
                month = curr_month.month(),
                date = event.date_of_month,
                hour = event.finsh_time.hour(),
                minute = event.finsh_time.minute(),
            )
            .as_str(),
        )
        .unwrap();

        // If the event is from 22:00 to 00:30, then add one day to the end date
        if event.goes_over_midnight {
            local_finsh = local_finsh + Duration::days(1);
        }

        let summary = format!("Stage {} Loadshedding", event.stage);
        for national in &mis.changes {
            if national.stage == event.stage {
                if national.start < local_finsh && national.finsh > local_start {
                    let area_name = csv_path.replace("generated/", "").replace(".csv", "");
                    let description = format!(
                        "{area_name} loadshedding from {} to {}\n\nNational loadshedding from {} to {}\n\nIncorrect? Make a suggestion here: https://github.com/beyarkay/eskom-calendar/pulls\n\n---\nGenerated by Boyd Kane https://github.com/beyarkay/eskom-calendar\n\nInformation scraped from {}\n\nCalendar compiled at {:?}",
                        local_start, local_finsh, national.start, national.finsh, national.source, chrono::offset::Local::now()
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
                }
            }
        }
    }

    let fname = csv_path.replace("csv", "ics").replace("generated", "calendars");
    let mut file = File::create(fname.as_str()).unwrap();

    writeln!(&mut file, "{}", calendar).unwrap();
}

fn _get_all_data() {
    let provinces = vec![
        Province::EasternCape,
        Province::FreeState,
        Province::Gauteng,
        Province::KwaZuluNatal,
        Province::Limpopo,
        Province::Mpumalanga,
        Province::NorthWest,
        Province::NorthernCape,
        Province::WesternCape,
    ];
    for province in provinces {
        match province {
            Province::WesternCape => {}
            _ => {
                continue;
            }
        }
        eprintln!("{:?}", province);
        let municipalities = _get_municipalities(&province);
        for municipality in municipalities {
            eprintln!("- {} {}", municipality.name, municipality.id);
            let suburbs = _get_suburbs(&municipality);
            for suburb in suburbs {
                let url = _get_schedule(&province, &municipality, &suburb, 6);
                eprintln!(
                    "  - {:<30} {:>10} data?: {:>10} url: {url}",
                    suburb.name, suburb.id, suburb.has_schedule
                );
            }
        }
    }
}

fn _encode_province(province: &Province) -> u8 {
    match province {
        Province::EasternCape => 1,
        Province::FreeState => 2,
        Province::Gauteng => 3,
        Province::KwaZuluNatal => 4,
        Province::Limpopo => 5,
        Province::Mpumalanga => 6,
        Province::NorthWest => 7,
        Province::NorthernCape => 8,
        Province::WesternCape => 9,
    }
}

fn _get_municipalities(province: &Province) -> Vec<MunicipalityInfo> {
    let encoded_province = _encode_province(&province);
    let url = format!(
        "https://loadshedding.eskom.co.za/LoadShedding/GetMunicipalities/?Id={encoded_province}"
    );
    eprintln!("Getting municipalities via `{}`", url);
    let val = reqwest::blocking::get(url).unwrap();
    let mun_info = val
        .json::<Vec<RawMunicipalityInfo>>()
        .unwrap()
        .into_iter()
        .map(|rmi| rmi.into())
        .collect::<Vec<MunicipalityInfo>>();
    // println!("{:?}", mun_info);
    return mun_info;
}

fn _get_suburbs(mun_info: &MunicipalityInfo) -> Vec<SuburbInfo> {
    let municipality_id = mun_info.id;
    // TODO: is there any issue with just giving a really big page size
    let page_size = 1000;
    let url = format!(
        "http://loadshedding.eskom.co.za/LoadShedding/GetSurburbData/?pageSize={page_size}&pageNum=1&id={municipality_id}"
    );
    eprintln!(
        "Getting suburbs for municipality {} via `{url}`",
        mun_info.name
    );
    let val = reqwest::blocking::get(url).unwrap();
    let suburb_infos = val
        .json::<GetSuburbDataResult>()
        .unwrap()
        .Results
        .into_iter()
        .map(|rsi| rsi.into())
        .collect::<Vec<SuburbInfo>>();

    // let suburb_infos = suburb_info
    //     .into_iter()
    //     .filter(|s| s.has_schedule)
    //     .collect::<Vec<SuburbInfo>>();
    // println!("{:#?}", suburb_infos);
    return suburb_infos;
}

fn _get_schedule(
    province: &Province,
    _mun_info: &MunicipalityInfo,
    sub_info: &SuburbInfo,
    stage: u8,
) -> String {
    let url = format!(
        "http://loadshedding.eskom.co.za/LoadShedding/GetScheduleM/{suburb_id}/{stage}/{province_id}/1",
        suburb_id=sub_info.id,
        province_id=_encode_province(&province)
    );
    // println!(
    //     "Province {:?}, mun_info {}, suburb {}, stage {}, `{url}`",
    //     province, mun_info.name, sub_info.name, stage
    // );
    // println!("    {url}");
    return url;
    // let val = reqwest::blocking::get(url).unwrap();
    // let html = val.text().unwrap();
    // println!("{:?}", html);
    // let document = Html::parse_document(&html);
    // let day_div = Selector::parse("div.scheduleDay").unwrap();
    // println!("{:#?}", document);
}
