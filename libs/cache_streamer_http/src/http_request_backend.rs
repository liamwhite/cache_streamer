use std::sync::Arc;

use cache_streamer_lib::types::*;
use reqwest::{Client, Url};

use crate::http_requester::HTTPRequester;
use crate::http_response::HTTPResponse;

pub struct HTTPRequestBackend {
    client: Arc<Client>,
    base_url: Url,
}

impl HTTPRequestBackend {
    pub fn new(client: Arc<Client>, base_url: Url) -> Self {
        Self { client, base_url }
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
