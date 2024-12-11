use std::sync::Arc;

use bytes::BytesMut;
use futures::StreamExt;

use super::{SimpleRequester, SimpleResponse, GOODBYE};
use crate::response_builder::ResponseBuilder;
use crate::types::*;

#[tokio::test]
async fn test_response_builder() {
    let requester = Arc::new(SimpleRequester);
    let (resp, builder) =
        ResponseBuilder::new(SimpleResponse, &ResponseRange::default(), (), requester);

    let stream = resp
        .into_body()
        .map(|x| x.unwrap())
        .collect::<BytesMut>()
        .await;
    assert_eq!(stream.as_ref(), GOODBYE);
}
