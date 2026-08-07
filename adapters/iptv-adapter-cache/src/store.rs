use redb::Database;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;

/**
* This trait wraps the basic store functionality required of a
* persistent/in-memory cache store. It provides the ability to retrieve, insert
* and evict a cache item.
*/
pub trait Store: Debug + Send + Sync {
    // get an item from the cache and deserialize it into the appropriate type
    fn get_item<T: DeserializeOwned>(&self, key: &str) -> Option<T>;

    // save an item into the cache but serialize it before
    fn save_item<T: Serialize>(&self, key: &str, value: T);

    // evict an item from the cache whether based on its ttl or not
    fn evict(&self, key: &str);
}

/**
* This is the concrete implementation of Store providing a file store type
* of cache.
*/
#[derive(Debug, Clone)]
pub struct FileStore {
    pub ttl_days: u8,
    pub path: PathBuf,
    db: Arc<Database>,
}
const TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("catalog_cache");
impl FileStore {
    pub fn new(path: PathBuf, ttl_days: u8) -> Self {
        let db = Arc::new(Database::create(&path).expect("Failed to create local cache"));
        Self { ttl_days, path, db }
    }
}

impl Store for FileStore {
    fn get_item<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        todo!()
    }

    fn save_item<T: Serialize>(&self, key: &str, value: T) {
        todo!()
    }

    fn evict(&self, key: &str) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::domain::{Category, Language};
    use std::env::temp_dir;
    use std::sync::atomic::{AtomicU8, Ordering};

    static TEST_COUNTER: AtomicU8 = AtomicU8::new(0);

    fn setup_store(ttl_days: u8) -> FileStore {
        let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = temp_dir().join(format!("test_cache_{}.redb", counter));
        // Clean up any previous test db
        let _ = std::fs::remove_file(&path);
        FileStore::new(path, ttl_days)
    }

    #[test]
    fn test_save_and_get_category() {
        let store = setup_store(7);
        let cat = Category {
            id: "sports".into(),
            name: "Sports".into(),
            description: "Sports and athletics channels".into(),
        };
        store.save_item("category:sports", &cat);
        let result: Option<Category> = store.get_item("category:sports");
        assert_eq!(result, Some(cat));
    }

    #[test]
    fn test_get_non_existent_key_returns_none() {
        let store = setup_store(7);
        let result: Option<Category> = store.get_item("non_existent");
        assert_eq!(result, None);
    }

    #[test]
    fn test_overwrite_existing_key() {
        let store = setup_store(7);
        let lang1 = Language {
            code: "en".into(),
            name: "English".into(),
        };
        let lang2 = Language {
            code: "fr".into(),
            name: "French".into(),
        };
        store.save_item("lang:1", &lang1);
        store.save_item("lang:1", &lang2);
        let result: Option<Language> = store.get_item("lang:1");
        assert_eq!(result, Some(lang2));
    }

    #[test]
    fn test_evict_removes_item() {
        let store = setup_store(7);
        let cat = Category {
            id: "news".into(),
            name: "News".into(),
            description: "News channels".into(),
        };
        store.save_item("category:news", &cat);
        let before: Option<Category> = store.get_item("category:news");
        assert_eq!(before, Some(cat));
        store.evict("category:news");
        let after: Option<Category> = store.get_item("category:news");
        assert_eq!(after, None);
    }

    #[test]
    fn test_evict_non_existent_key_is_noop() {
        let store = setup_store(7);
        // Should not panic
        store.evict("non_existent");
        // Store should still be usable after
        let lang = Language {
            code: "de".into(),
            name: "German".into(),
        };
        store.save_item("lang:de", &lang);
        let result: Option<Language> = store.get_item("lang:de");
        assert_eq!(result, Some(lang));
    }

    #[test]
    fn test_multiple_domain_types_independent() {
        let store = setup_store(7);
        let cat = Category {
            id: "movies".into(),
            name: "Movies".into(),
            description: "Movie channels".into(),
        };
        let lang = Language {
            code: "es".into(),
            name: "Spanish".into(),
        };
        store.save_item("cat:1", &cat);
        store.save_item("lang:1", &lang);
        assert_eq!(store.get_item::<Category>("cat:1"), Some(cat));
        assert_eq!(store.get_item::<Language>("lang:1"), Some(lang));
    }

    #[test]
    fn test_primitive_and_struct_mix() {
        let store = setup_store(7);
        let lang = Language {
            code: "pt".into(),
            name: "Portuguese".into(),
        };
        store.save_item("lang:pt", &lang);
        store.save_item("count", 42i32);
        assert_eq!(store.get_item::<Language>("lang:pt"), Some(lang));
        assert_eq!(store.get_item::<i32>("count"), Some(42));
    }

    #[test]
    fn test_cache_ttl_expiry() {
        // ttl_days = 0 means items expire immediately
        let store = setup_store(0);
        let cat = Category {
            id: "temp".into(),
            name: "Temporary".into(),
            description: "Should expire".into(),
        };
        store.save_item("category:temp", &cat);
        // With TTL = 0, now - inserted_at >= 0 is always true
        let result: Option<Category> = store.get_item("category:temp");
        assert_eq!(result, None);
    }
}
