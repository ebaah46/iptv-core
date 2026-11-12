use crate::services::{CatalogService, FavoritesService, PlaybackController};
use std::sync::Arc;

/**
* This facade is the main entry point for the core library.
* It organizes the different components provided by the core under a single
* struct and exposes their handles. Each service owns its own operations, so
* the facade stays readable as more and more features are added.
*/
pub trait CoreFacade {
    fn get_catalog_service(&self) -> Arc<dyn CatalogService>;

    fn get_favorites_service(&self) -> Arc<dyn FavoritesService>;

    fn get_playback_controller(&self) -> Arc<dyn PlaybackController>;
}

pub struct IptvFacade {
    pub catalog_service: Arc<dyn CatalogService>,
    pub favorites_service: Arc<dyn FavoritesService>,
    pub playback_controller: Arc<dyn PlaybackController>,
}

impl IptvFacade {
    pub fn new(
        catalog: Arc<dyn CatalogService>,
        favorites: Arc<dyn FavoritesService>,
        player: Arc<dyn PlaybackController>,
    ) -> Self {
        Self {
            catalog_service: catalog,
            favorites_service: favorites,
            playback_controller: player,
        }
    }
}

impl CoreFacade for IptvFacade {
    fn get_catalog_service(&self) -> Arc<dyn CatalogService> {
        self.catalog_service.clone()
    }

    fn get_favorites_service(&self) -> Arc<dyn FavoritesService> {
        self.favorites_service.clone()
    }

    fn get_playback_controller(&self) -> Arc<dyn PlaybackController> {
        self.playback_controller.clone()
    }
}
