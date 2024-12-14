use cache_streamer_lib::types::{RequestRange, ResponseRange};
use chrono::{DateTime, Utc};
use headers::{CacheControl, ContentLength, ContentRange, HeaderMap, HeaderMapExt};
use http::StatusCode;
use range_header::{ByteRangeSpec, Range};

/// Converts a HTTP `Range` bound to [`RequestRange`].
pub fn get_request_range(headers: &HeaderMap) -> Result<RequestRange, StatusCode> {
    let ranges = match headers.typed_get::<Range>() {
        Some(Range::Bytes(ranges)) => ranges,
        Some(..) => return Err(StatusCode::RANGE_NOT_SATISFIABLE),
        None => return Ok(RequestRange::None),
    };

    let range = match (ranges.first(), ranges.len()) {
        (Some(range), 1) => range,
        _ => return Err(StatusCode::RANGE_NOT_SATISFIABLE),
    };

    let range = match range {
        ByteRangeSpec::FromTo(start, end) if start > end => {
            return Err(StatusCode::RANGE_NOT_SATISFIABLE)
        }
        ByteRangeSpec::FromTo(start, end) => {
            RequestRange::FromTo(*start as usize, *end as usize + 1)
        }
        ByteRangeSpec::AllFrom(start) => RequestRange::AllFrom(*start as usize),
        ByteRangeSpec::Last(size) => RequestRange::Last(size.get() as usize),
    };

    Ok(range)
}

/// Converts HTTP `Content-Length` and `Content-Range` into a [`ResponseRange`].
pub fn into_response_range(
    response_headers: &HeaderMap,
    request_range: &RequestRange,
) -> Option<ResponseRange> {
    let has_request_range = !matches!(request_range, RequestRange::None);

    // Upstreams which do not return a content length header aren't usable.
    let content_length = response_headers.typed_get::<ContentLength>()?;

    // Check to see if we have a content range.
    let response_range = match response_headers.typed_get::<ContentRange>() {
        None if !has_request_range => {
            // No response range, no request range.
            // Fill from ContentLength header.
            return Some(ResponseRange {
                bytes_len: content_length.0 as usize,
                bytes_range: RequestRange::None,
            });
        }
        Some(..) if !has_request_range => {
            // Response range but no request range.
            return None;
        }
        None => {
            // Request range but no response range.
            return None;
        }
        Some(range) => range,
    };

    let (Some(bytes_range), Some(bytes_len)) =
        (response_range.bytes_range(), response_range.bytes_len())
    else {
        // Incomplete range returned. Bytes missing or complete length missing.
        return None;
    };

    Some(ResponseRange {
        bytes_len: bytes_len as usize,
        bytes_range: RequestRange::FromTo(bytes_range.0 as usize, bytes_range.1 as usize),
    })
}

pub fn get_cache_possible_and_expire_time(
    response_headers: &HeaderMap,
) -> (bool, Option<DateTime<Utc>>) {
    let cache_control = match response_headers.typed_get::<CacheControl>() {
        Some(header) => header,
        None => {
            // No cache-control header, so no restrictions.
            return (true, None);
        }
    };

    if cache_control.no_cache() || cache_control.no_store() {
        // Not allowed to cache.
        return (false, None);
    }

    (true, cache_control.max_age().map(|age| Utc::now() + age))
}
