use serde::Deserialize;

/**
* Categories of broadcasts that channels provide.
*/

#[derive(Debug, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub description: String,
}