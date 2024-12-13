use cache_streamer_lib::types::{RequestRange, ResponseRange};
use headers::{ContentLength, ContentRange, HeaderMap, HeaderMapExt};
use range_header::ByteRangeBuilder;

/// Adds the appropriate HTTP `Range` header from the given [`RequestRange`],
/// if one is necessary.
pub fn put_request_range(range: &RequestRange) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let builder = ByteRangeBuilder::new();
    let builder = match range {
        RequestRange::None => Ok(builder),
        RequestRange::AllFrom(start) => builder.range(*start as u64..),
        RequestRange::FromTo(start, end) => builder.range((*start as u64)..(*end as u64)),
        RequestRange::Last(size) => builder.suffix(*size as u64),
    };

    if let Ok(h) = builder.and_then(|b| b.finish()) {
        headers.typed_insert(h);
    }

    headers
}

/// Adds the appropriate HTTP `Content-Length` and `Content-Range` headers from
/// the given [`ResponseRange`] to the given [`HeaderMap`].
pub fn put_response_range(headers: HeaderMap, range: Option<ResponseRange>) -> HeaderMap {
    let range = match range {
        None => return headers,
        Some(range) => range,
    };

    match range.bytes_range {
        RequestRange::None => put_content_length(headers, range.bytes_len),
        RequestRange::AllFrom(start) => {
            put_content_range(headers, start..range.bytes_len, range.bytes_len)
        }
        RequestRange::Last(size) => put_content_range(
            headers,
            (range.bytes_len - size)..range.bytes_len,
            range.bytes_len,
        ),
        RequestRange::FromTo(start, end) => put_content_range(headers, start..end, range.bytes_len),
    }
}

/// Adds the HTTP `Content-Length` header to the given [`HeaderMap`].
fn put_content_length(mut headers: HeaderMap, complete_length: usize) -> HeaderMap {
    headers.typed_insert(ContentLength(complete_length as u64));
    headers
}

/// Adds the HTTP `Content-Range` header to the given [`HeaderMap`].
fn put_content_range(
    headers: HeaderMap,
    range: core::ops::Range<usize>,
    complete_length: usize,
) -> HeaderMap {
    let mut headers = put_content_length(headers, complete_length);

    headers.typed_insert(
        ContentRange::bytes(
            (range.start as u64)..(range.end as u64),
            complete_length as u64,
        )
        .expect("content range"),
    );

    headers
}
