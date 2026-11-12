use crate::domain::Channels;
use crate::ports::FavoritesStore;
use crate::services::CatalogRepository;
use anyhow::{Result as Res, bail};
use log::warn;
use parking_lot::RwLock;
use std::sync::Arc;

/**
* Business logic for the user's favorite channels.
*
* Favorites are stored as channel ids and resolved against the catalog on
* demand, so they survive catalog refreshes and channel metadata changes.
* The in-memory list is the read path; every mutation is written through to
* the `FavoritesStore` before it is committed in memory.
*/
pub trait FavoritesService: Send + Sync {
    /// Marks a channel as a favorite. Idempotent.
    fn add(&self, channel_id: &str) -> Res<()>;

    /// Removes a channel from favorites. Idempotent.
    fn remove(&self, channel_id: &str) -> Res<()>;

    /// Adds or removes a channel, returning `true` when it is now a favorite.
    fn toggle(&self, channel_id: &str) -> Res<bool>;

    /// Whether the channel is currently a favorite.
    fn is_favorite(&self, channel_id: &str) -> bool;

    /// Favorite channel ids, most-recent-first.
    fn get_favorite_ids(&self) -> Vec<String>;

    /// Favorite channels resolved against the catalog, most-recent-first.
    /// Ids that no longer exist in the catalog are skipped.
    fn get_favorites(&self) -> Channels;
}

#[derive(Debug)]
pub struct IptvFavoritesService {
    store: Arc<dyn FavoritesStore>,
    catalog: Arc<dyn CatalogRepository>,
    favorites: RwLock<Vec<String>>,
}

impl IptvFavoritesService {
    /// Creates the service and loads persisted favorites.
    ///
    /// A load failure is logged and treated as "no favorites" so a broken
    /// store cannot prevent the application from starting. Mutations still
    /// surface persistence errors to the caller.
    pub fn new(store: Arc<dyn FavoritesStore>, catalog: Arc<dyn CatalogRepository>) -> Self {
        let favorites = match store.load() {
            Ok(ids) => ids,
            Err(error) => {
                warn!("IptvFavoritesService - new - to load favorites: {error}");
                Vec::new()
            }
        };
        Self {
            store,
            catalog,
            favorites: RwLock::new(favorites),
        }
    }

    fn normalize(channel_id: &str) -> Res<&str> {
        let trimmed = channel_id.trim();
        if trimmed.is_empty() {
            bail!("IptvFavoritesService - normalize - channel id must not be empty");
        }
        Ok(trimmed)
    }

    fn persist(&self, channel_ids: &[String]) -> Res<()> {
        self.store.save(channel_ids)
    }
}

impl FavoritesService for IptvFavoritesService {
    fn add(&self, channel_id: &str) -> Res<()> {
        let channel_id = Self::normalize(channel_id)?;
        let mut guard = self.favorites.write();
        if guard.iter().any(|id| id == channel_id) {
            return Ok(());
        }

        let mut next = guard.clone();
        next.insert(0, channel_id.to_string());
        self.persist(&next)?;
        *guard = next;
        Ok(())
    }

    fn remove(&self, channel_id: &str) -> Res<()> {
        let channel_id = Self::normalize(channel_id)?;
        let mut guard = self.favorites.write();
        if !guard.iter().any(|id| id == channel_id) {
            return Ok(());
        }

        let next: Vec<String> = guard
            .iter()
            .filter(|id| id.as_str() != channel_id)
            .cloned()
            .collect();
        self.persist(&next)?;
        *guard = next;
        Ok(())
    }

    fn toggle(&self, channel_id: &str) -> Res<bool> {
        let channel_id = Self::normalize(channel_id)?;
        let mut guard = self.favorites.write();
        let was_favorite = guard.iter().any(|id| id == channel_id);

        let mut next = guard.clone();
        if was_favorite {
            next.retain(|id| id != channel_id);
        } else {
            next.insert(0, channel_id.to_string());
        }
        self.persist(&next)?;
        *guard = next;
        Ok(!was_favorite)
    }

    fn is_favorite(&self, channel_id: &str) -> bool {
        let Ok(channel_id) = Self::normalize(channel_id) else {
            return false;
        };
        self.favorites.read().iter().any(|id| id == channel_id)
    }

    fn get_favorite_ids(&self) -> Vec<String> {
        self.favorites.read().clone()
    }

