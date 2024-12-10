use std::sync::Arc;

use crate::blocks::Blocks;
use crate::body_reader::AdaptiveReader;
use crate::types::*;

/// Builder for response data based on a requester and template response.
pub struct ResponseBuilder<R>
where
    R: Response,
{
    requester: Arc<dyn Requester<R>>,
    size: usize,
    data: R::Data,
    blocks: Blocks,
}

impl<R> ResponseBuilder<R>
where
    R: Response,
{
    /// Create a new builder based on a response, then return self and a response
    /// created from the builder.
    pub fn new(
        response: R,
        range: &ResponseRange,
        data: R::Data,
        requester: Arc<dyn Requester<R>>,
    ) -> (R, Self) {
        let this = Self {
            requester,
            size: range.bytes_len,
            data,
            blocks: Blocks::default(),
        };

        let blocks = this.blocks.clone();
        let reader = AdaptiveReader::new_from_body_stream(blocks, response.into_body());

        (this.stream_with_reader(&range.bytes_range, reader), this)
    }

    /// Create a new response which streams body data from the given request range.
    /// If the request range is invalid, it is clipped to the underlying size of the body.
    pub fn stream(&self, range: &RequestRange) -> R {
        let blocks = self.blocks.clone();
        let requester = self.requester.clone();

        self.stream_with_reader(range, AdaptiveReader::new_adaptive(requester, blocks))
    }

    /// Create a new response from the template data given a range and a reader.
    fn stream_with_reader(&self, range: &RequestRange, reader: AdaptiveReader<R>) -> R {
        let (start, end) = get_start_and_end(self.size, range);

        let range = ResponseRange {
            bytes_len: self.size,
            bytes_range: RequestRange::Bounded(start, end),
        };

        R::from_parts(
            self.data.clone(),
            range,
            Box::pin(reader.into_stream(start, end)),
        )
    }
}

/// Find the bounded byte range of the given potentially unbounded request range,
/// given the overall size of a file.
fn get_start_and_end(size: usize, range: &RequestRange) -> (usize, usize) {
    match *range {
        RequestRange::None => (0, size),
        RequestRange::Prefix(start) => (start.min(size), size),
        RequestRange::Suffix(count) => (size - count.min(size), size),
        RequestRange::Bounded(start, end) => (start.min(size), end.min(size)),
    }
}
