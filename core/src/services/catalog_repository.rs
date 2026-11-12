use crate::domain::{
    CatalogSnapshot, Categories, Channel, Channels, Countries, Feeds, Languages, Logos, Streams,
};
use crate::ports::{CacheStore, ChannelDataSource};
use anyhow::{Result as Res, anyhow};
use log::{info, warn};
use parking_lot::{Mutex, RwLock};
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
    fn get_channel_by_id(&self, channel_id: &str) -> Option<Arc<Channel>>;

    /// Get all the feeds associated with a channel
    fn get_feeds_by_channel(&self, channel_id: &str) -> Option<Feeds>;

    /// Get the streams associated with a given channel and feed. A `None` (or empty)
    /// feed id selects the streams that are not tied to any feed.
    fn get_candidate_streams(&self, channel_id: &str, feed_id: Option<&str>) -> Option<Streams>;

    /// Get every stream associated with a given channel, regardless of feed.
    fn get_streams_by_channel(&self, channel_id: &str) -> Option<Streams>;

    /// Whether at least one stream exists for the channel, regardless of feed.
    fn has_streams(&self, channel_id: &str) -> bool;

    /// Retrieve all categories of channels available
    fn get_categories(&self) -> Categories;

    /// Retrieve all countries of channels available
    fn get_countries(&self) -> Countries;

    /// Retrieve all languages of channels available
    fn get_languages(&self) -> Languages;

    /// Retrieve all logos of the channels available
    fn get_logos(&self) -> Logos;

    /// Load the catalog, preferring a fresh cache and falling back to the
    /// network when the cache is missing or stale.
    fn load(&self) -> Res<LoadOutcome> {
        self.refresh()?;
        Ok(LoadOutcome::network())
    }

    /// Refresh or reload data from data sources
    fn refresh(&self) -> Res<()>;
}

/**
* Where the catalog data was loaded from.
*/
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogSource {
    Cache,
    Network,
}

/**
* The outcome of a catalog load operation.
*/
#[derive(Debug, Clone, Copy)]
pub struct LoadOutcome {
    pub source: CatalogSource,
}

impl LoadOutcome {
    pub fn cache() -> Self {
        Self {
            source: CatalogSource::Cache,
        }
    }

    pub fn network() -> Self {
        Self {
            source: CatalogSource::Network,
        }
    }
}

/**
* This repository is a concrete implementation of the repository trait that provides
* iptv data layer access to the services in this library.
*/
#[derive(Debug)]
pub struct IptvCatalogRepository {
    cache: Arc<dyn CacheStore>,
    data_source: Arc<dyn ChannelDataSource>,
    /// Serializes load/refresh so concurrent callers cannot interleave.
    load_lock: Mutex<()>,
    channels: RwLock<HashMap<String, Arc<Channel>>>,
    feeds_by_channel: RwLock<HashMap<String, Feeds>>,
    /// Streams grouped by channel id, then by feed id (`None` for feedless streams).
    streams_by_channel: RwLock<HashMap<String, HashMap<Option<String>, Streams>>>,
    categories: RwLock<Categories>,
    countries: RwLock<Countries>,
    languages: RwLock<Languages>,
    logos: RwLock<Logos>,
}

