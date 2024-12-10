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

    /// This many bytes at the beginning of the file.
    Prefix(usize),

    /// This many bytes at the end of the file.
    Suffix(usize),

    /// This specific range of bytes from the file.
    /// Start inclusive, end exclusive.
    Bounded(usize, usize),
}

/// A file range returned by the server.
#[derive(Copy, Clone)]
pub struct ResponseRange {
    /// The total number of bytes in the file.
    pub bytes_len: usize,

    /// The number of bytes being returned by this request.
    pub bytes_range: usize,
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
    type Data;

    /// Construct a new response from its constituent parts.
    fn from_parts(data: Self::Data, body: BodyStream) -> Self;

    /// Get the range satisfied by this response.
    fn get_range(&self) -> Option<ResponseRange>;

    /// Get the expiration time of this response. If [`None`], it never expires.
    fn expiration_time(&self) -> Option<Self::Timepoint>;

    /// Return a new copy of the `Data` to be used for cache responses.
    fn get_data_for_cache(&self) -> Self::Data;

    /// Consume the response into its streaming body.
    fn into_body(self) -> BodyStream;
}

/// Response variant for [`Requester`], indicating the cacheability of the response.
pub enum ResponseType<R: Response> {
    /// Cache this response.
    ///
    /// This should only be returned if all of the following are true:
    ///    * The request was successful
    ///    * The response returned the same range as the request
    Cache(R),

    /// Passthrough this response.
    Passthrough(R)
}

/// The type of a request which can be repeated with different ranges.
pub trait Requester<R: Response>: Send + Sync + 'static {
    /// Fetch a new copy of the response with the given range.
    fn fetch(&self, range: &RequestRange) -> Pin<Box<dyn Future<Output = Result<ResponseType<R>>> + Send + '_>>;
}

/// The type of a factory for requesters. Given a key, it will create
/// a new requester specific to the key.
pub trait RequestBackend<K, R: Response>: Send + Sync + 'static {
    /// Create a new [`Requester`] that fetches requests for this key.
    fn create_for_key(&self, key: K) -> Arc<dyn Requester<R>>;
}

/// The type of a file sparse map.
pub(crate) type Blocks = parking_lot::Mutex<sparse_map::SparseMap<Bytes>>;
