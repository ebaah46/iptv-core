use crate::dto::{CategoryDTO, ChannelDTO, CountryDTO, FeedDTO, LanguageDTO, StreamDTO};
use crate::HttpClient;
use anyhow::Result as Res;
use core::domain::{
    channel::Channels, country::Countries, feed::Feeds, language::Languages, program::Programs,
    stream::Streams, Categories,
};
use core::ports::ChannelDataSource;
use std::sync::Arc;

/**
* This struct defines the rest client for IPTV HTTP API.
* It provides the concrete implementation for ChannelDataSource.
*/
#[derive(Debug)]
pub struct IptvRestClient {
    client: Arc<HttpClient>,
}

impl IptvRestClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = Arc::new(HttpClient::new(base_url));
        Self { client }
    }
}

impl ChannelDataSource for IptvRestClient {
    fn fetch_channels(&self) -> Res<Channels> {
        let path = "/channels.json";
        let query: Option<&()> = None;
        let channels = self.client.get_stream(path, query, |dto: ChannelDTO| {
            crate::mapper::map_channel_dto(dto)
        })?;
        Ok(channels)
    }

    fn fetch_feeds(&self) -> Res<Feeds> {
        let path = "/feeds.json";
        let query: Option<&()> = None;
        let feeds = self
            .client
            .get_stream(path, query, |dto: FeedDTO| crate::mapper::map_feed_dto(dto))?;
        Ok(feeds)
    }

    fn fetch_streams(&self) -> Res<Streams> {
        let path = "/streams.json";
        let query: Option<&()> = None;
        let streams = self.client.get_stream(path, query, |dto: StreamDTO| {
            crate::mapper::map_stream_dto(dto)
        })?;
        Ok(streams)
    }

    fn fetch_countries(&self) -> Res<Countries> {
        let path = "/countries.json";
        let query: Option<&()> = None;
        let countries = self.client.get_stream(path, query, |dto: CountryDTO| {
            crate::mapper::map_country_dto(dto)
        })?;
        Ok(countries)
    }

    fn fetch_languages(&self) -> Res<Languages> {
        let path = "/languages.json";
        let query: Option<&()> = None;
        let languages = self.client.get_stream(path, query, |dto: LanguageDTO| {
            crate::mapper::map_language_dto(dto)
        })?;
        Ok(languages)
    }

    fn fetch_categories(&self) -> Res<Categories> {
        let path = "/categories.json";
        let query: Option<&()> = None;
        let categories = self.client.get_stream(path, query, |dto: CategoryDTO| {
            crate::mapper::map_category_dto(dto)
        })?;
        Ok(categories)
    }

    fn fetch_guides(&self) -> Res<Programs> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use core::domain::{Channel, Country, Feed, Language, Stream};
    use httpmock::prelude::{Method, GET};
    use httpmock::MockServer;
    use serde_json::{json, Value};
    use std::str::FromStr;

    struct TestServer {
        server: MockServer,
    }

    impl TestServer {
        fn start() -> Self {
            Self {
                server: MockServer::start(),
            }
        }

        fn register_resource<T: Into<Value>>(
            &self,
            url: impl Into<String>,
            method: Method,
            response: T,
        ) {
            self.server.mock(|when, then| {
                when.method(method).path(url);
                then.status(200).json_body(response);
            });
        }

        fn base_url(&self) -> String {
            format!("http://{}", self.server.address())
        }
    }

    #[test]
    fn test_fetch_channels_success() {
        let server = TestServer::start();
        server.register_resource(
            "/channels.json",
            GET,
            json!([{
                "id": "channel-1",
                "name": "France 3",
                "alt_names": ["France 3 Paris"],
                "categories": ["general"],
                "country": "FR",
                "is_nsfw": false,
                "launched": "1992-09-07",
                "closed": null,
                "website": "https://www.france.tv/france-3/",
                "network": "France Télévisions",
                "owners": ["FranceAlpha"],
                "replaced_by": null
            }]),
        );

        let client = IptvRestClient::new(server.base_url());
        let channels = client.fetch_channels().unwrap();

        assert_eq!(
            channels,
            vec![Channel {
                id: "channel-1".to_string(),
                name: "France 3".to_string(),
                alt_names: vec!["France 3 Paris".to_string()],
                category_ids: vec!["general".to_string()],
                country_code: "FR".to_string(),
                is_nsfw: false,
                launched: Some(NaiveDate::from_str("1992-09-07").unwrap()),
                closed: None,
                website: "https://www.france.tv/france-3/".to_string(),
                network: "France Télévisions".to_string(),
            }]
        );
    }

    #[test]
    fn test_fetch_feeds_success() {
        let server = TestServer::start();
        server.register_resource(
            "/feeds.json",
            GET,
            json!([{
                "id": "feed-1",
                "channel": "channel-1",
                "name": "France 3 Paris IDF",
                "broadcast_areas": ["ParisIleDeFrance"],
                "languages": ["fr"],
                "alt_names": ["channel1"],
                "is_main": false,
                "timezones": ["Europe/Paris"],
                "country": "fra",
                "format": "576i"
            }]),
        );

        let client = IptvRestClient::new(server.base_url());
        let feeds = client.fetch_feeds().unwrap();

        assert_eq!(
            feeds,
            vec![Feed {
                id: "feed-1".to_string(),
                channel_id: "channel-1".to_string(),
                name: "France 3 Paris IDF".to_string(),
                broadcast_codes: vec!["ParisIleDeFrance".to_string()],
                language_codes: vec!["fr".to_string()],
                is_main: false,
            }]
        );
    }

    #[test]
    fn test_fetch_streams_success() {
        let server = TestServer::start();
        server.register_resource(
            "/streams.json",
            GET,
            json!([{
                "channel": "channel-1",
                "feed": "feed-1",
                "url": "https://example.com/stream.m3u8",
                "quality": "720p",
                "referrer": "https://example.com",
                "title": "France 3",
                "user_agent": "Mozilla/5.0"
            }]),
        );

        let client = IptvRestClient::new(server.base_url());
        let streams = client.fetch_streams().unwrap();

        assert_eq!(
            streams,
            vec![Stream {
                channel_id: "channel-1".to_string(),
                feed_id: "feed-1".to_string(),
                url: "https://example.com/stream.m3u8".to_string(),
                quality: "720p".to_string(),
                referrer: "https://example.com".to_string(),
                title: "France 3".to_string(),
                user_agent: "Mozilla/5.0".to_string(),
            }]
        );
    }

    #[test]
    fn test_fetch_countries_success() {
        let server = TestServer::start();
        server.register_resource(
            "/countries.json",
            GET,
            json!([{
                "code": "FR",
                "name": "France",
                "languages": ["fr"],
                "flag": "🇫🇷"
            }]),
        );

        let client = IptvRestClient::new(server.base_url());
        let countries = client.fetch_countries().unwrap();

        assert_eq!(
            countries,
            vec![Country {
                code: "FR".to_string(),
                name: "France".to_string(),
                languages: vec!["fr".to_string()],
                flag_url: "🇫🇷".to_string(),
            }]
        );
    }

    #[test]
    fn test_fetch_languages_success() {
        let server = TestServer::start();
        server.register_resource(
            "/languages.json",
            GET,
            json!([{
                "code": "fr",
                "name": "French"
            }]),
        );

        let client = IptvRestClient::new(server.base_url());
        let languages = client.fetch_languages().unwrap();

        assert_eq!(
            languages,
            vec![Language {
                code: "fr".to_string(),
                name: "French".to_string(),
            }]
        );
    }
}
