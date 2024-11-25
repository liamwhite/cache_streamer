use std::error::Error;
use std::ops::{Deref, Range};
use std::sync::Arc;

use crate::request::backend::Backend;
use crate::sparse::SparseMap;
use bytes::Bytes;
use futures::{stream, Stream, StreamExt};
use headers::{ContentType, HeaderMapExt};
use parking_lot::Mutex;
use reqwest::{Method, Response};
use tokio::sync::{futures::Notified, Notify};

use super::{fetch, FetchResponse, FetchStream};

#[derive(Default, PartialEq)]
enum StreamStatus {
    #[default]
    Streamable,
    Aborted,
}

#[derive(Default)]
struct InnerStream {
    promised: SparseMap<usize>,
    completed: SparseMap<Bytes>,
    status: StreamStatus,
}

impl InnerStream {
    fn new(init: Range<usize>) -> Self {
        let mut this: InnerStream = Default::default();
        this.promised.put_new(init.start, init.len());
        this
    }
}

pub struct CacheReader<B: Backend> {
    backend: Arc<B>,
    path: String,
    length: usize,
    content_type: Option<ContentType>,
    inner: Mutex<InnerStream>,
    signal: Notify,
}

struct StreamGuard<B: Backend> {
    reader: Arc<CacheReader<B>>,
    canceled: bool,
}

impl<B: Backend> StreamGuard<B> {
    fn new(reader: Arc<CacheReader<B>>) -> Self {
        Self {
            reader,
            canceled: false,
        }
    }

    fn resolve<T, E>(&mut self, res: Result<T, E>) {
        self.canceled = res.is_ok();
    }
}

impl<B: Backend> Drop for StreamGuard<B> {
    fn drop(&mut self) {
        if !self.canceled {
            self.reader.inner.lock().status = StreamStatus::Aborted;
            self.reader.signal.notify_waiters();
        }
    }
}

impl<B: Backend> Deref for StreamGuard<B> {
    type Target = CacheReader<B>;

    fn deref(&self) -> &Self::Target {
        Arc::deref(&self.reader)
    }
}

impl<B: Backend + 'static> CacheReader<B> {
    pub fn new(backend: Arc<B>, path: String, stream: FetchStream) -> Arc<Self> {
        let this = Arc::new(Self {
            backend,
            path,
            length: stream.length,
            content_type: stream.response.headers().typed_get::<ContentType>(),
            inner: Mutex::new(InnerStream::new(stream.range.clone())),
            signal: Notify::default(),
        });

        let mut guard = StreamGuard::new(this.clone());
        tokio::spawn(async move {
            guard.resolve(guard.download_response(stream.range, stream.response).await);
        });

        this
    }

    pub fn complete_length(&self) -> usize {
        self.length
    }

    pub fn content_type(&self) -> Option<ContentType> {
        self.content_type.clone()
    }

    pub fn output_range(
        self: &Arc<Self>,
        range: &Option<Range<usize>>,
    ) -> impl Stream<Item = Result<Bytes, Box<dyn Error + Send + Sync>>> + Send {
        let range: Range<usize> = range.clone().unwrap_or(0..self.length);

        self.promise_response(range.clone());

        let start = range.start;
        let end = range.end;

        stream::unfold(
            (start, end, self.clone()),
            move |(current, end, this)| async move {
                // Check bounds.
                if current >= end {
                    return None;
                }

                let result = loop {
                    match this.try_output_bytes(current, end) {
                        Ok(result) => break result,
                        Err(notified) => {
                            notified.await;
                            continue;
                        }
                    }
                };

                match result {
                    Ok(bytes) => {
                        let len = bytes.len();
                        Some((Ok(bytes), (current + len, end, this)))
                    }
                    Err(..) => Some((Err("stream aborted".into()), (current, current, this))),
                }
            },
        )
    }

    fn try_output_bytes(
        &self,
        current: usize,
        end: usize,
    ) -> Result<Result<Bytes, ()>, Notified<'_>> {
        let inner = self.inner.lock();

        // 1. Output chunks available.
        // If the stream has completed, this will cause it to read all remaining without waiting.
        if let Some(chunk) = inner.completed.get(current, end - current) {
            return Ok(Ok(chunk));
        }

        // 2. Stream aborted.
        if let StreamStatus::Aborted = &inner.status {
            return Ok(Err(()));
        }

        // 3. Wait for change in the above.
        Err(self.signal.notified())
    }

    async fn download_response(&self, promise: Range<usize>, resp: Response) -> Result<(), ()> {
        let mut offset = promise.start;
        let mut stream = resp.bytes_stream();

        while let Some(res) = stream.next().await {
            let bytes = res.map_err(|_| ())?;
            offset += bytes.len();

            // Check to make sure we aren't exceeding the length we were created with
            if offset > self.length {
                return Err(());
            }

            self.inner.lock().completed.put_new(offset, bytes);
            self.signal.notify_waiters();
        }

        Ok(())
    }

    async fn fetch_subsequent_response(&self, request_range: Range<usize>) -> Result<(), ()> {
        // Using usize::MAX as the max length, because we will check it when we download.
        let FetchResponse::Cache(stream) = fetch(
            &*self.backend,
            &Method::GET,
            &self.path,
            &Some(request_range),
            usize::MAX,
        )
        .await
        else {
            return Err(());
        };

        self.download_response(stream.range, stream.response).await
    }

    fn promise_response(self: &Arc<Self>, range: Range<usize>) {
        let range = {
            // Don't do anything if the range is empty.
            if range.is_empty() {
                return;
            }

            // Check to see if we even need to fetch this range.
            let mut inner = self.inner.lock();

            // Update the area we are about to cover with this request.
            match inner.promised.put_new(range.start, range.len()) {
                None => return,
                Some(range) => range,
            }
        };

        let mut guard = StreamGuard::new(self.clone());
        tokio::spawn(async move {
            guard.resolve(guard.fetch_subsequent_response(range).await);
        });
    }
}
