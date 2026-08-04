use crate::domain::category::Categories;
use crate::domain::country::Countries;
use crate::domain::feed::Feeds;
use crate::domain::language::Languages;
use crate::domain::program::Programs;
use crate::domain::stream::Streams;
use crate::domain::Channels;
use anyhow::Result as Res;
use std::fmt::Debug;

/**
* Describes the way in which the library retrieves the raw catalog and guide
* data into the system from wherever they actually live.
* ChannelDataSource trait is a way of exposing different iptv data sources to
* this system.
* This trait is model after what is provided by https:://www.iptv.org
*/

pub trait ChannelDataSource: Debug + Send + Sync {
    // Retrieve channels available in this IPTV repository
    fn fetch_channels(&self) -> Res<Channels>;

    // Retrieve a list of feeds provided by this IPTV repository
    fn fetch_feeds(&self) -> Res<Feeds>;

    // Retrieves a list of streams provided by this IPTV repository
    fn fetch_streams(&self) -> Res<Streams>;

    // Retrieve a list of countries provided by this IPTV repository
    fn fetch_countries(&self) -> Res<Countries>;

    // Retrieves a list of languages provided by this IPTV repository
    fn fetch_languages(&self) -> Res<Languages>;

    // Retrieves a list of languages provided by this IPTV repository
    fn fetch_guides(&self) -> Res<Programs>;

    // Retrieves a list of categories for the provided channels
    fn fetch_categories(&self) -> Res<Categories>;
}
