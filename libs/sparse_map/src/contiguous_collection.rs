use bytes::Bytes;
use core::ops::Range;

/// A collection which has a length representable as a [`usize`], and can be sliced
/// with a half-open range. This is not limited to arrays, and can also represent
/// integer ranges.
pub trait ContiguousCollection {
    /// The type of this data when sliced using a half-open range.
    type Slice;

    /// Get a subset of this collection.
    fn slice(&self, range: Range<usize>) -> Self::Slice;

    /// Get an owned copy subset of this collection.
    fn slice_unshare(&self, range: Range<usize>) -> Self::Slice;

    /// Get the length of this collection.
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl ContiguousCollection for usize {
    type Slice = usize;

    fn slice(&self, range: Range<usize>) -> Self::Slice {
        range.len()
    }

    fn slice_unshare(&self, range: Range<usize>) -> Self::Slice {
        range.len()
    }

    fn len(&self) -> usize {
        *self
    }
}

impl ContiguousCollection for Bytes {
    type Slice = Bytes;

    fn slice(&self, range: Range<usize>) -> Self::Slice {
        Bytes::slice(self, range)
    }

    fn slice_unshare(&self, range: Range<usize>) -> Self::Slice {
        Bytes::copy_from_slice(Bytes::slice(self, range).as_ref())
    }

    fn len(&self) -> usize {
        Bytes::len(self)
    }
}
