use crate::blocks::Blocks;
use crate::types::*;

use bytes::Bytes;
use futures::{stream, Stream, StreamExt};
use std::sync::Arc;

/// A simple body reader which tracks a blocks object and exhausts once there are no
/// remaining blocks at a given offset.
pub struct BlockBodyReader(Blocks);

impl BlockBodyReader {
    pub fn new(blocks: Blocks) -> Self {
        Self(blocks)
    }

    /// Attempt to pull bytes from the sparse file at the given offset. If bytes can
    /// be pulled from the sparse map at this location, then the offset is updated,
    /// and a view of the bytes is returned. Otherwise, [`None`] is returned.
    ///
    /// The caller is responsible for ensuring `offset < end` before calling this function.
    /// Failure to do so will result in unpredictable behavior.
    pub fn next(&self, offset: &mut usize, end: usize) -> Option<Bytes> {
        debug_assert!(*offset < end);

        self.0.get(*offset, end - *offset).inspect(|bytes| {
            *offset += bytes.len();
        })
    }

    /// Consume the block reader into the blocks object.
    pub fn into_inner(self) -> Blocks {
        self.0
    }
}

/// A simple body reader which tracks an underlying stream and exhausts once the stream
/// exhausts.
pub struct StreamBodyReader(BodyStream);

impl StreamBodyReader {
    pub fn new(stream: BodyStream) -> Self {
        Self(stream)
    }

    /// Attempt to pull bytes from the stream. If bytes can be pulled from the stream,
    /// then the offset is updated, and the bytes are returned. Otherwise, [`None`] is
    /// returned.
    ///
    /// The caller is responsible for ensuring `offset < end` before calling this function.
    /// Failure to do so will result in unpredictable behavior.
    pub async fn next(&mut self, offset: &mut usize, end: usize) -> Option<Result<Bytes>> {
        debug_assert!(*offset < end);

        let bytes = match self.0.next().await? {
            Ok(bytes) => bytes,
            Err(e) => return Some(Err(e)),
        };

        *offset += bytes.len();
        assert!(*offset <= end);

        Some(Ok(bytes))
    }
}

/// A body reader which pipes the results of a body stream into a blocks
/// object while also returning the results.
pub struct TeeBodyReader {
    blocks: Blocks,
    stream_reader: StreamBodyReader,
}

impl TeeBodyReader {
    pub fn new(blocks: Blocks, stream: BodyStream) -> Self {
        Self {
            blocks,
            stream_reader: StreamBodyReader::new(stream),
        }
    }

    /// Attempt to pull bytes from the stream. If bytes can be pulled from the stream,
    /// then the offset is updated, and the bytes are returned. Otherwise, [`None`] is
    /// returned. The bytes are added to the blocks object at the current offset if
    /// they are not already present.
    ///
    /// The caller is responsible for ensuring `offset < end` before calling this function.
    /// Failure to do so will result in unpredictable behavior.
    pub async fn next(&mut self, offset: &mut usize, end: usize) -> Option<Result<Bytes>> {
        let current_offset = *offset;

        Some(
            self.stream_reader
                .next(offset, end)
                .await?
                .inspect(|bytes| {
                    self.blocks.put_new(current_offset, bytes.clone());
                }),
        )
    }
}

async fn make_tee_reader<R>(
    requester: Arc<dyn Requester<R>>,
    blocks: Blocks,
    range: &RequestRange,
) -> Result<TeeBodyReader>
where
    R: Response,
{
    let result = match requester.fetch(range).await? {
        RequesterStatus::Cache(r, ..) => r,
        RequesterStatus::Passthrough(..) => return Err("invalid upstream status".into()),
    };

    Ok(TeeBodyReader::new(blocks, result.into_body()))
}

/// A reader type which tracks a blocks object and a requester, and if the blocks
/// object exhausts during a pull, makes a new tee body reader covering the remaining
/// range.
pub enum AdaptiveReader<R> {
    Block(Arc<dyn Requester<R>>, BlockBodyReader),
    Tee(TeeBodyReader),
    Error,
}

impl<R> AdaptiveReader<R>
where
    R: Response,
{
    pub fn new_adaptive(requester: Arc<dyn Requester<R>>, blocks: Blocks) -> Self {
        Self::Block(requester, BlockBodyReader::new(blocks))
    }

    pub fn new_from_body_stream(blocks: Blocks, stream: BodyStream) -> Self {
        Self::Tee(TeeBodyReader::new(blocks, stream))
    }

    /// If currently reading blocks, attempts to pull new data from the blocks. If reading
    /// blocks fails, creates a new tee body reader at the current offset. Otherwise, attempts
    /// to pull data from the tee body reader.
    ///
    /// The caller is responsible for ensuring `offset < end` before calling this function.
    /// Failure to do so will result in unpredictable behavior.
    pub async fn next(&mut self, offset: &mut usize, end: usize) -> Option<Result<Bytes>> {
        // Consume ourself into the error type.
        //
        // We assume we are going to handle the tee reader case, since it occurs twice,
        // and handle the other cases internally to this match.
        let mut tee = match std::mem::replace(self, Self::Error) {
            Self::Error => return None,
            Self::Tee(tee) => tee,
            Self::Block(requester, reader) => {
                // Block reader may have bytes available immediately, in which case we
                // can just return them here.
                if let Some(bytes) = reader.next(offset, end) {
                    // Reset error state.
                    *self = Self::Block(requester, reader);

                    return Some(Ok(bytes));
                }

                // Build the new tee reader from the input range.
                let range = RequestRange::FromTo(*offset, end);

                match make_tee_reader(requester, reader.into_inner(), &range).await {
                    Err(e) => return Some(Err(e)),
                    Ok(tee) => tee,
                }
            }
        };

        let result = tee.next(offset, end).await;

        // Reset error state.
        *self = Self::Tee(tee);

        result
    }

    /// Consumes and converts the reader into a stream of `Result<Bytes>`.
    ///
    /// The caller should check that `start <= end` before calling this function.
    /// Failure to do so may result in unexpected stream output.
    pub fn into_stream(self, start: usize, end: usize) -> impl Stream<Item = Result<Bytes>> {
        stream::unfold(
            (start, end, self),
            |(mut offset, end, mut this)| async move {
                if offset >= end {
                    return None;
                }

                if let Some(result) = this.next(&mut offset, end).await {
                    return Some((result, (offset, end, this)));
                }

                None
            },
        )
    }
}
