use core::ops::Range;

use super::Reader;
use http::HeaderMap;

pub trait Entry: Send + Sync + 'static {
    fn headers(&self) -> &HeaderMap;
    fn length(&self) -> usize;
    fn abort(&self);
    fn reader(&self, range: &Option<Range<usize>>) -> Box<dyn Reader>;
}
