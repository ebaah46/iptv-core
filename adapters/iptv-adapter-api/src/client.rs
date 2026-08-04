use anyhow::{bail, Context, Result as Res};
use log::info;
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::Method;
use serde::de::{DeserializeOwned, Deserializer, SeqAccess, Visitor};
use serde::Serialize;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::io::BufReader;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

/**
* This client is responsible for providing the actually making the API request
* to the IPTV provider for the catalog data.
*/
#[derive(Debug, Clone)]
pub struct HttpClient {
    base_url: String,
    client: Arc<Client>,
}

impl HttpClient {
    /// Predefined timeout
    const TIMEOUT_SECS: u8 = 20;
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(Self::TIMEOUT_SECS as u64))
            .build()
            .unwrap_or_else(|_| Client::new());
        let mut base_url = base_url.into();
        if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
            base_url = format!("https://{}", base_url);
        }
        Self {
            base_url,
            client: Arc::new(client),
        }
    }

    /// Internal helper to build endpoints using the given base_url
    fn build_request(&self, method: &str, path: &str) -> RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let method = match method {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "PATCH" => Method::PATCH,
            "PUT" => Method::PUT,
            _ => Method::GET,
        };
        self.client
            .request(method, url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
    }

    /// Internal helper to handle general responses with small paylod
    fn handle_response<T: DeserializeOwned>(&self, response: Response) -> Res<T> {
        let status = response.status();
        if status.is_success() {
            response
                .json::<T>()
                .with_context(|| "Failed to deserialize JSON response payload")
        } else {
            let error_body = response.text().unwrap_or_default();
            bail!("API returned error status {}: {}", status, error_body)
        }
    }

    /// Perform an Http::Get request
    pub fn get<T, Q, P>(&self, path: P, query: Option<&Q>) -> Res<T>
    where
        T: DeserializeOwned,
        Q: Serialize + Sized,
        P: Into<String>,
    {
        let mut req = self.build_request("GET", &path.into());
        if let Some(q) = query {
            req = req.query(q);
        }
        let response = req.send().context("Failed to send GET request")?;
        self.handle_response(response)
    }

    pub fn get_stream<T, Target, Q, P, F>(
        &self,
        path: P,
        query: Option<&Q>,
        mut callback: F,
    ) -> Res<Vec<Target>>
    where
        T: DeserializeOwned + Debug,
        Q: Serialize + Sized,
        P: Into<String>,
        F: FnMut(T) -> Option<Target>,
    {
        let mut req = self.build_request("GET", &path.into());
        if let Some(q) = query {
            req = req.query(q);
        }

        let response = req.send().context("Failed to send streaming GET request")?;
        info!("url:{}", response.url());
        let status = response.status();
        if !status.is_success() {
            let error = response.text().unwrap_or_default();
            bail!("Stream API error {}: {}", status, error);
        }
        let buffer_reader = BufReader::new(response);
        let mut deserializer = serde_json::Deserializer::from_reader(buffer_reader);

        let mut transformed_collection = vec![];

        let visitor: StreamArrayVisitor<T, _> = StreamArrayVisitor::new(|data_dto: T| {
            println!("Starting mapper with callback dto:{:?}", data_dto);
            if let Some(mapped) = callback(data_dto) {
                transformed_collection.push(mapped);
            }
        });

        if let Err(e) = deserializer.deserialize_seq(visitor) {
            println!(
                "HttpClient - get_stream - failed to deserialize sequence: {}",
                e
            );
        }
        Ok(transformed_collection)
    }
}

/**
* This is an internal component to deal with the streaming of a response.
* It handles the deserialization of a json sequence using the streaming visitor
* provided by the serde crate.
*/
struct StreamArrayVisitor<T, Callback> {
    callback: Callback,
    _marker: PhantomData<T>,
}

impl<T, Callback> StreamArrayVisitor<T, Callback> {
    fn new(callback: Callback) -> Self {
        Self {
            callback,
            _marker: PhantomData,
        }
    }
}

