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

    fn channel_has_feeds(&self, channel_id: &str) -> bool;
}

/**
* This is a concrete stream resolve implementation that retrieves all
* streams available in the repository. Streams are returned
*in no particular order.
*/
#[derive(Debug)]
pub struct IptvStreamResolver {
    catalog: Arc<dyn CatalogRepository>,
}

impl IptvStreamResolver {
    pub fn new(catalog: Arc<dyn CatalogRepository>) -> Self {
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

    fn channel_has_feeds(&self, channel_id: &str) -> bool {
        self.catalog
            .get_feeds_by_channel(channel_id)
            .map_or(false, |feeds| !feeds.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Categories, Channel, Channels, Countries, Feed, Feeds, Languages, Stream};
    use std::collections::HashMap;

    /// Mock repository that provides controlled data for testing.
    #[derive(Debug)]
    struct MockCatalogRepository {
        feeds_by_channel: HashMap<String, Feeds>,
        streams_by_channel: HashMap<String, HashMap<Option<String>, Streams>>,
    }

    impl Default for MockCatalogRepository {
        fn default() -> Self {
            Self {
                feeds_by_channel: HashMap::new(),
                streams_by_channel: HashMap::new(),
            }
        }
    }

    impl MockCatalogRepository {
        fn with_feeds_for_channel(mut self, channel_id: &str, feeds: Feeds) -> Self {
            self.feeds_by_channel.insert(channel_id.to_string(), feeds);
            self
        }

        fn with_streams_for_channel(
            mut self,
            channel_id: &str,
            feed_id: Option<&str>,
            streams: Streams,
        ) -> Self {
            let feed_key = feed_id.map(|s| s.to_string());
            self.streams_by_channel
                .entry(channel_id.to_string())
                .or_default()
                .insert(feed_key, streams);
            self
        }
    }

    impl CatalogRepository for MockCatalogRepository {
        fn get_channels(&self) -> Channels {
            vec![]
        }

        fn get_channel_by_id(&self, _channel_id: &str) -> Option<Arc<Channel>> {
            None
        }

        fn get_feeds_by_channel(&self, channel_id: &str) -> Option<Feeds> {
            self.feeds_by_channel.get(channel_id).cloned()
        }

        fn get_candidate_streams(
            &self,
            channel_id: &str,
            feed_id: Option<&str>,
        ) -> Option<Streams> {
            let feed_key = feed_id.map(|s| s.to_string());
            self.streams_by_channel
                .get(channel_id)?
                .get(&feed_key)
                .cloned()
        }

        fn get_streams_by_channel(&self, channel_id: &str) -> Option<Streams> {
            self.streams_by_channel
                .get(channel_id)
                .map(|by_feed| by_feed.values().flatten().cloned().collect())
        }

        fn has_streams(&self, channel_id: &str) -> bool {
            self.streams_by_channel.contains_key(channel_id)
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

    fn stream(channel_id: &str, feed_id: &str, quality: &str) -> Arc<Stream> {
        Arc::new(Stream {
            channel_id: channel_id.to_string(),
            feed_id: feed_id.to_string(),
            url: format!("https://example.com/{}/{}.m3u8", channel_id, feed_id),
            quality: quality.to_string(),
            referrer: String::new(),
            title: format!("Stream {}/{}", channel_id, feed_id),
            user_agent: String::new(),
        })
    }

    fn feed(id: &str, channel_id: &str) -> Arc<Feed> {
        Arc::new(Feed {
            id: id.to_string(),
            channel_id: channel_id.to_string(),
            name: format!("Feed {}", id),
            broadcast_codes: vec![],
            language_codes: vec![],
            is_main: false,
        })
    }

    #[test]
    fn channel_has_feeds_returns_true_when_feeds_exist() {
        let catalog = MockCatalogRepository::default()
            .with_feeds_for_channel("ch1", vec![feed("f1", "ch1")]);
        let resolver = IptvStreamResolver::new(Arc::new(catalog));

        assert!(resolver.channel_has_feeds("ch1"));
    }

    #[test]
    fn channel_has_feeds_returns_false_when_channel_has_empty_feeds() {
        let catalog = MockCatalogRepository::default();
        let resolver = IptvStreamResolver::new(Arc::new(catalog));

        assert!(!resolver.channel_has_feeds("ch1"));
    }

    #[test]
    fn channel_has_feeds_returns_false_for_unknown_channel() {
        let catalog = MockCatalogRepository::default()
            .with_feeds_for_channel("ch1", vec![feed("f1", "ch1")]);
        let resolver = IptvStreamResolver::new(Arc::new(catalog));

        assert!(!resolver.channel_has_feeds("unknown"));
    }

    #[test]
    fn returns_all_streams_across_all_feeds_of_a_channel() {
        let catalog = MockCatalogRepository::default()
            .with_feeds_for_channel("ch1", vec![feed("f1", "ch1"), feed("f2", "ch1")])
            .with_streams_for_channel("ch1", Some("f1"), vec![stream("ch1", "f1", "720p")])
            .with_streams_for_channel("ch1", Some("f2"), vec![stream("ch1", "f2", "1080p")]);
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
        let catalog = MockCatalogRepository::default()
            .with_feeds_for_channel("ch1", vec![feed("f1", "ch1")]);
        let resolver = IptvStreamResolver::new(Arc::new(catalog));

        let streams = resolver.get_candidate_streams("unknown");

        assert!(streams.is_empty());
    }

    #[test]
    fn includes_streams_with_no_feed_association() {
        let catalog = MockCatalogRepository::default()
            .with_feeds_for_channel("ch1", vec![feed("f1", "ch1")])
            .with_streams_for_channel(
                "ch1",
                None::<&str>,
                vec![stream("ch1", "", "1080p"), stream("ch1", "", "720p")],
            )
            .with_streams_for_channel(
                "ch1",
                Some("f1"),
                vec![stream("ch1", "f1", "480p")],
            );
        let resolver = IptvStreamResolver::new(Arc::new(catalog));

        let streams = resolver.get_candidate_streams("ch1");

        assert_eq!(streams.len(), 3);
        let qualities: Vec<&str> = streams.iter().map(|s| s.quality.as_str()).collect();
        assert!(qualities.contains(&"1080p"));
        assert!(qualities.contains(&"720p"));
        assert!(qualities.contains(&"480p"));
    }
}
