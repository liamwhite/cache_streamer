use std::ops::Range;

use super::{backend::Backend, merge_range_request, set_path};
use crate::aws::{configuration::Configuration, signer::Signer};
use reqwest::{Client, Error, Method, Response};
use url::{ParseError, Url};

pub struct S3Backend<C: Configuration> {
    client: Client,
    base_url: Url,
    configuration: C,
}

impl<C: Configuration> S3Backend<C> {
    // TODO S3
    #[allow(dead_code)]
    pub fn new(config: C) -> Result<Self, ParseError> {
        let url = format!("{}://{}", config.scheme(), config.host());
        let url = url.parse::<Url>()?;

        Ok(Self {
            client: Client::new(),
            base_url: url,
            configuration: config,
        })
    }
}

impl<C: Configuration> Backend for S3Backend<C> {
    async fn fetch(
        &self,
        method: &Method,
        path: &str,
        range: &Option<Range<usize>>,
    ) -> Result<Response, Error> {
        let url = set_path(self.base_url.clone(), path);
        let signature = Signer::new(&self.configuration).sign_request(method.as_str(), path, b"");

        let req = self
            .client
            .request(method.clone(), url)
            .header("authorization", signature.authorization)
            .header("x-amz-date", signature.x_amz_date)
            .header("x-amz-content-sha256", signature.x_amz_content_sha256)
            .header("host", self.configuration.host());
        let req = merge_range_request(req, range.clone());

        req.send().await
    }
}
