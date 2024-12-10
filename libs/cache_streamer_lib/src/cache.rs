use std::sync::Arc;

use crate::types::*;
use crate::body_reader::AdaptiveReader;

pub(crate) struct Cache<R>
where
    R: Response,
{
    blocks: Arc<Blocks>,
    data: R::Data,
    requester: Arc<dyn Requester<R>>,
    size: usize,
}

impl<R> Cache<R>
where
    R: Response,
{
    pub(crate) fn new(requester: Arc<dyn Requester<R>>, size: usize) -> Self {
        Self {
            blocks: Arc::default(),
            requester,
            size,
        }
    }

    pub(crate) fn stream(&self, range: &RequestRange) -> Result<R> {
        let blocks = self.blocks.clone();
        let requester = self.requester.clone();

        let size = self.size;
        let (start, end) = match *range {
            RequestRange::None => (0, size),
            RequestRange::Prefix(start) => (start.min(size), size),
            RequestRange::Suffix(count) => (size - count.min(size), size),
            RequestRange::Bounded(start, end) => (start.min(size), end.min(size)),
        };

        let stream = AdaptiveReader::new_adaptive(requester, blocks).into_stream(start, end);
    }
}
