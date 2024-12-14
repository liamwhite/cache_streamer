#[cfg(not(target_pointer_width = "64"))]
compile_error!("compilation is only allowed for 64-bit targets");

pub use http_response::HTTPResponse;
pub use http_service::HTTPService;

mod header_util;
mod http_request_backend;
mod http_requester;
mod http_response;
mod http_service;
mod parse;
mod render;
