use crate::services::{CatalogService, PlaybackController};
use std::sync::Arc;

/**
* This facade is the main entry point for the core library.
* It organizes or the different components provided by the core under
* a single struct provides a unified interface for the IPTV core system.
*/
pub trait CoreFacade {
    fn get_catalog_service(&self) -> Arc<dyn CatalogService>;

    fn play(&self, channel_id: &str);

    fn pause(&self);
    fn stop(&self);

    fn seek(&self, position: u32);

    fn refresh(&self);
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

    fn refresh(&self) {
        self.catalog_service.refresh();
    }
}
