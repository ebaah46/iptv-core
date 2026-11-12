use crate::domain::Channels;
use crate::services::{CatalogQuery, CatalogService, LoadOutcome, PlaybackController};
use anyhow::Result as Res;
use std::sync::Arc;

/**
* This facade is the main entry point for the core library.
* It organizes or the different components provided by the core under
* a single struct provides a unified interface for the IPTV core system.
*/
pub trait CoreFacade {
    fn get_catalog_service(&self) -> Arc<dyn CatalogService>;

    /// Runs a combined catalog query through the catalog service.
    fn query_catalog(&self, query: &CatalogQuery) -> Channels;

    fn play(&self, channel_id: &str);

    fn pause(&self);
    fn stop(&self);

    fn seek(&self, position: u32);

    /// Load the catalog, preferring a fresh cache and falling back to the
    /// network when the cache is missing or stale.
    fn load_catalog(&self) -> Res<LoadOutcome> {
        self.refresh()?;
        Ok(LoadOutcome::network())
    }

    fn refresh(&self) -> Res<()>;
}
pub struct IptvFacade {
    pub catalog_service: Arc<dyn CatalogService>,
    pub playback_controller: Arc<dyn PlaybackController>,
}
impl IptvFacade {
    pub fn new(catalog: Arc<dyn CatalogService>, player: Arc<dyn PlaybackController>) -> Self {
        Self {
            catalog_service: catalog,
            playback_controller: player,
        }
    }
}

impl CoreFacade for IptvFacade {
    fn get_catalog_service(&self) -> Arc<dyn CatalogService> {
        self.catalog_service.clone()
    }

    fn query_catalog(&self, query: &CatalogQuery) -> Channels {
        self.catalog_service.query(query)
    }

    fn play(&self, channel_id: &str) {
        self.playback_controller.play(channel_id);
    }

    fn pause(&self) {
        self.playback_controller.pause();
    }

    fn stop(&self) {
        self.playback_controller.stop();
    }

    fn seek(&self, position: u32) {
        self.playback_controller.seek(position);
    }

    fn load_catalog(&self) -> Res<LoadOutcome> {
        self.catalog_service.load()
    }

    fn refresh(&self) -> Res<()> {
        self.catalog_service.refresh()
    }
}
