use std::sync::Arc;

use crate::{async_trait, Error, Method};
use crate::fetch::{Fetcher, FetchResponse};
use crate::request::Range;
use crate::response::Reader;
use super::InnerStream;
use axum::body::BodyDataStream;
use bytes::Bytes;
use futures::StreamExt;
use parking_lot::Mutex;

struct CacheReader {
    fetcher: Arc<dyn Fetcher>,
    inner_stream: Arc<Mutex<InnerStream>>,
    stream_offset: usize,
    stream_end: usize,
    path: String,
    response: Option<BodyDataStream>,
    response_offset: usize,
}

impl CacheReader {
    fn output_cached(&mut self, size: usize) -> Option<Result<Bytes, Error>> {
        let inner_stream = self.inner_stream.lock();

        if let Some(chunk) = inner_stream.get(self.stream_offset, size) {
            self.stream_offset += chunk.len();
            return Some(Ok(chunk));
        }

        if inner_stream.is_aborted() {
            return Some(Err("stream aborted".into()));
        }

        None
    }

    async fn pull_next(&mut self, size: usize) -> Option<Result<Bytes, Error>> {
        let response = match &mut self.response {
            None => {
                let request_range = (self.stream_offset..self.stream_end).try_into().ok()?;
                self.response = match self.fetcher.fetch(&Method::GET, &self.path, &request_range).await {
                    FetchResponse::Cache(stream) => Some(stream.response.into_body().into_data_stream()),
                    _ => return Some(Err("invalid fetch response".into()))
                };

                self.response.as_mut().expect("invalid response")
            },
            Some(response) => response
        };

        match response.next().await {
            Some(Ok(bytes)) => {
                let len = bytes.len();

                self.inner_stream.lock().put_new(self.response_offset, bytes.clone());
                self.response_offset += len;

                let chunk = bytes.slice(0..len.min(size));

                self.stream_offset += chunk.len();
                Some(Ok(chunk))
            }
            Some(Err(err)) => Some(Err(err.into())),
            _ => None,
        }
    }
}

#[async_trait]
impl Reader for CacheReader {
    async fn next(&mut self) -> Option<Result<Bytes, Error>> {
        if self.stream_offset >= self.stream_end {
            return None;
        }

        let remaining = self.stream_end - self.stream_offset;

        match self.output_cached(remaining) {
            Some(result) => return Some(result),
            None => self.pull_next(remaining).await
        }
    }
}
