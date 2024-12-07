use core::ops::Range;
use std::cell::RefCell;

use super::range;
use super::{ContiguousCollection, HoleTracker};
use intrusive_collections::intrusive_adapter;
use intrusive_collections::rbtree::CursorMut;
use intrusive_collections::{Bound, KeyAdapter, RBTree, RBTreeAtomicLink};

struct Node<T> {
    link: RBTreeAtomicLink,
    start: usize,
    block: T,
}

impl<T> Node<T>
where
    T: ContiguousCollection,
{
    fn new(start: usize, block: T) -> Box<Self> {
        Box::new(Self {
            link: RBTreeAtomicLink::default(),
            start,
            block,
        })
    }

    fn range(&self) -> Range<usize> {
        self.start..(self.start + self.block.len())
    }
}

intrusive_adapter!(NodeTreeAdapter<T> = Box<Node<T>>: Node<T> { link: RBTreeAtomicLink });

impl<'a, T> KeyAdapter<'a> for NodeTreeAdapter<T> {
    type Key = usize;

    fn get_key(&self, node: &'a Node<T>) -> Self::Key {
        node.start
    }
}

/// A sparse mapping of [`usize`]-bounded intervals to [`ContiguousCollection`]s of type `T`.
///
/// When `T` is [`bytes::Bytes`], [`SparseMap`] provides the semantics of a sparse file, which
/// contains various mapped intervals of bytes, and holes otherwise.
///
/// When `T` is an integer type like [`usize`], [`SparseMap`] provides the semantics of
/// an interval set.
///
/// For simplicity, no merging of adjacent intervals is implemented.
#[derive(Default)]
pub struct SparseMap<T> {
    blocks: RefCell<RBTree<NodeTreeAdapter<T>>>,
}

