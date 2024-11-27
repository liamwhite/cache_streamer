use super::{convert, merge_range_request, set_path, Backend, Range};
use crate::aws::{Configuration, Signer};
use crate::{Error, Method, Response};
use reqwest::Client;
use url::{ParseError, Url};

pub struct S3Backend {
    client: Client,
    base_url: Url,
    configuration: Configuration,
}

impl S3Backend {
    // TODO S3
    #[allow(dead_code)]
    pub fn new(config: Configuration) -> Result<Self, ParseError> {
        let url = format!("{}://{}", config.scheme, config.host);
        let url = url.parse::<Url>()?;

        Ok(Self {
            client: Client::new(),
            base_url: url,
            configuration: config,
        })
    }
}

impl Backend for S3Backend {
    async fn fetch(&self, method: &Method, path: &str, range: &Range) -> Result<Response, Error> {
        let url = set_path(self.base_url.clone(), path);
        let signature = Signer::new(&self.configuration).sign_request(method.as_str(), path, b"");

        let req = self
            .client
            .request(method.clone(), url)
            .header("authorization", signature.authorization)
            .header("x-amz-date", signature.x_amz_date)
            .header("x-amz-content-sha256", signature.x_amz_content_sha256)
            .header("host", &self.configuration.host);
        let req = merge_range_request(req, range);

        Ok(convert(req.send().await?))
    }
}
