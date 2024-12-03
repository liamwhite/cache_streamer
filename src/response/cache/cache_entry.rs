use core::ops::Range;
use std::sync::Arc;

use crate::response::{Entry, Reader};
use super::InnerStream;
use headers::HeaderMap;
use parking_lot::Mutex;

struct CacheEntry {
    inner: Arc<Mutex<InnerStream>>,
    headers: HeaderMap,
    length: usize,
}

impl Entry for CacheEntry {
    fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    fn length(&self) -> usize {
        self.length
    }

    fn abort(&self) {
        todo!()
    }

    fn reader(&self, range: &Option<Range<usize>>) -> Box<dyn Reader> {
        todo!()
    }
}
