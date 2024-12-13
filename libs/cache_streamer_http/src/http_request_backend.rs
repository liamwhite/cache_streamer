use std::sync::Arc;
use std::time::Duration;

use cache_streamer_lib::types::*;
use reqwest::{Client, Url};

use crate::http_requester::HTTPRequester;
use crate::http_response::HTTPResponse;

pub struct HTTPRequestBackend {
    client: Arc<Client>,
    base_url: Url,
}

impl HTTPRequestBackend {
    pub fn new(base_url: Url) -> Self {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::limited(1))
            .connect_timeout(Duration::from_secs(10))
            .http2_adaptive_window(true)
            .build()
            .expect("reqwest HTTP client");

        Self {
            client: Arc::new(client),
            base_url,
        }
    }
}

impl RequestBackend<String, HTTPResponse> for HTTPRequestBackend {
    fn create_for_key(&self, key: String) -> Arc<dyn Requester<HTTPResponse>> {
        let client = self.client.clone();

        let mut url = self.base_url.clone();
        url.set_path(&key);

        Arc::new(HTTPRequester::new(client, url))
    }
}