impl<'de, T, Callback> Visitor<'de> for StreamArrayVisitor<T, Callback>
where
    T: DeserializeOwned,
    Callback: FnMut(T),
{
    type Value = ();

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("a Top-level JSON Array")
    }

    fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while let Some(element) = seq.next_element::<T>()? {
            (self.callback)(element)
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::client::HttpClient;
    use httpmock::prelude::{Method, GET};
    use httpmock::MockServer;
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value};

    struct TestServer {
        pub server: MockServer,
    }
    impl TestServer {
        fn start() -> Self {
            let server = MockServer::start();
            Self { server }
        }

        fn register_resource<T: Into<Value>>(
            &self,
            url: impl Into<String>,
            method: Method,
            response: T,
        ) {
            self.server.mock(|when, then| {
                when.method(method).path(url);
                then.status(200).json_body(response);
            });
        }
    }

    #[derive(Debug, Deserialize, Serialize, Clone, Default, PartialEq)]
    struct TestLogo {
        pub channel: String,
        pub feed: String,
        pub in_use: bool,
        pub tags: Vec<String>,
        pub width: u32,
        pub height: u32,
        pub format: String,
        pub url: String,
    }

    #[derive(Debug, Deserialize, Default, PartialEq)]
    pub struct TestLogoDTO {
        pub channel: String,
        pub feed: String,
        pub in_use: bool,
        pub tags: Vec<String>,
        pub width: u32,
        pub height: u32,
        pub format: String,
        pub url: String,
        pub is_main: bool,
    }

    fn map_logos_dto(dto: TestLogoDTO) -> Option<TestLogo> {
        Some(TestLogo {
            channel: dto.channel,
            feed: dto.feed,
            in_use: dto.in_use,
            tags: dto.tags,
            width: dto.width,
            height: dto.height,
            format: dto.format,
            url: dto.url,
        })
    }
    #[test]
    fn test_get_resource_success() {
        let server = TestServer::start();
        let expected_json = json!(  {
          "channel": "France3.fr",
          "feed": "ParisIledeFrance",
          "in_use": true,
          "tags": ["horizontal", "white"],
          "width": 1000,
          "height": 468,
          "format": "SVG",
          "url": "https://example.com/logo.svg",
          "is_main": false
        });
        server.register_resource("/logos.json", GET, expected_json);
        let base_url = server.server.address().to_string();
        let base_url = format!("http://{}", base_url);
        let client = HttpClient::new(base_url);
        let logo: TestLogoDTO = client.get("/logos.json", None::<&()>).unwrap_or_else(|e| {
            println!("Err:{}", e.to_string());
            TestLogoDTO::default()
        });
        assert_eq!(
            logo,
            TestLogoDTO {
                channel: "France3.fr".to_string(),
                feed: "ParisIledeFrance".to_string(),
                in_use: true,
                tags: vec!["horizontal".into(), "white".into()],
                width: 1000,
                height: 468,
                format: "SVG".to_string(),
                url: "https://example.com/logo.svg".to_string(),
                is_main: false
            }
        );
    }

    #[test]
    fn test_get_stream_resource_success() {
        let server = TestServer::start();
        let expected_json = json!( [ {
          "channel": "France3.fr",
          "feed": "ParisIledeFrance",
          "in_use": true,
          "tags": ["horizontal", "white"],
          "width": 1000,
          "height": 468,
          "format": "SVG",
          "url": "https://example.com/logo.svg",
          "is_main": false
        },{
          "channel": "BBC Africa",
          "feed": "LagosBBCOne",
          "in_use": true,
          "tags": ["horizontal", "red"],
          "width": 1000,
          "height": 468,
          "format": "SVG",
          "url": "https://example.com/logo2.svg",
          "is_main": false
        }]);
        server.register_resource("/logos.json", GET, expected_json);
        let base_url = server.server.address().to_string();
        let base_url = format!("http://{}", base_url);
        let client = HttpClient::new(base_url);
        let logos: Vec<TestLogo> = client
            .get_stream("/logos.json", None::<&()>, |logo: TestLogoDTO| {
                map_logos_dto(logo)
            })
            .unwrap_or_else(|e| vec![]);
        assert!(!logos.is_empty());

        assert_eq!(
            logos,
            [
                TestLogo {
                    channel: "France3.fr".to_string(),
                    feed: "ParisIledeFrance".to_string(),
                    in_use: true,
                    tags: vec!["horizontal".into(), "white".into()],
                    width: 1000,
                    height: 468,
                    format: "SVG".to_string(),
                    url: "https://example.com/logo.svg".to_string(),
                },
                TestLogo {
                    channel: "BBC Africa".to_string(),
                    feed: "LagosBBCOne".to_string(),
                    in_use: true,
                    tags: vec!["horizontal".into(), "red".into()],
                    width: 1000,
                    height: 468,
                    format: "SVG".to_string(),
                    url: "https://example.com/logo2.svg".to_string(),
                },
            ]
        );
    }
}
