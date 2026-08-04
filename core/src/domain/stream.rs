/**
* Stream is the core component of the iptv system that contains the url
* used for accessing the TV channel.
*/
use serde::{Deserialize, Serialize};


#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct Stream {
    pub channel_id: String,
    pub feed_id: String,
    pub url: String,
    pub quality: String,
    pub referrer: String,
    pub title: String,
    pub user_agent: String,
}

pub type Streams = Vec<Stream>;
