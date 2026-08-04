use crate::domain::{Channel, Channels, Feeds, Streams};
use crate::ports::{CacheStore, ChannelDataSource};
use log::info;
use parking_lot::RwLock;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

/**
* This is a repository that deals with access to the data available in the
* library. Thus, it serves as the boundary between the core logic and the
* raw data access.
* Services that handle the core logic can depend on this trait for access to data.
*/
pub trait CatalogRepository {
    /// Fetch all channels in this repository.
    fn get_channels(&self) -> Channels;

    /// Get channels with a given channel id
    fn get_channel_by_id(&self, channel_id: impl Into<String>) -> Option<Channel>;

    /// Get all the feeds associated with a channel
    fn get_feeds_by_channel(&self, channel_id: impl Into<String>) -> Option<Feeds>;

    /// Get all streams associated with a given channel
    fn get_candidate_streams(
        &self,
        channel_id: impl Into<String>,
        feed_id: Option<impl Into<String>>,
    ) -> Option<Streams>;

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
}

impl IptvCatalogRepository {
    pub fn new(data_source: Arc<dyn ChannelDataSource>, cache: Arc<dyn CacheStore>) -> Self {
        Self {
            cache,
            data_source,
            channels: Default::default(),
            feeds_by_channel: Default::default(),
            streams_by_key: Default::default(),
        }
    }
    fn fetch_catalog(&mut self) {
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
                        info!("IptvCatalogRepository - fetch_catalog - languages thread crashed unexpectedly:{:?}", e);
                        vec![]
                    }),
                )
            },
        );
        // Cache fetched data
        // let channels = channels.serialize();
    }
}
impl CatalogRepository for IptvCatalogRepository {
    fn get_channels(&self) -> Channels {
        let read_guard = self.channels.read();
        read_guard.values().cloned().collect()
    }

    fn get_channel_by_id(&self, channel_id: impl Into<String>) -> Option<Channel> {
        let read_guard = self.channels.read();
        read_guard.get(&channel_id.into()).cloned()
    }

    fn get_feeds_by_channel(&self, channel_id: impl Into<String>) -> Option<Feeds> {
        let read_guard = self.feeds_by_channel.read();
        read_guard.get(&channel_id.into()).cloned()
    }

    fn get_candidate_streams(
        &self,
        channel_id: impl Into<String>,
        feed_id: Option<impl Into<String>>,
    ) -> Option<Streams> {
        let read_guard = self.streams_by_key.read();
        read_guard
            .get(&(channel_id.into(), feed_id.map(|s| s.into())))
            .cloned()
    }

    fn refresh(&self) {
        todo!()
    }
}
