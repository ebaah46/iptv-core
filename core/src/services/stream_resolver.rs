use crate::domain::Streams;
use crate::services::CatalogRepository;
use std::fmt::Debug;
use std::sync::Arc;

/**
* This trait defines a way to resolve streams in the library.
* Streams may be resolved based on various algorithms.
*/
pub trait StreamResolver: Debug + Send + Sync {
    fn get_candidate_streams(&self, channel_id: &str) -> Streams;
}

/**
* This is a concrete stream resolve implementation that retrieves all
* streams available in the repository. Streams are returned
*in no particular order.
*/
#[derive(Debug)]
pub(crate) struct IptvStreamResolver {
    catalog: Arc<dyn CatalogRepository>,
}

impl IptvStreamResolver {
    pub(crate) fn new(catalog: Arc<dyn CatalogRepository>) -> Self {
        Self { catalog }
    }
}

impl StreamResolver for IptvStreamResolver {
    fn get_candidate_streams(&self, channel_id: &str) -> Streams {
        let feeds = match self.catalog.get_feeds_by_channel(channel_id) {
            Some(feeds) => feeds,
            None => return vec![],
        };

        let mut all_streams = Streams::new();

        // Streams with None for feed section should be included
        if let Some(streams) = self.catalog.get_candidate_streams(channel_id, None) {
            all_streams.extend(streams);
        }

        // Include streams for each feed
        for feed in &feeds {
            if let Some(streams) = self
                .catalog
                .get_candidate_streams(channel_id, Some(&feed.id))
            {
                all_streams.extend(streams);
            }
        }

        all_streams
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Categories, Channel, Channels, Countries, Feeds, Languages, Stream};
    use std::sync::RwLock;

    /// Mock repository that provides controlled data for testing.
    #[derive(Debug, Default)]
    struct MockCatalogRepository {
        feeds: RwLock<Vec<(String, Feeds)>>,
        streams: RwLock<Vec<((String, Option<String>), Streams)>>,
    }

    impl CatalogRepository for MockCatalogRepository {
        fn get_channels(&self) -> Channels {
            vec![]
        }

        fn get_channel_by_id(&self, _channel_id: &str) -> Option<Channel> {
            None
        }

        fn get_feeds_by_channel(&self, channel_id: &str) -> Option<Feeds> {
            let guard = self.feeds.read().unwrap();
            guard
                .iter()
                .find(|(cid, _)| cid == channel_id)
                .map(|(_, feeds)| feeds.clone())
        }

        fn get_candidate_streams(
            &self,
            channel_id: &str,
            feed_id: Option<&str>,
        ) -> Option<Streams> {
            let guard = self.streams.read().unwrap();
            let feed_key = feed_id.map(|s| s.to_string());
            guard
                .iter()
                .find(|((cid, fk), _)| cid == channel_id && *fk == feed_key)
                .map(|(_, streams)| streams.clone())
        }

        fn get_categories(&self) -> Categories {
            vec![]
        }

        fn get_countries(&self) -> Countries {
            vec![]
        }

        fn get_languages(&self) -> Languages {
            vec![]
        }

        fn refresh(&self) {}
    }

    fn stream(channel_id: &str, feed_id: &str, quality: &str) -> Stream {
        Stream {
            channel_id: channel_id.to_string(),
            feed_id: feed_id.to_string(),
            url: format!("https://example.com/{}/{}.m3u8", channel_id, feed_id),
            quality: quality.to_string(),
            referrer: String::new(),
            title: format!("Stream {}/{}", channel_id, feed_id),
            user_agent: String::new(),
        }
    }

    fn feed(id: &str, channel_id: &str) -> crate::domain::Feed {
        crate::domain::Feed {
            id: id.to_string(),
            channel_id: channel_id.to_string(),
            name: format!("Feed {}", id),
            broadcast_codes: vec![],
            language_codes: vec![],
            is_main: false,
        }
    }

    #[test]
    fn returns_all_streams_across_all_feeds_of_a_channel() {
        let catalog = MockCatalogRepository {
            feeds: RwLock::new(vec![(
                "ch1".to_string(),
                vec![feed("f1", "ch1"), feed("f2", "ch1")],
            )]),
            streams: RwLock::new(vec![
                (
                    ("ch1".to_string(), Some("f1".to_string())),
                    vec![stream("ch1", "f1", "720p")],
                ),
                (
                    ("ch1".to_string(), Some("f2".to_string())),
                    vec![stream("ch1", "f2", "1080p")],
                ),
            ]),
        };
        let resolver = IptvStreamResolver::new(Arc::new(catalog));

        let streams = resolver.get_candidate_streams("ch1");

        assert_eq!(streams.len(), 2);
        let qualities: Vec<&str> = streams.iter().map(|s| s.quality.as_str()).collect();
        assert!(qualities.contains(&"720p"));
        assert!(qualities.contains(&"1080p"));
    }

    #[test]
    fn returns_empty_when_channel_has_no_feeds() {
        let catalog = MockCatalogRepository::default();
        let resolver = IptvStreamResolver::new(Arc::new(catalog));

        let streams = resolver.get_candidate_streams("ch1");

        assert!(streams.is_empty());
    }

    #[test]
    fn returns_empty_for_unknown_channel() {
        let catalog = MockCatalogRepository {
            feeds: RwLock::new(vec![("ch1".to_string(), vec![feed("f1", "ch1")])]),
            ..Default::default()
        };
        let resolver = IptvStreamResolver::new(Arc::new(catalog));

        let streams = resolver.get_candidate_streams("unknown");

        assert!(streams.is_empty());
    }

    #[test]
    fn includes_streams_with_no_feed_association() {
        let catalog = MockCatalogRepository {
            feeds: RwLock::new(vec![("ch1".to_string(), vec![feed("f1", "ch1")])]),
            streams: RwLock::new(vec![
                (
                    ("ch1".to_string(), None),
                    vec![stream("ch1", "", "1080p"), stream("ch1", "", "720p")],
                ),
                (
                    ("ch1".to_string(), Some("f1".to_string())),
                    vec![stream("ch1", "f1", "480p")],
                ),
            ]),
        };
        let resolver = IptvStreamResolver::new(Arc::new(catalog));

        let streams = resolver.get_candidate_streams("ch1");

        assert_eq!(streams.len(), 3);
        let qualities: Vec<&str> = streams.iter().map(|s| s.quality.as_str()).collect();
        assert!(qualities.contains(&"1080p"));
        assert!(qualities.contains(&"720p"));
        assert!(qualities.contains(&"480p"));
    }
}
