/**
* Channel contains iptv channels identity and metadata.
*/
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};


#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub alt_names: Vec<String>,
    pub category_ids: Vec<String>,
    pub country_code: String,
    pub is_nsfw: bool,
    pub launched: Option<NaiveDate>,
    pub closed: Option<NaiveDate>,
    pub website: String,
    pub network: String,
}

pub type Channels = Vec<Channel>;
