use std::ops::Range;

use super::header::{copy_header_if_exists, put_content_length_and_range, put_content_type};
use super::ReaderArc;
use axum::body::Body;
use axum::response::Response as AxumResponse;
use headers::{ContentLength, ContentRange, ContentType};
use http::{Method, StatusCode};
use reqwest::Response as ReqwestResponse;

pub fn reader_response(
    method: &Method,
    response_range: &Option<Range<usize>>,
    reader: ReaderArc,
) -> Option<AxumResponse> {
    let complete_length = reader.complete_length();

    if let Some(range) = response_range {
        if range.start > range.end || range.end > complete_length {
            return Some(error_response(StatusCode::RANGE_NOT_SATISFIABLE));
        }
    }

    let resp = AxumResponse::builder();
    let resp = put_content_length_and_range(resp, complete_length, response_range)?;
    let resp = put_content_type(resp, reader.content_type())?;
    let status = if response_range.is_some() {
        StatusCode::PARTIAL_CONTENT
    } else if complete_length > 0 {
        StatusCode::OK
    } else {
        StatusCode::NO_CONTENT
    };
    let resp = resp.status(status);

    match *method {
        Method::HEAD => resp.body(Body::empty()).ok(),
        _ => {
            let body_stream = reader.output_range(response_range);

            resp.body(Body::from_stream(body_stream)).ok()
        }
    }
}

pub fn passthrough_response(response: ReqwestResponse) -> Option<AxumResponse> {
    let input_headers = response.headers();

    let resp = AxumResponse::builder();
    let resp = copy_header_if_exists::<ContentLength>(resp, input_headers)?;
    let resp = copy_header_if_exists::<ContentRange>(resp, input_headers)?;
    let resp = copy_header_if_exists::<ContentType>(resp, input_headers)?;

    resp.status(response.status())
        .body(Body::from_stream(response.bytes_stream()))
        .ok()
}

pub fn error_response(status: StatusCode) -> AxumResponse {
    let reason = status.canonical_reason().unwrap_or("Unknown Error");

    AxumResponse::builder()
        .status(status)
        .body(Body::new(reason.to_owned()))
        .expect("Failed to create error response body")
}
