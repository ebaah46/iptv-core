use serde::Deserialize;

/**
* Countries that channels broadcast from
*/

#[derive(Debug, Deserialize, PartialEq)]
pub struct Country {
    pub code: String,
    pub name: String,
    pub languages: Vec<String>,
    pub flag_url: String,
}
