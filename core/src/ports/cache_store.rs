use crate::domain::{Categories, Channels, Countries, Feeds, Languages, Programs, Streams};
use anyhow::Result as Res;
use std::fmt::Debug;

/**
* Describes the way in which the library receives or holds the
* disposable snapshot of the catalog of stream information.
* CacheStore trait provides access to a cache that holds retrieved
* catalog information. Cache can be file-based or in-memory.
*/

pub trait CacheStore: Debug + Send + Sync {
    // Cache channels data
    fn save_channels(&self, channels: &Channels) -> Res<()>;

    // Retrieved cached channels
    fn load_channels(&self) -> Res<Channels>;

    // Cache feeds data
    fn save_feeds(&self, feeds: &Feeds) -> Res<()>;

    // Retrieved cached feeds
    fn load_feeds(&self) -> Res<Feeds>;

    // Cache streams data
    fn save_streams(&self, streams: &Streams) -> Res<()>;

    // Retrieved cached streams
    fn load_streams(&self) -> Res<Streams>;

    // Cache categories data
    fn save_categories(&self, categories: &Categories) -> Res<()>;

    // Retrieved cached categories
    fn load_categories(&self) -> Res<Categories>;

    // Cache countries data
    fn save_countries(&self, countries: &Countries) -> Res<()>;

    // Retrieved cached countries
    fn load_countries(&self) -> Res<Countries>;

    // Cache languages data
    fn save_languages(&self, languages: &Languages) -> Res<()>;

    // Retrieved cached languages
    fn load_languages(&self) -> Res<Languages>;

    // Cache programs data
    fn save_programs(&self, programs: &Programs) -> Res<()>;

    // Retrieved cached programs
    fn load_programs(&self) -> Res<Programs>;
}
