use anyhow::{Context, Result as Res};
use core::ports::FavoritesStore;
use log::warn;
use redb::{Database, ReadableDatabase};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

/**
* File-backed implementation of `FavoritesStore` using redb.
*
* Favorites are a single ordered list of channel ids, so they are stored as
* one versioned snapshot under a fixed key. The version field leaves room to
* migrate the on-disk format later without guessing at the payload shape.
*/
const TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("favorites");
const KEY: &str = "favorites:channel_ids";
const FORMAT_VERSION: u8 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct FavoritesSnapshot {
    version: u8,
    channel_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RedbFavoritesStore {
    db: Arc<Database>,
}

impl RedbFavoritesStore {
    pub fn new(path: impl Into<PathBuf>) -> Res<Self> {
        let path = path.into();
        let db = Database::create(&path)
            .with_context(|| format!("failed to open favorites database at {}", path.display()))?;
        Ok(Self { db: Arc::new(db) })
    }
}

impl FavoritesStore for RedbFavoritesStore {
    fn load(&self) -> Res<Vec<String>> {
        let read_txn = self.db.begin_read()?;
        let table = match read_txn.open_table(TABLE) {
            Ok(table) => table,
            // Nothing has been saved yet, so the table does not exist.
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };

        let Some(raw) = table.get(KEY)? else {
            return Ok(Vec::new());
        };
        let snapshot: FavoritesSnapshot = postcard::from_bytes(raw.value())?;
        if snapshot.version != FORMAT_VERSION {
            warn!(
                "RedbFavoritesStore - load - snapshot version {} does not match expected {}; loading anyway",
                snapshot.version, FORMAT_VERSION
            );
        }
        Ok(snapshot.channel_ids)
    }

    fn save(&self, channel_ids: &[String]) -> Res<()> {
        let snapshot = FavoritesSnapshot {
            version: FORMAT_VERSION,
            channel_ids: channel_ids.to_vec(),
        };
        let data = postcard::to_stdvec(&snapshot)?;

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(TABLE)?;
            table.insert(KEY, data.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn setup_store() -> (RedbFavoritesStore, PathBuf) {
        let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = temp_dir().join(format!("test_favorites_{counter}.redb"));
        let _ = std::fs::remove_file(&path);
        let store = RedbFavoritesStore::new(path.clone()).unwrap();
        (store, path)
    }

    #[test]
    fn load_returns_empty_when_nothing_saved() {
        let (store, _path) = setup_store();

        assert!(store.load().unwrap().is_empty());
    }

    #[test]
    fn save_and_load_round_trip() {
        let (store, _path) = setup_store();
        let ids = vec!["a".to_string(), "b".to_string()];

        store.save(&ids).unwrap();

        assert_eq!(store.load().unwrap(), ids);
    }

    #[test]
    fn save_replaces_previous_list() {
        let (store, _path) = setup_store();

        store.save(&["a".to_string()]).unwrap();
        store.save(&["b".to_string(), "c".to_string()]).unwrap();

        assert_eq!(store.load().unwrap(), vec!["b", "c"]);
    }

    #[test]
    fn favorites_survive_reopening_the_database() {
        let (store, path) = setup_store();
        store.save(&["a".to_string(), "b".to_string()]).unwrap();
        drop(store);

        let reopened = RedbFavoritesStore::new(path).unwrap();

        assert_eq!(reopened.load().unwrap(), vec!["a", "b"]);
    }
}
