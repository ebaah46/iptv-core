use crate::domain::{Categories, Channels, Countries, Feeds, Languages, Logos, Streams};

/**
* A point-in-time snapshot of the entire catalog: every entity fetched from
* the data source (or loaded from cache) in one consistent view.
*/
#[derive(Debug, Clone, Default)]
pub struct CatalogSnapshot {
    pub channels: Channels,
    pub feeds: Feeds,
    pub streams: Streams,
    pub countries: Countries,
    pub categories: Categories,
    pub languages: Languages,
    pub logos: Logos,
}
