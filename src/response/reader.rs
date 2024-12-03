use crate::async_trait;
use crate::Error;
use bytes::Bytes;

#[async_trait]
pub trait Reader: Send + 'static {
    async fn next(&mut self) -> Option<Result<Bytes, Error>>;
}
