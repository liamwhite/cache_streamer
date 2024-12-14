// https://doc.rust-lang.org/core/mem/fn.size_of.html
// The types *const T, &T, .. has the same size as usize.
#[cfg(not(target_pointer_width = "64"))]
compile_error!("compilation is only allowed for 64-bit targets");

pub use http_request_backend::HTTPRequestBackend;
pub use http_requester::HTTPRequester;
pub use http_response::HTTPResponse;
pub use http_service::HTTPService;

mod header_util;
mod http_request_backend;
mod http_requester;
mod http_response;
mod http_service;
mod parse;
mod render;
