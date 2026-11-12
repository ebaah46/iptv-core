pub mod catalog_query;
pub mod catalog_repository;
pub mod catalog_service;
pub mod favorites_service;
mod playback_controller;
pub(crate) mod stream_resolver;

#[cfg(test)]
pub(crate) mod test_support;

pub use catalog_query::{CatalogQuery, CategoryMatch};
pub use catalog_repository::{CatalogRepository, CatalogSource, LoadOutcome};
pub use catalog_service::{CatalogService, IptvCatalogService};
pub use favorites_service::{FavoritesService, IptvFavoritesService};
pub use playback_controller::{IpTvPlaybackController, PlaybackController};
pub use stream_resolver::{IptvStreamResolver, StreamResolver};
