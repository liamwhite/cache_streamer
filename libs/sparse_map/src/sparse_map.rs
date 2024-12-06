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

#[derive(Default)]
pub struct SparseMap<T> {
    blocks: RefCell<RBTree<NodeTreeAdapter<T>>>,
}

impl<T> SparseMap<T>
where
    T: ContiguousCollection,
{
    pub fn get(&self, offset: usize, max_size: usize) -> Option<T::Slice> {
        let requested_range = offset..(offset + max_size);
        let blocks = self.blocks.borrow();

        match blocks.lower_bound(Bound::Included(&offset)).get() {
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

    pub fn put_new<C>(&mut self, offset: usize, data: C)
    where
        C: ContiguousCollection<Slice = C>,
        T: From<C>,
    {
        self.walk_discontinuous_regions(offset, data, |it, offset, data| {
            it.insert_before(Node::new(offset, data.into()));
        });
    }

    pub fn union_discontinuous_range(&self, range: Range<usize>) -> Option<Range<usize>> {
        let mut out = HoleTracker::default();

        self.walk_discontinuous_regions(range.start, range.len(), |_, offset, data| {
            out.update(offset, offset + data.len());
        });

        out.into()
    }

    fn walk_discontinuous_regions<C, F>(&self, mut offset: usize, mut data: C, mut on_hole: F)
    where
        C: ContiguousCollection<Slice = C>,
        F: for<'a> FnMut(&mut CursorMut<'a, NodeTreeAdapter<T>>, usize, C),
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
