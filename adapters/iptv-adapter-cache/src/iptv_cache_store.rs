use crate::store::Store;
use anyhow::{Context, Result as Res};
use core::ports::CacheStore;
use std::fmt::Debug;
use std::sync::Arc;

/**
* This is the concrete cache implementation that supports file based caching
* for the IPTV library. This cache implementation provides a persistent
* file store relying on the underlying store to provide the basic caching
* functionality needed.
*/

#[derive(Debug)]
pub struct IptvCacheStore<S: Store> {
    store: Arc<S>,
}

impl<S: Store> IptvCacheStore<S> {
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }
}

impl<S: Store> CacheStore for IptvCacheStore<S> {
    fn save_channels(&self, channels: &core::domain::channel::Channels) -> Res<()> {
        self.store.save_item("iptv:channels", channels)
    }

    fn load_channels(&self) -> Res<core::domain::channel::Channels> {
        self.store
            .get_item("iptv:channels")
            .with_context(|| "No known channels for given key:iptv:channels")
    }

    fn save_feeds(&self, feeds: &core::domain::feed::Feeds) -> Res<()> {
        self.store.save_item("iptv:feeds", feeds)
    }

    fn load_feeds(&self) -> anyhow::Result<core::domain::feed::Feeds> {
        self.store
            .get_item("iptv:feeds")
            .with_context(|| "No known feeds for given key:iptv:feeds")
    }

    fn save_streams(&self, streams: &core::domain::stream::Streams) -> Res<()> {
        self.store.save_item("iptv:streams", streams)
    }

    fn load_streams(&self) -> anyhow::Result<core::domain::stream::Streams> {
        self.store
            .get_item("iptv:streams")
            .with_context(|| "No know streams for given key:iptv:streams")
    }

    fn save_categories(
        &self,
        categories: &core::domain::category::Categories,
    ) -> anyhow::Result<()> {
        self.store.save_item("iptv:categories", categories)
    }

    fn load_categories(&self) -> anyhow::Result<core::domain::category::Categories> {
        self.store
            .get_item("iptv:categories")
            .with_context(|| "No known categories for given key: iptv:categories")
    }

    fn save_countries(&self, countries: &core::domain::country::Countries) -> Res<()> {
        self.store.save_item("iptv:countries", countries)
    }

    fn load_countries(&self) -> anyhow::Result<core::domain::country::Countries> {
        self.store
            .get_item("iptv:countries")
            .with_context(|| "No known countries for key: iptv:countries")
    }

    fn save_languages(&self, languages: &core::domain::language::Languages) -> Res<()> {
        self.store.save_item("iptv:languages", languages)
    }

    fn load_languages(&self) -> anyhow::Result<core::domain::language::Languages> {
        self.store
            .get_item("iptv:languages")
            .with_context(|| "No known languages for given key:iptv:languages")
    }

    fn save_programs(&self, programs: &core::domain::program::Programs) -> Res<()> {
        self.store.save_item("iptv:programs", programs)
    }

    fn load_programs(&self) -> anyhow::Result<core::domain::program::Programs> {
        self.store
            .get_item("iptv:programs")
            .with_context(|| "No known programs for key: iptv:programs")
    }
}
