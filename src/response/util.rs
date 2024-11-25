use headers::{ContentLength, ContentRange, HeaderMap, HeaderMapExt};
use http::{Method, StatusCode};
use std::ops::Range;

pub fn should_cache(status: StatusCode) -> bool {
    matches!(status.as_u16(), 200..=204 | 206)
}

pub fn empty_range_if_head(method: &Method, range: Range<usize>) -> Range<usize> {
    match *method {
        Method::HEAD => range.start..range.start,
        _ => range,
    }
}

pub fn try_get_content_range(
    request_range: &Option<Range<usize>>,
    headers: &HeaderMap,
) -> Option<(Range<usize>, usize)> {
    match request_range {
        Some(..) => try_get_content_range_range(headers),
        None => try_get_content_range_full(headers),
    }
}

fn try_get_content_range_full(headers: &HeaderMap) -> Option<(Range<usize>, usize)> {
    let content_length = headers.typed_get::<ContentLength>()?;
    let content_length = usize::try_from(content_length.0).ok()?;

    Some((0..content_length, content_length))
}

fn try_get_content_range_range(headers: &HeaderMap) -> Option<(Range<usize>, usize)> {
    let content_range = headers.typed_get::<ContentRange>()?;
    let bytes_len = content_range.bytes_len()?;
    let bytes_len = usize::try_from(bytes_len).ok()?;

    let (start, end) = content_range.bytes_range()?;
    let start = usize::try_from(start).ok()?;
    let end = usize::try_from(end).ok()? + 1;

    Some((start..end, bytes_len))
}
