use intrusive_lru_cache::LRUCache;

pub struct TransientCache<T: Clone> {
    cache: LRUCache<String, Entry<T>>,
    capacity_bytes: usize,
    size_bytes: usize,
}

struct Entry<T> {
    size_bytes: usize,
    use_count: usize,
    inner: T,
}

impl<T: Clone> TransientCache<T> {
    pub fn new(capacity_bytes: usize) -> Self {
        Self {
            cache: LRUCache::default(),
            capacity_bytes,
            size_bytes: 0,
        }
    }

    pub fn get(&mut self, key: &str) -> Option<T> {
        self.cache.get(key).map(|entry| {
            entry.update_use_count();
            entry.inner.clone()
        })
    }

    pub fn get_or_insert(&mut self, key: &str, size_bytes: usize, inner: T) -> T {
        self.shrink();

        let entry = self.cache.get_or_insert2(key, || {
            self.size_bytes += size_bytes;
            Entry::new(size_bytes, inner)
        });

        entry.update_use_count();
        entry.inner.clone()
    }

    // TODO LRU
    #[allow(dead_code)]
    pub fn remove(&mut self, key: &str) -> Option<T> {
        self.cache.remove(key).map(|entry| entry.inner)
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

impl<T> Entry<T> {
    fn new(size_bytes: usize, inner: T) -> Self {
        Self {
            size_bytes,
            use_count: 0,
            inner,
        }
    }

    fn update_use_count(&mut self) {
        self.use_count += 1;
    }
}
