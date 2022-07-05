use chrono::{Duration, Utc};
use icalendar::{Calendar, Class, Component, Event, Property};
use structs::{MunicipalityInfo, Province, RawMunicipalityInfo, SuburbInfo};

use crate::structs::GetSuburbDataResult;
use scraper::Html;
use scraper::Selector;
use std::process::Command;

mod structs;
fn main() {
    // dl_pdfs("https://www.eskom.co.za/distribution/customer-service/outages/municipal-loadshedding-schedules/western-cape/".to_string());
    create_calendars("".to_string());
}

fn dl_pdfs(url: String) {
    let val = reqwest::blocking::get(url).unwrap();
    let html = val.text().unwrap();
    let document = Html::parse_document(&html);
    let link_selector = Selector::parse("div>div>p>span>a").unwrap();
    for element in document.select(&link_selector) {
        if let Some(href) = element.value().attr("href") {
            if href.starts_with("https://www.eskom.co.za/distribution/wp-content/uploads") {
                let fname = element.inner_html().replace(":", "").replace(" ", "");
                println!(
                    "Downloading: {:?} {:?}",
                    element.value().attr("href"),
                    fname
                );
                Command::new("python3")
                    .args(["parse_pdf.py", href, &fname])
                    .output()
                    .expect("Failed to execute command");
            } else {
                println!(
                    "Cannot parse: {:?} {:?}",
                    element.value().attr("href"),
                    element.inner_html()
                );
            }
        }
    }
}

fn create_calendars(csv_path: String) {
    let mut calendar = Calendar::new();
    let event = Event::new()
        .summary("test event")
        .description("here I have something really important to do")
        .starts(Utc::now())
        .ends(Utc::now() + Duration::days(1))
        .done();
    calendar.push(event);
    calendar.print().unwrap();
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
        println!("{:?}", province);
        let municipalities = get_municipalities(&province);
        for municipality in municipalities {
            println!("- {} {}", municipality.name, municipality.id);
            let suburbs = get_suburbs(&municipality);
            for suburb in suburbs {
                let url = get_schedule(&province, &municipality, &suburb, 6);
                println!(
                    "  - {:<30} {:>10} data?: {:>10} url: {url}",
                    suburb.name, suburb.id, suburb.has_schedule
                );
            }
        }
    }
}

fn encode_province(province: &Province) -> u8 {
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

fn get_municipalities(province: &Province) -> Vec<MunicipalityInfo> {
    let encoded_province = encode_province(&province);
    let url = format!(
        "https://loadshedding.eskom.co.za/LoadShedding/GetMunicipalities/?Id={encoded_province}"
    );
    println!("Getting municipalities via `{}`", url);
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

fn get_suburbs(mun_info: &MunicipalityInfo) -> Vec<SuburbInfo> {
    let municipality_id = mun_info.id;
    // TODO: is there any issue with just giving a really big page size
    let page_size = 1000;
    let url = format!(
        "http://loadshedding.eskom.co.za/LoadShedding/GetSurburbData/?pageSize={page_size}&pageNum=1&id={municipality_id}"
    );
    println!(
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

fn get_schedule(
    province: &Province,
    mun_info: &MunicipalityInfo,
    sub_info: &SuburbInfo,
    stage: u8,
) -> String {
    let url = format!(
        "http://loadshedding.eskom.co.za/LoadShedding/GetScheduleM/{suburb_id}/{stage}/{province_id}/1",
        suburb_id=sub_info.id,
        province_id=encode_province(&province)
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
