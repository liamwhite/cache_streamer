use crate::render;
use cache_streamer_lib::types::{BodyStream, Response, ResponseRange};
use chrono::{DateTime, Utc};
use headers::HeaderMap;
use http::StatusCode;

pub struct HTTPResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: BodyStream,
}

impl HTTPResponse {
    pub fn new(status: StatusCode, headers: HeaderMap, body: BodyStream) -> Self {
        Self {
            status,
            headers,
            body,
        }
    }

    pub fn set_status(&mut self, status: StatusCode) {
        self.status = status;
    }

    pub fn set_body(&mut self, body: BodyStream) {
        self.body = body;
    }

    pub fn into_parts(self) -> (StatusCode, HeaderMap, BodyStream) {
        (self.status, self.headers, self.body)
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
