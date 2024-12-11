use std::pin::Pin;

use crate::types::*;
use bytes::Bytes;
use futures::{future, stream, Future};

mod blocks;
mod body_reader;

const HELLO_WORLD: &[u8] = b"hello world";
const GOODBYE: &[u8] = b"goodbye";

struct AdaptiveResponse;
struct AdaptiveRequester;

impl Response for AdaptiveResponse {
    type Data = ();
    type Timepoint = usize;

    fn from_parts(_data: Self::Data, _range: ResponseRange, _body: BodyStream) -> Self {
        Self
    }

    fn into_body(self) -> BodyStream {
        Box::pin(stream::once(async { Ok(Bytes::from(GOODBYE)) }))
    }
}

impl Requester<AdaptiveResponse> for AdaptiveRequester {
    fn fetch(
        &self,
        _range: &RequestRange,
    ) -> Pin<Box<dyn Future<Output = Result<ResponseType<AdaptiveResponse>>> + Send + Sync>> {
        Box::pin(future::ready(Ok(ResponseType::Cache(
            AdaptiveResponse,
            ResponseRange::default(),
            None,
            (),
        ))))
    }
}
