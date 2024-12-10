use parking_lot::Mutex;
use std::sync::Arc;

use crate::cache::Cache;
use crate::types::*;
use sized_ttl_cache::{Entry, SizedTTLCache};

// Main service for cache streamer.
pub struct Service<K, R>
where
    K: Ord + 'static,
    R: Response,
{
    backend: Arc<dyn RequestBackend<K, R>>,
    cache: Mutex<SizedTTLCache<K, R::Timepoint, Cache<R>>>,
}

impl<K, R> Service<K, R>
where
    K: Ord + 'static,
    R: Response,
{
    pub fn new(backend: Arc<dyn RequestBackend<K, R>>, cache_capacity: usize) -> Self {
        Self {
            backend,
            cache: Mutex::new(SizedTTLCache::with_capacity(cache_capacity)),
        }
    }

    pub async fn call(&self, time: &R::Timepoint, key: K, range: &RequestRange) -> Result<R>
    where
        K: ToOwned<Owned = K>,
    {
        // Try to get the item from cache.
        //
        // The cache may also contain partial items which have not finished streaming yet.
        // This is fine, because our response will fetch unfinished bytes and continue
        // to feed the stream.
        if let Some(streamer) = self.cache.lock().get(time, &key) {
            return streamer;
        }

        // The item was not in the cache, so make a request.
        let requester = self.backend.create_for_key(key);

        // Check to see if we should cache this at all.
        let initial_response = match requester.fetch(range).await? {
            ResponseType::Cache(r) => r,
            ResponseType::Passthrough(r) => return Ok(r),
        };

        let response_range = initial_response.get_range();
        let expiration_time = initial_response.expiration_time();

        // Check for cacheability.
        // Even if the request is potentially cacheable, we only cache requests that return
        // some form of valid response range. Without this, we can't support suffix queries
        // correctly.
        match (response_range, expiration_time) {
            (Some(range), expiration_time) => {
                let streamer = {
                    let streamer = Cache::new(requester, range.bytes_len);
                    let entry = Entry::from_parts(range.bytes_len, expiration_time, streamer);

                    self.cache.lock().get_or_insert(time, &key, entry)
                };

                Ok(streamer)
            }
            _ => Ok(initial_response),
        }
    }
}
