use std::sync::Arc;
use parking_lot::Mutex;

use crate::types::*;
use crate::streamer::Streamer;
use sized_ttl_cache::{Entry, SizedTTLCache};

// Main service for cache streamer.
pub struct Service<K, R>
where
    K: Ord + 'static,
    R: Response
{
    backend: Arc<dyn RequestBackend<K, R>>,
    cache: Mutex<SizedTTLCache<K, R::Timepoint, Streamer>>
}

impl<K, R> Service<K, R>
where 
    K: Ord + 'static,
    R: Response,
{
    pub fn new(backend: Arc<dyn RequestBackend<K, R>>, cache_capacity: usize) -> Self {
        Self {
            backend,
            cache: Mutex::new(SizedTTLCache::with_capacity(cache_capacity))
        }
    }

    pub async fn call(&self, time: &R::Timepoint, key: K, range: &RequestRange) -> Result<R> {
        // Try to get the item from cache.
        //
        // The cache may also contain partial items which have not finished streaming yet.
        // This is fine, because our response will calculated unfinished bytes and continue
        // to feed the stream.
        if let Some(streamer) = self.cache.lock().get(time, &key) {
            return streamer;
        }

        // The item was not in the cache, so make a request.
        let requester = self.backend.create_for_key(key);
        let initial_response = requester.fetch(range).await?;

        if initial_response.is_cacheable() {
            let streamer = {
                let streamer = Entry::from_parts(size_bytes, expiration_time, inner)
                self.cache.lock().get_or_insert(time, &key, value)
            };
        } else {
            Ok(initial_response)
        }
    }
}
