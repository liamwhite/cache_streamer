use core::ops::Range;
use std::sync::Arc;

use crate::request::{Backend, Range as RequestRange};
use crate::{Method, Response};
pub use cache_reader::CacheReader;
use util::{empty_range_if_head, should_cache, try_get_content_range};

pub mod cache_reader;
pub mod util;

pub struct FetchStream {
    pub length: usize,
    range: Range<usize>,
    response: Response,
}

pub enum FetchResponse {
    Passthrough(Response),
    Cache(FetchStream),
    Err,
}

pub async fn fetch(
    backend: &Arc<dyn Backend>,
    method: &Method,
    path: &str,
    request_range: &RequestRange,
    max_cache_length: usize,
) -> FetchResponse {
    let response = match backend.fetch(method, path, request_range).await {
        Err(err) => {
            log::error!("{}", err.to_string());
            return FetchResponse::Err;
        }
        Ok(resp) => resp,
    };

    // Some responses are informational and should not be cached by this proxy
    if !should_cache(response.status()) {
        return FetchResponse::Passthrough(response);
    }

    // We need to know the expected resource length to make a determination on caching
    match try_get_content_range(request_range, response.headers()) {
        Some((range, length)) if length <= max_cache_length => FetchResponse::Cache(FetchStream {
            range: empty_range_if_head(method, range),
            length,
            response,
        }),
        _ => FetchResponse::Passthrough(response),
    }
}
