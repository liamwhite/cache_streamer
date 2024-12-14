use crate::render;
use cache_streamer_lib::types::{BodyStream, Response, ResponseRange};
use chrono::{DateTime, Utc};
use headers::HeaderMap;
use http::StatusCode;

/// [`Response`] trait implementation for HTTP.
///
/// Represents all intermediate and output responses used by the library.
pub struct HTTPResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: BodyStream,
}

impl HTTPResponse {
    /// Build a new [`HTTPResponse`] purely from its components, without further
    /// modifications.
    ///
    /// This is in contract to the implementation of [`Response::from_parts`], which
    /// requires and will apply the range header from the input range.
    pub fn new(status: StatusCode, headers: HeaderMap, body: BodyStream) -> Self {
        Self {
            status,
            headers,
            body,
        }
    }

    /// Consume and extract all of the components of the [`HTTPResponse`] for
    /// further processing.
    pub fn into_parts(self) -> (StatusCode, HeaderMap, BodyStream) {
        (self.status, self.headers, self.body)
    }

    /// Override the status of the response.
    pub fn set_status(&mut self, status: StatusCode) {
        self.status = status;
    }

    /// Override the body of the response.
    pub fn set_body(&mut self, body: BodyStream) {
        self.body = body;
    }
}

impl Response for HTTPResponse {
    type Timepoint = DateTime<Utc>;
    type Data = (StatusCode, HeaderMap);

    fn from_parts((status, headers): Self::Data, range: ResponseRange, body: BodyStream) -> Self {
        let headers = render::put_response_range(headers, range);

        Self::new(status, headers, body)
    }

    fn into_body(self) -> BodyStream {
        self.body
    }
}
