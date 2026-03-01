use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::hash::Hash;

/// A debouncer that prevents rapid-fire events from overwhelming the system.
/// Only emits the latest value after a specified delay with no new updates.
#[allow(dead_code)]
pub struct Debouncer<K: Eq + Hash, V> {
    delay: Duration,
    pending: Arc<Mutex<HashMap<K, (V, Instant)>>>,
}

#[allow(dead_code)]
impl<K: Eq + Hash + Clone, V: Clone> Debouncer<K, V> {
    /// Creates a new debouncer with the specified delay
    pub fn new(delay_ms: u64) -> Self {
        Self {
            delay: Duration::from_millis(delay_ms),
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Debounce a value. Returns Some(value) if enough time has passed since last update,
    /// None if we should wait longer.
    pub fn debounce(&self, key: K, value: V) -> Option<V> {
        let mut pending = self.pending.lock()
            .expect("pending lock poisoned");
        let now = Instant::now();

        // Check if we should debounce
        let should_wait = if let Some((_old_value, last_update)) = pending.get(&key) {
            now.duration_since(*last_update) < self.delay
        } else {
            false
        };

        if should_wait {
            // Update pending value but don't emit yet
            pending.insert(key, (value, now));
            None
        } else {
            // Enough time has passed or first time, emit the value
            pending.insert(key.clone(), (value.clone(), now));
            Some(value)
        }
    }

    /// Force emit a value, bypassing the debounce delay
    pub fn force_emit(&self, key: K, value: V) -> V {
        let mut pending = self.pending.lock()
            .expect("pending lock poisoned");
        pending.insert(key, (value.clone(), Instant::now()));
        value
    }

    /// Clear all pending values
    pub fn clear(&self) {
        let mut pending = self.pending.lock()
            .expect("pending lock poisoned");
        pending.clear();
    }
}

/// Simple time-based cache with TTL
pub struct TtlCache<K: Eq + Hash, V> {
    ttl: Duration,
    cache: Arc<Mutex<HashMap<K, (V, Instant)>>>,
}

#[allow(dead_code)]
impl<K: Eq + Hash + Clone, V: Clone> TtlCache<K, V> {
    /// Creates a new cache with the specified TTL in seconds
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            ttl: Duration::from_secs(ttl_seconds),
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get a value from cache if it exists and hasn't expired
    pub fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.cache.lock()
            .expect("cache lock poisoned");
        if let Some((value, timestamp)) = cache.get(key) {
            if timestamp.elapsed() < self.ttl {
                return Some(value.clone());
            } else {
                // Expired, remove it
                cache.remove(key);
            }
        }
        None
    }

    /// Insert a value into the cache
    pub fn insert(&self, key: K, value: V) {
        let mut cache = self.cache.lock()
            .expect("cache lock poisoned");
        cache.insert(key, (value, Instant::now()));
    }

    /// Clear all cached values
    pub fn clear(&self) {
        let mut cache = self.cache.lock()
            .expect("cache lock poisoned");
        cache.clear();
    }

    /// Remove expired entries
    pub fn cleanup(&self) {
        let mut cache = self.cache.lock()
            .expect("cache lock poisoned");
        cache.retain(|_, (_, timestamp)| timestamp.elapsed() < self.ttl);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_debouncer() {
        let debouncer = Debouncer::new(100); // 100ms delay

        // First call should emit
        assert_eq!(debouncer.debounce("key1", "value1"), Some("value1"));

        // Immediate second call should not emit
        assert_eq!(debouncer.debounce("key1", "value2"), None);

        // Wait for delay to pass
        thread::sleep(Duration::from_millis(150));

        // Now it should emit
        assert_eq!(debouncer.debounce("key1", "value3"), Some("value3"));
    }

    #[test]
    fn test_ttl_cache() {
        let cache = TtlCache::new(1); // 1 second TTL

        // Insert and retrieve immediately
        cache.insert("key1", "value1");
        assert_eq!(cache.get(&"key1"), Some("value1"));

        // Wait for expiration
        thread::sleep(Duration::from_secs(2));

        // Should be expired
        assert_eq!(cache.get(&"key1"), None);
    }
}
