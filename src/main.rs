use structs::{MunicipalityInfo, Province, RawMunicipalityInfo, SuburbInfo};
use scraper::Selector;

use crate::structs::GetSuburbDataResult;
use scraper::Html;

mod structs;
fn main() {
    let province = Province::WesternCape;
    let municipalities = get_municipalities(&province);
    let municipality = municipalities.first().unwrap();
    let suburbs = get_suburbs(&municipality);
    let suburb = suburbs.first().unwrap();
    let stage = 1;
    get_schedule(province, &municipality, &suburb, stage);
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
    let suburb_info = val
        .json::<GetSuburbDataResult>()
        .unwrap()
        .Results
        .into_iter()
        .map(|rsi| rsi.into())
        .collect::<Vec<SuburbInfo>>();

    let suburb_infos = suburb_info
        .into_iter()
        .filter(|s| s.has_schedule)
        .collect::<Vec<SuburbInfo>>();
    // println!("{:#?}", suburb_infos);
    return suburb_infos;
}

fn get_schedule(province: Province, mun_info: &MunicipalityInfo, sub_info: &SuburbInfo, stage: u8) {
    let url = format!(
        "http://loadshedding.eskom.co.za/LoadShedding/GetScheduleM/{suburb_id}/{stage}/{province_id}/1",
        suburb_id=sub_info.id,
        province_id=encode_province(&province)
    );
    println!(
        "Getting schedule for province {:?}, mun_info {}, suburb {}, stage {}, `{url}`",
        province, mun_info.name, sub_info.name, stage
    );
    let val = reqwest::blocking::get(url).unwrap();
    let html = val.text().unwrap();
    let document = Html::parse_document(&html);
    let day_div = Selector::parse("div.scheduleDay").unwrap();
    println!("{:#?}", document);
}
