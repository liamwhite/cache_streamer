use core::ops::Range;

use super::{ContiguousCollection, HoleTracker};
use super::range;
use intrusive_collections::intrusive_adapter;
use intrusive_collections::{Bound, KeyAdapter, RBTree, RBTreeAtomicLink};

struct Node<T> {
    link: RBTreeAtomicLink,
    start: usize,
    block: T,
}

impl<T: ContiguousCollection> Node<T> {
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


#[derive(Default)]
pub struct SparseMap<T: ContiguousCollection> {
    blocks: RBTree<NodeTreeAdapter<T>>,
}

impl<T: ContiguousCollection> SparseMap<T> {
    pub fn get(&self, offset: usize, max_size: usize) -> Option<T::Slice> {
        let requested_range = offset..(offset + max_size);

        match self.blocks.lower_bound(Bound::Included(&offset)).get() {
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

    pub fn put_new<C>(&mut self, mut offset: usize, mut data: C) -> Option<Range<usize>>
    where
        C: ContiguousCollection<Slice = C>,
        T: From<C>,
    {
        let mut it = self.blocks.lower_bound_mut(Bound::Included(&offset));
        let mut out = HoleTracker::default();

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
                    // We intersect a block at a higher start, but can insert a block here.
                    let data_advance = data.len().min(node.start - offset);
                    out.update(offset, offset + data_advance);
                    it.insert_before(Node::new(offset, data.slice(0..data_advance).into()));

                    (data_advance, false)
                }
                _ => {
                    // No intersections. If the next block exists, it is higher.
                    out.update(offset, offset + data.len());
                    it.insert_before(Node::new(offset, data.into()));

                    break;
                }
            };

            data = data.slice(data_advance..data.len());
            offset += data_advance;

            if it_advance {
                it.move_next();
            }
        }

        out.into()
    }
}
