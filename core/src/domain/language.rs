use serde::{Deserialize, Serialize};
use std::sync::Arc;

/**
* Languages that broadcasts are provided in
*/

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct Language {
    pub code: String,
    pub name: String,
}

pub type Languages = Vec<Arc<Language>>;
