use std::sync::Arc;

use bytes::BytesMut;
use futures::StreamExt;

use super::{SimpleRequester, GOODBYE};
use crate::response_builder::ResponseBuilder;
use crate::types::*;

#[tokio::test]
async fn test_response_builder() {
    let requester = Arc::new(SimpleRequester::new());
    let range = RequestRange::default();
    let ResponseType::Cache(resp, range, _, data) = requester.fetch(&range).await.unwrap() else {
        panic!()
    };
    let (resp, builder) = ResponseBuilder::new(resp, &range, data, requester.clone());

    let stream = resp
        .into_body()
        .map(|x| x.unwrap())
        .collect::<BytesMut>()
        .await;
    assert_eq!(stream.as_ref(), GOODBYE);
    assert_eq!(requester.request_count(), 1);

    let stream = builder
        .stream(&RequestRange::Bounded(0, 0))
        .into_body()
        .map(|x| x.unwrap())
        .collect::<BytesMut>()
        .await;
    assert_eq!(stream.as_ref(), &b""[..]);
    assert_eq!(requester.request_count(), 1);
}
