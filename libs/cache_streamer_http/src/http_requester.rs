use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use cache_streamer_lib::types::*;
use futures::StreamExt;
use reqwest::{Client, Response as ReqwestResponse, Url};

use crate::http_response::HTTPResponse;
use crate::{header_util, parse, render};

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
        let range = range.clone();
        let req = self
            .client
            .get(self.url.clone())
            .headers(render::put_request_range(&range))
            .send();

        Box::pin(async move {
            // Convert to response here to avoid unnecessarily tying lifetime to `self`
            req.await
                .map(|r| into_response_type(r, range))
                .map_err(|e| e.into())
        })
    }
}

fn into_response_type(
    response: ReqwestResponse,
    request_range: RequestRange,
) -> ResponseType<HTTPResponse> {
    let status = response.status();
    let input_headers = response.headers();

    // Headers from the response determine which headers will be sent,
    // the range to be sent, and the cacheability and expiration time.
    let output_headers = header_util::collect_headers(input_headers);
    let response_range = parse::get_response_range(input_headers, request_range);
    let (cache, expire_time) = parse::get_cache_possible_and_expire_time(input_headers);

    // Eagerly construct the output response.
    let output_response = HTTPResponse::from_parts(
        (status, output_headers.clone()),
        response_range.clone(),
        Box::pin(response.bytes_stream().map(|r| r.map_err(|e| e.into()))),
    );

    // Check response code, range, and cache-control header.
    if !status.is_success() || response_range.is_none() || !cache {
        return ResponseType::Passthrough(output_response);
    }

    // At this point we know the request can be cached, so we're set.
    ResponseType::Cache(
        output_response,
        response_range.unwrap(),
        expire_time,
        (status, output_headers),
    )
}
