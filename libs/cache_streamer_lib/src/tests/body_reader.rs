use std::pin::Pin;
use std::sync::Arc;

use crate::body_reader::*;
use crate::types::*;
use crate::{blocks::Blocks, types::Requester};

use bytes::Bytes;
use futures::{future, stream, Future, StreamExt};

const HELLO_WORLD: &[u8] = b"hello world";
const GOODBYE: &[u8] = b"goodbye";

#[test]
fn test_block_body_reader() {
    let blocks = Blocks::default();
    blocks.put_new(0, HELLO_WORLD.into());
    blocks.put_new(HELLO_WORLD.len(), GOODBYE.into());

    let reader = BlockBodyReader::new(blocks);
    let mut offset = 0;
    let end = HELLO_WORLD.len() + GOODBYE.len();

    let value = reader.next(&mut offset, end);
    assert_eq!(value.unwrap().as_ref(), HELLO_WORLD);
    assert_eq!(offset, HELLO_WORLD.len());

    let value = reader.next(&mut offset, end);
    assert_eq!(value.unwrap().as_ref(), GOODBYE);
    assert_eq!(offset, end);

    let value = reader.next(&mut offset, end + 1);
    assert_eq!(value, None);
}

#[tokio::test]
async fn test_stream_body_reader() {
    let values = stream::iter(vec![HELLO_WORLD, GOODBYE]).map(|v| Ok(Bytes::from(v)));

    let mut reader = StreamBodyReader::new(Box::pin(values));
    let mut offset = 0;
    let end = HELLO_WORLD.len() + GOODBYE.len();

    let value = reader.next(&mut offset, end).await;
    assert_eq!(value.unwrap().unwrap().as_ref(), HELLO_WORLD);
    assert_eq!(offset, HELLO_WORLD.len());

    let value = reader.next(&mut offset, end).await;
    assert_eq!(value.unwrap().unwrap().as_ref(), GOODBYE);
    assert_eq!(offset, end);

    let value = reader.next(&mut offset, end + 1).await;
    assert!(value.is_none());
}

#[tokio::test]
async fn test_tee_body_reader() {
    let values = stream::iter(vec![HELLO_WORLD, GOODBYE]).map(|v| Ok(Bytes::from(v)));
    let blocks = Blocks::default();

    let mut reader = TeeBodyReader::new(blocks.clone(), Box::pin(values));
    let mut offset = 0;
    let end = HELLO_WORLD.len() + GOODBYE.len();

    let value = reader.next(&mut offset, end).await;
    assert_eq!(value.unwrap().unwrap().as_ref(), HELLO_WORLD);
    assert_eq!(offset, HELLO_WORLD.len());

    let value = reader.next(&mut offset, end).await;
    assert_eq!(value.unwrap().unwrap().as_ref(), GOODBYE);
    assert_eq!(offset, end);

    let value = reader.next(&mut offset, end + 1).await;
    assert!(value.is_none());

    assert_eq!(
        blocks.get(0, HELLO_WORLD.len()).unwrap().as_ref(),
        HELLO_WORLD
    );
    assert_eq!(
        blocks
            .get(HELLO_WORLD.len(), GOODBYE.len())
            .unwrap()
            .as_ref(),
        GOODBYE
    );
}

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

#[tokio::test]
async fn test_adaptive_body_reader() {
    let blocks = Blocks::default();
    blocks.put_new(0, HELLO_WORLD.into());

    let requester = Arc::new(AdaptiveRequester);
    let mut reader = AdaptiveReader::new_adaptive(requester, blocks.clone());
    let mut offset = 0;
    let end = HELLO_WORLD.len() + GOODBYE.len();

    let value = reader.next(&mut offset, end).await;
    assert_eq!(value.unwrap().unwrap().as_ref(), HELLO_WORLD);
    assert_eq!(offset, HELLO_WORLD.len());

    let value = reader.next(&mut offset, end).await;
    assert_eq!(value.unwrap().unwrap().as_ref(), GOODBYE);
    assert_eq!(offset, end);

    let value = reader.next(&mut offset, end + 1).await;
    assert!(value.is_none());

    assert_eq!(
        blocks.get(0, HELLO_WORLD.len()).unwrap().as_ref(),
        HELLO_WORLD
    );
    assert_eq!(
        blocks
            .get(HELLO_WORLD.len(), GOODBYE.len())
            .unwrap()
            .as_ref(),
        GOODBYE
    );
}
