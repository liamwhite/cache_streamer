use cache_streamer_lib::types::{RequestRange, ResponseRange};
use chrono::{DateTime, Utc};
use headers::{CacheControl, ContentLength, ContentRange, HeaderMap, HeaderMapExt};
use range_header::{ByteRangeSpec, Range};

/// Converts HTTP request `range` to [`RequestRange`].
///
/// * If the header is not present, the range is valid and [`RequestRange::None`].
/// * If the header is not parseable or not a byte range, the range is invalid.
/// * If the header is a multipart range, the range is invalid.
/// * If the header specifies a from-to range with from > to, the range is invalid.
///
/// Otherwise, the range is a valid [`RequestRange`].
pub fn get_request_range(request_headers: &HeaderMap) -> Option<RequestRange> {
    let ranges = match request_headers.typed_get::<Range>() {
        Some(Range::Bytes(ranges)) => ranges,
        Some(..) => return None,
        None => return Some(RequestRange::None),
    };

    let range = match (ranges.first(), ranges.len()) {
        (Some(range), 1) => range,
        _ => return None,
    };

    let range = match range {
        ByteRangeSpec::FromTo(start, end) if start > end => return None,
        ByteRangeSpec::FromTo(start, end) => {
            RequestRange::FromTo(l(*start)?, l(*end)?.checked_add(1)?)
        }
        ByteRangeSpec::AllFrom(start) => RequestRange::AllFrom(l(*start)?),
        ByteRangeSpec::Last(size) => RequestRange::Last(l(size.get())?),
    };

    Some(range)
}

/// Converts HTTP response `content-length` and `content-range` into a [`ResponseRange`].
///
/// For this function to return a valid [`ResponseRange`], the following conditions
/// must be met:
/// * The `content-length` header must be set
/// * The `content-range` header is set _if and only if_ the request range was not
///   [`RequestRange::None`].
/// * The `content-range` header returns a complete range with no missing components
///   (asterisks in the textual representation)
///
/// If these conditions are not met, returns [`None`]. Otherwise, returns the response
/// range.
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
            // Fill from content-length header.
            return Some(ResponseRange {
                bytes_len: l(content_length.0)?,
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
        bytes_len: l(bytes_len)?,
        bytes_range: RequestRange::FromTo(l(bytes_range.0)?, l(bytes_range.1)?),
    })
}

/// Determines whether the given headers correspond to a cacheable response, and if so,
/// if and when that response would expire.
///
/// Responses are cacheable if the `cache-control` header is not present, or present
/// and does not contain `no-cache` / `no-store`.
///
/// The expiration time is calculated from `max-age` if it is present, or [`None`] if it is not.
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

/// Fallibly convert a `u64` length to a `usize` length with a very short method name.
///
/// The `l` stands for length.
fn l(x: u64) -> Option<usize> {
    x.try_into().ok()
}
