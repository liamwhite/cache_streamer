use std::sync::Arc;
use std::time::Duration;

use cache_streamer_lib::types::*;
use reqwest::{Client, Url};

use crate::http_requester::HTTPRequester;
use crate::http_response::HTTPResponse;

pub struct HTTPRequestBackend {
    client: Arc<Client>,
    base_url: Url,
    cache_limit: usize,
}

impl HTTPRequestBackend {
    pub fn new(base_url: Url, cache_limit: usize) -> Self {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::limited(1))
            .connect_timeout(Duration::from_secs(10))
            .http2_adaptive_window(true)
            .build()
            .expect("reqwest HTTP client");

        Self {
            client: Arc::new(client),
            base_url,
            cache_limit,
        }
    }
}

impl RequestBackend<String, HTTPResponse> for HTTPRequestBackend {
    fn create_for_key(&self, key: String) -> Arc<dyn Requester<HTTPResponse>> {
        let cache_limit = self.cache_limit;
        let client = self.client.clone();

        let mut url = self.base_url.clone();
        url.set_path(&key);

        Arc::new(HTTPRequester::new(client, url, cache_limit))
    }
}