impl IptvCatalogRepository {
    pub fn new(data_source: Arc<dyn ChannelDataSource>, cache: Arc<dyn CacheStore>) -> Self {
        Self {
            cache,
            data_source,
            load_lock: Mutex::new(()),
            channels: Default::default(),
            feeds_by_channel: Default::default(),
            streams_by_channel: Default::default(),
            categories: Default::default(),
            countries: Default::default(),
            languages: Default::default(),
            logos: Default::default(),
        }
    }
    /// Fetch the whole catalog from the data source. Any endpoint failure
    /// fails the entire fetch; nothing is persisted or published.
    fn fetch_catalog(&self) -> Res<CatalogSnapshot> {
        let source = &self.data_source;

        std::thread::scope(|s| {
            let channels = s.spawn(|| source.fetch_channels());
            let feeds = s.spawn(|| source.fetch_feeds());
            let streams = s.spawn(|| source.fetch_streams());
            let countries = s.spawn(|| source.fetch_countries());
            let categories = s.spawn(|| source.fetch_categories());
            let languages = s.spawn(|| source.fetch_languages());
            let logos = s.spawn(|| source.fetch_logos());

            Ok(CatalogSnapshot {
                channels: channels
                    .join()
                    .map_err(|_| anyhow!("channels thread panicked"))
                    .and_then(|r| r)?,
                feeds: feeds
                    .join()
                    .map_err(|_| anyhow!("feeds thread panicked"))
                    .and_then(|r| r)?,
                streams: streams
                    .join()
                    .map_err(|_| anyhow!("streams thread panicked"))
                    .and_then(|r| r)?,
                categories: categories
                    .join()
                    .map_err(|_| anyhow!("categories thread panicked"))
                    .and_then(|r| r)?,
                countries: countries
                    .join()
                    .map_err(|_| anyhow!("countries thread panicked"))
                    .and_then(|r| r)?,
                languages: languages
                    .join()
                    .map_err(|_| anyhow!("languages thread panicked"))
                    .and_then(|r| r)?,
                logos: logos
                    .join()
                    .map_err(|_| anyhow!("logos thread panicked"))
                    .and_then(|r| r)?,
            })
        })
    }

    /// Replace the in-memory maps with the given snapshot, wholesale.
    fn publish(&self, snapshot: CatalogSnapshot) {
        let channels: HashMap<String, Arc<Channel>> = snapshot
            .channels
            .into_iter()
            .map(|channel| (channel.id.clone(), channel))
            .collect();
        *self.channels.write() = channels;

        let mut feeds_by_channel: HashMap<String, Feeds> = HashMap::new();
        for feed in snapshot.feeds {
            feeds_by_channel
                .entry(feed.channel_id.clone())
                .or_default()
                .push(feed);
        }
        *self.feeds_by_channel.write() = feeds_by_channel;

        let mut streams_by_channel: HashMap<String, HashMap<Option<String>, Streams>> =
            HashMap::new();
        for stream in snapshot.streams {
            let feed_key = if stream.feed_id.is_empty() {
                None
            } else {
                Some(stream.feed_id.clone())
            };
            streams_by_channel
                .entry(stream.channel_id.clone())
                .or_default()
                .entry(feed_key)
                .or_default()
                .push(stream);
        }
        *self.streams_by_channel.write() = streams_by_channel;

        *self.categories.write() = snapshot.categories;
        *self.countries.write() = snapshot.countries;
        *self.languages.write() = snapshot.languages;
        *self.logos.write() = snapshot.logos;
    }

    /// Write the snapshot to the cache. Failures are logged but never block
    /// publishing.
    fn persist(&self, snapshot: &CatalogSnapshot) {
        if let Err(e) = self.cache.save_channels(&snapshot.channels) {
            warn!("IptvCatalogRepository - persist - failed to cache channels: {}", e);
        }
        if let Err(e) = self.cache.save_feeds(&snapshot.feeds) {
            warn!("IptvCatalogRepository - persist - failed to cache feeds: {}", e);
        }
        if let Err(e) = self.cache.save_streams(&snapshot.streams) {
            warn!("IptvCatalogRepository - persist -  failed to cache streams: {}", e);
        }
        if let Err(e) = self.cache.save_categories(&snapshot.categories) {
            warn!("IptvCatalogRepository - persist -  failed to cache categories: {}", e);
        }
        if let Err(e) = self.cache.save_countries(&snapshot.countries) {
            warn!("IptvCatalogRepository - persist - failed to cache countries: {}", e);
        }
        if let Err(e) = self.cache.save_languages(&snapshot.languages) {
            warn!("IptvCatalogRepository - persist -  failed to cache languages: {}", e);
        }
        if let Err(e) = self.cache.save_logos(&snapshot.logos) {
            warn!("IptvCatalogRepository - persist -  failed to cache logos: {}", e);
        }
    }

