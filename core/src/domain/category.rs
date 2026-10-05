use serde::{Deserialize, Serialize};
use std::sync::Arc;

/**
* Categories of broadcasts that channels provide.
*/

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub description: String,
}

pub type Categories = Vec<Arc<Category>>;
