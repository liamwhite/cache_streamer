use intrusive_lru_cache::{InsertOrGetResult, LRUCache};
use std::cell::Cell;

pub struct TransientCache<T: Clone> {
    cache: LRUCache<String, Entry<T>>,
    capacity_bytes: usize,
    size_bytes: usize,
}

struct Entry<T> {
    size_bytes: usize,
    use_count: Cell<usize>,
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

    pub fn insert_or_get(&mut self, key: String, size_bytes: usize, inner: T) -> T {
        self.shrink();

        let entry = match self.cache.insert_or_get(key, Entry::new(size_bytes, inner)) {
            InsertOrGetResult::Existed(entry, ..) => {
                entry.update_use_count();
                entry
            }
            InsertOrGetResult::Inserted(entry) => {
                self.size_bytes += size_bytes;
                entry
            }
        };

        entry.inner.clone()
    }

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
            use_count: Cell::new(0),
            inner,
        }
    }

    fn update_use_count(&self) {
        self.use_count.set(self.use_count.get().saturating_add(1));
    }
}