impl<T> SparseMap<T>
where
    T: ContiguousCollection,
{
    /// Gets the largest slice (smaller than `max_size`) available at `offset`, or
    /// [`None`] if there is nothing mapped at `offset`.
    ///
    /// If a data block is mapped below `offset`, but its size extends into `offset`
    /// then a slice of the block adjusted to start at `offset` will be returned.
    ///
    /// If a data block is mapped with the same start as `offset`, then that block
    /// will be returned.
    pub fn get(&self, offset: usize, max_size: usize) -> Option<T::Slice> {
        let requested_range = offset..(offset + max_size);
        let blocks = self.blocks.borrow();

        match blocks.upper_bound(Bound::Included(&offset)).get() {
            Some(node) if range::gte_intersecting(&requested_range, &node.range()) => {
                // Return a view of this block.
                Some(
                    node.block
                        .slice((offset - node.start)..node.block.len().min(max_size)),
                )
            }
            _ => {
                // Nothing mapped at offset, so nothing to return.
                None
            }
        }
    }

    /// Maps a [`ContiguousCollection`] at the given offset.
    ///
    /// This progressively slices the collection to fit into discontinuous regions,
    /// and discards sections which correspond to offsets which have already been mapped.
    pub fn put_new<C>(&mut self, offset: usize, data: C)
    where
        C: ContiguousCollection<Slice = C>,
        T: From<C>,
    {
        self.walk_discontinuous_regions(offset, data, |it, offset, data| {
            it.insert_before(Node::new(offset, data.into()));
        });
    }

    /// Finds the largest discontinuous range which intersects the input range.
    ///
    /// If there are any discontinuities within the input range, returns a range consisting of
    /// the first unmapped offset up to the last unmapped offset. Otherwise, returns [`None`].
    pub fn union_discontinuous_range(&self, range: Range<usize>) -> Option<Range<usize>> {
        let mut out = HoleTracker::default();

        self.walk_discontinuous_regions(range.start, range.len(), |_, offset, data| {
            out.update(offset, offset + data.len());
        });

        out.into()
    }

    /// Returns the number of indices which are covered by any mapped block.
    pub fn mapped_len(&self) -> usize {
        self.blocks.borrow().iter().map(|n| n.block.len()).sum()
    }

    /// Returns the range of indices which are covered by the sparse map.
    pub fn len(&self) -> usize {
        let blocks = self.blocks.borrow();

        let start = blocks.front().get().map_or(0, |n| n.start);
        let end = blocks.back().get().map_or(0, |n| n.start + n.block.len());

        end - start
    }

    fn walk_discontinuous_regions<C, F>(&self, mut offset: usize, mut data: C, mut on_hole: F)
    where
        C: ContiguousCollection<Slice = C>,
        F: FnMut(&mut CursorMut<'_, NodeTreeAdapter<T>>, usize, C),
    {
        let mut blocks = self.blocks.borrow_mut();
        let mut it = blocks.lower_bound_mut(Bound::Included(&offset));

        while !data.is_empty() {
            let requested_range = offset..(offset + data.len());
            let (data_advance, it_advance) = match it.get() {
                Some(node) if range::gte_intersecting(&requested_range, &node.range()) => {
                    // We are already inside this block, so skip it.
                    (
                        data.len().min(node.block.len() - (offset - node.start)),
                        true,
                    )
                }
                Some(node) if range::lt_intersecting(&requested_range, &node.range()) => {
                    // We intersect a block at a higher start, but there is a hole here.
                    let data_advance = data.len().min(node.start - offset);
                    on_hole(&mut it, offset, data.slice(0..data_advance));

                    (data_advance, false)
                }
                _ => {
                    // No intersections. If the next block exists, it is higher.
                    on_hole(&mut it, offset, data);

                    break;
                }
            };

            data = data.slice(data_advance..data.len());
            offset += data_advance;

            if it_advance {
                it.move_next();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_get_boundary_conditions() {
        let mut map = SparseMap::<usize>::default();
        map.put_new(0, 1024);
        map.put_new(1024, 1024);

        assert_eq!(map.get(0, 1024), Some(1024));
        assert_eq!(map.get(64, 1024), Some(1024 - 64));
        assert_eq!(map.get(1024, 1024), Some(1024));
        assert_eq!(map.get(1024 + 64, 1024), Some(1024 - 64));
        assert_eq!(map.get(2048, 1024), None);
        assert_eq!(map.get(2048 + 64, 1024), None);
    }

    #[test]
    fn test_repeatable_ranges() {
        let mut map = SparseMap::<usize>::default();

        map.put_new(0, 1024);
        assert_eq!(map.get(0, 1024), Some(1024));

        map.put_new(1024, 1024);
        assert_eq!(map.get(0, 1024), Some(1024));
    }

    #[test]
    fn test_overlapping_ranges() {
        let mut map = SparseMap::<usize>::default();

        map.put_new(0, 1024);
        assert_eq!(map.get(0, 1024), Some(1024));

        map.put_new(1024 - 64, 1024);
        assert_eq!(map.get(0, 1024), Some(1024));
        assert_eq!(map.get(1024, 1024), Some(1024 - 64));
    }

    #[test]
    fn test_discontinuous() {
        let mut map = SparseMap::<usize>::default();
        assert_eq!(map.union_discontinuous_range(0..1024), Some(0..1024));

        map.put_new(0, 1024);
        assert_eq!(map.union_discontinuous_range(0..1024), None);
        assert_eq!(
            map.union_discontinuous_range(0..(1024 + 64)),
            Some(1024..(1024 + 64))
        );

        map.put_new(2048, 1024);
        map.put_new(4096, 1024);
        assert_eq!(map.union_discontinuous_range(0..8192), Some(1024..8192));
    }

    #[test]
    fn test_lengths() {
        let mut map = SparseMap::<usize>::default();
        map.put_new(0, 1024);

        assert_eq!(map.len(), 1024);
        assert_eq!(map.mapped_len(), 1024);

        map.put_new(1024, 1024);
        assert_eq!(map.len(), 2048);
        assert_eq!(map.mapped_len(), 2048);

        map.put_new(4096, 1024);
        assert_eq!(map.len(), 4096 + 1024);
        assert_eq!(map.mapped_len(), 2048 + 1024);
    }
}
