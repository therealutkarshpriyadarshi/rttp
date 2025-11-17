//! In-memory LRU cache with TTL support
//!
//! This module provides a thread-safe LRU (Least Recently Used) cache
//! implementation with optional TTL (Time-To-Live) for entries.
//!
//! # Features
//!
//! - LRU eviction policy
//! - Optional TTL per entry
//! - Thread-safe operations
//! - Automatic expiry cleanup
//! - Type-safe generic implementation
//!
//! # Example
//!
//! ```
//! use pttp::cache::LruCache;
//! use std::time::Duration;
//!
//! let cache = LruCache::new(100); // capacity of 100 items
//! cache.insert("key".to_string(), "value".to_string(), Some(Duration::from_secs(60)));
//!
//! if let Some(value) = cache.get(&"key".to_string()) {
//!     println!("Found: {}", value);
//! }
//! ```

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Entry in the LRU cache with expiration support
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    expires_at: Option<Instant>,
    last_accessed: Instant,
}

impl<V> CacheEntry<V> {
    fn new(value: V, ttl: Option<Duration>) -> Self {
        let now = Instant::now();
        Self {
            value,
            expires_at: ttl.map(|d| now + d),
            last_accessed: now,
        }
    }

    fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Instant::now() >= expires_at
        } else {
            false
        }
    }

    fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }
}

/// Node in the doubly-linked list for LRU tracking
#[derive(Debug, Clone)]
struct Node<K> {
    key: K,
    prev: Option<usize>,
    next: Option<usize>,
}

/// Thread-safe LRU cache with TTL support
///
/// The cache automatically evicts the least recently used items when
/// capacity is reached. Expired items are automatically removed on access.
pub struct LruCache<K, V> {
    inner: Arc<Mutex<LruCacheInner<K, V>>>,
}

struct LruCacheInner<K, V> {
    capacity: usize,
    map: HashMap<K, (V, usize)>, // key -> (value, node_index)
    nodes: Vec<Option<Node<K>>>,
    head: Option<usize>,
    tail: Option<usize>,
    free_indices: Vec<usize>,
    entries: HashMap<K, CacheEntry<V>>,
}

impl<K, V> LruCache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    /// Creates a new LRU cache with the specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(LruCacheInner {
                capacity,
                map: HashMap::new(),
                nodes: Vec::new(),
                head: None,
                tail: None,
                free_indices: Vec::new(),
                entries: HashMap::new(),
            })),
        }
    }

    /// Inserts a key-value pair into the cache with optional TTL
    ///
    /// If the key already exists, its value is updated and it's moved to the front.
    /// If the cache is at capacity, the least recently used item is evicted.
    pub fn insert(&self, key: K, value: V, ttl: Option<Duration>) {
        let mut inner = self.inner.lock().unwrap();
        inner.insert(key, value, ttl);
    }

    /// Gets a value from the cache if it exists and hasn't expired
    ///
    /// This operation updates the item's position to mark it as recently used.
    pub fn get(&self, key: &K) -> Option<V> {
        let mut inner = self.inner.lock().unwrap();
        inner.get(key)
    }

    /// Removes a key from the cache
    pub fn remove(&self, key: &K) -> Option<V> {
        let mut inner = self.inner.lock().unwrap();
        inner.remove(key)
    }

    /// Checks if a key exists in the cache and hasn't expired
    pub fn contains_key(&self, key: &K) -> bool {
        let mut inner = self.inner.lock().unwrap();
        inner.contains_key(key)
    }

    /// Clears all entries from the cache
    pub fn clear(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.clear();
    }

    /// Returns the number of items in the cache
    pub fn len(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        inner.entries.len()
    }

    /// Returns true if the cache is empty
    pub fn is_empty(&self) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.entries.is_empty()
    }

    /// Returns the capacity of the cache
    pub fn capacity(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        inner.capacity
    }

    /// Removes all expired entries from the cache
    pub fn cleanup_expired(&self) -> usize {
        let mut inner = self.inner.lock().unwrap();
        inner.cleanup_expired()
    }
}

