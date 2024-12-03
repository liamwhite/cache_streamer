use std::ops::Range;

use super::contiguous_collection::ContiguousCollection;
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

/// Returns (`ab` intersects `cd` && `ab` starts before `cd`).
fn range_lt_intersecting<I: PartialOrd>(ab: &Range<I>, cd: &Range<I>) -> bool {
    ab.start < cd.start && cd.start < ab.end
}

/// Returns (`ab` intersects `cd` && `ab` starts at or after `cd`).
fn range_gte_intersecting<I: PartialOrd>(ab: &Range<I>, cd: &Range<I>) -> bool {
    ab.start < cd.end && cd.start <= ab.start
}

#[derive(Default)]
struct HoleTracker(Option<(usize, usize)>);

impl HoleTracker {
    fn update(&mut self, start: usize, end: usize) {
        self.0 = Some(
            self.0
                .map_or_else(|| (start, end), |(prev_start, _)| (prev_start, end)),
        );
    }
}

impl From<HoleTracker> for Option<Range<usize>> {
    fn from(value: HoleTracker) -> Self {
        value.0.map(|(start, end)| start..end)
    }
}

#[derive(Default)]
pub struct IntrusiveMap<T: ContiguousCollection> {
    blocks: RBTree<NodeTreeAdapter<T>>,
}

impl<T: ContiguousCollection> IntrusiveMap<T> {
    pub fn get(&self, offset: usize, max_size: usize) -> Option<T::Slice> {
        let requested_range = offset..(offset + max_size);

        match self.blocks.lower_bound(Bound::Included(&offset)).get() {
            Some(node) if range_gte_intersecting(&requested_range, &node.range()) => {
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
                Some(node) if range_gte_intersecting(&requested_range, &node.range()) => {
                    // We are already inside this block, so skip it.
                    (
                        data.len().min(node.block.len() - (offset - node.start)),
                        true,
                    )
                }
                Some(node) if range_lt_intersecting(&requested_range, &node.range()) => {
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

    pub fn union_disjoint(&self, mut range: Range<usize>) -> Option<Range<usize>> {
        let mut it = self.blocks.lower_bound(Bound::Included(&range.start));
        let mut out = HoleTracker::default();

        while range.len() > 0 {
            let (data_advance, it_advance) = match it.get() {
                Some(node) if range_gte_intersecting(&range, &node.range()) => {
                    // We are already inside this block, so skip it.
                    (
                        range.len().min(node.block.len() - (range.start - node.start)),
                        true,
                    )
                }
                Some(node) if range_lt_intersecting(&range, &node.range()) => {
                    // We intersect a block at a higher start, but can insert a block here.
                    let data_advance = range.len().min(node.start - range.start);
                    out.update(range.start, range.start + data_advance);

                    (data_advance, false)
                }
                _ => {
                    // No intersections. If the next block exists, it is higher.
                    out.update(range.start, range.start + range.len());

                    break;
                }
            };

            range.start += data_advance;

            if it_advance {
                it.move_next();
            }
        }

        out.into()
    }
}
