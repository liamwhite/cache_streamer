use core::sync::atomic::{AtomicUsize, Ordering};
use std::pin::Pin;

use crate::types::*;
use bytes::Bytes;
use futures::{future, stream, Future};

mod blocks;
mod body_reader;
mod response_builder;

const HELLO_WORLD: &[u8] = b"hello world";
const GOODBYE: &[u8] = b"goodbye";

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

struct SimpleRequester(AtomicUsize);

impl SimpleRequester {
    fn new() -> Self {
        Self(AtomicUsize::default())
    }

    fn request_count(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
}

impl Requester<SimpleResponse> for SimpleRequester {
    fn fetch(
        &self,
        range: &RequestRange,
    ) -> Pin<Box<dyn Future<Output = Result<ResponseType<SimpleResponse>>> + Send + Sync>> {
        self.0.fetch_add(1, Ordering::Relaxed);

        Box::pin(future::ready(Ok(ResponseType::Cache(
            SimpleResponse::new(),
            ResponseRange {
                bytes_len: GOODBYE.len(),
                bytes_range: range.clone(),
            },
            None,
            (),
        ))))
    }
}

struct PassthroughRequester;

impl Requester<SimpleResponse> for PassthroughRequester {
    fn fetch(
        &self,
        _range: &RequestRange,
    ) -> Pin<Box<dyn Future<Output = Result<ResponseType<SimpleResponse>>> + Send + Sync>> {
        Box::pin(future::ready(Ok(ResponseType::Passthrough(
            SimpleResponse::new(),
        ))))
    }
}
