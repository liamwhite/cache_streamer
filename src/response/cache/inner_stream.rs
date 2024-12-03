use core::ops::Range;
use crate::container::{ContiguousCollection, SparseMap};
use bytes::Bytes;

#[derive(Default, PartialEq)]
pub enum StreamStatus {
    #[default]
    Streamable,
    Aborted,
}

#[derive(Default)]
pub struct InnerStream {
    completed: SparseMap<Bytes>,
    status: StreamStatus,
}

impl InnerStream {
    pub fn get(&self, offset: usize, max_size: usize) -> Option<<Bytes as ContiguousCollection>::Slice> {
        self.completed.get(offset, max_size)
    }

    pub fn union_disjoint(&self, range: Range<usize>) -> Option<Range<usize>> {
       self.completed.union_disjoint(range)
    }

    pub fn is_aborted(&self) -> bool {
        self.status == StreamStatus::Aborted
    }

    pub fn put_new(&mut self, offset: usize, data: Bytes) {
        self.completed.put_new(offset, data);
    }

    pub fn abort(&mut self) {
        self.status = StreamStatus::Aborted;
    }
}
