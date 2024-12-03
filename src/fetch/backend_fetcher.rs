use std::sync::Arc;

use crate::{async_trait, Method};
use crate::request::{Backend, Range};
use super::{Fetcher, FetchResponse, FetchStream};
use super::util::{empty_range_if_head, should_cache, try_get_content_range};

pub struct BackendFetcher {
    backend: Arc<dyn Backend>,
    max_cache_length: usize,
}

impl BackendFetcher {
    pub fn new(backend: Arc<dyn Backend>, max_cache_length: usize) -> Self {
        Self {
            backend,
            max_cache_length,
        }
    }
}

#[async_trait]
impl Fetcher for BackendFetcher {
    async fn fetch(
        &self,
        method: &Method,
        path: &str,
        request_range: &Range
    ) -> FetchResponse {
        let response = match self.backend.fetch(method, path, request_range).await {
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
            Some((range, length)) if length <= self.max_cache_length => FetchResponse::Cache(FetchStream {
                range: empty_range_if_head(method, range),
                length,
                response,
            }),
            _ => FetchResponse::Passthrough(response),
        }
    }
}
