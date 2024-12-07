use std::sync::Arc;

use crate::types::*;
use crate::streamer::Streamer;
use bytes::Bytes;
use parking_lot::Mutex;
use sparse_map::SparseMap;

pub(crate) struct Cache<R: Response> {
    blocks: Arc<Mutex<SparseMap<Bytes>>>,
    requester: Arc<dyn Requester<R>>,
    size_bytes: usize,
}

impl<R: Response> Cache<R> {
    pub(crate) fn new(requester: Arc<dyn Requester<R>>, size_bytes: usize) -> Self {
        Self {
            blocks: Arc::default(),
            requester,
            size_bytes,
        }
    }

    pub(crate) fn create_streamer(range: &RequestRange) -> Streamer {
        
    }
}
