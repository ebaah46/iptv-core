use serde::{Deserialize, Serialize};

/**
* Countries that channels broadcast from
*/

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct Country {
    pub code: String,
    pub name: String,
    pub languages: Vec<String>,
    pub flag_url: String,
}

pub type Countries = Vec<Country>;
