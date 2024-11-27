use super::{convert, merge_range_request, set_path, Backend, Range};
use crate::{Error, Method, Response};
use reqwest::Client;
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
    async fn fetch(&self, method: &Method, path: &str, range: &Range) -> Result<Response, Error> {
        let url = set_path(self.base_url.clone(), path);

        let req = self.client.request(method.clone(), url);
        let req = merge_range_request(req, range);

        Ok(convert(req.send().await?))
    }
}
