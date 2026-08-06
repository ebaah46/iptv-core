use crate::domain::{Categories, Channel, Channels, Countries, Feeds, Languages, Streams};
use crate::ports::{CacheStore, ChannelDataSource};
use log::info;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

/**
* This is a repository that deals with access to the data available in the
* library. Thus, it serves as the boundary between the core logic and the
* raw data access.
* Services that handle the core logic can depend on this trait for access to data.
*/
pub trait CatalogRepository: Debug + Send + Sync {
    /// Fetch all channels in this repository.
    fn get_channels(&self) -> Channels;

    /// Get channels with a given channel id
    fn get_channel_by_id(&self, channel_id: &str) -> Option<Channel>;

    /// Get all the feeds associated with a channel
    fn get_feeds_by_channel(&self, channel_id: &str) -> Option<Feeds>;

    /// Get all streams associated with a given channel
    fn get_candidate_streams(&self, channel_id: &str, feed_id: Option<&str>) -> Option<Streams>;

    /// Retrieve all categories of channels available
    fn get_categories(&self) -> Categories;

    /// Retrieve all countries of channels available
    fn get_countries(&self) -> Countries;

    /// Retrieve all languages of channels available
    fn get_languages(&self) -> Languages;

    /// Refresh or reload data from data sources
    fn refresh(&self);
}

/**
* This repository is a concrete implementation of the repository trait that provides
* iptv data layer access to the services in this library.
*/
#[derive(Debug)]
pub struct IptvCatalogRepository {
    cache: Arc<dyn CacheStore>,
    data_source: Arc<dyn ChannelDataSource>,
    channels: RwLock<HashMap<String, Channel>>,
    feeds_by_channel: RwLock<HashMap<String, Feeds>>,
    streams_by_key: RwLock<HashMap<(String, Option<String>), Streams>>,
    categories: RwLock<Categories>,
    countries: RwLock<Countries>,
    languages: RwLock<Languages>,
}

impl IptvCatalogRepository {
    pub fn new(data_source: Arc<dyn ChannelDataSource>, cache: Arc<dyn CacheStore>) -> Self {
        Self {
            cache,
            data_source,
            channels: Default::default(),
            feeds_by_channel: Default::default(),
            streams_by_key: Default::default(),
            categories: Default::default(),
            countries: Default::default(),
            languages: Default::default(),
        }
    }
    fn fetch_catalog(&self) -> CatalogSnapShot {
        let source = &self.data_source;

        let (channels, feeds, streams, categories, countries, languages) = std::thread::scope(
            |s| {
                let channels = s.spawn(|| source.fetch_channels().unwrap_or_default());
                let feeds = s.spawn(|| source.fetch_feeds().unwrap_or_default());
                let streams = s.spawn(|| source.fetch_streams().unwrap_or_default());
                let countries = s.spawn(|| source.fetch_countries().unwrap_or_default());
                let categories = s.spawn(|| source.fetch_categories().unwrap_or_default());
                let languages = s.spawn(|| source.fetch_languages().unwrap_or_default());

                (
                    channels.join().unwrap_or_else(|e| {
                        info!("IptvCatalogRepository - fetch_catalog - channels thread crashed unexpectedly:{:?}", e);
                        vec![]
                    }),
                    feeds.join().unwrap_or_else(|e| {
                        info!("IptvCatalogRepository - fetch_catalog - feeds thread crashed unexpectedly:{:?}", e);
                        vec![]
                    }),
                    streams.join().unwrap_or_else(|e| {
                        info!("IptvCatalogRepository - fetch_catalog - streams thread crashed unexpectedly:{:?}", e);
                        vec![]
                    }),
                    categories.join().unwrap_or_else(|e| {
                        info!("IptvCatalogRepository - fetch_catalog - categories thread crashed unexpectedly:{:?}", e);
                        vec![]
                    }),
                    countries.join().unwrap_or_else(|e| {
                        info!("IptvCatalogRepository - fetch_catalog - countries thread crashed unexpectedly:{:?}", e);
                        vec![]
                    }),
                    languages.join().unwrap_or_else(|e| {
                        info!("IptvCatalogRepository - fetch_catalog - languages thread crashed unexpectedly:{:?}", e);                        vec![]
                    }),
                )
            },
        );
        CatalogSnapShot {
            channels,
            feeds,
            streams,
            countries,
            categories,
            languages,
        }
    }
}
impl CatalogRepository for IptvCatalogRepository {
    fn get_channels(&self) -> Channels {
        let read_guard = self.channels.read();
        read_guard.values().cloned().collect()
    }

