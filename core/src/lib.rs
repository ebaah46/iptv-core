pub mod domain;
pub mod ports;

pub mod facade;
pub mod services;

pub use domain::CatalogSnapshot;
pub use ports::FavoritesStore;
pub use services::{
    CatalogQuery, CatalogSource, CategoryMatch, FavoritesService, IptvFavoritesService, LoadOutcome,
};
