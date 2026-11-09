use crate::domain::{Categories, Channels, Countries, Languages, Logos};
use crate::services::CatalogRepository;
use crate::services::catalog_query::{CatalogQuery, CategoryMatch};
use log::info;
use std::sync::Arc;

/**
* This trait provides the actual business logic that could be implemented in
* the IPTV catalog library. It handles core use cases like searching, filtering, etc.
*/
pub trait CatalogService: Send + Sync {
    /// Search for a given channel. The search query should match to either
    /// the name of the channel or the alt_names
    fn search(&self, query: &str) -> Channels;

    fn filter_by_category_id(&self, category_id: &str) -> Channels;

    fn filter_by_country(&self, country_code: &str) -> Channels;

    fn filter_by_language(&self, language_code: &str) -> Channels;

    /// Runs a combined catalog query in a single pass over the catalog.
    ///
    /// Unset or empty filters are ignored. Category matching is controlled
    /// by [`CategoryMatch`]; an empty category selection applies no category
    /// filter for every mode.
    fn query(&self, query: &CatalogQuery) -> Channels;

    fn get_all(&self) -> Channels;

    fn get_active_channels(&self) -> Channels;

    fn get_categories(&self) -> Categories;

    fn get_countries(&self) -> Countries;

    fn get_languages(&self) -> Languages;

    fn get_logos(&self) -> Logos;

    fn refresh(&self);
    // Other filtering APIs will be provided as we go. But for not, this
    // is what is available
}

#[derive(Debug)]
pub struct IptvCatalogService {
    inner: Arc<dyn CatalogRepository>,
}

impl IptvCatalogService {
    pub fn new(registry: Arc<dyn CatalogRepository>) -> Self {
        Self { inner: registry }
    }

    fn has_streams(&self, channel_id: &str) -> bool {
        self.inner.has_streams(channel_id)
    }
}

impl CatalogService for IptvCatalogService {
    fn search(&self, query: &str) -> Channels {
        if query.is_empty() {
            return vec![];
        }
        let query = query.to_lowercase();
        self.inner
            .get_channels()
            .into_iter()
            .filter(|channel| {
                channel.name.contains(&query)
                    || channel.alt_names.iter().any(|alt| alt.contains(&query))
            })
            .collect()
    }

    fn filter_by_category_id(&self, category_id: &str) -> Channels {
        if category_id.is_empty() {
            return vec![];
        }
        self.inner
            .get_channels()
            .into_iter()
            .filter(|channel| {
                channel
                    .category_ids
                    .iter()
                    .any(|id| id.contains(category_id))
                    && self.has_streams(&channel.id)
            })
            .collect()
    }

    fn filter_by_country(&self, country_code: &str) -> Channels {
        if country_code.is_empty() {
            return vec![];
        }
        self.inner
            .get_channels()
            .into_iter()
            .filter(|channel| {
                channel.country_code.contains(&country_code) && self.has_streams(&channel.id)
            })
            .collect()
    }

    fn filter_by_language(&self, language_code: &str) -> Channels {
        if language_code.is_empty() {
            return vec![];
        }
        let country = self
            .inner
            .get_countries()
            .into_iter()
            .find(|country| country.languages.iter().any(|lang| lang == &language_code));
        match country {
            None => vec![],
            Some(country) => self.filter_by_country(&country.code),
        }
    }