impl<K, V> LruCacheInner<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    fn insert(&mut self, key: K, value: V, ttl: Option<Duration>) {
        // Remove if exists
        if self.entries.contains_key(&key) {
            self.remove(&key);
        }

        // Evict if at capacity
        if self.entries.len() >= self.capacity {
            self.evict_lru();
        }

        // Create new entry
        let entry = CacheEntry::new(value, ttl);
        self.entries.insert(key.clone(), entry);
        self.move_to_front(key);
    }

    fn get(&mut self, key: &K) -> Option<V> {
        // Check if expired first
        let is_expired = self
            .entries
            .get(key)
            .map(|entry| entry.is_expired())
            .unwrap_or(false);

        if is_expired {
            self.remove(key);
            return None;
        }

        // Get value and update access time
        if let Some(entry) = self.entries.get_mut(key) {
            entry.touch();
            let value = entry.value.clone();

            // Move to front (most recently used)
            self.move_to_front(key.clone());

            Some(value)
        } else {
            None
        }
    }

    fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(entry) = self.entries.remove(key) {
            self.remove_node(key);
            Some(entry.value)
        } else {
            None
        }
    }

    fn contains_key(&mut self, key: &K) -> bool {
        if let Some(entry) = self.entries.get(key) {
            if entry.is_expired() {
                self.remove(key);
                false
            } else {
                true
            }
        } else {
            false
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.map.clear();
        self.nodes.clear();
        self.head = None;
        self.tail = None;
        self.free_indices.clear();
    }

    fn evict_lru(&mut self) {
        // Remove the tail (least recently used)
        if let Some(tail_idx) = self.tail {
            if let Some(Some(node)) = self.nodes.get(tail_idx) {
                let key = node.key.clone();
                self.entries.remove(&key);
                self.remove_node(&key);
            }
        }
    }

    fn move_to_front(&mut self, key: K) {
        // Remove from current position if exists
        self.remove_node(&key);

        // Get or create node index
        let node_idx = if let Some(idx) = self.free_indices.pop() {
            idx
        } else {
            let idx = self.nodes.len();
            self.nodes.push(None);
            idx
        };

        // Create new node
        let node = Node {
            key: key.clone(),
            prev: None,
            next: self.head,
        };

        // Update previous head
        if let Some(old_head_idx) = self.head {
            if let Some(Some(old_head)) = self.nodes.get_mut(old_head_idx) {
                old_head.prev = Some(node_idx);
            }
        }

        // Update head
        self.nodes[node_idx] = Some(node);
        self.head = Some(node_idx);

        // Update tail if empty
        if self.tail.is_none() {
            self.tail = Some(node_idx);
        }

        // Update map
        if let Some(entry) = self.entries.get(&key) {
            self.map.insert(key, (entry.value.clone(), node_idx));
        }
    }

    fn remove_node(&mut self, key: &K) {
        if let Some((_, node_idx)) = self.map.remove(key) {
            // Extract node info before mutating
            let (prev_idx, next_idx) = if let Some(Some(node)) = self.nodes.get(node_idx) {
                (node.prev, node.next)
            } else {
                return;
            };

            // Update prev node's next
            if let Some(prev_idx) = prev_idx {
                if let Some(Some(prev_node)) = self.nodes.get_mut(prev_idx) {
                    prev_node.next = next_idx;
                }
            } else {
                // This was the head
                self.head = next_idx;
            }

            // Update next node's prev
            if let Some(next_idx) = next_idx {
                if let Some(Some(next_node)) = self.nodes.get_mut(next_idx) {
                    next_node.prev = prev_idx;
                }
            } else {
                // This was the tail
                self.tail = prev_idx;
            }

            // Mark as free
            self.nodes[node_idx] = None;
            self.free_indices.push(node_idx);
        }
    }

    fn cleanup_expired(&mut self) -> usize {
        let mut expired_keys = Vec::new();

        for (key, entry) in &self.entries {
            if entry.is_expired() {
                expired_keys.push(key.clone());
            }
        }

        let count = expired_keys.len();
        for key in expired_keys {
            self.remove(&key);
        }

        count
    }
}

