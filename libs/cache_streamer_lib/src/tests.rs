use core::sync::atomic::{AtomicUsize, Ordering};
use std::pin::Pin;
use std::sync::Arc;

use crate::types::*;
use bytes::Bytes;
use futures::{future, stream, Future};

mod blocks;
mod body_reader;
mod response_builder;
mod service;

const HELLO_WORLD: &[u8] = b"hello world";
const GOODBYE: &[u8] = b"goodbye";
const EXPIRE_TIME: usize = 2;

struct SimpleResponse(BodyStream);

impl SimpleResponse {
    fn new() -> Self {
        Self(Box::pin(stream::once(async { Ok(Bytes::from(GOODBYE)) })))
    }
}

impl Response for SimpleResponse {
    type Data = ();
    type Timepoint = usize;

    fn from_parts(_data: Self::Data, _range: ResponseRange, body: BodyStream) -> Self {
        Self(body)
    }

    fn into_body(self) -> BodyStream {
        self.0
    }
}

struct SimpleRequester {
    count: Arc<AtomicUsize>,
    is_cache: bool,
}

impl SimpleRequester {
    fn new(count: Arc<AtomicUsize>, is_cache: bool) -> Self {
        Self { count, is_cache }
    }
}

impl Requester<SimpleResponse> for SimpleRequester {
    fn fetch(
        &self,
        range: &RequestRange,
    ) -> Pin<Box<dyn Future<Output = Result<ResponseType<SimpleResponse>>> + Send + Sync>> {
        self.count.fetch_add(1, Ordering::Relaxed);

        let resp = SimpleResponse::new();

        Box::pin(future::ready(Ok(if self.is_cache {
            ResponseType::Cache(
                resp,
                ResponseRange {
                    bytes_len: GOODBYE.len(),
                    bytes_range: range.clone(),
                },
                Some(EXPIRE_TIME),
                (),
            )
        } else {
            ResponseType::Passthrough(resp)
        })))
    }
}

struct SimpleRequestBackend {
    count: Arc<AtomicUsize>,
    is_cache: bool,
}

impl SimpleRequestBackend {
    fn new(is_cache: bool) -> Self {
        Self {
            count: Arc::default(),
            is_cache,
        }
    }

    fn request_count(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }
}

impl RequestBackend<String, SimpleResponse> for SimpleRequestBackend {
    fn create_for_key(&self, _key: String) -> Arc<dyn Requester<SimpleResponse>> {
        Arc::new(SimpleRequester::new(self.count.clone(), self.is_cache))
    }
}