    fn query(&self, query: &CatalogQuery) -> Channels {
        let search = query
            .search_term()
            .map(str::to_lowercase)
            .filter(|term| !term.trim().is_empty());
        let country = query.country_code().filter(|code| !code.trim().is_empty());
        let language = query.language_code().filter(|code| !code.trim().is_empty());
        let categories: Vec<&str> = query
            .category_ids()
            .iter()
            .map(String::as_str)
            .filter(|id| !id.trim().is_empty())
            .collect();

        // Resolve the language to every country that speaks it once, rather
        // than re-scanning the country list for each channel.
        let language_countries = language.map(|language| {
            self.inner
                .get_countries()
                .into_iter()
                .filter(|country| {
                    country
                        .languages
                        .iter()
                        .any(|code| code.eq_ignore_ascii_case(language))
                })
                .map(|country| country.code.clone())
                .collect::<Vec<_>>()
        });

        self.inner
            .get_channels()
            .into_iter()
            .filter(|channel| {
                if let Some(search) = &search {
                    let name_matches = channel.name.to_lowercase().contains(search.as_str());
                    let alt_matches = channel
                        .alt_names
                        .iter()
                        .any(|alt| alt.to_lowercase().contains(search.as_str()));
                    if !name_matches && !alt_matches {
                        return false;
                    }
                }

                if country.is_some_and(|code| !channel.country_code.eq_ignore_ascii_case(code)) {
                    return false;
                }

                if language_countries.as_ref().is_some_and(|countries| {
                    !countries
                        .iter()
                        .any(|code| code.eq_ignore_ascii_case(&channel.country_code))
                }) {
                    return false;
                }

                if !categories.is_empty() {
                    let has_category = |id: &str| {
                        channel
                            .category_ids
                            .iter()
                            .any(|category| category.eq_ignore_ascii_case(id))
                    };
                    let matches = match query.category_match_mode() {
                        CategoryMatch::All => categories.iter().copied().all(&has_category),
                        CategoryMatch::Any => categories.iter().copied().any(&has_category),
                        CategoryMatch::None => {
                            categories.iter().copied().all(|id| !has_category(id))
                        }
                    };
                    if !matches {
                        return false;
                    }
                }

                if query.is_active_only() && !self.has_streams(&channel.id) {
                    return false;
                }

                true
            })
            .collect()
    }

    fn get_all(&self) -> Channels {
        self.inner.get_channels()
    }

    fn get_active_channels(&self) -> Channels {
        info!("Total active channels:{}", self.inner.get_channels().len());
        self.inner
            .get_channels()
            .into_iter()
            .filter(|channel| self.has_streams(&channel.id))
            .collect()
    }

    fn get_categories(&self) -> Categories {
        self.inner.get_categories()
    }

    fn get_countries(&self) -> Countries {
        self.inner.get_countries()
    }

    fn get_languages(&self) -> Languages {
        self.inner.get_languages()
    }

    fn get_logos(&self) -> Logos {
        self.inner.get_logos()
    }

    fn refresh(&self) {
        self.inner.refresh();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Channel, Country, Logo};
    use crate::services::test_support::MockCatalogRepository;

    fn channel_with(
        id: &str,
        name: &str,
        alt_names: Vec<&str>,
        category_ids: Vec<&str>,
        country_code: &str,
    ) -> Arc<Channel> {
        Arc::new(Channel {
            id: id.to_string(),
            name: name.to_string(),
            alt_names: alt_names.into_iter().map(|s| s.to_string()).collect(),
            category_ids: category_ids.into_iter().map(|s| s.to_string()).collect(),
            country_code: country_code.to_string(),
            is_nsfw: false,
            launched: None,
            closed: None,
            website: String::new(),
            network: String::new(),
        })
    }

    fn service(repo: MockCatalogRepository) -> IptvCatalogService {
        IptvCatalogService::new(Arc::new(repo))
    }

    fn country_with(code: &str, name: &str, languages: &[String]) -> Arc<Country> {
        Arc::new(Country {
            code: code.into(),
            name: name.into(),
            languages: languages.into(),
            flag_url: "".to_string(),
        })
    }

    fn logo_with(channel: &str, url: &str) -> Arc<Logo> {
        Arc::new(Logo {
            channel: channel.to_string(),
            format: "".to_string(),
            url: url.to_string(),
            width: 0,
            height: 0,
        })
    }

