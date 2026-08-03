use serde::Deserialize;

/**
* Raw structs that loaded JSON data is deserialized into.
* This is an intermediary state that represents the data loaded from
* the IPTV provider API. It is later transformed into the models
* supported by this library.
*/

#[derive(Debug, Deserialize)]
pub struct ChannelDTO {
    pub id: String,
    pub name: String,
    pub alt_names: Vec<String>,
    pub network: Option<String>,
    pub owners: Vec<String>,
    pub country: String,
    pub categories: Vec<String>,
    pub is_nsfw: bool,
    pub launched: Option<String>,
    pub closed: Option<String>,
    pub replaced_by: Option<String>,
    pub website: String,
}

#[derive(Debug, Deserialize)]
pub struct FeedDTO {
    pub id: String,
    pub channel: String,
    pub name: String,
    pub alt_names: Vec<String>,
    pub is_main: bool,
    pub broadcast_areas: Vec<String>,
    pub timezones: Vec<String>,
    pub languages: Vec<String>,
    pub country: String,
    pub format: String,
}

#[derive(Debug, Deserialize)]
pub struct StreamDTO {
    pub channel: Option<String>,
    pub feed: Option<String>,
    pub title: String,
    pub url: String,
    pub quality: Option<String>,
    pub label: Option<String>,
    pub user_agent: Option<String>,
    pub referrer: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CountryDTO {
    pub name: String,
    pub code: String,
    pub flag: String,
    pub languages: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct LanguageDTO {
    pub name: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct CategoryDTO {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct ProgramDTO {
    pub id: String,
    pub name: String,
    pub description: String,
}
