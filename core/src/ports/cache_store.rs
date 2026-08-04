use anyhow::Result as Res;

/**
* Describes the way in which the library receives or holds the
* disposable snapshot of the catalog of stream information.
* CacheStore trait provide access to a cache that holds retrieved
* catalog information. Cache can be file-based or in-memory.
*/

pub trait CacheStore {
    // Retrieve cached data
    fn get(&self, key: &str) -> Res<String>;

    // Store data in cache
    fn set(&self, key: &str, value: &str) -> Res<()>;

    // Remove item from cache
    fn remove(&self, key: &str) -> Res<()>;
}
