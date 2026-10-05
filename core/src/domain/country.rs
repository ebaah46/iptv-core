use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

pub type Countries = Vec<Arc<Country>>;
