use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/**
* Programs that are scheduled for channels
*/

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct Program {
    pub channel_id: String,
    pub title: String,
    pub description: String,
    pub start: DateTime<Utc>,
    pub stop: DateTime<Utc>,
}

pub type Programs = Vec<Arc<Program>>;
