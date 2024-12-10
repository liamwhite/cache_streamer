use crate::types::*;

use bytes::Bytes;
use futures::StreamExt;
use std::sync::Arc;

/// A simple body reader which tracks a blocks object and exhausts once there are no
/// remaining blocks at a given offset.
struct BlockBodyReader(Arc<Blocks>);

impl BlockBodyReader {
    fn new(blocks: Arc<Blocks>) -> Self {
        Self(blocks)
    }

    /// Attempt to pull bytes from the sparse file at the given offset. If bytes can
    /// be pulled from the sparse map at this location, then the offset is updated,
    /// and a view of the bytes is returned. Otherwise, [`None`] is returned.
    /// 
    /// The caller is responsible for ensuring `offset < end` before calling this function.
    /// Failure to do so will result in unpredictable behavior.
    fn next(&self, offset: &mut usize, end: usize) -> Option<Bytes> {
        debug_assert!(*offset < end);

        // Return next if bytes are immediately readable.
        self.0
            .lock()
            .get(*offset, end - *offset)
            .map(|bytes| {
                *offset += bytes.len();
                bytes
            })
    }

    /// Consume the block reader into the blocks shared reference.
    fn into_inner(self) -> Arc<Blocks> {
        self.0
    }
}

/// A simple body reader which tracks an underlying stream and exhausts once the stream
/// exhausts.
struct StreamBodyReader(BodyStream);

impl StreamBodyReader {
    fn new(stream: BodyStream) -> Self {
        Self(stream)
    }

    /// Attempt to pull bytes from the stream. If bytes can be pulled from the stream,
    /// then the offset is updated, and the bytes are returned. Otherwise, [`None`] is
    /// returned.
    ///
    /// The caller is responsible for ensuring `offset < end` before calling this function.
    /// Failure to do so will result in unpredictable behavior.
    async fn next(&mut self, offset: &mut usize, end: usize) -> Option<Result<Bytes>> {
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
/// reference while also returning the results.
struct TeeBodyReader {
    blocks: Arc<Blocks>,
    stream_reader: StreamBodyReader,
}

impl TeeBodyReader {
    fn new(blocks: Arc<Blocks>, stream: BodyStream) -> Self {
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
    async fn next(&mut self, offset: &mut usize, end: usize) -> Option<Result<Bytes>> {
        let current_offset = *offset;

        Some(self.stream_reader
            .next(offset, end)
            .await?
            .map(|bytes| {
                self.blocks.lock().put_new(current_offset, bytes.clone());
                bytes
            }))
    }
}

async fn make_tee_reader<R>(
    requester: Arc<dyn Requester<R>>,
    blocks: Arc<Blocks>,
    range: &RequestRange
) -> Result<TeeBodyReader>
where
    R: Response,
{
    let result = match requester.fetch(range).await {
        Ok(ResponseType::Cache(r)) => r,
        Ok(ResponseType::Passthrough(..)) => return Err("invalid upstream status".into()),
        Err(e) => return Err(e),
    };

    Ok(TeeBodyReader::new(blocks, result.into_body()))
}

/// A reader type which tracks a blocks object and a requester, and if the blocks
/// object exhausts during a pull, makes a new tee body reader covering the remaining
/// range.
enum AdaptiveReader<R> {
    Block(Arc<dyn Requester<R>>, BlockBodyReader),
    Tee(TeeBodyReader),
    Error,
}

impl<R> AdaptiveReader<R>
where
    R: Response,
{
    /// If currently reading blocks, attempts to pull new data from the blocks. If reading
    /// blocks fails, creates a new tee body reader at the current offset. Otherwise, attempts
    /// to pull data from the tee body reader.
    ///
    /// The caller is responsible for ensuring `offset < end` before calling this function.
    /// Failure to do so will result in unpredictable behavior.
    async fn next(&mut self, offset: &mut usize, end: usize) -> Option<Result<Bytes>> {
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
                let range = RequestRange::Bounded(*offset, end);
                let tee = make_tee_reader(requester, reader.into_inner(), &range).await;

                match tee {
                    Err(e) => return Some(Err(e)),
                    Ok(tee) => tee,
                }
            },
        };

        let result = tee.next(offset, end).await;

        // Reset error state.
        *self = Self::Tee(tee);

        result
    }
}