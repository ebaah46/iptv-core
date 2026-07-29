/**
* The feed that each channel contains. Feed has access to the streams
* which contain urls that can be loaded into a video player to begin
* streaming.
*/

use serde::Deserialize;
use crate::domain::stream::Stream;

#[derive(Debug, Deserialize)]
pub struct Feed{
    pub id: String,
    pub channel_id: String,
    pub name: String,
    pub broadcast_codes: Vec<String>,
    pub language_codes: Vec<String>,
    pub is_main: bool,
    streams: Vec<Stream>
}