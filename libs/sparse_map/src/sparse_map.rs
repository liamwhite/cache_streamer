use core::ops::Range;

use super::range;
use super::{ContiguousCollection, HoleTracker};
use intrusive_collections::intrusive_adapter;
use intrusive_collections::rbtree::{Cursor, CursorMut};
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
    blocks: RBTree<NodeTreeAdapter<T>>,
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

        match self.blocks.upper_bound(Bound::Included(&offset)).get() {
            Some(node) if range::gte_intersecting(&requested_range, &node.range()) => {
                // Return a view of this block.
                let start = offset - node.start;
                let size = (node.block.len() - start).min(max_size);

                Some(node.block.slice(start..(start + size)))
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
        self.walk_discontinuous_regions_mut(offset, data, |cursor, offset, data| {
            cursor.insert_before(Node::new(offset, data.into()));
        });
    }

    /// Finds the largest discontinuous range which intersects the input range.
    pub fn union_discontinuous_range(&self, range: Range<usize>) -> Option<Range<usize>> {
        let mut out = HoleTracker::default();

        self.walk_discontinuous_regions(range.start, range.len(), |_, offset, data| {
            out.update(offset, offset + data);
        });

        out.into()
    }

    /// Returns the number of indices which are covered by any mapped block.
    pub fn mapped_len(&self) -> usize {
        self.blocks.iter().map(|n| n.block.len()).sum()
    }

    /// Returns the range of indices which are covered by the sparse map.
    pub fn len(&self) -> usize {
        let start = self.blocks.front().get().map_or(0, |n| n.start);
        let end = self
            .blocks
            .back()
            .get()
            .map_or(0, |n| n.start + n.block.len());

        end - start
    }

    /// Returns whether the sparse map covers any indices.
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    // NB: the following two methods are identical but differ only in mutability.

    fn walk_discontinuous_regions_mut<C, F>(
        &mut self,
        mut offset: usize,
        mut data: C,
        mut on_hole: F,
    ) where
        C: ContiguousCollection<Slice = C>,
        F: FnMut(&mut CursorMut<'_, NodeTreeAdapter<T>>, usize, C),
    {
        let mut it = self.blocks.upper_bound_mut(Bound::Included(&offset));
        let requested_range = offset..(offset + data.len());

        // Special case for block before first.
        match it.get() {
            Some(node) if range::gte_intersecting(&requested_range, &node.range()) => {
                // We are already inside this block, so skip it.
                let data_advance = data.len().min(node.block.len() - (offset - node.start));
                data = data.slice(data_advance..data.len());
                offset += data_advance;
            }
            _ => {
                // Block before first, if extant, does not intersect.
            }
        }

        // Reposition to exclusive lower bound. (Inclusive case is handled above.)
        it.move_next();

        while !data.is_empty() {
            let requested_range = offset..(offset + data.len());
            let data_advance = match it.get() {
                Some(node) if range::gte_intersecting(&requested_range, &node.range()) => {
                    // We are already inside this block, so skip it.
                    let data_advance = data.len().min(node.block.len() - (offset - node.start));
                    it.move_next();

                    data_advance
                }
                Some(node) if range::lt_intersecting(&requested_range, &node.range()) => {
                    // We intersect a block at a higher start, but there is a hole here.
                    let data_advance = data.len().min(node.start - offset);
                    on_hole(&mut it, offset, data.slice_unshare(0..data_advance));

                    data_advance
                }
                _ => {
                    // No intersections. If the next block exists, it is higher.
                    on_hole(&mut it, offset, data.slice_unshare(0..data.len()));

                    break;
                }
            };

            data = data.slice(data_advance..data.len());
            offset += data_advance;
        }
    }

    fn walk_discontinuous_regions<C, F>(&self, mut offset: usize, mut data: C, mut on_hole: F)
    where
        C: ContiguousCollection<Slice = C>,
        F: FnMut(&mut Cursor<'_, NodeTreeAdapter<T>>, usize, C),
    {
        let mut it = self.blocks.upper_bound(Bound::Included(&offset));
        let requested_range = offset..(offset + data.len());

        // Special case for block before first.
        match it.get() {
            Some(node) if range::gte_intersecting(&requested_range, &node.range()) => {
                // We are already inside this block, so skip it.
                let data_advance = data.len().min(node.block.len() - (offset - node.start));
                data = data.slice(data_advance..data.len());
                offset += data_advance;
            }
            _ => {
                // Block before first, if extant, does not intersect.
            }
        }

        // Reposition to exclusive lower bound. (Inclusive case is handled above.)
        it.move_next();

        while !data.is_empty() {
            let requested_range = offset..(offset + data.len());
            let data_advance = match it.get() {
                Some(node) if range::gte_intersecting(&requested_range, &node.range()) => {
                    // We are already inside this block, so skip it.
                    let data_advance = data.len().min(node.block.len() - (offset - node.start));
                    it.move_next();

                    data_advance
                }
                Some(node) if range::lt_intersecting(&requested_range, &node.range()) => {
                    // We intersect a block at a higher start, but there is a hole here.
                    let data_advance = data.len().min(node.start - offset);
                    on_hole(&mut it, offset, data.slice_unshare(0..data_advance));

                    data_advance
                }
                _ => {
                    // No intersections. If the next block exists, it is higher.
                    on_hole(&mut it, offset, data.slice_unshare(0..data.len()));

                    break;
                }
            };

            data = data.slice(data_advance..data.len());
            offset += data_advance;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

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

        map.put_new(0, 1024);
        assert_eq!(map.get(0, 1024), Some(1024));

        map.put_new(1024, 1024);
        assert_eq!(map.get(0, 1024), Some(1024));
        assert_eq!(map.get(1024, 1024), Some(1024));
    }

    #[test]
    fn test_overlapping_ranges() {
        let mut map = SparseMap::<usize>::default();

        map.put_new(0, 1024);
        assert_eq!(map.get(0, 1024), Some(1024));

        map.put_new(1024 - 64, 1024);
        assert_eq!(map.get(0, 1024), Some(1024));
        assert_eq!(map.get(1024 - 64, 64), Some(64));
        assert_eq!(map.get(1024, 1024), Some(1024 - 64));
        assert_eq!(map.get(1024, 960), Some(960));
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
        assert_eq!(map.len(), 5120);
        assert_eq!(map.mapped_len(), 1024 + 1024 + 1024);
    }

    #[test]
    fn test_put_new_spanning_multiple_holes_and_blocks() {
        let mut map = SparseMap::<usize>::default();
        map.put_new(100, 100);
        map.put_new(300, 100);
        map.put_new(0, 500);

        assert_eq!(map.get(0, 100), Some(100));
        assert_eq!(map.get(50, 50), Some(50));
        assert_eq!(map.get(100, 100), Some(100));
        assert_eq!(map.get(150, 50), Some(50));
        assert_eq!(map.get(200, 100), Some(100));
        assert_eq!(map.get(250, 50), Some(50));
        assert_eq!(map.get(300, 100), Some(100));
        assert_eq!(map.get(350, 50), Some(50));
        assert_eq!(map.get(400, 100), Some(100));
        assert_eq!(map.get(450, 50), Some(50));
        assert_eq!(map.get(500, 100), None);
        assert_eq!(map.mapped_len(), 500);
        assert_eq!(map.len(), 500);
    }

    #[test]
    fn test_put_new_partially_overlapping_existing_data() {
        let mut map = SparseMap::<usize>::default();
        map.put_new(100, 100);
        map.put_new(300, 100);
        map.put_new(50, 300);

        assert_eq!(map.get(50, 50), Some(50));
        assert_eq!(map.get(100, 100), Some(100));
        assert_eq!(map.get(200, 100), Some(100));
        assert_eq!(map.get(300, 100), Some(100));
        assert_eq!(map.get(300, 50), Some(50));
        assert_eq!(map.get(350, 50), Some(50));
        assert_eq!(map.mapped_len(), 350);

        let mut blocks_found = Vec::new();
        for node in map.blocks.iter() {
            blocks_found.push(node.range());
        }
        assert_eq!(blocks_found, vec![50..100, 100..200, 200..300, 300..400,]);
    }

    #[test]
    fn test_put_new_bytes_spanning_multiple_holes() {
        let mut map = SparseMap::<Bytes>::default();
        map.put_new(100, Bytes::from_static(&[1; 50]));
        map.put_new(200, Bytes::from_static(&[2; 50]));

        let input_data = Bytes::from(vec![3u8; 175]);
        map.put_new(50, input_data);

        assert_eq!(map.get(50, 50), Some(Bytes::from_static(&[3; 50])));
        assert_eq!(map.get(100, 50), Some(Bytes::from_static(&[1; 50])));
        assert_eq!(map.get(150, 50), Some(Bytes::from_static(&[3; 50])));
        assert_eq!(map.get(200, 50), Some(Bytes::from_static(&[2; 50])));
        assert_eq!(map.get(200, 25), Some(Bytes::from_static(&[2; 25])));
        assert_eq!(map.get(250, 50), None);
        assert_eq!(map.mapped_len(), 50 + 50 + 50 + 50);

        let mut blocks_found = Vec::new();
        for node in map.blocks.iter() {
            blocks_found.push(node.range());
        }
        assert_eq!(blocks_found, vec![50..100, 100..150, 150..200, 200..250,]);
    }

    #[test]
    fn test_put_new_into_empty_map() {
        let mut map = SparseMap::<usize>::default();
        map.put_new(100, 200);
        assert_eq!(map.get(100, 200), Some(200));
        assert_eq!(map.get(0, 100), None);
        assert_eq!(map.get(300, 100), None);
        assert_eq!(map.mapped_len(), 200);
    }

    #[test]
    fn test_put_new_at_end_of_map_creating_trailing_hole() {
        let mut map = SparseMap::<usize>::default();
        map.put_new(0, 100);
        map.put_new(200, 100);

        assert_eq!(map.get(0, 100), Some(100));
        assert_eq!(map.get(100, 100), None);
        assert_eq!(map.get(200, 100), Some(100));
        assert_eq!(map.mapped_len(), 200);
    }

    #[test]
    fn test_put_new_lower_bound_condition() {
        let mut map = SparseMap::<Bytes>::default();
        map.put_new(200, Bytes::from_static(&[2; 100]));
        map.put_new(100, Bytes::from_static(&[1; 100]));
        map.put_new(150, Bytes::from_static(&[3; 50]));

        let mut blocks_found = Vec::new();
        for node in map.blocks.iter() {
            blocks_found.push(node.range());
        }
        assert_eq!(blocks_found, vec![100..200, 200..300,]);
    }
}
