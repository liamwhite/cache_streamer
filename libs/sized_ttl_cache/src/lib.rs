use core::borrow::Borrow;

use chrono::{DateTime, Utc};
use intrusive_lru_cache::LRUCache;

#[derive(Clone)]
pub struct Entry<V> {
    size_bytes: usize,
    expiration_time: Option<DateTime<Utc>>,
    inner: V,
}

impl<V> Entry<V> {
    /// Creates a new [`Entry`] with the given parameters.
    pub fn from_parts(size_bytes: usize, expiration_time: Option<DateTime<Utc>>, inner: V) -> Self {
        Self {
            size_bytes,
            expiration_time,
            inner,
        }
    }

    /// Consumes the given [`Entry`] and returns the inner value.
    pub fn into_inner(self) -> V {
        self.inner
    }

    fn is_valid(&self) -> bool {
        match self.expiration_time {
            Some(expiration_time) if expiration_time < Utc::now() => false,
            _ => true,
        }
    }
}

/// A LRU cache which has a maximum capacity in bytes (instead of entries), and supports
/// TTL-based expiry on a per-entry basis.
pub struct SizedTTLCache<K, V> {
    cache: LRUCache<K, Entry<V>>,
    capacity_bytes: usize,
    size_bytes: usize,
}

// TODO: relax Clone on V

impl<K, V> SizedTTLCache<K, V>
where
    K: Ord + 'static,
    V: Clone,
{
    /// Create a new [`SizedTTLCache`] with the given maximum capacity in bytes.
    pub fn with_capacity(capacity_bytes: usize) -> Self {
        Self {
            cache: LRUCache::default(),
            capacity_bytes,
            size_bytes: 0,
        }
    }

    /// Gets the non-expired value corresponding to a key, or [`None`] if no value
    /// is available for the key.
    pub fn get<Q>(&mut self, key: &Q) -> Option<Entry<V>>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let mut entry = self.cache.smart_get(key)?;

        if !entry.is_valid() {
            let _ = entry.remove();
            None
        } else {
            Some(entry.get_value().clone())
        }
    }

    /// Gets the non-expired value corresponding to a key, or inserts the given
    /// data as the new value.
    fn get_or_insert<Q>(&mut self, key: &Q, value: Entry<V>) -> Entry<V>
    where
        K: Borrow<Q>,
        Q: ToOwned<Owned = K> + Ord + ?Sized,
    {
        if let Some(entry) = self.get(key) {
            return entry;
        }

        self.shrink();

        self.cache
            .get_or_insert2(key, || {
                self.size_bytes += value.size_bytes;
                value
            })
            .clone()
    }

    fn shrink(&mut self) {
        while self.size_bytes > self.capacity_bytes {
            match self.cache.pop() {
                Some((_, entry)) => self.size_bytes -= entry.size_bytes,
                None => return,
            };
        }
    }
}
