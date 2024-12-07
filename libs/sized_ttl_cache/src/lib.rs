use core::borrow::Borrow;
use intrusive_lru_cache::LRUCache;

#[derive(Clone)]
pub struct Entry<T, V> {
    size_bytes: usize,
    expiration_time: Option<T>,
    inner: V,
}

impl<T, V> Entry<T, V>
where
    T: Ord,
{
    /// Creates a new [`Entry`] with the given size and optional expiration timepoint.
    pub fn from_parts(size_bytes: usize, expiration_time: Option<T>, inner: V) -> Self {
        Self {
            size_bytes,
            expiration_time,
            inner,
        }
    }

    fn is_expired(&self, now: &T) -> bool {
        matches!(&self.expiration_time, Some(expiration_time) if now > expiration_time)
    }
}

/// A LRU cache which has a maximum capacity in bytes (instead of entries), and supports
/// TTL-based expiry on a per-entry basis.
///
/// The `T` generic parameter is the type of the time point. This can be any [`Ord`] type.
/// Normal uses should default to [`chrono::DateTime`].
pub struct SizedTTLCache<K, T, V> {
    cache: LRUCache<K, Entry<T, V>>,
    capacity_bytes: usize,
    size_bytes: usize,
}

// TODO: relax Clone on V

impl<K, T, V> SizedTTLCache<K, T, V>
where
    K: Ord + 'static,
    T: Ord,
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
    pub fn get<Q>(&mut self, time: &T, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let mut entry = self.cache.smart_get(key)?;

        if entry.is_expired(time) {
            let _ = entry.remove();
            None
        } else {
            Some(entry.get_value().inner.clone())
        }
    }

    /// Gets the non-expired value corresponding to a key, or inserts the given
    /// data as the new value.
    pub fn get_or_insert<Q>(&mut self, time: &T, key: &Q, value: Entry<T, V>) -> V
    where
        K: Borrow<Q>,
        Q: ToOwned<Owned = K> + Ord + ?Sized,
    {
        if let Some(entry) = self.get(time, key) {
            return entry;
        }

        self.shrink();

        self.cache
            .get_or_insert2(key, || {
                self.size_bytes += value.size_bytes;
                value
            })
            .inner
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ttl_expire() {
        let mut cache = SizedTTLCache::<String, usize, usize>::with_capacity(0);
        cache.get_or_insert(&0, "0", Entry::from_parts(1, Some(1), 1));

        assert_eq!(cache.get(&2, "0"), None);
    }

    #[test]
    fn test_capacity_bound() {
        let mut cache = SizedTTLCache::<String, usize, usize>::with_capacity(0);
        cache.get_or_insert(&0, "0", Entry::from_parts(1, None, 1));
        cache.get_or_insert(&0, "1", Entry::from_parts(1, None, 1));

        assert_eq!(cache.get(&0, "0"), None);
        assert_eq!(cache.get(&0, "1"), Some(1));
    }
}
