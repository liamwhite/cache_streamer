use core::ops::Range;

use crate::request::Range as RequestRange;
use crate::{async_trait, Method, Response};
pub use backend_fetcher::BackendFetcher;

mod backend_fetcher;
mod util;

pub struct FetchStream {
    pub length: usize,
    pub range: Range<usize>,
    pub response: Response,
}

pub enum FetchResponse {
    Passthrough(Response),
    Cache(FetchStream),
    Err,
}

#[async_trait]
pub trait Fetcher: Send + Sync + 'static {
    async fn fetch(
        &self,
        method: &Method,
        path: &str,
        request_range: &RequestRange
    ) -> FetchResponse;
}
