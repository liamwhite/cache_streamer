use headers::{
    CacheControl, ContentDisposition, ContentLength, ContentRange, ContentType, Header, HeaderMap,
    HeaderMapExt,
};

/// Take headers which will be preserved during passthrough requests.
/// Currently, this list of headers is:
///    - `cache-control`
///    - `content-disposition`
///    - `content-length`
///    - `content-range`
///    - `content-type`
pub fn collect_headers(response_headers: &HeaderMap) -> HeaderMap {
    let mut headers = HeaderMap::new();

    clone_header::<CacheControl>(&mut headers, response_headers);
    clone_header::<ContentDisposition>(&mut headers, response_headers);
    clone_header::<ContentLength>(&mut headers, response_headers);
    clone_header::<ContentRange>(&mut headers, response_headers);
    clone_header::<ContentType>(&mut headers, response_headers);

    headers
}

fn clone_header<H: Header>(dest: &mut HeaderMap, src: &HeaderMap) {
    if let Some(header) = src.typed_get::<H>() {
        dest.typed_insert(header);
    }
}
