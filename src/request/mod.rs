use crate::{async_trait, Error, IntoResponse, Method, Response};
pub use plain_backend::PlainBackend;
pub use range::Range;
use reqwest::RequestBuilder;
use url::Url;

pub mod plain_backend;
pub mod range;
pub mod s3_backend;

#[async_trait]
pub trait Backend: Sync + Send + 'static {
    async fn fetch(&self, method: &Method, path: &str, range: &Range) -> Result<Response, Error>;
}

fn set_path(mut url: Url, path: &str) -> Url {
    url.set_path(path);
    url
}

fn merge_range_request(req: RequestBuilder, range: &Range) -> RequestBuilder {
    let range: Result<String, _> = range.try_into();
    match range {
        Ok(range) => req.header("range", range),
        Err(..) => req,
    }
}

fn convert(mut res: reqwest::Response) -> Response {
    let headers = std::mem::take(res.headers_mut());
    (
        res.status(),
        headers,
        Response::new(reqwest::Body::from(res)),
    )
        .into_response()
}
