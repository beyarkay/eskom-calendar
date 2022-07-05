use std::fmt::Debug;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
#[allow(dead_code)]
pub enum Province {
    EasternCape,
    FreeState,
    Gauteng,
    KwaZuluNatal,
    Limpopo,
    Mpumalanga,
    NorthWest,
    NorthernCape,
    WesternCape,
}

/// Looks something like:
/// {
///     "Disabled":false,
///     "Group":null,
///     "Selected":false,
///     "Text":"Beaufort West",
///     "Value":"336"
/// }
#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct RawMunicipalityInfo {
    /// The usage of this is unknown
    Disabled: bool,
    /// The usage of this is unknown, it's often just `null`
    Group: Option<String>,
    /// The usage of this is unknown
    Selected: bool,
    /// The name of this municipality
    Text: String,
    /// The ID of this municipality
    Value: String,
}

/// The information about a municipality.
#[derive(Debug)]
pub struct MunicipalityInfo {
    /// The usage of this is unknown
    is_disabled: bool,
    // group: Option<String>,
    /// The usage of this is unknown
    is_selected: bool,
    /// The name of this municipality
    pub name: String,
    /// The ID of this municipality
    pub id: u32,
}

impl From<RawMunicipalityInfo> for MunicipalityInfo {
    fn from(raw: RawMunicipalityInfo) -> Self {
        MunicipalityInfo {
            is_disabled: raw.Disabled,
            // group: raw.Group,
            is_selected: raw.Selected,
            name: raw.Text,
            id: raw.Value.parse().expect("Couldn't parse raw.Value"),
        }
    }
}

/// A struct representing the results of a `GetSurburbData`
/// request. Contains the results and the total number of entries
#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct GetSuburbDataResult {
    pub Results: Vec<RawSuburbInfo>,
    pub Total: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct RawSuburbInfo {
    id: String,
    text: String,
    Tot: u32,
}

/// Information about an eskom suburb
#[derive(Debug)]
pub struct SuburbInfo {
    /// The ID number
    pub id: u32,
    /// The name of the suburb
    pub name: String,
    /// Not sure what this is
    total: u32,
    /// If true, then this suburb will have a schedule associated with it
    pub has_schedule: bool,
}
impl From<RawSuburbInfo> for SuburbInfo {
    fn from(raw: RawSuburbInfo) -> Self {
        // let mut chars = raw.id.chars();
        // chars.next();
        // chars.next_back();
        // let id = chars.as_str();
        SuburbInfo {
            id: raw.id.parse().expect("Couldn't parse suburb id to u32"),
            name: raw.text,
            total: raw.Tot,
            has_schedule: raw.Tot > 0,
        }
    }
}

struct Suburb {
    /// The ID number
    pub id: u32,
    /// The name of the suburb
    pub name: String,
    /// If true, then this suburb will have a schedule associated with it
    pub has_schedule: bool,
    /// The info about this suburbs municipality
    pub municipality: MunicipalityInfo,
}

/// A multitude of load shedding for a particular suburb
struct Schedule {
    suburb: Suburb,
    sheddings: Vec<Shedding>,
}

/// A single duration of loadshedding that only has one stage.
pub struct Shedding {
    /// The time when LoadShedding *should* start
    start: DateTime<Utc>,
    /// The time when LoadShedding *should* end
    finsh: DateTime<Utc>,
    /// The stage of loadshedding
    stage: u8,
}
