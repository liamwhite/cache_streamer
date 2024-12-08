use std::sync::Arc;

use crate::types::*;
use crate::streamer::Streamer;
use bytes::Bytes;
use parking_lot::Mutex;
use sparse_map::SparseMap;

pub(crate) struct Cache<R>
where
    R: Response,
{
    blocks: Arc<Mutex<SparseMap<Bytes>>>,
    requester: Arc<dyn Requester<R>>,
    size_bytes: usize,
}

impl<R> Cache<R>
where
    R: Response,
{
    pub(crate) fn new(requester: Arc<dyn Requester<R>>, size_bytes: usize) -> Self {
        Self {
            blocks: Arc::default(),
            requester,
            size_bytes,
        }
    }

    pub(crate) fn stream(&self, range: &RequestRange) -> Result<R> {
        let blocks = self.blocks.clone();
        let requester = self.requester.clone();
        let size_bytes = self.size_bytes;

        let (start, end) = match *range {
            RequestRange::None => (0, size_bytes),
            RequestRange::Prefix(start) => (start.min(size_bytes), size_bytes),
            RequestRange::Suffix(count) => (size_bytes - count.min(size_bytes), size_bytes),
            RequestRange::Bounded(start, end) => (start.min(size_bytes), end.min(size_bytes)),
        };
    }
}
