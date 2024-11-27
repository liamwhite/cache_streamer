use std::ops::Range;

use super::{backend::Backend, merge_range_request, set_path};
use reqwest::{Client, Error, Method, Response};
use url::{ParseError, Url};

pub struct PlainBackend {
    client: Client,
    base_url: Url,
}

impl PlainBackend {
    pub fn create(scheme_and_host: &str) -> Result<Self, ParseError> {
        Ok(PlainBackend {
            client: Client::new(),
            base_url: scheme_and_host.parse::<Url>()?,
        })
    }
}

impl Backend for PlainBackend {
    async fn fetch(
        &self,
        method: &Method,
        path: &str,
        range: &Option<Range<usize>>,
    ) -> Result<Response, Error> {
        let url = set_path(self.base_url.clone(), path);

        let req = self.client.request(method.clone(), url);
        let req = merge_range_request(req, range);

        req.send().await
    }
}
