use cache_streamer_lib::types::{RequestRange, ResponseRange};
use core::ops::Range;
use headers::{ContentLength, ContentRange, HeaderMap, HeaderMapExt};

/// Adds the appropriate HTTP `Content-Length` and `Content-Range` headers from
/// the given [`ResponseRange`] to the given [`HeaderMap`].
pub fn put_response_range(headers: HeaderMap, range: ResponseRange) -> HeaderMap {
    match range.bytes_range {
        RequestRange::None => put_length(headers, range.bytes_len),
        RequestRange::Prefix(start) => put_range(headers, start..range.bytes_len, range.bytes_len),
        RequestRange::Suffix(size) => put_range(
            headers,
            (range.bytes_len - size)..range.bytes_len,
            range.bytes_len,
        ),
        RequestRange::Bounded(start, end) => put_range(headers, start..end, range.bytes_len),
    }
}

/// Adds the HTTP `Content-Length` header to the given [`HeaderMap`].
fn put_length(mut headers: HeaderMap, complete_length: usize) -> HeaderMap {
    headers.typed_insert(ContentLength(complete_length as u64));
    headers
}

/// Adds the HTTP `Content-Range` header to the given [`HeaderMap`].
fn put_range(headers: HeaderMap, range: Range<usize>, complete_length: usize) -> HeaderMap {
    let mut headers = put_length(headers, complete_length);

    headers.typed_insert(
        ContentRange::bytes(
            (range.start as u64)..(range.end as u64),
            complete_length as u64,
        )
        .expect("content range"),
    );

    headers
}
