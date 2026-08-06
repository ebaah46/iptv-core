use crate::domain::Channels;
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

    // Other filtering APIs will be provided as we go. But for not, this
    // is what is available
}

#[derive(Debug)]
pub struct IptvCatalogService {
    inner: Arc<dyn CatalogRepository>,
}

impl IptvCatalogService {
    fn new(registry: Arc<dyn CatalogRepository>) -> Self {
        Self { inner: registry }
    }
}

impl CatalogService for IptvCatalogService {
    fn search(&self, query: &str) -> Channels {
        let channels = self.inner.get_channels();
        if query.is_empty() {
            return vec![];
        }
        let query = query.to_ascii_lowercase();
        channels
            .iter()
            .filter(|channel| {
                channel.name.to_ascii_lowercase().contains(&query)
                    || !channel
                        .alt_names
                        .iter()
                        .filter(|c| c.to_ascii_lowercase().contains(&query))
                        .collect::<Vec<&String>>()
                        .is_empty()
            })
            .cloned()
            .collect()
    }

    fn filter_by_category_id(&self, category_id: &str) -> Channels {
        if category_id.is_empty() {
            return vec![];
        }
        let category_id = category_id.to_ascii_lowercase();
        self.inner
            .get_channels()
            .iter()
            .filter(|channel| {
                !channel
                    .category_ids
                    .iter()
                    .filter(|category| category.contains(&category_id))
                    .collect::<Vec<&String>>()
                    .is_empty()
            })
            .cloned()
            .collect()
    }

    fn filter_by_country(&self, country_code: &str) -> Channels {
        if country_code.is_empty() {
            return vec![];
        }
        let country_code = country_code.to_ascii_lowercase();
        self.inner
            .get_channels()
            .iter()
            .filter(|channel| {
                channel
                    .country_code
                    .to_ascii_lowercase()
                    .contains(&country_code)
            })
            .cloned()
            .collect()
    }

    fn filter_by_language(&self, language_code: &str) -> Channels {
        // Requires access to countries list
        // if language_code.is_empty() {
        //     return vec![];
        // }
        // self.inner.get_channels().iter().filter(|channel| channel.)

        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Channel, Feed, Feeds, Streams};
    use std::collections::HashMap;

    /// Test double for `CatalogRepository` that only supplies the data the
    /// service reads (channels and feeds). The remaining repository methods
    /// are stubbed out because the service under test never calls them.
    #[derive(Debug, Default)]
    struct MockCatalogRepository {
        channels: Channels,
        feeds_by_channel: HashMap<String, Feeds>,
    }

    impl MockCatalogRepository {
        fn new(channels: Channels) -> Self {
            Self {
                channels,
                feeds_by_channel: HashMap::new(),
            }
        }

        fn with_feeds(mut self, feeds: Feeds) -> Self {
            for feed in feeds {
                self.feeds_by_channel
                    .entry(feed.channel_id.clone())
                    .or_default()
                    .push(feed);
            }
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

        fn refresh(&self) {}
    }

    fn channel(id: &str) -> Channel {
        Channel {
            id: id.to_string(),
            name: format!("Channel {}", id),
            alt_names: vec![],
            category_ids: vec![],
            country_code: "US".to_string(),
            is_nsfw: false,
            launched: None,
            closed: None,
            website: String::new(),
            network: String::new(),
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

    fn feed(id: &str, channel_id: &str, language_codes: Vec<&str>) -> Feed {
        Feed {
            id: id.to_string(),
            channel_id: channel_id.to_string(),
            name: format!("Feed {}", id),
            broadcast_codes: vec![],
            language_codes: language_codes.into_iter().map(|s| s.to_string()).collect(),
            is_main: false,
        }
    }

    fn service(repo: MockCatalogRepository) -> IptvCatalogService {
        IptvCatalogService::new(Arc::new(repo))
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
    fn filter_by_language_matches_channel_whose_feed_has_code() {
        let svc = service(
            MockCatalogRepository::new(vec![channel("ch1"), channel("ch2")])
                .with_feeds(vec![feed("feed1", "ch1", vec!["en"])]),
        );

        let result = svc.filter_by_language("en");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "ch1");
    }

    #[test]
    fn filter_by_language_excludes_channel_without_feeds() {
        let svc = service(
            MockCatalogRepository::new(vec![channel("ch1"), channel("ch2")])
                .with_feeds(vec![feed("feed1", "ch1", vec!["en"])]),
        );

        let result = svc.filter_by_language("en");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "ch1");
    }

    #[test]
    fn filter_by_language_matches_multi_language_feed() {
        let svc = service(
            MockCatalogRepository::new(vec![channel("ch1")]).with_feeds(vec![feed(
                "feed1",
                "ch1",
                vec!["en", "fr"],
            )]),
        );

        let result = svc.filter_by_language("fr");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "ch1");
    }

    #[test]
    fn filter_by_language_respects_feed_language() {
        let svc = service(
            MockCatalogRepository::new(vec![channel("ch1"), channel("ch2")]).with_feeds(vec![
                feed("feed1", "ch1", vec!["en"]),
                feed("feed2", "ch2", vec!["de"]),
            ]),
        );

        let result = svc.filter_by_language("de");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "ch2");
    }

    #[test]
    fn filter_by_language_returns_empty_for_unknown_code() {
        let svc = service(
            MockCatalogRepository::new(vec![channel("ch1")]).with_feeds(vec![feed(
                "feed1",
                "ch1",
                vec!["en"],
            )]),
        );

        assert!(svc.filter_by_language("xx").is_empty());
    }

    #[test]
    fn filter_by_language_with_empty_string_returns_empty() {
        let svc = service(
            MockCatalogRepository::new(vec![channel("ch1")]).with_feeds(vec![feed(
                "feed1",
                "ch1",
                vec!["en"],
            )]),
        );

        assert!(svc.filter_by_language("").is_empty());
    }

    #[test]
    fn filter_by_language_deduplicates_channels() {
        let svc = service(
            MockCatalogRepository::new(vec![channel("ch1")]).with_feeds(vec![
                feed("feed1", "ch1", vec!["en"]),
                feed("feed2", "ch1", vec!["en"]),
            ]),
        );

        let result = svc.filter_by_language("en");

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn filter_by_language_matches_any_feed_of_channel() {
        let svc = service(
            MockCatalogRepository::new(vec![channel("ch1")]).with_feeds(vec![
                feed("feed1", "ch1", vec!["en"]),
                feed("feed2", "ch1", vec!["es"]),
            ]),
        );

        let result = svc.filter_by_language("es");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "ch1");
    }
}
