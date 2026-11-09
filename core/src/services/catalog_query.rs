/**
* A combined catalog query that composes search, country, language, and
* category filters into a single request.
*/

/// How a channel is matched against the selected category ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CategoryMatch {
    /// A channel must have every selected category.
    All,
    /// A channel must have at least one selected category.
    #[default]
    Any,
    /// A channel must have none of the selected categories.
    None,
}

/// A set of filters that can be applied to the catalog in one pass.
///
/// Filters are optional: an unset or empty filter is ignored. Category
/// matching is controlled by [`CategoryMatch`]; an empty category selection
/// is treated as "no category filter" for every mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogQuery {
    search_term: Option<String>,
    country_code: Option<String>,
    language_code: Option<String>,
    category_ids: Vec<String>,
    category_match: CategoryMatch,
    active_only: bool,
}

impl Default for CatalogQuery {
    fn default() -> Self {
        Self {
            search_term: None,
            country_code: None,
            language_code: None,
            category_ids: Vec::new(),
            category_match: CategoryMatch::Any,
            active_only: true,
        }
    }
}

impl CatalogQuery {
    /// Creates a query with no filters. Only active channels are returned.
    pub fn new() -> Self {
        Self::default()
    }

    /// Matches channels whose name or alt names contain `term`
    /// (case-insensitive). An empty or whitespace-only term is ignored.
    pub fn search(mut self, term: impl Into<String>) -> Self {
        self.search_term = Some(term.into());
        self
    }

    /// Matches channels whose country code equals `code`
    /// (case-insensitive). An empty code is ignored.
    pub fn country(mut self, code: impl Into<String>) -> Self {
        self.country_code = Some(code.into());
        self
    }

    /// Matches channels whose country speaks `code`
    /// (case-insensitive). An empty code is ignored.
    pub fn language(mut self, code: impl Into<String>) -> Self {
        self.language_code = Some(code.into());
        self
    }

    /// Replaces the selected category ids.
    pub fn categories<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.category_ids = ids.into_iter().map(Into::into).collect();
        self
    }

    /// Adds one category id to the selection.
    pub fn category(mut self, id: impl Into<String>) -> Self {
        self.category_ids.push(id.into());
        self
    }

    /// Sets how the selected categories are matched.
    pub fn category_match(mut self, mode: CategoryMatch) -> Self {
        self.category_match = mode;
        self
    }

    /// Includes channels without streams when set to `false`.
    pub fn active_only(mut self, active_only: bool) -> Self {
        self.active_only = active_only;
        self
    }

    pub fn search_term(&self) -> Option<&str> {
        self.search_term.as_deref()
    }

    pub fn country_code(&self) -> Option<&str> {
        self.country_code.as_deref()
    }

    pub fn language_code(&self) -> Option<&str> {
        self.language_code.as_deref()
    }

    pub fn category_ids(&self) -> &[String] {
        &self.category_ids
    }

    pub fn category_match_mode(&self) -> CategoryMatch {
        self.category_match
    }

    pub fn is_active_only(&self) -> bool {
        self.active_only
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_query_has_no_filters_and_is_active_only() {
        let query = CatalogQuery::new();

        assert_eq!(query.search_term(), None);
        assert_eq!(query.country_code(), None);
        assert_eq!(query.language_code(), None);
        assert!(query.category_ids().is_empty());
        assert_eq!(query.category_match_mode(), CategoryMatch::Any);
        assert!(query.is_active_only());
    }

    #[test]
    fn builders_set_the_expected_fields() {
        let query = CatalogQuery::new()
            .search("news")
            .country("US")
            .language("en")
            .categories(["news", "sports"])
            .category_match(CategoryMatch::All)
            .active_only(false);

        assert_eq!(query.search_term(), Some("news"));
        assert_eq!(query.country_code(), Some("US"));
        assert_eq!(query.language_code(), Some("en"));
        assert_eq!(query.category_ids().len(), 2);
        assert_eq!(query.category_ids()[0], "news");
        assert_eq!(query.category_ids()[1], "sports");
        assert_eq!(query.category_match_mode(), CategoryMatch::All);
        assert!(!query.is_active_only());
    }

    #[test]
    fn category_adds_to_the_selection() {
        let query = CatalogQuery::new().category("news").category("sports");

        assert_eq!(query.category_ids().len(), 2);
        assert_eq!(query.category_ids()[0], "news");
        assert_eq!(query.category_ids()[1], "sports");
    }

    #[test]
    fn categories_replaces_the_selection() {
        let query = CatalogQuery::new()
            .categories(["news", "sports"])
            .categories(["movies"]);

        assert_eq!(query.category_ids().len(), 1);
        assert_eq!(query.category_ids()[0], "movies");
    }

    #[test]
    fn category_match_defaults_to_any() {
        assert_eq!(
            CatalogQuery::new().category_match_mode(),
            CategoryMatch::Any
        );
    }
}
