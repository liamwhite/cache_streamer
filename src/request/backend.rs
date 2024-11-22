use core::ops::Range;
use reqwest::{Error, Method, Response};

pub trait Backend: Sync + Send {
    fn fetch(
        &self,
        method: &Method,
        path: &str,
        range: &Option<Range<usize>>,
    ) -> impl std::future::Future<Output = Result<Response, Error>> + std::marker::Send;
}