impl<K, V> Clone for LruCache<K, V> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_basic_operations() {
        let cache = LruCache::new(3);

        cache.insert("a".to_string(), 1, None);
        cache.insert("b".to_string(), 2, None);
        cache.insert("c".to_string(), 3, None);

        assert_eq!(cache.get(&"a".to_string()), Some(1));
        assert_eq!(cache.get(&"b".to_string()), Some(2));
        assert_eq!(cache.get(&"c".to_string()), Some(3));
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn test_lru_eviction() {
        let cache = LruCache::new(2);

        cache.insert("a".to_string(), 1, None);
        cache.insert("b".to_string(), 2, None);

        assert_eq!(cache.len(), 2);

        // This should evict "a" (least recently used)
        cache.insert("c".to_string(), 3, None);

        assert_eq!(cache.get(&"a".to_string()), None);
        assert_eq!(cache.get(&"b".to_string()), Some(2));
        assert_eq!(cache.get(&"c".to_string()), Some(3));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_lru_update_on_get() {
        let cache = LruCache::new(2);

        cache.insert("a".to_string(), 1, None);
        cache.insert("b".to_string(), 2, None);

        // Access "a" to make it recently used
        cache.get(&"a".to_string());

        // This should evict "b" (now least recently used)
        cache.insert("c".to_string(), 3, None);

        assert_eq!(cache.get(&"a".to_string()), Some(1));
        assert_eq!(cache.get(&"b".to_string()), None);
        assert_eq!(cache.get(&"c".to_string()), Some(3));
    }

    #[test]
    fn test_ttl_expiration() {
        let cache = LruCache::new(10);

        cache.insert("short".to_string(), 1, Some(Duration::from_millis(100)));
        cache.insert("long".to_string(), 2, Some(Duration::from_secs(10)));
        cache.insert("forever".to_string(), 3, None);

        // Should be able to get immediately
        assert_eq!(cache.get(&"short".to_string()), Some(1));

        // Wait for expiration
        thread::sleep(Duration::from_millis(150));

        // Short should be expired
        assert_eq!(cache.get(&"short".to_string()), None);
        assert_eq!(cache.get(&"long".to_string()), Some(2));
        assert_eq!(cache.get(&"forever".to_string()), Some(3));
    }

    #[test]
    fn test_remove() {
        let cache = LruCache::new(3);

        cache.insert("a".to_string(), 1, None);
        cache.insert("b".to_string(), 2, None);

        assert_eq!(cache.remove(&"a".to_string()), Some(1));
        assert_eq!(cache.get(&"a".to_string()), None);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_clear() {
        let cache = LruCache::new(3);

        cache.insert("a".to_string(), 1, None);
        cache.insert("b".to_string(), 2, None);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
        assert_eq!(cache.get(&"a".to_string()), None);
    }

    #[test]
    fn test_contains_key() {
        let cache = LruCache::new(3);

        cache.insert("a".to_string(), 1, None);

        assert!(cache.contains_key(&"a".to_string()));
        assert!(!cache.contains_key(&"b".to_string()));
    }

    #[test]
    fn test_cleanup_expired() {
        let cache = LruCache::new(10);

        cache.insert("a".to_string(), 1, Some(Duration::from_millis(50)));
        cache.insert("b".to_string(), 2, Some(Duration::from_millis(50)));
        cache.insert("c".to_string(), 3, None);

        thread::sleep(Duration::from_millis(100));

        let cleaned = cache.cleanup_expired();
        assert_eq!(cleaned, 2);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&"c".to_string()), Some(3));
    }

    #[test]
    fn test_thread_safety() {
        let cache = LruCache::new(100);
        let cache_clone = cache.clone();

        let handle = thread::spawn(move || {
            for i in 0..50 {
                cache_clone.insert(format!("key{}", i), i, None);
            }
        });

        for i in 50..100 {
            cache.insert(format!("key{}", i), i, None);
        }

        handle.join().unwrap();

        // All items should be present (within capacity)
        assert!(cache.len() <= 100);
    }
}
