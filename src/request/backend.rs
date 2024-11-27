use core::ops::Range;
use std::future::Future;

use crate::{Error, Method, Response};

pub trait Backend: Sync + Send + 'static {
    fn fetch(
        &self,
        method: &Method,
        path: &str,
        range: &Option<Range<usize>>,
    ) -> impl Future<Output = Result<Response, Error>> + Send;
}
