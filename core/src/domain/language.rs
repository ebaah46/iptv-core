use serde::Deserialize;

/**
* Languages that broadcasts are provided in
*/

#[derive(Debug, Deserialize, PartialEq)]
pub struct Language {
    pub code: String,
    pub name: String,
}