    /// Fetch, persist, and publish in one step. A fetch error propagates and
    /// nothing is persisted or published.
    fn fetch_persist_publish(&self) -> Res<()> {
        let snapshot = self.fetch_catalog()?;
        self.persist(&snapshot);
        self.publish(snapshot);
        Ok(())
    }
}
impl CatalogRepository for IptvCatalogRepository {
    fn get_channels(&self) -> Channels {
        let read_guard = self.channels.read();
        read_guard.values().cloned().collect()
    }

    fn get_channel_by_id(&self, channel_id: &str) -> Option<Arc<Channel>> {
        let read_guard = self.channels.read();
        read_guard.get(channel_id).cloned()
    }

    fn get_feeds_by_channel(&self, channel_id: &str) -> Option<Feeds> {
        let read_guard = self.feeds_by_channel.read();
        read_guard.get(channel_id).cloned()
    }

    fn get_candidate_streams(&self, channel_id: &str, feed_id: Option<&str>) -> Option<Streams> {
        let read_guard = self.streams_by_channel.read();
        // Normalize: treat Some("") the same as None so reads align
        // with the feed side which stores empty feed_id as None.
        let feed_key = feed_id.filter(|s| !s.is_empty()).map(|s| s.into());
        read_guard.get(channel_id)?.get(&feed_key).cloned()
    }

    fn get_streams_by_channel(&self, channel_id: &str) -> Option<Streams> {
        let read_guard = self.streams_by_channel.read();
        let streams_by_feed = read_guard.get(channel_id)?;
        Some(streams_by_feed.values().flatten().cloned().collect())
    }

    fn has_streams(&self, channel_id: &str) -> bool {
        self.streams_by_channel.read().contains_key(channel_id)
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

    fn get_logos(&self) -> Logos {
        let read_guard = self.logos.read();
        read_guard.clone()
    }

    fn load(&self) -> Res<LoadOutcome> {
        let _guard = self.load_lock.lock();
        if let Some(snapshot) = self.cache.load_snapshot() {
            info!("IptvCatalogRepository - load -  served from cache");
            self.publish(snapshot);
            return Ok(LoadOutcome::cache());
        }
        info!("IptvCatalogRepository - load -  cache missing or stale, fetching");
        self.fetch_persist_publish()?;
        Ok(LoadOutcome::network())
    }

    fn refresh(&self) -> Res<()> {
        let _guard = self.load_lock.lock();
        self.fetch_persist_publish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        CatalogSnapshot, Category, Country, Feed, Language, Logo, Logos, Programs, Stream,
    };
    use anyhow::Result as Res;
    use anyhow::anyhow;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

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
        logos: RwLock<Logos>,
        fetch_calls: AtomicUsize,
        fail_fetches: AtomicBool,
    }

    impl ChannelDataSource for MockDataSource {
        fn fetch_channels(&self) -> Res<Channels> {
            self.fetch_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_fetches.load(Ordering::SeqCst) {
                return Err(anyhow!("network down"));
            }
            Ok(self.channels.read().clone())
        }

        fn fetch_feeds(&self) -> Res<Feeds> {
            self.fetch_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_fetches.load(Ordering::SeqCst) {
                return Err(anyhow!("network down"));
            }
            Ok(self.feeds.read().clone())
        }

        fn fetch_streams(&self) -> Res<Streams> {
            self.fetch_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_fetches.load(Ordering::SeqCst) {
                return Err(anyhow!("network down"));
            }
            Ok(self.streams.read().clone())
        }

        fn fetch_countries(&self) -> Res<Countries> {
            self.fetch_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_fetches.load(Ordering::SeqCst) {
                return Err(anyhow!("network down"));
            }
            Ok(self.countries.read().clone())
        }

        fn fetch_languages(&self) -> Res<Languages> {
            self.fetch_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_fetches.load(Ordering::SeqCst) {
                return Err(anyhow!("network down"));
            }
            Ok(self.languages.read().clone())
        }

        fn fetch_guides(&self) -> Res<Programs> {
            Ok(vec![])
        }