    fn get_channel_by_id(&self, channel_id: &str) -> Option<Channel> {
        let read_guard = self.channels.read();
        read_guard.get(channel_id).cloned()
    }

    fn get_feeds_by_channel(&self, channel_id: &str) -> Option<Feeds> {
        let read_guard = self.feeds_by_channel.read();
        read_guard.get(channel_id).cloned()
    }

    fn get_candidate_streams(&self, channel_id: &str, feed_id: Option<&str>) -> Option<Streams> {
        let read_guard = self.streams_by_key.read();
        // Normalize: treat Some("") the same as None so reads align
        // with the feed side which stores empty feed_id as None.
        let feed_key = feed_id.filter(|s| !s.is_empty()).map(|s| s.into());
        read_guard.get(&(channel_id.into(), feed_key)).cloned()
    }

    fn get_categories(&self) -> Categories {
        let read_guard = self.categories.read();
        read_guard.clone()
    }

    fn get_countries(&self) -> Countries {
        let read_guard = self.countries.read();
        read_guard.clone()
    }

    fn get_languages(&self) -> Languages {
        let read_guard = self.languages.read();
        read_guard.clone()
    }

    fn refresh(&self) {
        let snapshot = self.fetch_catalog();
        if let Ok(()) = self.cache.save_channels(&snapshot.channels) {
            let mut channel_guard = self.channels.write();
            for channel in snapshot.channels {
                channel_guard.insert(channel.id.clone(), channel);
            }
        }

        if let Ok(()) = self.cache.save_feeds(&snapshot.feeds) {
            let mut feeds_by_channel: HashMap<String, Feeds> = HashMap::new();
            for feed in snapshot.feeds {
                feeds_by_channel
                    .entry(feed.channel_id.clone())
                    .or_default()
                    .push(feed);
            }
            *self.feeds_by_channel.write() = feeds_by_channel;
        }

        if let Ok(()) = self.cache.save_streams(&snapshot.streams) {
            let mut streams_by_key: HashMap<(String, Option<String>), Streams> = HashMap::new();
            for stream in snapshot.streams {
                let feed_key = if stream.feed_id.is_empty() {
                    None
                } else {
                    Some(stream.feed_id.clone())
                };
                streams_by_key
                    .entry((stream.channel_id.clone(), feed_key))
                    .or_default()
                    .push(stream);
            }
            *self.streams_by_key.write() = streams_by_key;
        }

        if let Ok(()) = self.cache.save_categories(&snapshot.categories) {
            *self.categories.write() = snapshot.categories;
        }

        if let Ok(()) = self.cache.save_countries(&snapshot.countries) {
            *self.countries.write() = snapshot.countries;
        }

        if let Ok(()) = self.cache.save_languages(&snapshot.languages) {
            *self.languages.write() = snapshot.languages;
        }
    }
}

