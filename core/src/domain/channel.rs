/**
* Channel contains iptv channels identity and metadata.
*/
use chrono::{DateTime, Utc};
use serde::Deserialize;
use crate::domain::feed::Feed;

#[derive(Debug, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub alt_names: Vec<String>,
    pub category_ids: Vec<String>,
    pub country_code: String,
    pub owners: Vec<String>,
    pub is_nsfw: bool,
    pub launched: DateTime<Utc>,
    pub logo_url: String,
    pub closed: DateTime<Utc>,
    pub website: String,
    pub network: String,
    feeds: Vec<Feed>,

}