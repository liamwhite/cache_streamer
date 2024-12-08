use parking_lot::Mutex;
use std::sync::Arc;

use crate::types::*;
use bytes::Bytes;
use futures::Stream;
use sparse_map::SparseMap;

pub(crate) struct Streamer<R>
where
    R: Response,
{
    requester: Arc<dyn Requester<R>>,
    blocks: Arc<Mutex<SparseMap<Bytes>>>,
    offset: usize,
    end: usize,
    inner_stream: Option<BodyStream>,
    inner_stream_offset: usize,
}

impl<R> Streamer<R>
where
    R: Response,
{
    pub(crate) fn new(
        requester: Arc<dyn Requester<R>>,
        blocks: Arc<Mutex<SparseMap<Bytes>>>,
        offset: usize,
        end: usize,
    ) -> Self {
        Self {
            requester,
            blocks,
            offset: offset,
            end: end,
            inner_stream: Option::default(),
            inner_stream_offset: usize::default(),
        }
    }

    pub(crate) async fn next(&mut self) -> Option<Result<Bytes>> {
        loop {
            // Complete on stream end condition.
            if self.offset >= self.end {
                return None;
            }

            // Emplace body stream if necessary.
            let range = {
                let blocks = self.blocks.lock();

                // Return next if bytes are immediately readable.
                if let Some(bytes) = blocks.get(self.offset, self.end - self.offset) {
                    self.offset += bytes.len();
                    return Some(Ok(bytes));
                }

                if self.inner_stream.is_some() {
                    None
                } else {
                    // NOTE: the None case must be unreachable because we are still locked,
                    // and a block was not found in the get call above. Therefore some of this
                    // range must be discontinous.
                    Some(blocks
                        .union_discontinuous_range(self.offset..self.end)
                        .expect("discontinuous range"))
                }
            };

            // Possibly create new request.
            let stream = match self.inner_stream {
                Some(stream) => &mut stream,
                None => {
                    // The range must exist here because this block can only match
                    // when 
                    let range = range.expect("missing range");

                    let result = self
                        .requester
                        .fetch(&RequestRange::Bounded(range.start, range.end))
                        .await;

                    let result = match result {
                        Ok(r) => r,
                        Err(e) => return Some(Err(e)),
                    };

                    if !result.is_cacheable() || result.get_range().is_none() {
                        return Some(Err("cache status changed during stream".into()))
                    }

                    self.inner_stream.get_or_insert(result.into_body())
                }
            };
        }
    }
}

