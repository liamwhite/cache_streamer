use cache_streamer_lib::types::RequestRange;
use headers::{HeaderMap, HeaderMapExt};
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
