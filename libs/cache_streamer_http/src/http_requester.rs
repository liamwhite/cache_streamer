use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use cache_streamer_lib::types::*;
use futures::StreamExt;
use reqwest::{Client, Response as ReqwestResponse, Url};

use crate::http_response::HTTPResponse;
use crate::{header_util, parse, render};

/// [`Requester`] trait implementation for HTTP.
///
/// Makes HTTP requests against a fixed [`Url`] via [`reqwest`].
pub struct HTTPRequester {
    client: Arc<Client>,
    url: Url,
    cache_limit: usize,
}

impl HTTPRequester {
    /// Builds a new [`HTTPRequester`] using the shared [`Client`].
    ///
    /// `url` will be used without changes to make all requests.
    ///
    /// A response will switch to [`RequesterStatus::Passthrough`] if the response
    /// would have otherwise been cached, but the length is more than `cache_limit`.
    pub fn new(client: Arc<Client>, url: Url, cache_limit: usize) -> Self {
        Self {
            client,
            url,
            cache_limit,
        }
    }
}

impl Requester<HTTPResponse> for HTTPRequester {
    fn fetch(
        &self,
        range: &RequestRange,
    ) -> Pin<Box<dyn Future<Output = Result<RequesterStatus<HTTPResponse>>> + Send + Sync>> {
        let range = range.clone();
        let limit = self.cache_limit;
        let req = self
            .client
            .get(self.url.clone())
            .headers(render::request_range_headers(&range))
            .send();

        Box::pin(async move {
            // Convert to response here to avoid unnecessarily tying lifetime to `self`
            req.await
                .map(|r| into_requester_status(r, range, limit))
                .map_err(|e| e.into())
        })
    }
}

/// Convert the response from [`reqwest`] into a suitable [`HTTPResponse`].
///
/// The following conditions are required to ensure that the output status
/// is [`RequesterStatus::Cache`]:
/// * Response status is success (2xx)
/// * Response range corresponds to request range
/// * Response total length is less than `cache_limit`
/// * Response `cache-control` header does not disallow caching
///
/// Otherwise, [`RequesterStatus::Passthrough`] will be returned.
fn into_requester_status(
    response: ReqwestResponse,
    request_range: RequestRange,
    cache_limit: usize,
) -> RequesterStatus<HTTPResponse> {
    let status = response.status();
    let input_headers = response.headers();

    // Headers from the response determine which headers will be sent, the range to be sent,
    // and the cacheability and expiration time.
    let output_headers = header_util::collect_headers(input_headers);
    let response_range = parse::into_response_range(input_headers, &request_range);
    let (cache, expire_time) = parse::get_cache_possible_and_expire_time(input_headers);

    // Get the body stream.
    let body = Box::pin(response.bytes_stream().map(|r| r.map_err(|e| e.into())));

    // Don't report responses which do not report a length or are too large as cacheable.
    let cacheable_total_size = response_range
        .as_ref()
        .map(|r| r.bytes_len <= cache_limit)
        .unwrap_or(false);

    // Check all preconditions.
    if !status.is_success() || !cacheable_total_size || !cache {
        return RequesterStatus::Passthrough(HTTPResponse::new(status, output_headers, body));
    }

    // At this point we know the request can be cached and has a valid response range.
    let response_range = response_range.unwrap();

    // Build the cache response.
    let output_response = HTTPResponse::from_parts(
        (status, output_headers.clone()),
        response_range.clone(),
        body,
    );

    RequesterStatus::Cache(
        output_response,
        response_range,
        expire_time,
        (status, output_headers),
    )
}
