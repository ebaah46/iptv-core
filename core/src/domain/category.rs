use serde::{Deserialize, Serialize};

/**
* Categories of broadcasts that channels provide.
*/

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub description: String,
}

pub type Categories = Vec<Category>;
