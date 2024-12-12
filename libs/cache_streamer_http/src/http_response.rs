use crate::render;
use cache_streamer_lib::types::{BodyStream, Response, ResponseRange};
use chrono::{DateTime, Utc};
use headers::HeaderMap;

pub struct HTTPResponse {
    headers: HeaderMap,
    body: BodyStream,
}

impl HTTPResponse {
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn into_body(self) -> BodyStream {
        self.body
    }
}

impl Response for HTTPResponse {
    type Timepoint = DateTime<Utc>;
    type Data = HeaderMap;

    fn from_parts(headers: Self::Data, range: ResponseRange, body: BodyStream) -> Self {
        let headers = render::put_response_range(headers, range);

        Self { headers, body }
    }

    fn into_body(self) -> BodyStream {
        HTTPResponse::into_body(self)
    }
}
