use anyhow::Result as Res;

/**
* Describes the way in which the library receives or holds the
* disposable snapshot of the catalog of stream information.
* CacheStore trait provide access to a cache that holds retrieved
* catalog information. Cache can be file-based or in-memory.
*/

pub trait CacheStore {
    // Retrieve cached data
    fn get<T: Into<String>>(&self, key: T) -> Res<String>;

    // Store data in cache
    fn set<T: Into<String>>(&self, key: T, value: T) -> Res<()>;

    // Remove item from cache
    fn remove<T: Into<String>>(&self, key: T) -> Res<()>;
}
