use crate::domain::{Categories, Channel, Channels, Countries, Languages};
use crate::services::CatalogRepository;
use std::sync::Arc;

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

    fn get_all(&self) -> Channels;

    fn get_categories(&self) -> Categories;

    fn get_countries(&self) -> Countries;

    fn get_languages(&self) -> Languages;

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
}

impl CatalogService for IptvCatalogService {
    fn search(&self, query: &str) -> Channels {
        if query.is_empty() {
            return vec![];
        }
        let query = query.to_ascii_lowercase();
        self.inner
            .get_channels()
            .into_iter()
            .filter(|channel| {
                channel.name.to_ascii_lowercase().contains(&query)
                    || !channel
                        .alt_names
                        .iter()
                        .filter(|c| c.to_ascii_lowercase().contains(&query))
                        .collect::<Vec<&String>>()
                        .is_empty()
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
                !channel
                    .category_ids
                    .iter()
                    .filter(|category| category.eq_ignore_ascii_case(category_id))
                    .collect::<Vec<&String>>()
                    .is_empty()
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
            .filter(|channel| channel.country_code.eq_ignore_ascii_case(&country_code))
            .collect()
    }

    fn filter_by_language(&self, language_code: &str) -> Channels {
        if language_code.is_empty() {
            return vec![];
        }
        let country = self.inner.get_countries().into_iter().find(|country| {
            country
                .languages
                .iter()
                .any(|lang| lang.to_ascii_lowercase() == language_code.to_ascii_lowercase())
        });
        match country {
            None => vec![],
            Some(country) => self.filter_by_country(&country.code),
        }
    }

    fn get_all(&self) -> Channels {
        self.inner.get_channels()
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

    fn refresh(&self) {
        self.inner.refresh();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Categories, Channel, Countries, Country, Feeds, Languages, Streams};
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Test double for `CatalogRepository` that only supplies the data the
    /// service reads (channels and feeds). The remaining repository methods
    /// are stubbed out because the service under test never calls them.
    #[derive(Debug)]
    struct MockCatalogRepository {
        channels: Channels,
        feeds_by_channel: HashMap<String, Feeds>,
        languages: Languages,
        categories: Categories,
        countries: Countries,
        refresh_count: AtomicUsize,
    }

    impl Default for MockCatalogRepository {
        fn default() -> Self {
            Self {
                channels: vec![],
                feeds_by_channel: HashMap::new(),
                languages: vec![],
                categories: vec![],
                countries: vec![],
                refresh_count: AtomicUsize::new(0),
            }
        }
    }

    impl MockCatalogRepository {
        fn new(channels: Channels) -> Self {
            Self {
                channels,
                feeds_by_channel: HashMap::new(),
                languages: vec![],
                categories: vec![],
                countries: vec![],
                refresh_count: AtomicUsize::new(0),
            }
        }

        fn with_countries(mut self, countries: Countries) -> Self {
            self.countries = countries;
            self
        }
    }

    impl CatalogRepository for MockCatalogRepository {
        fn get_channels(&self) -> Channels {
            self.channels.clone()
        }

        fn get_channel_by_id(&self, _channel_id: &str) -> Option<Channel> {
            None
        }

        fn get_feeds_by_channel(&self, channel_id: &str) -> Option<Feeds> {
            self.feeds_by_channel.get(channel_id).cloned()
        }

        fn get_candidate_streams(
            &self,
            _channel_id: &str,
            _feed_id: Option<&str>,
        ) -> Option<Streams> {
            None
        }

        fn get_categories(&self) -> Categories {
            self.categories.clone()
        }

        fn get_countries(&self) -> Countries {
            self.countries.clone()
        }

        fn get_languages(&self) -> Languages {
            self.languages.clone()
        }

        fn refresh(&self) {
            self.refresh_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn channel_with(
        id: &str,
        name: &str,
        alt_names: Vec<&str>,
        category_ids: Vec<&str>,
        country_code: &str,
    ) -> Channel {
        Channel {
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
        }
    }

    fn service(repo: MockCatalogRepository) -> IptvCatalogService {
        IptvCatalogService::new(Arc::new(repo))
    }

    fn country_with(code: &str, name: &str, languages: &[String]) -> Country {
        Country {
            code: code.into(),
            name: name.into(),
            languages: languages.into(),
            flag_url: "".to_string(),
        }
    }

    #[test]
    fn refresh_delegates_to_repository() {
        let repo = Arc::new(MockCatalogRepository::new(vec![]));
        let svc = IptvCatalogService::new(repo.clone());
        assert_eq!(repo.refresh_count.load(Ordering::SeqCst), 0);
        svc.refresh();
        assert_eq!(repo.refresh_count.load(Ordering::SeqCst), 1);
        svc.refresh();
        assert_eq!(repo.refresh_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn get_all_channels() {
        let svc = service(MockCatalogRepository::new(vec![
            channel_with("bbc1", "BBC One", vec![], vec![], "GB"),
            channel_with("cnn", "CNN", vec![], vec![], "US"),
        ]));

        svc.refresh();

        let result = svc.get_all();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "bbc1");
        assert_eq!(result[1].id, "cnn");
    }

    #[test]
    fn search_matches_channel_by_name() {
        let svc = service(MockCatalogRepository::new(vec![
            channel_with("bbc1", "BBC One", vec![], vec![], "GB"),
            channel_with("cnn", "CNN", vec![], vec![], "US"),
        ]));

        let result = svc.search("BBC");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "bbc1");
    }

    #[test]
    fn search_matches_channel_by_alt_name() {
        let svc = service(MockCatalogRepository::new(vec![channel_with(
            "cnn",
            "CNN",
            vec!["Cable News Network"],
            vec![],
            "US",
        )]));

        let result = svc.search("Cable News");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "cnn");
    }

    #[test]
    fn search_matches_name_and_alt_names() {
        let svc = service(MockCatalogRepository::new(vec![
            channel_with("bbc1", "BBC One", vec!["BBC1"], vec![], "GB"),
            channel_with("bbcworld", "BBC World", vec![], vec![], "GB"),
        ]));

        let result = svc.search("bbc");

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn search_is_case_insensitive() {
        let svc = service(MockCatalogRepository::new(vec![channel_with(
            "disc",
            "Discovery",
            vec![],
            vec![],
            "US",
        )]));

        let result = svc.search("discovery");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "disc");
    }

    #[test]
    fn search_returns_empty_when_no_match() {
        let svc = service(MockCatalogRepository::new(vec![channel_with(
            "bbc1",
            "BBC One",
            vec![],
            vec![],
            "GB",
        )]));

        assert!(svc.search("xyzzy").is_empty());
    }

    #[test]
    fn search_matches_only_name_or_alt_names() {
        let svc = service(MockCatalogRepository::new(vec![
            channel_with("news24", "News24", vec![], vec!["sports"], "US"),
            channel_with("lateshow", "Late Show", vec![], vec!["sports"], "US"),
        ]));

        // Category ids and country codes must not leak into search results.
        assert!(svc.search("sports").is_empty());
        assert!(svc.search("US").is_empty());
        // A name match still works.
        assert_eq!(svc.search("news").len(), 1);
    }

    #[test]
    fn search_with_empty_or_whitespace_query_returns_empty() {
        let svc = service(MockCatalogRepository::new(vec![channel_with(
            "bbc1",
            "BBC One",
            vec![],
            vec![],
            "GB",
        )]));
        assert!(svc.search("").is_empty());
        assert!(svc.search("   ").is_empty());
    }

    #[test]
    fn search_returns_all_partial_matches() {
        let svc = service(MockCatalogRepository::new(vec![
            channel_with("news", "News", vec![], vec![], "GB"),
            channel_with("news24", "News24", vec![], vec![], "GB"),
            channel_with("sportsnews", "Sports News", vec![], vec![], "GB"),
        ]));

        let result = svc.search("news");

        assert_eq!(result.len(), 3);
    }

    #[test]
    fn filter_by_category_id_matches_channels_with_id() {
        let svc = service(MockCatalogRepository::new(vec![
            channel_with("bbc1", "BBC One", vec![], vec!["news"], "GB"),
            channel_with("mov1", "Movie One", vec![], vec!["movies"], "US"),
        ]));

        let result = svc.filter_by_category_id("news");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "bbc1");
    }

    #[test]
    fn filter_by_category_id_matches_any_entry() {
        let svc = service(MockCatalogRepository::new(vec![channel_with(
            "bbc1",
            "BBC One",
            vec![],
            vec!["news", "sports"],
            "GB",
        )]));

        let result = svc.filter_by_category_id("sports");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "bbc1");
    }

    #[test]
    fn filter_by_category_id_returns_empty_for_unknown() {
        let svc = service(MockCatalogRepository::new(vec![channel_with(
            "bbc1",
            "BBC One",
            vec![],
            vec!["news"],
            "GB",
        )]));

        assert!(svc.filter_by_category_id("movies").is_empty());
    }

    #[test]
    fn filter_by_category_id_with_empty_string_returns_empty() {
        let svc = service(MockCatalogRepository::new(vec![channel_with(
            "bbc1",
            "BBC One",
            vec![],
            vec!["news"],
            "GB",
        )]));

        assert!(svc.filter_by_category_id("").is_empty());
    }

    #[test]
    fn filter_by_category_id_returns_all_matching_channels() {
        let svc = service(MockCatalogRepository::new(vec![
            channel_with("bbc1", "BBC One", vec![], vec!["news"], "GB"),
            channel_with("sky1", "Sky News", vec![], vec!["news"], "GB"),
            channel_with("mov1", "Movie One", vec![], vec!["movies"], "US"),
        ]));

        let result = svc.filter_by_category_id("news");

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_by_country_matches_exact_code() {
        let svc = service(MockCatalogRepository::new(vec![
            channel_with("us1", "US One", vec![], vec![], "US"),
            channel_with("gb1", "GB One", vec![], vec![], "GB"),
            channel_with("de1", "DE One", vec![], vec![], "DE"),
        ]));

        let result = svc.filter_by_country("US");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "us1");
    }

    #[test]
    fn filter_by_country_returns_empty_for_unknown() {
        let svc = service(MockCatalogRepository::new(vec![
            channel_with("us1", "US One", vec![], vec![], "US"),
            channel_with("gb1", "GB One", vec![], vec![], "GB"),
        ]));

        assert!(svc.filter_by_country("FR").is_empty());
    }

    #[test]
    fn filter_by_country_with_empty_string_returns_empty() {
        let svc = service(MockCatalogRepository::new(vec![channel_with(
            "us1",
            "US One",
            vec![],
            vec![],
            "US",
        )]));

        assert!(svc.filter_by_country("").is_empty());
    }

    #[test]
    fn filter_by_country_returns_all_matching_channels() {
        let svc = service(MockCatalogRepository::new(vec![
            channel_with("us1", "US One", vec![], vec![], "US"),
            channel_with("sky1", "Sky News", vec![], vec![], "GB"),
            channel_with("bbc1", "BBC One", vec![], vec![], "GB"),
        ]));

        let result = svc.filter_by_country("GB");

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_by_country_matches_all_with_that_code_in_order() {
        let svc = service(MockCatalogRepository::new(vec![
            channel_with("a", "A", vec![], vec![], "US"),
            channel_with("b", "B", vec![], vec![], "DE"),
            channel_with("c", "C", vec![], vec![], "US"),
            channel_with("d", "D", vec![], vec![], "FR"),
            channel_with("e", "E", vec![], vec![], "US"),
        ]));

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
            ]),
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
            ]),
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
            ]),
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
            ]),
        );

        svc.refresh();

        let result = svc.filter_by_language("de");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "de1");
    }

    #[test]
    fn filter_by_language_is_case_insensitive() {
        let svc = service(
            MockCatalogRepository::new(vec![channel_with("us1", "US One", vec![], vec![], "US")])
                .with_countries(vec![country_with("US", "United States", &["EN".into()])]),
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
                .with_countries(vec![country_with("US", "United States", &["en".into()])]),
        );

        svc.refresh();

        assert!(svc.filter_by_language("xx").is_empty());
    }

    #[test]
    fn filter_by_language_with_empty_string_returns_empty() {
        let svc = service(
            MockCatalogRepository::new(vec![channel_with("us1", "US One", vec![], vec![], "US")])
                .with_countries(vec![country_with("US", "United States", &["en".into()])]),
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
            .with_countries(vec![country_with("US", "United States", &["en".into()])]),
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
            ]),
        );

        svc.refresh();

        let result = svc.filter_by_language("fr");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "ca1");
    }
}
