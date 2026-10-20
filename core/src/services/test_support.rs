use crate::domain::{
    Categories, Channel, Channels, Countries, Feeds, Languages, Logos, Stream, Streams,
};
use crate::services::CatalogRepository;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A shared in-memory test double for `CatalogRepository`.
///
/// This is the single test implementation of the `CatalogRepository` trait used by
/// the unit tests of the services in this module. It holds preloaded data and serves
/// it back through the trait methods, so tests can verify the behaviour of callers
/// (e.g. services) against controlled data without a real data source.
///
/// Data is grouped per channel; streams are further grouped by the feed id to which
/// they belong (`None` for feedless streams), mirroring the layout used by the real
/// `IptvCatalogRepository`.
#[derive(Debug, Default)]
pub(crate) struct MockCatalogRepository {
    channels: Channels,
    feeds_by_channel: HashMap<String, Feeds>,
    streams_by_channel: HashMap<String, HashMap<Option<String>, Streams>>,
    categories: Categories,
    countries: Countries,
    languages: Languages,
    logos: Logos,
    refresh_count: AtomicUsize,
}

impl MockCatalogRepository {
    pub(crate) fn new(channels: Channels) -> Self {
        Self {
            channels,
            ..Default::default()
        }
    }

    /// Number of times `refresh` has been called. Useful for verifying that a
    /// service delegates refreshes to the repository.
    pub(crate) fn refresh_count(&self) -> usize {
        self.refresh_count.load(Ordering::SeqCst)
    }

    pub(crate) fn with_feeds_for_channel(mut self, channel_id: &str, feeds: Feeds) -> Self {
        self.feeds_by_channel.insert(channel_id.to_string(), feeds);
        self
    }

    pub(crate) fn with_streams_for_channel(
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

    /// Convenience: mark a `channel_id` as playable by storing a single feedless
    /// stream for it, so `has_streams`/`get_active_channels` report it as active.
    pub(crate) fn with_feedless_streams_for_channel(mut self, channel_id: &str) -> Self {
        let stream = Arc::new(Stream {
            channel_id: channel_id.to_string(),
            feed_id: String::new(),
            url: format!("https://example.com/{}.m3u8", channel_id),
            quality: String::new(),
            referrer: String::new(),
            title: String::new(),
            user_agent: String::new(),
        });
        self.streams_by_channel
            .entry(channel_id.to_string())
            .or_default()
            .insert(None, vec![stream]);
        self
    }

    pub(crate) fn with_streams_for_channels(self, channel_ids: &[&str]) -> Self {
        channel_ids.iter().fold(self, |repo, channel_id| {
            repo.with_feedless_streams_for_channel(channel_id)
        })
    }

    pub(crate) fn with_countries(mut self, countries: Countries) -> Self {
        self.countries = countries;
        self
    }

    pub(crate) fn with_logos(mut self, logos: Logos) -> Self {
        self.logos = logos;
        self
    }
}

impl CatalogRepository for MockCatalogRepository {
    fn get_channels(&self) -> Channels {
        self.channels.clone()
    }

    fn get_channel_by_id(&self, _channel_id: &str) -> Option<Arc<Channel>> {
        None
    }

    fn get_feeds_by_channel(&self, channel_id: &str) -> Option<Feeds> {
        self.feeds_by_channel.get(channel_id).cloned()
    }

    fn get_candidate_streams(&self, channel_id: &str, feed_id: Option<&str>) -> Option<Streams> {
        // Normalize: treat Some("") the same as None so reads align with how
        // feedless streams are stored (under the None key).
        let feed_key = feed_id.filter(|s| !s.is_empty()).map(|s| s.into());
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
        self.categories.clone()
    }

    fn get_countries(&self) -> Countries {
        self.countries.clone()
    }

    fn get_languages(&self) -> Languages {
        self.languages.clone()
    }

    fn get_logos(&self) -> Logos {
        self.logos.clone()
    }

    fn refresh(&self) {
        self.refresh_count.fetch_add(1, Ordering::SeqCst);
    }
}
