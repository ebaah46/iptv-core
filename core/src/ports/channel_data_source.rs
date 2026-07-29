use crate::domain::Channel;
use crate::domain::feed::Feed;
use crate::domain::stream::Stream;
use anyhow::{Result as Res};
use crate::domain::country::Country;
use crate::domain::language::Language;
use crate::domain::program::Program;

/**
* Describes the way in which the library retrieves the raw catalog and guide
* data into the system from wherever they actually live.
* ChannelDataSource trait is a way of exposing different iptv data sources to
* this system.
* This trait is model after what is provided by https:://www.iptv.org
*/

pub trait ChannelDataSource {
    // Retrieve channels available in this IPTV repository
    fn fetch_channels(&self) -> Res<Vec<Channel>>;

    // Retrieve list of feeds provided by this IPTV repository
    fn fetch_feeds(&self) -> Res<Vec<Feed>>;

    // Retrieves list of streams provided by this IPTV repository
    fn fetch_streams(&self) -> Res<Vec<Stream>>;

    // Retrieve list of countries provided by this IPTV repository
    fn fetch_countries(&self) -> Res<Vec<Country>>;

    // Retrieves list of languages provided by this IPTV repository
    fn fetch_languages(&self) -> Res<Vec<Language>>;

    // Retrieves list of languages provided by this IPTV repository
    fn fetch_guides(&self) -> Res<Vec<Program>>;

}
