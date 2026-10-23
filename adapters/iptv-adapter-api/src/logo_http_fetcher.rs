use crate::HttpClient;
use anyhow::Result as Res;
use std::sync::Arc;

/**
* This object implements the basic functionality for fetching a logo
* from the given url.
*/
pub struct LogoHttpFetcher {
    client: Arc<HttpClient>,
}

impl LogoHttpFetcher {
    pub fn new() -> Self {
        let client = Arc::new(HttpClient::default());
        Self { client }
    }

    pub fn fetch(&self, url: &str) -> Res<Vec<u8>> {
        self.client.get_raw(url)
    }
}
