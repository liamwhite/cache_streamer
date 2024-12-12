use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use cache_streamer_lib::types::*;
use reqwest::{Client, Response, Url};

use crate::http_response::HTTPResponse;
use crate::render;

pub struct HTTPRequester {
    client: Arc<Client>,
    url: Url,
}

impl HTTPRequester {
    pub fn new(client: Arc<Client>, url: Url) -> Self {
        Self { client, url }
    }
}

impl Requester<HTTPResponse> for HTTPRequester {
    fn fetch(
        &self,
        range: &RequestRange,
    ) -> Pin<Box<dyn Future<Output = Result<ResponseType<HTTPResponse>>> + Send + Sync>> {
        let req = self
            .client
            .get(self.url.clone())
            .headers(render::put_request_range(range))
            .send();

        Box::pin(async move {
            // Convert to response here to avoid unnecessarily tying lifetime to `self`
            req.await.map(into_response_type).map_err(|e| e.into())
        })
    }
}

fn into_response_type(response: Response) -> ResponseType<HTTPResponse> {
    todo!()
}
