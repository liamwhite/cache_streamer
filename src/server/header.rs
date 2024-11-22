use std::ops::Range;

use axum::http::response::Builder;
use headers::{ContentLength, ContentRange, ContentType, Header, HeaderMap, HeaderMapExt};

pub fn put_content_length_and_range(
    mut builder: Builder,
    complete_length: usize,
    response_range: &Option<Range<usize>>,
) -> Option<Builder> {
    let headers = builder.headers_mut()?;

    let body_length = if let Some(r) = response_range {
        let range = u64::try_from(r.start).ok()?..u64::try_from(r.end).ok()?;
        let complete_length = u64::try_from(complete_length).ok()?;
        headers.typed_insert(ContentRange::bytes(range, complete_length).ok()?);

        r.len()
    } else {
        complete_length
    };

    let body_length = u64::try_from(body_length).ok()?;
    headers.typed_insert(ContentLength(body_length));

    Some(builder)
}

pub fn put_content_type(
    mut builder: Builder,
    content_type: Option<ContentType>,
) -> Option<Builder> {
    let headers = builder.headers_mut()?;

    if let Some(content_type) = content_type {
        headers.typed_insert(content_type);
    }

    Some(builder)
}

pub fn copy_header_if_exists<H>(mut builder: Builder, input_headers: &HeaderMap) -> Option<Builder>
where
    H: Header,
{
    if let Some(h) = input_headers.typed_get::<H>() {
        let headers = builder.headers_mut()?;
        headers.typed_insert(h);
    }

    Some(builder)
}
