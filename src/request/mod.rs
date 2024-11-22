use std::ops::Range;

pub use plain_backend::PlainBackend;
use reqwest::RequestBuilder;
use url::Url;

pub mod backend;
pub mod plain_backend;
pub mod s3_backend;

fn set_path(mut url: Url, path: &str) -> Url {
    url.set_path(path);
    url
}

fn merge_range_request(req: RequestBuilder, range: Option<Range<usize>>) -> RequestBuilder {
    match range {
        Some(Range { start, end }) => req.header("range", format!("bytes={}-{}", start, end)),
        _ => req,
    }
}