        fn fetch_categories(&self) -> Res<Categories> {
            self.fetch_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_fetches.load(Ordering::SeqCst) {
                return Err(anyhow!("network down"));
            }
            Ok(self.categories.read().clone())
        }

        fn fetch_logos(&self) -> Res<Logos> {
            self.fetch_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_fetches.load(Ordering::SeqCst) {
                return Err(anyhow!("network down"));
            }
            Ok(self.logos.read().clone())
        }
    }

    /// Test double for `CacheStore` with per-save success flags, call
    /// counters, and an optional set of cached entities. When an entity is
    /// `None` it behaves as if it is missing from the cache (load fails).
    #[derive(Debug)]
    struct MockCacheStore {
        save_channels_ok: bool,
        save_feeds_ok: bool,
        save_streams_ok: bool,
        save_channels_calls: AtomicUsize,
        save_feeds_calls: AtomicUsize,
        save_streams_calls: AtomicUsize,
        channels: Option<Channels>,
        feeds: Option<Feeds>,
        streams: Option<Streams>,
        categories: Option<Categories>,
        countries: Option<Countries>,
        languages: Option<Languages>,
        logos: Option<Logos>,
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
                channels: None,
                feeds: None,
                streams: None,
                categories: None,
                countries: None,
                languages: None,
                logos: None,
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

        /// Simulate a fully populated cache holding the given snapshot.
        fn with_cached_snapshot(mut self, snapshot: CatalogSnapshot) -> Self {
            self.channels = Some(snapshot.channels);
            self.feeds = Some(snapshot.feeds);
            self.streams = Some(snapshot.streams);
            self.categories = Some(snapshot.categories);
            self.countries = Some(snapshot.countries);
            self.languages = Some(snapshot.languages);
            self.logos = Some(snapshot.logos);
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
            self.channels
                .clone()
                .ok_or_else(|| anyhow!("no cached channels"))
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
            self.feeds.clone().ok_or_else(|| anyhow!("no cached feeds"))
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
            self.streams
                .clone()
                .ok_or_else(|| anyhow!("no cached streams"))
        }

        fn save_categories(&self, _categories: &Categories) -> Res<()> {
            Ok(())
        }

        fn load_categories(&self) -> Res<Categories> {
            self.categories
                .clone()
                .ok_or_else(|| anyhow!("no cached categories"))
        }

        fn save_countries(&self, _countries: &Countries) -> Res<()> {
            Ok(())
        }

        fn load_countries(&self) -> Res<Countries> {
            self.countries
                .clone()
                .ok_or_else(|| anyhow!("no cached countries"))
        }

        fn save_languages(&self, _languages: &Languages) -> Res<()> {
            Ok(())
        }

        fn load_languages(&self) -> Res<Languages> {
            self.languages
                .clone()
                .ok_or_else(|| anyhow!("no cached languages"))
        }

        fn save_programs(&self, _programs: &Programs) -> Res<()> {
            Ok(())
        }

        fn load_programs(&self) -> Res<Programs> {
            Err(anyhow!("no cached programs"))
        }

        fn save_logos(&self, _logos: &Logos) -> Res<()> {
            Ok(())
        }

        fn load_logos(&self) -> Res<Logos> {
            self.logos.clone().ok_or_else(|| anyhow!("no cached logos"))
        }
    }

