use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use cache_streamer_lib::types::*;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use headers::{Header, HeaderMap, HeaderMapExt};
use reqwest::{Client, Response as ReqwestResponse, Url};

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

fn clone_header<H: Header>(dest: &mut HeaderMap, src: &HeaderMap) {
    if let Some(header) = src.typed_get::<H>() {
        dest.typed_insert(header);
    }
}

fn collect_headers(response_headers: &HeaderMap) -> HeaderMap {
    use headers::{ContentDisposition, ContentLength, ContentRange, ContentType};

    let mut headers = HeaderMap::new();

    clone_header::<ContentDisposition>(&mut headers, response_headers);
    clone_header::<ContentLength>(&mut headers, response_headers);
    clone_header::<ContentRange>(&mut headers, response_headers);
    clone_header::<ContentType>(&mut headers, response_headers);

    headers
}

fn get_response_range(
    response_headers: &HeaderMap,
    request_range: RequestRange,
) -> Option<ResponseRange> {
    use headers::{ContentLength, ContentRange};

    let has_request_range = !matches!(request_range, RequestRange::None);

    // Upstreams which do not return a content length header aren't usable.
    let content_length = response_headers.typed_get::<ContentLength>()?;

    // Check to see if we have a content range.
    let response_range = match response_headers.typed_get::<ContentRange>() {
        None if !has_request_range => {
            // No response range, no request range.
            // Fill from ContentLength header.
            return Some(ResponseRange {
                bytes_len: content_length.0 as usize,
                bytes_range: RequestRange::None,
            });
        }
        Some(..) if !has_request_range => {
            // Response range but no request range.
            return None;
        }
        None => {
            // Request range but no response range.
            return None;
        }
        Some(range) => range,
    };

    let (Some(bytes_range), Some(bytes_len)) =
        (response_range.bytes_range(), response_range.bytes_len())
    else {
        // Incomplete range returned. Bytes missing or complete length missing.
        return None;
    };

    Some(ResponseRange {
        bytes_len: bytes_len as usize,
        bytes_range: RequestRange::FromTo(bytes_range.0 as usize, bytes_range.1 as usize),
    })
}

fn get_cache_possible_and_expire_time(
    response_headers: &HeaderMap,
) -> (bool, Option<DateTime<Utc>>) {
    todo!()
}

fn into_response_type(
    response: ReqwestResponse,
    request_range: RequestRange,
) -> ResponseType<HTTPResponse> {
    let status = response.status();
    let input_headers = response.headers();
    let output_headers = collect_headers(input_headers);
    let response_range = get_response_range(input_headers, request_range);
    let (cache, expire_time) = get_cache_possible_and_expire_time(input_headers);

    let output_response = HTTPResponse::from_parts(
        (status, output_headers.clone()),
        response_range.clone(),
        Box::pin(response.bytes_stream().map(|r| r.map_err(|e| e.into()))),
    );

    // Check response code, range, cache-control header.
    if !status.is_success() || response_range.is_none() || !cache {
        return ResponseType::Passthrough(output_response);
    }

    // At this point we know the request can be cached, so we're good to go.
    ResponseType::Cache(
        output_response,
        response_range.expect("response range"),
        expire_time,
        (status, output_headers),
    )
}
