use redb::Database;
use serde::{Deserialize, Serialize};
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
    fn get_item<'a, T: Deserialize<'a>>(&self, key: &str) -> Option<T>;

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
    fn get_item<'a, T: Deserialize<'a>>(&self, key: &str) -> Option<T> {
        todo!()
    }

    fn save_item<T: Serialize>(&self, key: &str, value: T) {
        todo!()
    }

    fn evict(&self, key: &str) {
        todo!()
    }
}
