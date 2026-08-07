use crate::store::Store;
use anyhow::Result as Res;
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
        todo!()
    }

    fn load_channels(&self) -> anyhow::Result<core::domain::channel::Channels> {
        todo!()
    }

    fn save_feeds(&self, feeds: &core::domain::feed::Feeds) -> Res<()> {
        todo!()
    }

    fn load_feeds(&self) -> anyhow::Result<core::domain::feed::Feeds> {
        todo!()
    }

    fn save_streams(&self, streams: &core::domain::stream::Streams) -> Res<()> {
        todo!()
    }

    fn load_streams(&self) -> anyhow::Result<core::domain::stream::Streams> {
        todo!()
    }

    fn save_categories(
        &self,
        categories: &core::domain::category::Categories,
    ) -> anyhow::Result<()> {
        todo!()
    }

    fn load_categories(&self) -> anyhow::Result<core::domain::category::Categories> {
        todo!()
    }

    fn save_countries(&self, countries: &core::domain::country::Countries) -> Res<()> {
        todo!()
    }

    fn load_countries(&self) -> anyhow::Result<core::domain::country::Countries> {
        todo!()
    }

    fn save_languages(&self, languages: &core::domain::language::Languages) -> Res<()> {
        todo!()
    }

    fn load_languages(&self) -> anyhow::Result<core::domain::language::Languages> {
        todo!()
    }

    fn save_programs(&self, programs: &core::domain::program::Programs) -> Res<()> {
        todo!()
    }

    fn load_programs(&self) -> anyhow::Result<core::domain::program::Programs> {
        todo!()
    }
}
