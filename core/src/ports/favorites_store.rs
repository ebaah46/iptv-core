use anyhow::Result as Res;
use std::fmt::Debug;

/**
* Describes the way in which the library persists the user's favorite
* channels. Favorites are user-generated data: unlike the disposable catalog
* snapshot, they must survive refreshes and catalog changes.
*
* The store deals in channel ids only. Resolving those ids to full channels is
* the responsibility of the favorites service, which reads the catalog.
*/
pub trait FavoritesStore: Debug + Send + Sync {
    /// Load the persisted favorite channel ids, most-recent-first.
    /// Returns an empty list when nothing has been saved yet.
    fn load(&self) -> Res<Vec<String>>;

    /// Persist the complete favorite channel id list, replacing any
    /// previously saved list.
    fn save(&self, channel_ids: &[String]) -> Res<()>;
}
