use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use futures::{Future, Stream};

/// The file range requested by the downstream client.
#[derive(Default, Clone)]
pub enum RequestRange {
    /// Entire file.
    #[default]
    None,

    /// All bytes starting from this offset.
    AllFrom(usize),

    /// This many bytes at the end of the file.
    Last(usize),

    /// This specific range of bytes from the file.
    /// Start inclusive, end exclusive.
    /// If `start > end`, unpredictable behavior may occur.
    FromTo(usize, usize),
}

/// A file range returned by the server.
#[derive(Default, Clone)]
pub struct ResponseRange {
    /// The total number of bytes in the file.
    pub bytes_len: usize,

    /// The bytes being returned by this request.
    pub bytes_range: RequestRange,
}

/// The type of results to be returned by this cache.
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// The type of body streams to be returned by this cache.
pub type BodyStream = Pin<Box<dyn Stream<Item = Result<Bytes>> + Send + Sync>>;

/// The type of responses to be returned by this cache, and by upstream servers.
pub trait Response: 'static {
    /// The type of cache expiration times.
    type Timepoint: Ord;

    /// Arbitrary data to store alongside a generic response.
    ///
    /// For HTTP, this could be used to store headers.
    /// If not needed, it can be set to `()`.
    type Data: Clone;

    /// Construct a new response from its constituent parts.
    fn from_parts(data: Self::Data, range: ResponseRange, body: BodyStream) -> Result<Self>
    where
        Self: Sized;

    /// Consume the response into its streaming body.
    fn into_body(self) -> BodyStream;
}

/// Response variant for [`Requester`], indicating the cacheability of the response
/// from the requester.
pub enum RequesterStatus<R: Response> {
    /// Cache this response with the given output range, cache expire time, and
    /// associated cache data. If the expire time is [`None`], it will never be
    /// revalidated.
    ///
    /// This should only be returned if all of the following are true:
    ///    * The request was successful
    ///    * The response returned a valid range
    ///    * The response returned the same range as the request
    Cache(R, ResponseRange, Option<R::Timepoint>, R::Data),

    /// Passthrough this response.
    Passthrough(R),
}

/// Response variant for services, indicating whether the response from the service
/// was served from cache or passed through.
pub enum ServiceStatus<R: Response> {
    /// The response was served from cache.
    Cache(R),

    /// The response was passed through.
    Passthrough(R),
}

/// The type of a request which can be repeated with different ranges.
pub trait Requester<R: Response>: Send + Sync + 'static {
    /// Fetch a new copy of the response with the given range.
    fn fetch(
        &self,
        range: &RequestRange,
    ) -> Pin<Box<dyn Future<Output = Result<RequesterStatus<R>>> + Send + Sync>>;
}

/// The type of a factory for requesters. Given a key, it will create
/// a new requester specific to the key.
pub trait RequestBackend<K, R: Response>: Send + Sync + 'static {
    /// Create a new [`Requester`] that fetches requests for this key.
    fn create_for_key(&self, key: &K) -> Arc<dyn Requester<R>>;
}
