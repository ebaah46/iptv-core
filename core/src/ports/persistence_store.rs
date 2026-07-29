use anyhow::Result as Res;

/**
* Describes the way in which the library receives or stores the
* disposable user-generated data that must survive regardless refresh or
* catalog data changes. These are user preferences.
* Concrete implementation could be a Simple database or a flat serialized file.
*/

pub trait PersistenceStore {
    // Retrieve stored item using given string key.
    fn get<T: Into<String>>(&self, key: T) -> Res<String>;

    // Set value in store with the given string key.
    fn set<T: Into<String>>(&self, key: T, value: T) -> Res<()>;
}