    fn channel(id: &str) -> Arc<Channel> {
        Arc::new(Channel {
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

    fn stream(channel_id: &str, feed_id: &str) -> Arc<Stream> {
        Arc::new(Stream {
            channel_id: channel_id.to_string(),
            feed_id: feed_id.to_string(),
            url: format!("https://example.com/{}/{}.m3u8", channel_id, feed_id),
            quality: "720p".to_string(),
            referrer: String::new(),
            title: format!("Stream {}/{}", channel_id, feed_id),
            user_agent: String::new(),
        })
    }

    fn category(id: &str, name: &str) -> Arc<Category> {
        Arc::new(Category {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
        })
    }

    fn country(code: &str, name: &str, languages: &[&str]) -> Arc<Country> {
        Arc::new(Country {
            code: code.to_string(),
            name: name.to_string(),
            languages: languages.iter().map(|s| s.to_string()).collect(),
            flag_url: String::new(),
        })
    }

    fn language(code: &str, name: &str) -> Arc<Language> {
        Arc::new(Language {
            code: code.to_string(),
            name: name.to_string(),
        })
    }

    fn logo(channel: &str, url: &str) -> Arc<Logo> {
        Arc::new(Logo {
            channel: channel.to_string(),
            format: "".to_string(),
            url: url.to_string(),
            width: 0,
            height: 0,
        })
    }

    /// A fully populated snapshot used by the cache-aware load tests.
    fn snapshot() -> CatalogSnapshot {
        CatalogSnapshot {
            channels: vec![channel("ch1")],
            feeds: vec![feed("feed1", "ch1")],
            streams: vec![stream("ch1", "feed1")],
            categories: vec![category("news", "News")],
            countries: vec![country("US", "United States", &["en"])],
            languages: vec![language("en", "English")],
            logos: vec![logo("ch1", "https://ch1-logo.png")],
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

        repository.refresh().unwrap();

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
        assert_eq!(repository.get_categories(), vec![category("news", "News")]);
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

        repository.refresh().unwrap();

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

        repository.refresh().unwrap();

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

        repository.refresh().unwrap();

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

        repository.refresh().unwrap();

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

        repository.refresh().unwrap();
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

        repository.refresh().unwrap();

        // Channels are wholesale-replaced across refreshes, so ch1 is gone.
        assert!(repository.get_channel_by_id("ch1").is_none());
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

        repository.refresh().unwrap();

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

        repository.refresh().unwrap();

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

        repository.refresh().unwrap();

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
    fn has_streams_is_true_for_channel_with_feed_streams() {
        let data_source = MockDataSource {
            streams: RwLock::new(vec![stream("ch1", "feed1")]),
            ..Default::default()
        };
        let repository =
            IptvCatalogRepository::new(Arc::new(data_source), Arc::new(MockCacheStore::default()));

        repository.refresh().unwrap();

        assert!(repository.has_streams("ch1"));
    }

    #[test]
    fn has_streams_is_true_for_channel_with_only_feedless_streams() {
        let data_source = MockDataSource {
            streams: RwLock::new(vec![stream("ch1", "")]),
            ..Default::default()
        };
        let repository =
            IptvCatalogRepository::new(Arc::new(data_source), Arc::new(MockCacheStore::default()));

        repository.refresh().unwrap();

        assert!(repository.has_streams("ch1"));
    }

    #[test]
    fn has_streams_is_false_for_channel_with_feeds_but_no_streams() {
        let data_source = MockDataSource {
            channels: RwLock::new(vec![channel("ch1"), channel("ch2")]),
            feeds: RwLock::new(vec![feed("feed1", "ch1"), feed("feed2", "ch2")]),
            streams: RwLock::new(vec![stream("ch2", "feed2")]),
            ..Default::default()
        };
        let repository =
            IptvCatalogRepository::new(Arc::new(data_source), Arc::new(MockCacheStore::default()));

        repository.refresh().unwrap();

        assert!(repository.get_feeds_by_channel("ch1").is_some());
        assert!(!repository.has_streams("ch1"));
        assert!(repository.has_streams("ch2"));
    }

    #[test]
    fn has_streams_is_false_for_unknown_channel() {
        let data_source = MockDataSource {
            streams: RwLock::new(vec![stream("ch1", "feed1")]),
            ..Default::default()
        };
        let repository =
            IptvCatalogRepository::new(Arc::new(data_source), Arc::new(MockCacheStore::default()));

        repository.refresh().unwrap();

        assert!(!repository.has_streams("unknown"));
    }

    #[test]
    fn has_streams_reflects_latest_refresh() {
        let data_source = Arc::new(MockDataSource {
            streams: RwLock::new(vec![stream("ch1", "feed1")]),
            ..Default::default()
        });
        let repository =
            IptvCatalogRepository::new(data_source.clone(), Arc::new(MockCacheStore::default()));

        repository.refresh().unwrap();
        assert!(repository.has_streams("ch1"));

        *data_source.streams.write() = vec![stream("ch2", "feed2")];
        repository.refresh().unwrap();

        assert!(!repository.has_streams("ch1"));
        assert!(repository.has_streams("ch2"));
    }

    #[test]
    fn get_streams_by_channel_returns_streams_across_all_feeds() {
        let data_source = MockDataSource {
            streams: RwLock::new(vec![
                stream("ch1", ""),
                stream("ch1", "feed1"),
                stream("ch1", "feed2"),
                stream("ch2", "feed1"),
            ]),
            ..Default::default()
        };
        let repository =
            IptvCatalogRepository::new(Arc::new(data_source), Arc::new(MockCacheStore::default()));

        repository.refresh().unwrap();

        let streams = repository
            .get_streams_by_channel("ch1")
            .expect("ch1 should have streams");
        assert_eq!(streams.len(), 3);
        assert!(streams.contains(&stream("ch1", "")));
        assert!(streams.contains(&stream("ch1", "feed1")));
        assert!(streams.contains(&stream("ch1", "feed2")));
        assert!(!streams.contains(&stream("ch2", "feed1")));
    }

    #[test]
    fn get_streams_by_channel_returns_none_for_unknown_channel() {
        let data_source = MockDataSource {
            streams: RwLock::new(vec![stream("ch1", "feed1")]),
            ..Default::default()
        };
        let repository =
            IptvCatalogRepository::new(Arc::new(data_source), Arc::new(MockCacheStore::default()));

        repository.refresh().unwrap();

        assert!(repository.get_streams_by_channel("unknown").is_none());
    }

    #[test]
    fn refresh_publishes_channels_when_cache_save_fails() {
        let data_source = MockDataSource {
            channels: RwLock::new(vec![channel("ch1")]),
            ..Default::default()
        };
        let cache = Arc::new(MockCacheStore::default().with_save_channels_failing());
        let repository = IptvCatalogRepository::new(Arc::new(data_source), cache.clone());

        repository.refresh().unwrap();

        // The save was attempted ...
        assert_eq!(cache.save_channels_calls.load(Ordering::SeqCst), 1);
        // ... but the data was still published.
        assert_eq!(repository.get_channels(), vec![channel("ch1")]);
        assert_eq!(repository.get_channel_by_id("ch1"), Some(channel("ch1")));
    }

    #[test]
    fn refresh_publishes_feeds_when_cache_save_fails() {
        let data_source = MockDataSource {
            feeds: RwLock::new(vec![feed("feed1", "ch1")]),
            ..Default::default()
        };
        let cache = Arc::new(MockCacheStore::default().with_save_feeds_failing());
        let repository = IptvCatalogRepository::new(Arc::new(data_source), cache.clone());

        repository.refresh().unwrap();

        assert_eq!(cache.save_feeds_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            repository.get_feeds_by_channel("ch1"),
            Some(vec![feed("feed1", "ch1")])
        );
    }

    #[test]
    fn refresh_publishes_streams_when_cache_save_fails() {
        let data_source = MockDataSource {
            streams: RwLock::new(vec![stream("ch1", "feed1")]),
            ..Default::default()
        };
        let cache = Arc::new(MockCacheStore::default().with_save_streams_failing());
        let repository = IptvCatalogRepository::new(Arc::new(data_source), cache.clone());

        repository.refresh().unwrap();

        assert_eq!(cache.save_streams_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            repository.get_candidate_streams("ch1", Some("feed1")),
            Some(vec![stream("ch1", "feed1")])
        );
        assert!(repository.has_streams("ch1"));
        assert_eq!(
            repository.get_streams_by_channel("ch1"),
            Some(vec![stream("ch1", "feed1")])
        );
    }

    #[test]
    fn refresh_with_empty_data_source_leaves_maps_empty() {
        let repository = IptvCatalogRepository::new(
            Arc::new(MockDataSource::default()),
            Arc::new(MockCacheStore::default()),
        );

        repository.refresh().unwrap();

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

        repository.refresh().unwrap();

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

        repository.refresh().unwrap();

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

        repository.refresh().unwrap();

        assert_eq!(
            repository.get_languages(),
            vec![language("en", "English"), language("fr", "French")]
        );
    }

    #[test]
    fn refresh_populates_logo() {
        let data_source = MockDataSource {
            logos: RwLock::new(vec![
                logo("ch1", "https://ch1-logo.png"),
                logo("ch2", "https://ch2-logo.jpg"),
            ]),
            ..Default::default()
        };
        let repository =
            IptvCatalogRepository::new(Arc::new(data_source), Arc::new(MockCacheStore::default()));

        repository.refresh().unwrap();

        assert_eq!(
            repository.get_logos(),
            vec![
                logo("ch1", "https://ch1-logo.png"),
                logo("ch2", "https://ch2-logo.jpg")
            ]
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

        repository.refresh().unwrap();
        assert_eq!(repository.get_categories(), vec![category("news", "News")]);

        *data_source.categories.write() = vec![category("sports", "Sports")];

        repository.refresh().unwrap();

        assert_eq!(
            repository.get_categories(),
            vec![category("sports", "Sports")]
        );
    }

    #[test]
    fn refresh_replaces_countries_on_second_call() {
        let data_source = Arc::new(MockDataSource {
            countries: RwLock::new(vec![country("US", "United States", &["en"])]),
            ..Default::default()
        });
        let repository =
            IptvCatalogRepository::new(data_source.clone(), Arc::new(MockCacheStore::default()));

        repository.refresh().unwrap();
        assert_eq!(
            repository.get_countries(),
            vec![country("US", "United States", &["en"])]
        );

        *data_source.countries.write() = vec![country("DE", "Germany", &["de"])];

        repository.refresh().unwrap();

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

        repository.refresh().unwrap();
        assert_eq!(repository.get_languages(), vec![language("en", "English")]);

        *data_source.languages.write() = vec![language("fr", "French")];

        repository.refresh().unwrap();

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

    #[test]
    fn load_uses_fresh_cache_without_calling_source() {
        let data_source = Arc::new(MockDataSource::default());
        let cache = Arc::new(MockCacheStore::default().with_cached_snapshot(snapshot()));
        let repository = IptvCatalogRepository::new(data_source.clone(), cache);

        let outcome = repository.load().unwrap();

        assert_eq!(outcome.source, CatalogSource::Cache);
        assert_eq!(data_source.fetch_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn load_publishes_cached_data() {
        let repository = IptvCatalogRepository::new(
            Arc::new(MockDataSource::default()),
            Arc::new(MockCacheStore::default().with_cached_snapshot(snapshot())),
        );

        repository.load().unwrap();

        assert_eq!(repository.get_channels(), vec![channel("ch1")]);
        assert_eq!(repository.get_channel_by_id("ch1"), Some(channel("ch1")));
        assert_eq!(
            repository.get_feeds_by_channel("ch1"),
            Some(vec![feed("feed1", "ch1")])
        );
        assert_eq!(
            repository.get_candidate_streams("ch1", Some("feed1")),
            Some(vec![stream("ch1", "feed1")])
        );
        assert_eq!(repository.get_categories(), vec![category("news", "News")]);
        assert_eq!(
            repository.get_countries(),
            vec![country("US", "United States", &["en"])]
        );
        assert_eq!(repository.get_languages(), vec![language("en", "English")]);
        assert_eq!(
            repository.get_logos(),
            vec![logo("ch1", "https://ch1-logo.png")]
        );
    }

    #[test]
    fn load_does_not_persist_on_cache_hit() {
        let cache = Arc::new(MockCacheStore::default().with_cached_snapshot(snapshot()));
        let repository =
            IptvCatalogRepository::new(Arc::new(MockDataSource::default()), cache.clone());

        repository.load().unwrap();

        assert_eq!(cache.save_channels_calls.load(Ordering::SeqCst), 0);
        assert_eq!(cache.save_feeds_calls.load(Ordering::SeqCst), 0);
        assert_eq!(cache.save_streams_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn load_fetches_when_cache_empty() {
        let data_source = Arc::new(MockDataSource {
            channels: RwLock::new(vec![channel("ch1")]),
            feeds: RwLock::new(vec![feed("feed1", "ch1")]),
            streams: RwLock::new(vec![stream("ch1", "feed1")]),
            ..Default::default()
        });
        let cache = Arc::new(MockCacheStore::default());
        let repository = IptvCatalogRepository::new(data_source.clone(), cache.clone());

        let outcome = repository.load().unwrap();

        assert_eq!(outcome.source, CatalogSource::Network);
        assert!(data_source.fetch_calls.load(Ordering::SeqCst) > 0);
        assert_eq!(repository.get_channels(), vec![channel("ch1")]);
        assert_eq!(cache.save_channels_calls.load(Ordering::SeqCst), 1);
        assert_eq!(cache.save_feeds_calls.load(Ordering::SeqCst), 1);
        assert_eq!(cache.save_streams_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn load_fetches_when_cache_partially_populated() {
        let data_source = Arc::new(MockDataSource {
            channels: RwLock::new(vec![channel("ch2")]),
            ..Default::default()
        });
        let cache = Arc::new(MockCacheStore {
            channels: Some(vec![channel("ch1")]),
            ..Default::default()
        });
        let repository = IptvCatalogRepository::new(data_source.clone(), cache);

        let outcome = repository.load().unwrap();

        assert_eq!(outcome.source, CatalogSource::Network);
        assert!(data_source.fetch_calls.load(Ordering::SeqCst) > 0);
        // The freshly fetched data is published, not the partial cache.
        assert_eq!(repository.get_channels(), vec![channel("ch2")]);
    }

    #[test]
    fn refresh_always_fetches_even_with_fresh_cache() {
        let data_source = Arc::new(MockDataSource {
            channels: RwLock::new(vec![channel("ch2")]),
            ..Default::default()
        });
        let repository = IptvCatalogRepository::new(
            data_source.clone(),
            Arc::new(MockCacheStore::default().with_cached_snapshot(snapshot())),
        );

        repository.refresh().unwrap();

        assert!(data_source.fetch_calls.load(Ordering::SeqCst) > 0);
        assert_eq!(repository.get_channels(), vec![channel("ch2")]);
    }

    #[test]
    fn load_propagates_source_error_and_keeps_existing_state() {
        let data_source = Arc::new(MockDataSource {
            channels: RwLock::new(vec![channel("ch1")]),
            ..Default::default()
        });
        let repository =
            IptvCatalogRepository::new(data_source.clone(), Arc::new(MockCacheStore::default()));

        repository.load().unwrap();
        assert_eq!(repository.get_channels(), vec![channel("ch1")]);

        // The source now fails and the cache is empty.
        data_source.fail_fetches.store(true, Ordering::SeqCst);
        let err = repository.load().unwrap_err();

        assert!(err.to_string().contains("network down"));
        // Existing state is preserved: nothing was published.
        assert_eq!(repository.get_channels(), vec![channel("ch1")]);
    }

    #[test]
    fn refresh_publishes_even_when_cache_save_fails() {
        let data_source = MockDataSource {
            channels: RwLock::new(vec![channel("ch1")]),
            feeds: RwLock::new(vec![feed("feed1", "ch1")]),
            streams: RwLock::new(vec![stream("ch1", "feed1")]),
            ..Default::default()
        };
        let cache = Arc::new(MockCacheStore::default().with_save_channels_failing());
        let repository = IptvCatalogRepository::new(Arc::new(data_source), cache.clone());

        repository.refresh().unwrap();

        // The save was attempted ...
        assert_eq!(cache.save_channels_calls.load(Ordering::SeqCst), 1);
        // ... but the data was still published.
        assert_eq!(repository.get_channels(), vec![channel("ch1")]);
        assert_eq!(
            repository.get_feeds_by_channel("ch1"),
            Some(vec![feed("feed1", "ch1")])
        );
        assert_eq!(
            repository.get_candidate_streams("ch1", Some("feed1")),
            Some(vec![stream("ch1", "feed1")])
        );
    }
}