/// This struct holds a snapshot of the data received when the fetch_catalog method is called to fetch
/// content from the catalog. At this point that is the only use of this snapshot
struct CatalogSnapShot {
    pub channels: Channels,
    pub feeds: Feeds,
    pub streams: Streams,
    pub countries: Countries,
    pub categories: Categories,
    pub languages: Languages,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Category, Country, Feed, Language, Programs, Stream};
    use anyhow::anyhow;
    use anyhow::Result as Res;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Test double for `ChannelDataSource` whose contents can be changed
    /// between refreshes so stale-data behaviour can be exercised.
    /// This Mock contains member fields for testing.
    /// Actual adapter implementations do not hold fetched values; instead,
    ///  the repository is responsible for storing what is fetched in its locked maps
    /// after they have been cached.
    #[derive(Debug, Default)]
    struct MockDataSource {
        channels: RwLock<Channels>,
        feeds: RwLock<Feeds>,
        streams: RwLock<Streams>,
        categories: RwLock<Categories>,
        countries: RwLock<Countries>,
        languages: RwLock<Languages>,
    }

    impl ChannelDataSource for MockDataSource {
        fn fetch_channels(&self) -> Res<Channels> {
            Ok(self.channels.read().clone())
        }

        fn fetch_feeds(&self) -> Res<Feeds> {
            Ok(self.feeds.read().clone())
        }

        fn fetch_streams(&self) -> Res<Streams> {
            Ok(self.streams.read().clone())
        }

        fn fetch_countries(&self) -> Res<Countries> {
            Ok(self.countries.read().clone())
        }

        fn fetch_languages(&self) -> Res<Languages> {
            Ok(self.languages.read().clone())
        }

        fn fetch_guides(&self) -> Res<Programs> {
            Ok(vec![])
        }

        fn fetch_categories(&self) -> Res<Categories> {
            Ok(self.categories.read().clone())
        }
    }

    /// Test double for `CacheStore` with per-save success flags and call
    /// counters, so negative tests can prove a save was attempted but the
    /// data was still not published.
    #[derive(Debug)]
    struct MockCacheStore {
        save_channels_ok: bool,
        save_feeds_ok: bool,
        save_streams_ok: bool,
        save_channels_calls: AtomicUsize,
        save_feeds_calls: AtomicUsize,
        save_streams_calls: AtomicUsize,
    }

    impl Default for MockCacheStore {
        fn default() -> Self {
            Self {
                save_channels_ok: true,
                save_feeds_ok: true,
                save_streams_ok: true,
                save_channels_calls: AtomicUsize::new(0),
                save_feeds_calls: AtomicUsize::new(0),
                save_streams_calls: AtomicUsize::new(0),
            }
        }
    }

    impl MockCacheStore {
        fn with_save_channels_failing(mut self) -> Self {
            self.save_channels_ok = false;
            self
        }

        fn with_save_feeds_failing(mut self) -> Self {
            self.save_feeds_ok = false;
            self
        }

        fn with_save_streams_failing(mut self) -> Self {
            self.save_streams_ok = false;
            self
        }
    }

    impl CacheStore for MockCacheStore {
        fn save_channels(&self, _channels: &Channels) -> Res<()> {
            self.save_channels_calls.fetch_add(1, Ordering::SeqCst);
            if self.save_channels_ok {
                Ok(())
            } else {
                Err(anyhow!("save_channels failed"))
            }
        }

        fn load_channels(&self) -> Res<Channels> {
            Ok(vec![])
        }

        fn save_feeds(&self, _feeds: &Feeds) -> Res<()> {
            self.save_feeds_calls.fetch_add(1, Ordering::SeqCst);
            if self.save_feeds_ok {
                Ok(())
            } else {
                Err(anyhow!("save_feeds failed"))
            }
        }

        fn load_feeds(&self) -> Res<Feeds> {
            Ok(vec![])
        }

        fn save_streams(&self, _streams: &Streams) -> Res<()> {
            self.save_streams_calls.fetch_add(1, Ordering::SeqCst);
            if self.save_streams_ok {
                Ok(())
            } else {
                Err(anyhow!("save_streams failed"))
            }
        }

        fn load_streams(&self) -> Res<Streams> {
            Ok(vec![])
        }

        fn save_categories(&self, _categories: &Categories) -> Res<()> {
            Ok(())
        }

        fn load_categories(&self) -> Res<Categories> {
            Ok(vec![])
        }

        fn save_countries(&self, _countries: &Countries) -> Res<()> {
            Ok(())
        }

        fn load_countries(&self) -> Res<Countries> {
            Ok(vec![])
        }

        fn save_languages(&self, _languages: &Languages) -> Res<()> {
            Ok(())
        }

        fn load_languages(&self) -> Res<Languages> {
            Ok(vec![])
        }

        fn save_programs(&self, _programs: &Programs) -> Res<()> {
            Ok(())
        }

        fn load_programs(&self) -> Res<Programs> {
            Ok(vec![])
        }
    }

    fn channel(id: &str) -> Channel {
        Channel {
            id: id.to_string(),
            name: format!("Channel {}", id),
            alt_names: vec![],
            category_ids: vec![],
            country_code: "US".to_string(),
            is_nsfw: false,
            launched: None,
            closed: None,
            website: String::new(),
            network: String::new(),
        }
    }

    fn feed(id: &str, channel_id: &str) -> Feed {
        Feed {
            id: id.to_string(),
            channel_id: channel_id.to_string(),
            name: format!("Feed {}", id),
            broadcast_codes: vec![],
            language_codes: vec![],
            is_main: false,
        }
    }

    fn stream(channel_id: &str, feed_id: &str) -> Stream {
        Stream {
            channel_id: channel_id.to_string(),
            feed_id: feed_id.to_string(),
            url: format!("https://example.com/{}/{}.m3u8", channel_id, feed_id),
            quality: "720p".to_string(),
            referrer: String::new(),
            title: format!("Stream {}/{}", channel_id, feed_id),
            user_agent: String::new(),
        }
    }

    fn category(id: &str, name: &str) -> Category {
        Category {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
        }
    }

    fn country(code: &str, name: &str, languages: &[&str]) -> Country {
        Country {
            code: code.to_string(),
            name: name.to_string(),
            languages: languages.iter().map(|s| s.to_string()).collect(),
            flag_url: String::new(),
        }
    }

    fn language(code: &str, name: &str) -> Language {
        Language {
            code: code.to_string(),
            name: name.to_string(),
        }
    }

    #[test]
    fn refresh_populates_all_catalog_data() {
        let data_source = MockDataSource {
            channels: RwLock::new(vec![channel("ch1"), channel("ch2")]),
            feeds: RwLock::new(vec![feed("feed1", "ch1"), feed("feed2", "ch2")]),
            streams: RwLock::new(vec![stream("ch1", "feed1"), stream("ch2", "feed2")]),
            categories: RwLock::new(vec![category("news", "News")]),
            countries: RwLock::new(vec![country("US", "United States", &["en"])]),
            languages: RwLock::new(vec![language("en", "English")]),
            ..Default::default()
        };
        let repository =
            IptvCatalogRepository::new(Arc::new(data_source), Arc::new(MockCacheStore::default()));

        repository.refresh();

        assert_eq!(repository.get_channels().len(), 2);
        assert_eq!(repository.get_channel_by_id("ch1"), Some(channel("ch1")));
        assert_eq!(
            repository.get_feeds_by_channel("ch1"),
            Some(vec![feed("feed1", "ch1")])
        );
        assert_eq!(
            repository.get_candidate_streams("ch1", Some("feed1")),
            Some(vec![stream("ch1", "feed1")])
        );
        assert_eq!(
            repository.get_categories(),
            vec![category("news", "News")]
        );
        assert_eq!(
            repository.get_countries(),
            vec![country("US", "United States", &["en"])]
        );
        assert_eq!(repository.get_languages(), vec![language("en", "English")]);
    }

    #[test]
    fn refresh_groups_multiple_feeds_per_channel() {
        let data_source = MockDataSource {
            feeds: RwLock::new(vec![
                feed("feed1", "ch1"),
                feed("feed2", "ch1"),
                feed("feed3", "ch1"),
            ]),
            ..Default::default()
        };
        let repository =
            IptvCatalogRepository::new(Arc::new(data_source), Arc::new(MockCacheStore::default()));

        repository.refresh();

        assert_eq!(
            repository.get_feeds_by_channel("ch1"),
            Some(vec![
                feed("feed1", "ch1"),
                feed("feed2", "ch1"),
                feed("feed3", "ch1"),
            ])
        );
    }

    #[test]
    fn refresh_groups_feeds_by_owning_channel() {
        let data_source = MockDataSource {
            feeds: RwLock::new(vec![feed("feed1", "ch1"), feed("feed2", "ch2")]),
            ..Default::default()
        };
        let repository =
            IptvCatalogRepository::new(Arc::new(data_source), Arc::new(MockCacheStore::default()));

        repository.refresh();

        assert_eq!(
            repository.get_feeds_by_channel("ch1"),
            Some(vec![feed("feed1", "ch1")])
        );
        assert_eq!(
            repository.get_feeds_by_channel("ch2"),
            Some(vec![feed("feed2", "ch2")])
        );
    }

    #[test]
    fn refresh_groups_streams_by_channel_and_feed_key() {
        let data_source = MockDataSource {
            streams: RwLock::new(vec![
                stream("ch1", "feed1"),
                stream("ch1", "feed2"),
                stream("ch2", "feed2"),
            ]),
            ..Default::default()
        };
        let repository =
            IptvCatalogRepository::new(Arc::new(data_source), Arc::new(MockCacheStore::default()));

        repository.refresh();

        assert_eq!(
            repository.get_candidate_streams("ch1", Some("feed1")),
            Some(vec![stream("ch1", "feed1")])
        );
        assert_eq!(
            repository.get_candidate_streams("ch1", Some("feed2")),
            Some(vec![stream("ch1", "feed2")])
        );
        assert_eq!(
            repository.get_candidate_streams("ch2", Some("feed2")),
            Some(vec![stream("ch2", "feed2")])
        );
    }

    #[test]
    fn refresh_stores_empty_feed_id_streams_under_none_key() {
        let data_source = MockDataSource {
            streams: RwLock::new(vec![stream("ch1", ""), stream("ch1", "feed1")]),
            ..Default::default()
        };
        let repository =
            IptvCatalogRepository::new(Arc::new(data_source), Arc::new(MockCacheStore::default()));

        repository.refresh();

        // Empty feed_id streams are stored under the None key ...
        assert_eq!(
            repository.get_candidate_streams("ch1", None::<&str>),
            Some(vec![stream("ch1", "")])
        );
        // ... and Some("") is normalized to None on the read path.
        assert_eq!(
            repository.get_candidate_streams("ch1", Some("")),
            Some(vec![stream("ch1", "")])
        );
        // Specific feeds still resolve independently.
        assert_eq!(
            repository.get_candidate_streams("ch1", Some("feed1")),
            Some(vec![stream("ch1", "feed1")])
        );
    }

    #[test]
    fn refresh_replaces_stale_data_on_second_call() {
        let data_source = Arc::new(MockDataSource {
            channels: RwLock::new(vec![channel("ch1")]),
            feeds: RwLock::new(vec![feed("old_feed", "ch1")]),
            streams: RwLock::new(vec![stream("ch1", "old_feed")]),
            ..Default::default()
        });
        let repository =
            IptvCatalogRepository::new(data_source.clone(), Arc::new(MockCacheStore::default()));

        repository.refresh();
        assert_eq!(repository.get_channels().len(), 1);
        assert!(repository.get_feeds_by_channel("ch1").is_some());
        assert!(
            repository
                .get_candidate_streams("ch1", Some("old_feed"))
                .is_some()
        );

        // Replace the source data entirely before the second refresh.
        *data_source.channels.write() = vec![channel("ch2")];
        *data_source.feeds.write() = vec![feed("new_feed", "ch2")];
        *data_source.streams.write() = vec![stream("ch2", "new_feed")];

        repository.refresh();

        // Channels ops are additive across refreshes (not wholesale replaced).
        // ch1 still exists because the imple uses insert per-channel.
        assert_eq!(repository.get_channel_by_id("ch1"), Some(channel("ch1")));
        assert_eq!(repository.get_channel_by_id("ch2"), Some(channel("ch2")));
        // Feeds and streams are wholesale-replaced, so old ones are gone.
        assert!(repository.get_feeds_by_channel("ch1").is_none());
        assert_eq!(
            repository.get_feeds_by_channel("ch2"),
            Some(vec![feed("new_feed", "ch2")])
        );
        assert!(
            repository
                .get_candidate_streams("ch1", Some("old_feed"))
                .is_none()
        );
        assert_eq!(
            repository.get_candidate_streams("ch2", Some("new_feed")),
            Some(vec![stream("ch2", "new_feed")])
        );
    }

    #[test]
    fn get_channel_by_id_returns_none_for_unknown_channel() {
        let repository = IptvCatalogRepository::new(
            Arc::new(MockDataSource::default()),
            Arc::new(MockCacheStore::default()),
        );

        repository.refresh();

        assert!(repository.get_channels().is_empty());
        assert!(repository.get_channel_by_id("unknown").is_none());
    }

    #[test]
    fn get_feeds_by_channel_returns_none_for_unknown_channel() {
        let data_source = MockDataSource {
            feeds: RwLock::new(vec![feed("feed1", "ch1")]),
            ..Default::default()
        };
        let repository =
            IptvCatalogRepository::new(Arc::new(data_source), Arc::new(MockCacheStore::default()));

        repository.refresh();

        assert_eq!(
            repository.get_feeds_by_channel("ch1"),
            Some(vec![feed("feed1", "ch1")])
        );
        assert!(repository.get_feeds_by_channel("ch2").is_none());
    }

    #[test]
    fn get_candidate_streams_returns_none_for_unknown_pair() {
        let data_source = MockDataSource {
            streams: RwLock::new(vec![stream("ch1", "feed1")]),
            ..Default::default()
        };
        let repository =
            IptvCatalogRepository::new(Arc::new(data_source), Arc::new(MockCacheStore::default()));

        repository.refresh();

        // Unknown channel.
        assert!(
            repository
                .get_candidate_streams("ch2", Some("feed1"))
                .is_none()
        );
        // Known channel, unknown feed.
        assert!(
            repository
                .get_candidate_streams("ch1", Some("feed2"))
                .is_none()
        );
        // Known channel, no feed (the only stream is tied to a specific feed).
        assert!(
            repository
                .get_candidate_streams("ch1", None::<&str>)
                .is_none()
        );
    }

    #[test]
    fn refresh_skips_channels_when_cache_save_fails() {
        let data_source = MockDataSource {
            channels: RwLock::new(vec![channel("ch1")]),
            ..Default::default()
        };
        let cache = Arc::new(MockCacheStore::default().with_save_channels_failing());
        let repository = IptvCatalogRepository::new(Arc::new(data_source), cache.clone());

        repository.refresh();

        // The save was attempted ...
        assert_eq!(cache.save_channels_calls.load(Ordering::SeqCst), 1);
        // ... but the data was not published.
        assert!(repository.get_channels().is_empty());
        assert!(repository.get_channel_by_id("ch1").is_none());
    }

    #[test]
    fn refresh_skips_feeds_when_cache_save_fails() {
        let data_source = MockDataSource {
            feeds: RwLock::new(vec![feed("feed1", "ch1")]),
            ..Default::default()
        };
        let cache = Arc::new(MockCacheStore::default().with_save_feeds_failing());
        let repository = IptvCatalogRepository::new(Arc::new(data_source), cache.clone());

        repository.refresh();

        assert_eq!(cache.save_feeds_calls.load(Ordering::SeqCst), 1);
        assert!(repository.get_feeds_by_channel("ch1").is_none());
    }

    #[test]
    fn refresh_skips_streams_when_cache_save_fails() {
        let data_source = MockDataSource {
            streams: RwLock::new(vec![stream("ch1", "feed1")]),
            ..Default::default()
        };
        let cache = Arc::new(MockCacheStore::default().with_save_streams_failing());
        let repository = IptvCatalogRepository::new(Arc::new(data_source), cache.clone());

        repository.refresh();

        assert_eq!(cache.save_streams_calls.load(Ordering::SeqCst), 1);
        assert!(
            repository
                .get_candidate_streams("ch1", Some("feed1"))
                .is_none()
        );
    }

    #[test]
    fn refresh_with_empty_data_source_leaves_maps_empty() {
        let repository = IptvCatalogRepository::new(
            Arc::new(MockDataSource::default()),
            Arc::new(MockCacheStore::default()),
        );

        repository.refresh();

        assert!(repository.get_channels().is_empty());
        assert!(repository.get_channel_by_id("any").is_none());
        assert!(repository.get_feeds_by_channel("any").is_none());
        assert!(
            repository
                .get_candidate_streams("any", Some("any"))
                .is_none()
        );
        assert!(
            repository
                .get_candidate_streams("any", None::<&str>)
                .is_none()
        );
        assert!(repository.get_categories().is_empty());
        assert!(repository.get_countries().is_empty());
        assert!(repository.get_languages().is_empty());
    }

    #[test]
    fn refresh_populates_categories() {
        let data_source = MockDataSource {
            categories: RwLock::new(vec![category("news", "News"), category("sports", "Sports")]),
            ..Default::default()
        };
        let repository =
            IptvCatalogRepository::new(Arc::new(data_source), Arc::new(MockCacheStore::default()));

        repository.refresh();

        assert_eq!(
            repository.get_categories(),
            vec![category("news", "News"), category("sports", "Sports")]
        );
    }

    #[test]
    fn refresh_populates_countries() {
        let data_source = MockDataSource {
            countries: RwLock::new(vec![country("US", "United States", &["en"])]),
            ..Default::default()
        };
        let repository =
            IptvCatalogRepository::new(Arc::new(data_source), Arc::new(MockCacheStore::default()));

        repository.refresh();

        assert_eq!(
            repository.get_countries(),
            vec![country("US", "United States", &["en"])]
        );
    }

    #[test]
    fn refresh_populates_languages() {
        let data_source = MockDataSource {
            languages: RwLock::new(vec![language("en", "English"), language("fr", "French")]),
            ..Default::default()
        };
        let repository =
            IptvCatalogRepository::new(Arc::new(data_source), Arc::new(MockCacheStore::default()));

        repository.refresh();

        assert_eq!(
            repository.get_languages(),
            vec![language("en", "English"), language("fr", "French")]
        );
    }

    #[test]
    fn refresh_replaces_categories_on_second_call() {
        let data_source = Arc::new(MockDataSource {
            categories: RwLock::new(vec![category("news", "News")]),
            ..Default::default()
        });
        let repository =
            IptvCatalogRepository::new(data_source.clone(), Arc::new(MockCacheStore::default()));

        repository.refresh();
        assert_eq!(repository.get_categories(), vec![category("news", "News")]);

        *data_source.categories.write() = vec![category("sports", "Sports")];

        repository.refresh();

        assert_eq!(repository.get_categories(), vec![category("sports", "Sports")]);
    }

    #[test]
    fn refresh_replaces_countries_on_second_call() {
        let data_source = Arc::new(MockDataSource {
            countries: RwLock::new(vec![country("US", "United States", &["en"])]),
            ..Default::default()
        });
        let repository =
            IptvCatalogRepository::new(data_source.clone(), Arc::new(MockCacheStore::default()));

        repository.refresh();
        assert_eq!(
            repository.get_countries(),
            vec![country("US", "United States", &["en"])]
        );

        *data_source.countries.write() = vec![country("DE", "Germany", &["de"])];

        repository.refresh();

        assert_eq!(
            repository.get_countries(),
            vec![country("DE", "Germany", &["de"])]
        );
    }

    #[test]
    fn refresh_replaces_languages_on_second_call() {
        let data_source = Arc::new(MockDataSource {
            languages: RwLock::new(vec![language("en", "English")]),
            ..Default::default()
        });
        let repository =
            IptvCatalogRepository::new(data_source.clone(), Arc::new(MockCacheStore::default()));

        repository.refresh();
        assert_eq!(repository.get_languages(), vec![language("en", "English")]);

        *data_source.languages.write() = vec![language("fr", "French")];

        repository.refresh();

        assert_eq!(repository.get_languages(), vec![language("fr", "French")]);
    }

    #[test]
    fn get_categories_returns_empty_before_refresh() {
        let repository = IptvCatalogRepository::new(
            Arc::new(MockDataSource {
                categories: RwLock::new(vec![category("news", "News")]),
                ..Default::default()
            }),
            Arc::new(MockCacheStore::default()),
        );

        // No refresh has happened yet, so nothing is stored.
        assert!(repository.get_categories().is_empty());
    }

    #[test]
    fn get_countries_returns_empty_before_refresh() {
        let repository = IptvCatalogRepository::new(
            Arc::new(MockDataSource {
                countries: RwLock::new(vec![country("US", "United States", &["en"])]),
                ..Default::default()
            }),
            Arc::new(MockCacheStore::default()),
        );

        assert!(repository.get_countries().is_empty());
    }

    #[test]
    fn get_languages_returns_empty_before_refresh() {
        let repository = IptvCatalogRepository::new(
            Arc::new(MockDataSource {
                languages: RwLock::new(vec![language("en", "English")]),
                ..Default::default()
            }),
            Arc::new(MockCacheStore::default()),
        );

        assert!(repository.get_languages().is_empty());
    }
}
