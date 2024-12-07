use parking_lot::Mutex;
use std::sync::Arc;

use crate::types::*;
use bytes::Bytes;
use sparse_map::SparseMap;

pub(crate) struct Streamer<R: Response> {
    blocks: Arc<Mutex<SparseMap<Bytes>>>,
    backend: Arc<dyn RequestBackend<R>>,
}