    #[test]
    fn refresh_delegates_to_repository() {
        let repo = Arc::new(MockCatalogRepository::new(vec![]));
        let svc = IptvCatalogService::new(repo.clone());
        assert_eq!(repo.refresh_count(), 0);
        svc.refresh();
        assert_eq!(repo.refresh_count(), 1);
        svc.refresh();
        assert_eq!(repo.refresh_count(), 2);
    }

    #[test]
    fn get_all_channels() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("bbc1", "BBC One", vec![], vec![], "GB"),
                channel_with("cnn", "CNN", vec![], vec![], "US"),
            ])
            .with_streams_for_channels(&["bbc1", "cnn"]),
        );

        svc.refresh();

        let result = svc.get_all();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "bbc1");
        assert_eq!(result[1].id, "cnn");
    }

    #[test]
    fn search_matches_channel_by_name() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("bbc1", "bbc one", vec![], vec![], "GB"),
                channel_with("cnn", "cnn", vec![], vec![], "US"),
            ])
            .with_streams_for_channels(&["bbc1", "cnn"]),
        );
        let result = svc.search("BBC");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "bbc1");
    }

    #[test]
    fn search_matches_channel_by_alt_name() {
        let svc = service(
            MockCatalogRepository::new(vec![channel_with(
                "cnn",
                "cnn",
                vec!["cable news network"],
                vec![],
                "US",
            )])
            .with_feedless_streams_for_channel("cnn"),
        );

        let result = svc.search("Cable News");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "cnn");
    }

    #[test]
    fn search_matches_name_and_alt_names() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("bbc1", "bbc one", vec!["bbc1"], vec![], "GB"),
                channel_with("bbcworld", "bbc world", vec![], vec![], "GB"),
            ])
            .with_streams_for_channels(&["bbc1", "bbcworld"]),
        );

        let result = svc.search("bbc");

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn search_is_case_insensitive() {
        let svc = service(
            MockCatalogRepository::new(vec![channel_with(
                "disc",
                "discovery",
                vec![],
                vec![],
                "US",
            )])
            .with_feedless_streams_for_channel("disc"),
        );

        let result = svc.search("discovery");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "disc");
    }

    #[test]
    fn search_returns_empty_when_no_match() {
        let svc = service(
            MockCatalogRepository::new(vec![channel_with("bbc1", "BBC One", vec![], vec![], "GB")])
                .with_feedless_streams_for_channel("bbc1"),
        );

        assert!(svc.search("xyzzy").is_empty());
    }

    #[test]
    fn search_matches_only_name_or_alt_names() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("news24", "news24", vec![], vec!["sports"], "US"),
                channel_with("lateshow", "late show", vec![], vec!["sports"], "US"),
            ])
            .with_streams_for_channels(&["news24", "lateshow"]),
        );

        // Category ids and country codes must not leak into search results.
        assert!(svc.search("sports").is_empty());
        assert!(svc.search("US").is_empty());
        // A name match still works.
        assert_eq!(svc.search("news").len(), 1);
    }

    #[test]
    fn search_with_empty_or_whitespace_query_returns_empty() {
        let svc = service(
            MockCatalogRepository::new(vec![channel_with("bbc1", "BBC One", vec![], vec![], "GB")])
                .with_feedless_streams_for_channel("bbc1"),
        );
        assert!(svc.search("").is_empty());
        assert!(svc.search("   ").is_empty());
    }

    #[test]
    fn search_returns_all_partial_matches() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("news", "news", vec![], vec![], "GB"),
                channel_with("news24", "news24", vec![], vec![], "GB"),
                channel_with("sportsnews", "sports news", vec![], vec![], "GB"),
            ])
            .with_streams_for_channels(&["news", "news24", "sportsnews"]),
        );

        let result = svc.search("news");

        assert_eq!(result.len(), 3);
    }

    #[test]
    fn filter_by_category_id_matches_channels_with_id() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("bbc1", "bbc one", vec![], vec!["news"], "GB"),
                channel_with("mov1", "movie one", vec![], vec!["movies"], "US"),
            ])
            .with_streams_for_channels(&["bbc1", "mov1"]),
        );

        let result = svc.filter_by_category_id("news");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "bbc1");
    }

    #[test]
    fn filter_by_category_id_matches_any_entry() {
        let svc = service(
            MockCatalogRepository::new(vec![channel_with(
                "bbc1",
                "bbc one",
                vec![],
                vec!["news", "sports"],
                "GB",
            )])
            .with_feedless_streams_for_channel("bbc1"),
        );

        let result = svc.filter_by_category_id("sports");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "bbc1");
    }

    #[test]
    fn filter_by_category_id_returns_empty_for_unknown() {
        let svc = service(
            MockCatalogRepository::new(vec![channel_with(
                "bbc1",
                "bbc one",
                vec![],
                vec!["news"],
                "GB",
            )])
            .with_feedless_streams_for_channel("bbc1"),
        );

        assert!(svc.filter_by_category_id("movies").is_empty());
    }

    #[test]
    fn filter_by_category_id_with_empty_string_returns_empty() {
        let svc = service(
            MockCatalogRepository::new(vec![channel_with(
                "bbc1",
                "BBC One",
                vec![],
                vec!["news"],
                "GB",
            )])
            .with_feedless_streams_for_channel("bbc1"),
        );

        assert!(svc.filter_by_category_id("").is_empty());
    }

    #[test]
    fn filter_by_category_id_returns_all_matching_channels() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("bbc1", "BBC One", vec![], vec!["news"], "GB"),
                channel_with("sky1", "Sky News", vec![], vec!["news"], "GB"),
                channel_with("mov1", "Movie One", vec![], vec!["movies"], "US"),
            ])
            .with_streams_for_channels(&["bbc1", "sky1", "mov1"]),
        );

        let result = svc.filter_by_category_id("news");

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_by_country_matches_exact_code() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("us1", "US One", vec![], vec![], "US"),
                channel_with("gb1", "GB One", vec![], vec![], "GB"),
                channel_with("de1", "DE One", vec![], vec![], "DE"),
            ])
            .with_streams_for_channels(&["us1", "gb1", "de1"]),
        );

        let result = svc.filter_by_country("US");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "us1");
    }

    #[test]
    fn filter_by_country_returns_empty_for_unknown() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("us1", "US One", vec![], vec![], "US"),
                channel_with("gb1", "GB One", vec![], vec![], "GB"),
            ])
            .with_streams_for_channels(&["us1", "gb1"]),
        );

        assert!(svc.filter_by_country("FR").is_empty());
    }

    #[test]
    fn filter_by_country_with_empty_string_returns_empty() {
        let svc = service(
            MockCatalogRepository::new(vec![channel_with("us1", "US One", vec![], vec![], "US")])
                .with_feedless_streams_for_channel("us1"),
        );

        assert!(svc.filter_by_country("").is_empty());
    }

    #[test]
    fn filter_by_country_returns_all_matching_channels() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("us1", "US One", vec![], vec![], "US"),
                channel_with("sky1", "Sky News", vec![], vec![], "GB"),
                channel_with("bbc1", "BBC One", vec![], vec![], "GB"),
            ])
            .with_streams_for_channels(&["us1", "sky1", "bbc1"]),
        );

        let result = svc.filter_by_country("GB");

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_by_country_matches_all_with_that_code_in_order() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("a", "A", vec![], vec![], "US"),
                channel_with("b", "B", vec![], vec![], "DE"),
                channel_with("c", "C", vec![], vec![], "US"),
                channel_with("d", "D", vec![], vec![], "FR"),
                channel_with("e", "E", vec![], vec![], "US"),
            ])
            .with_streams_for_channels(&["a", "b", "c", "d", "e"]),
        );

        let result = svc.filter_by_country("US");

        let ids: Vec<&str> = result.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "c", "e"]);
    }

    #[test]
    fn filter_by_language_matches_channels_in_country_speaking_language() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("us1", "US One", vec![], vec![], "US"),
                channel_with("us2", "US Two", vec![], vec![], "US"),
                channel_with("gb1", "GB One", vec![], vec![], "GB"),
            ])
            .with_countries(vec![
                country_with("US", "United States", &["en".into()]),
                country_with("GB", "United Kingdom", &["de".into()]),
                country_with("FR", "France", &["fr".into()]),
            ])
            .with_streams_for_channels(&["us1", "us2", "gb1"]),
        );

        svc.refresh();

        let result = svc.filter_by_language("en");

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "us1");
        assert_eq!(result[1].id, "us2");
    }

    #[test]
    fn filter_by_language_excludes_channels_in_other_countries() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("us1", "US One", vec![], vec![], "US"),
                channel_with("gb1", "GB One", vec![], vec![], "GB"),
            ])
            .with_countries(vec![
                country_with("US", "United States", &["en".into()]),
                country_with("GB", "United Kingdom", &["de".into()]),
                country_with("FR", "France", &["fr".into()]),
            ])
            .with_streams_for_channels(&["us1", "gb1"]),
        );

        svc.refresh();

        let result = svc.filter_by_language("en");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "us1");
    }

    #[test]
    fn filter_by_language_matches_multi_language_country() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("ca1", "CA One", vec![], vec![], "CA"),
                channel_with("us1", "US One", vec![], vec![], "US"),
            ])
            .with_countries(vec![
                country_with("CA", "Canada", &["en".into(), "fr".into()]),
                country_with("US", "United States", &["en".into()]),
            ])
            .with_streams_for_channels(&["ca1", "us1"]),
        );

        svc.refresh();

        let result = svc.filter_by_language("fr");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "ca1");
    }

    #[test]
    fn filter_by_language_respects_country_language() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("us1", "US One", vec![], vec![], "US"),
                channel_with("de1", "DE One", vec![], vec![], "DE"),
            ])
            .with_countries(vec![
                country_with("US", "United States", &["en".into()]),
                country_with("DE", "Germany", &["de".into()]),
                country_with("FR", "France", &["fr".into()]),
            ])
            .with_streams_for_channels(&["us1", "de1"]),
        );

        svc.refresh();

        let result = svc.filter_by_language("de");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "de1");
    }

    #[test]
    fn filter_by_language_is_case_insensitive() {
        let svc = service(
            MockCatalogRepository::new(vec![channel_with("us1", "us one", vec![], vec![], "US")])
                .with_countries(vec![country_with("US", "united states", &["en".into()])])
                .with_feedless_streams_for_channel("us1"),
        );

        svc.refresh();

        let result = svc.filter_by_language("en");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "us1");
    }

    #[test]
    fn filter_by_language_returns_empty_for_unknown_code() {
        let svc = service(
            MockCatalogRepository::new(vec![channel_with("us1", "US One", vec![], vec![], "US")])
                .with_countries(vec![country_with("US", "United States", &["en".into()])])
                .with_feedless_streams_for_channel("us1"),
        );

        svc.refresh();

        assert!(svc.filter_by_language("xx").is_empty());
    }

    #[test]
    fn filter_by_language_with_empty_string_returns_empty() {
        let svc = service(
            MockCatalogRepository::new(vec![channel_with("us1", "US One", vec![], vec![], "US")])
                .with_countries(vec![country_with("US", "United States", &["en".into()])])
                .with_feedless_streams_for_channel("us1"),
        );

        svc.refresh();

        assert!(svc.filter_by_language("").is_empty());
    }

    #[test]
    fn filter_by_language_deduplicates_channels() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("us1", "US One", vec![], vec![], "US"),
                channel_with("us2", "US Two", vec![], vec![], "US"),
            ])
            .with_countries(vec![country_with("US", "United States", &["en".into()])])
            .with_streams_for_channels(&["us1", "us2"]),
        );

        svc.refresh();

        let result = svc.filter_by_language("en");

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_by_language_matches_any_language_of_country() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("ca1", "CA One", vec![], vec![], "CA"),
                channel_with("us1", "US One", vec![], vec![], "US"),
            ])
            .with_countries(vec![
                country_with("CA", "Canada", &["en".into(), "fr".into()]),
                country_with("US", "United States", &["en".into()]),
            ])
            .with_streams_for_channels(&["ca1", "us1"]),
        );

        svc.refresh();

        let result = svc.filter_by_language("fr");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "ca1");
    }
    #[test]
    fn channels_with_feeds_but_no_streams_are_excluded_from_filtering() {
        // Channel with feeds but NO streams should be excluded from filters
        // that previously checked `has_feeds` but now check `has_streams`.
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("bbc1", "BBC One", vec![], vec!["news"], "GB"),
                channel_with("cnn", "CNN", vec![], vec![], "US"),
            ])
            // Only cnn has streams; bbc1 has no streams.
            .with_feedless_streams_for_channel("cnn"),
        );

        // `filter_by_category_id` — bbc1 has category "news" but no streams, so excluded.
        let by_cat = svc.filter_by_category_id("news");
        assert!(
            by_cat.is_empty(),
            "bbc1 has feeds but no streams → excluded"
        );

        // `filter_by_country` — bbc1 has country GB but no streams, so excluded.
        let by_country = svc.filter_by_country("GB");
        assert!(
            by_country.is_empty(),
            "bbc1 has country GB but no streams → excluded"
        );

        // `get_active_channels` — only channels with streams are active.
        let active = svc.get_active_channels();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "cnn");
    }

    #[test]
    fn get_logos_returns_all_logos() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("ch1", "Ch1", vec![], vec![], "US"),
                channel_with("ch2", "Ch2", vec![], vec![], "GB"),
            ])
            .with_streams_for_channels(&["ch1", "ch2"])
            .with_countries(vec![
                country_with("US", "United States", &["en".into()]),
                country_with("GB", "United Kingdom", &["en".into()]),
            ])
            // Add logos to the mock repository
            .with_logos(vec![
                logo_with("ch1", "https://ch1-logo.png"),
                logo_with("ch2", "https://ch2-logo.jpg"),
            ]),
        );

        let result = svc.get_logos();

        assert_eq!(result.len(), 2);

        assert_eq!(result[0].channel, "ch1");

        assert_eq!(result[0].url, "https://ch1-logo.png");

        assert_eq!(result[1].channel, "ch2");

        assert_eq!(result[1].url, "https://ch2-logo.jpg");
    }

    #[test]
    fn query_with_no_filters_returns_active_channels() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("bbc1", "bbc one", vec![], vec![], "GB"),
                channel_with("cnn", "cnn", vec![], vec![], "US"),
                channel_with("inactive", "inactive", vec![], vec![], "US"),
            ])
            .with_streams_for_channels(&["bbc1", "cnn"]),
        );

        let result = svc.query(&CatalogQuery::new());

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "bbc1");
        assert_eq!(result[1].id, "cnn");
    }

    #[test]
    fn query_can_include_inactive_channels() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("bbc1", "bbc one", vec![], vec![], "GB"),
                channel_with("inactive", "inactive", vec![], vec![], "US"),
            ])
            .with_feedless_streams_for_channel("bbc1"),
        );

        let result = svc.query(&CatalogQuery::new().active_only(false));

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn query_search_matches_name_and_alt_names_case_insensitively() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("bbc1", "BBC One", vec![], vec![], "GB"),
                channel_with("cnn", "cnn", vec!["Cable News Network"], vec![], "US"),
                channel_with("other", "other", vec![], vec![], "US"),
            ])
            .with_streams_for_channels(&["bbc1", "cnn", "other"]),
        );

        let by_name = svc.query(&CatalogQuery::new().search("bbc"));
        assert_eq!(by_name.len(), 1);
        assert_eq!(by_name[0].id, "bbc1");

        let by_alt_name = svc.query(&CatalogQuery::new().search("cable news"));
        assert_eq!(by_alt_name.len(), 1);
        assert_eq!(by_alt_name[0].id, "cnn");
    }

    #[test]
    fn query_search_ignores_empty_or_whitespace() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("bbc1", "bbc one", vec![], vec![], "GB"),
                channel_with("cnn", "cnn", vec![], vec![], "US"),
            ])
            .with_streams_for_channels(&["bbc1", "cnn"]),
        );

        assert_eq!(svc.query(&CatalogQuery::new().search("")).len(), 2);
        assert_eq!(svc.query(&CatalogQuery::new().search("   ")).len(), 2);
    }

    #[test]
    fn query_country_matches_exactly_case_insensitively() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("us1", "US One", vec![], vec![], "US"),
                channel_with("gb1", "GB One", vec![], vec![], "GB"),
                channel_with("aus1", "AUS One", vec![], vec![], "AUS"),
            ])
            .with_streams_for_channels(&["us1", "gb1", "aus1"]),
        );

        let result = svc.query(&CatalogQuery::new().country("us"));

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "us1");
    }

    #[test]
    fn query_country_ignores_empty() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("us1", "US One", vec![], vec![], "US"),
                channel_with("gb1", "GB One", vec![], vec![], "GB"),
            ])
            .with_streams_for_channels(&["us1", "gb1"]),
        );

        assert_eq!(svc.query(&CatalogQuery::new().country("")).len(), 2);
    }

    #[test]
    fn query_language_matches_all_countries_speaking_it() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("us1", "US One", vec![], vec![], "US"),
                channel_with("gb1", "GB One", vec![], vec![], "GB"),
                channel_with("de1", "DE One", vec![], vec![], "DE"),
            ])
            .with_streams_for_channels(&["us1", "gb1", "de1"])
            .with_countries(vec![
                country_with("US", "United States", &["en".into()]),
                country_with("GB", "United Kingdom", &["en".into()]),
                country_with("DE", "Germany", &["de".into()]),
            ]),
        );

        let result = svc.query(&CatalogQuery::new().language("en"));

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "us1");
        assert_eq!(result[1].id, "gb1");
    }

    #[test]
    fn query_language_is_case_insensitive() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("us1", "US One", vec![], vec![], "US"),
                channel_with("gb1", "GB One", vec![], vec![], "GB"),
            ])
            .with_streams_for_channels(&["us1", "gb1"])
            .with_countries(vec![
                country_with("US", "United States", &["en".into()]),
                country_with("GB", "United Kingdom", &["en".into()]),
            ]),
        );

        let result = svc.query(&CatalogQuery::new().language("EN"));

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn query_language_ignores_empty() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("us1", "US One", vec![], vec![], "US"),
                channel_with("gb1", "GB One", vec![], vec![], "GB"),
            ])
            .with_streams_for_channels(&["us1", "gb1"]),
        );

        assert_eq!(svc.query(&CatalogQuery::new().language("")).len(), 2);
    }

    #[test]
    fn query_language_returns_empty_for_unknown_code() {
        let svc = service(
            MockCatalogRepository::new(vec![channel_with("us1", "US One", vec![], vec![], "US")])
                .with_feedless_streams_for_channel("us1")
                .with_countries(vec![country_with("US", "United States", &["en".into()])]),
        );

        assert!(svc.query(&CatalogQuery::new().language("xx")).is_empty());
    }

    fn category_service() -> IptvCatalogService {
        service(
            MockCatalogRepository::new(vec![
                channel_with("both", "both", vec![], vec!["news", "sports"], "US"),
                channel_with("news", "news", vec![], vec!["news"], "US"),
                channel_with("sports", "sports", vec![], vec!["sports"], "US"),
            ])
            .with_streams_for_channels(&["both", "news", "sports"]),
        )
    }

    #[test]
    fn query_category_all_requires_every_selected_category() {
        let result = category_service().query(
            &CatalogQuery::new()
                .categories(["news", "sports"])
                .category_match(CategoryMatch::All),
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "both");
    }

    #[test]
    fn query_category_any_matches_at_least_one() {
        let result = category_service().query(
            &CatalogQuery::new()
                .categories(["news", "sports"])
                .category_match(CategoryMatch::Any),
        );

        assert_eq!(result.len(), 3);
    }

    #[test]
    fn query_category_none_excludes_selected() {
        let result = category_service().query(
            &CatalogQuery::new()
                .categories(["news"])
                .category_match(CategoryMatch::None),
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "sports");
    }

    #[test]
    fn query_category_empty_selection_is_no_filter_for_all_modes() {
        let svc = category_service();

        for mode in [CategoryMatch::All, CategoryMatch::Any, CategoryMatch::None] {
            let result = svc.query(&CatalogQuery::new().category_match(mode));
            assert_eq!(result.len(), 3, "mode {mode:?} should not filter");
        }
    }

    #[test]
    fn query_category_ignores_empty_ids() {
        let result = category_service().query(
            &CatalogQuery::new()
                .categories(["", "   "])
                .category_match(CategoryMatch::All),
        );

        assert_eq!(result.len(), 3);
    }

    #[test]
    fn query_combines_search_country_language_and_categories() {
        let svc = service(
            MockCatalogRepository::new(vec![
                channel_with("us-news", "US News", vec![], vec!["news"], "US"),
                channel_with("us-sports", "US Sports", vec![], vec!["sports"], "US"),
                channel_with("gb-news", "GB News", vec![], vec!["news"], "GB"),
                channel_with("de-news", "DE News", vec![], vec!["news"], "DE"),
            ])
            .with_streams_for_channels(&["us-news", "us-sports", "gb-news", "de-news"])
            .with_countries(vec![
                country_with("US", "United States", &["en".into()]),
                country_with("GB", "United Kingdom", &["en".into()]),
                country_with("DE", "Germany", &["de".into()]),
            ]),
        );

        let result = svc.query(
            &CatalogQuery::new()
                .search("news")
                .country("US")
                .language("en")
                .categories(["news"])
                .category_match(CategoryMatch::Any),
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "us-news");
    }

    #[test]
    fn query_returns_empty_when_no_channel_matches() {
        let svc = service(
            MockCatalogRepository::new(vec![channel_with("bbc1", "bbc one", vec![], vec![], "GB")])
                .with_feedless_streams_for_channel("bbc1"),
        );

        assert!(svc.query(&CatalogQuery::new().search("xyzzy")).is_empty());
    }

    #[test]
    fn query_unknown_country_returns_empty() {
        let svc = service(
            MockCatalogRepository::new(vec![channel_with("us1", "US One", vec![], vec![], "US")])
                .with_feedless_streams_for_channel("us1"),
        );

        assert!(svc.query(&CatalogQuery::new().country("FR")).is_empty());
    }

    #[test]
    fn query_unknown_category_returns_empty_for_all_and_any() {
        let svc = category_service();

        assert!(
            svc.query(
                &CatalogQuery::new()
                    .categories(["movies"])
                    .category_match(CategoryMatch::All)
            )
            .is_empty()
        );
        assert!(
            svc.query(
                &CatalogQuery::new()
                    .categories(["movies"])
                    .category_match(CategoryMatch::Any)
            )
            .is_empty()
        );
    }

    #[test]
    fn query_unknown_category_returns_all_for_none() {
        let result = category_service().query(
            &CatalogQuery::new()
                .categories(["movies"])
                .category_match(CategoryMatch::None),
        );

        assert_eq!(result.len(), 3);
    }
}
