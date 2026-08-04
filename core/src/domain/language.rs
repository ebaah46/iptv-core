use serde::{Deserialize, Serialize};

/**
* Languages that broadcasts are provided in
*/

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Language {
    pub code: String,
    pub name: String,
}

pub type Languages = Vec<Language>;
