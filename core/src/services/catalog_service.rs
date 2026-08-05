use crate::domain::Channels;

/**
* This trait provides the actual business logic that could be implemented in
* the IPTV catalog library. It handles core use cases like searching, filtering, etc.
*/
pub trait CatalogService {
    /// Search for a given channel. The search query should match to either
    /// the name of the channel or the alt_names
    fn search(&self, query: &str) -> Channels;

    fn filter_by_category_id(&self, category_id: &str) -> Channels;

    fn filter_by_country(&self, country_code: &str) -> Channels;

    fn filter_by_language(&self, language_code: &str) -> Channels;

    // Other filtering APIs will be provided as we go. But for not, this
    // is what is available
}