    fn get_favorites(&self) -> Channels {
        let ids = self.favorites.read();
        ids.iter()
            .filter_map(|id| self.catalog.get_channel_by_id(id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Channel;
    use crate::services::test_support::{MockCatalogRepository, MockFavoritesStore};

    fn channel(id: &str) -> Arc<Channel> {
        Arc::new(Channel {
            id: id.to_string(),
            name: format!("Channel {id}"),
            alt_names: Vec::new(),
            category_ids: Vec::new(),
            country_code: "US".to_string(),
            is_nsfw: false,
            launched: None,
            closed: None,
            website: String::new(),
            network: String::new(),
        })
    }

    fn service(store: Arc<MockFavoritesStore>, channels: Channels) -> IptvFavoritesService {
        IptvFavoritesService::new(store, Arc::new(MockCatalogRepository::new(channels)))
    }

    #[test]
    fn new_loads_persisted_favorites() {
        let store = Arc::new(MockFavoritesStore::with_ids(&["a", "b"]));
        let service = service(store, Vec::new());

        assert_eq!(service.get_favorite_ids(), vec!["a", "b"]);
    }

    #[test]
    fn new_starts_empty_when_load_fails() {
        let store = Arc::new(MockFavoritesStore::default().failing_load());
        let service = service(store, Vec::new());

        assert!(service.get_favorite_ids().is_empty());
    }

    #[test]
    fn add_persists_and_marks_favorite() {
        let store = Arc::new(MockFavoritesStore::default());
        let service = service(store.clone(), Vec::new());

        service.add("a").unwrap();

        assert!(service.is_favorite("a"));
        assert_eq!(service.get_favorite_ids(), vec!["a"]);
        assert_eq!(store.ids(), vec!["a"]);
        assert_eq!(store.save_calls(), 1);
    }

    #[test]
    fn add_is_idempotent() {
        let store = Arc::new(MockFavoritesStore::default());
        let service = service(store.clone(), Vec::new());

        service.add("a").unwrap();
        service.add("a").unwrap();

        assert_eq!(service.get_favorite_ids(), vec!["a"]);
        assert_eq!(store.save_calls(), 1);
    }

    #[test]
    fn add_puts_newest_first() {
        let store = Arc::new(MockFavoritesStore::default());
        let service = service(store, Vec::new());

        service.add("a").unwrap();
        service.add("b").unwrap();

        assert_eq!(service.get_favorite_ids(), vec!["b", "a"]);
    }

    #[test]
    fn remove_persists_and_unmarks_favorite() {
        let store = Arc::new(MockFavoritesStore::with_ids(&["a", "b"]));
        let service = service(store.clone(), Vec::new());

        service.remove("a").unwrap();

        assert!(!service.is_favorite("a"));
        assert_eq!(service.get_favorite_ids(), vec!["b"]);
        assert_eq!(store.ids(), vec!["b"]);
    }

    #[test]
    fn remove_is_idempotent() {
        let store = Arc::new(MockFavoritesStore::with_ids(&["a"]));
        let service = service(store.clone(), Vec::new());

        service.remove("missing").unwrap();

        assert_eq!(service.get_favorite_ids(), vec!["a"]);
        assert_eq!(store.save_calls(), 0);
    }

    #[test]
    fn toggle_adds_then_removes() {
        let store = Arc::new(MockFavoritesStore::default());
        let service = service(store, Vec::new());

        assert!(service.toggle("a").unwrap());
        assert!(service.is_favorite("a"));

        assert!(!service.toggle("a").unwrap());
        assert!(!service.is_favorite("a"));
    }

    #[test]
    fn is_favorite_is_false_for_unknown_and_blank_ids() {
        let store = Arc::new(MockFavoritesStore::with_ids(&["a"]));
        let service = service(store, Vec::new());

        assert!(!service.is_favorite("b"));
        assert!(!service.is_favorite("   "));
    }

    #[test]
    fn get_favorites_resolves_known_channels_in_order() {
        let store = Arc::new(MockFavoritesStore::with_ids(&["b", "a"]));
        let service = service(store, vec![channel("a"), channel("b")]);

        let favorites = service.get_favorites();

        assert_eq!(favorites.len(), 2);
        assert_eq!(favorites[0].id, "b");
        assert_eq!(favorites[1].id, "a");
    }

    #[test]
    fn get_favorites_skips_ids_missing_from_catalog() {
        let store = Arc::new(MockFavoritesStore::with_ids(&["gone", "a"]));
        let service = service(store, vec![channel("a")]);

        let favorites = service.get_favorites();

        assert_eq!(favorites.len(), 1);
        assert_eq!(favorites[0].id, "a");
    }

    #[test]
    fn add_rejects_blank_id() {
        let store = Arc::new(MockFavoritesStore::default());
        let service = service(store.clone(), Vec::new());

        assert!(service.add("   ").is_err());
        assert!(service.get_favorite_ids().is_empty());
        assert_eq!(store.save_calls(), 0);
    }

    #[test]
    fn add_trims_surrounding_whitespace() {
        let store = Arc::new(MockFavoritesStore::default());
        let service = service(store, Vec::new());

        service.add("  a  ").unwrap();

        assert_eq!(service.get_favorite_ids(), vec!["a"]);
        assert!(service.is_favorite(" a "));
    }

    #[test]
    fn add_rolls_back_when_persist_fails() {
        let store = Arc::new(MockFavoritesStore::default().failing_save());
        let service = service(store, Vec::new());

        assert!(service.add("a").is_err());
        assert!(!service.is_favorite("a"));
        assert!(service.get_favorite_ids().is_empty());
    }

    #[test]
    fn remove_keeps_favorite_when_persist_fails() {
        let store = Arc::new(MockFavoritesStore::with_ids(&["a"]).failing_save());
        let service = service(store, Vec::new());

        assert!(service.remove("a").is_err());
        assert!(service.is_favorite("a"));
        assert_eq!(service.get_favorite_ids(), vec!["a"]);
    }

    #[test]
    fn toggle_keeps_state_when_persist_fails() {
        let store = Arc::new(MockFavoritesStore::with_ids(&["a"]).failing_save());
        let service = service(store, Vec::new());

        assert!(service.toggle("a").is_err());
        assert!(service.is_favorite("a"));
    }
}
