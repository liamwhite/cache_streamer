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
    pub fn into_body(self) -> BodyStream {
        self.body
    }
}

impl Response for HTTPResponse {
    type Timepoint = DateTime<Utc>;
    type Data = (StatusCode, HeaderMap);

    fn from_parts(
        (status, headers): Self::Data,
        range: Option<ResponseRange>,
        body: BodyStream,
    ) -> Self {
        let headers = render::put_response_range(headers, range);

        Self {
            headers,
            status,
            body,
        }
    }

    fn into_body(self) -> BodyStream {
        HTTPResponse::into_body(self)
    }
}
