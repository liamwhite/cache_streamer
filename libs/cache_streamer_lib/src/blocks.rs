use bytes::Bytes;
use parking_lot::RwLock;
use sparse_map::SparseMap;
use std::sync::Arc;

/// The type of a file sparse map.
#[derive(Default, Clone)]
pub struct Blocks(Arc<RwLock<SparseMap<Bytes>>>);

impl Blocks {
    /// See [`SparseMap::get`].
    pub fn get(&self, offset: usize, max_size: usize) -> Option<Bytes> {
        self.0.read().get(offset, max_size)
    }

    /// See [`SparseMap::put_new`].
    pub fn put_new(&self, offset: usize, data: Bytes) {
        self.0.write().put_new(offset, data)
    }
}
