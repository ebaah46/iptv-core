pub mod domain;
pub mod ports;

pub mod facade;
pub mod services;

pub use domain::CatalogSnapshot;
pub use services::{CatalogQuery, CatalogSource, CategoryMatch, LoadOutcome};
