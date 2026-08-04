use serde::Deserialize;

/**
* Categories of broadcasts that channels provide.
*/

#[derive(Debug, Deserialize, PartialEq)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub description: String,
}
