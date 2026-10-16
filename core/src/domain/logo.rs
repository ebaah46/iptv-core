/**
* The logo for each channel. The main properties of the logo are
* the urls and the channel ids. The ids help to link the logo to the
* respective channel that it represents
*/
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct Logo {
    pub channel: String,
    pub format: String,
    pub url: String,
    pub width: u32,
    pub height: u32,
}

pub type Logos = Vec<Arc<Logo>>;
