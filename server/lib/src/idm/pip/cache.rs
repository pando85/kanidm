//! PIP Attribute Cache with TTL support
//!
//! This module provides caching for PIP retrieved attributes with configurable
//! time-to-live (TTL) to balance freshness with performance.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use super::{PipAttributeSet, PipId, PipSubject};

/// A cached attribute entry with timestamp
#[derive(Debug, Clone)]
struct CachedEntry {
    attributes: PipAttributeSet,
    cached_at: Instant,
    ttl: Duration,
}

impl CachedEntry {
    fn new(attributes: PipAttributeSet, ttl: Duration) -> Self {
        CachedEntry {
            attributes,
            cached_at: Instant::now(),
            ttl,
        }
    }

    fn is_expired(&self) -> bool {
        Instant::now().duration_since(self.cached_at) > self.ttl
    }

    fn remaining_ttl(&self) -> Option<Duration> {
        let elapsed = Instant::now().duration_since(self.cached_at);
        if elapsed > self.ttl {
            None
        } else {
            Some(self.ttl - elapsed)
        }
    }
}

/// Cache key combining PIP ID and subject UUID
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CacheKey {
    pip_id: PipId,
    subject_uuid: uuid::Uuid,
}

impl CacheKey {
    fn new(pip_id: PipId, subject: &PipSubject) -> Self {
        CacheKey {
            pip_id,
            subject_uuid: subject.uuid,
        }
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub total_entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub expired_evictions: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }
}

/// The PIP attribute cache with TTL support
#[derive(Debug)]
pub struct PipAttributeCache {
    entries: BTreeMap<CacheKey, CachedEntry>,
    default_ttl: Duration,
    stats: CacheStats,
    max_entries: usize,
}

impl PipAttributeCache {
    /// Create a new cache with default TTL
    pub fn new(default_ttl: Duration) -> Self {
        PipAttributeCache {
            entries: BTreeMap::new(),
            default_ttl,
            stats: CacheStats::default(),
            max_entries: 10000,
        }
    }

    /// Create a new cache with custom settings
    pub fn with_settings(default_ttl: Duration, max_entries: usize) -> Self {
        PipAttributeCache {
            entries: BTreeMap::new(),
            default_ttl,
            stats: CacheStats::default(),
            max_entries,
        }
    }

    /// Get cached attributes for a subject from a specific PIP
    ///
    /// Returns None if not cached or if entry is expired
    pub fn get(&mut self, pip_id: &PipId, subject: &PipSubject) -> Option<PipAttributeSet> {
        let key = CacheKey::new(pip_id.clone(), subject);

        if let Some(entry) = self.entries.get(&key) {
            if entry.is_expired() {
                self.stats.misses += 1;
                self.stats.expired_evictions += 1;
                self.entries.remove(&key);
                None
            } else {
                self.stats.hits += 1;
                Some(entry.attributes.clone())
            }
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// Store attributes in the cache
    ///
    /// If the cache is full, oldest entries are evicted
    pub fn put(&mut self, pip_id: &PipId, subject: &PipSubject, attributes: PipAttributeSet) {
        self.put_with_ttl(pip_id, subject, attributes, self.default_ttl);
    }

    /// Store attributes with a specific TTL
    pub fn put_with_ttl(
        &mut self,
        pip_id: &PipId,
        subject: &PipSubject,
        attributes: PipAttributeSet,
        ttl: Duration,
    ) {
        let key = CacheKey::new(pip_id.clone(), subject);

        if self.entries.len() >= self.max_entries {
            self.evict_oldest();
        }

        self.entries.insert(key, CachedEntry::new(attributes, ttl));
    }

    /// Remove cached entry for a specific subject
    pub fn remove(&mut self, pip_id: &PipId, subject: &PipSubject) {
        let key = CacheKey::new(pip_id.clone(), subject);
        if self.entries.remove(&key).is_some() {
            self.stats.evictions += 1;
        }
    }

    /// Clear all cached entries
    pub fn clear(&mut self) {
        let count = self.entries.len();
        self.entries.clear();
        self.stats.evictions += count as u64;
    }

    /// Remove all expired entries
    pub fn purge_expired(&mut self) -> usize {
        let expired_keys: Vec<_> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.is_expired())
            .map(|(key, _)| key.clone())
            .collect();

        let count = expired_keys.len();
        for key in expired_keys {
            self.entries.remove(&key);
        }
        self.stats.expired_evictions += count as u64;
        count
    }

    /// Get cache statistics
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Get number of entries in cache
    pub fn size(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Evict oldest entries to make room for new ones
    fn evict_oldest(&mut self) {
        if self.entries.is_empty() {
            return;
        }

        let oldest_key = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.cached_at)
            .map(|(key, _)| key.clone());

        if let Some(key) = oldest_key {
            self.entries.remove(&key);
            self.stats.evictions += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_cache_put_get() {
        let mut cache = PipAttributeCache::new(Duration::from_secs(60));
        let pip_id = PipId::new("test_pip");
        let subject = PipSubject::from_uuid(Uuid::new_v4());

        let mut attrs = PipAttributeSet::new();
        attrs.insert(
            super::super::PipAttributeName::new(&pip_id, "test_attr"),
            super::super::PipAttributeValue::String("test_value".to_string()),
        );

        cache.put(&pip_id, &subject, attrs.clone());

        let retrieved = cache.get(&pip_id, &subject);
        assert!(retrieved.is_some());
        assert_eq!(cache.stats().hits, 1);
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = PipAttributeCache::new(Duration::from_secs(60));
        let pip_id = PipId::new("test_pip");
        let subject = PipSubject::from_uuid(Uuid::new_v4());

        let retrieved = cache.get(&pip_id, &subject);
        assert!(retrieved.is_none());
        assert_eq!(cache.stats().misses, 1);
    }

    #[allow(clippy::disallowed_methods)]
    #[test]
    fn test_cache_expiration() {
        let mut cache = PipAttributeCache::new(Duration::from_millis(10));
        let pip_id = PipId::new("test_pip");
        let subject = PipSubject::from_uuid(Uuid::new_v4());

        let attrs = PipAttributeSet::new();
        cache.put(&pip_id, &subject, attrs);

        std::thread::sleep(Duration::from_millis(20));

        let retrieved = cache.get(&pip_id, &subject);
        assert!(retrieved.is_none());
        assert_eq!(cache.stats().expired_evictions, 1);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = PipAttributeCache::new(Duration::from_secs(60));
        let pip_id = PipId::new("test_pip");
        let subject = PipSubject::from_uuid(Uuid::new_v4());

        cache.put(&pip_id, &subject, PipAttributeSet::new());
        assert_eq!(cache.size(), 1);

        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_max_entries_eviction() {
        let mut cache = PipAttributeCache::with_settings(Duration::from_secs(60), 3);
        let pip_id = PipId::new("test_pip");

        for _ in 0..5 {
            let subject = PipSubject::from_uuid(Uuid::new_v4());
            cache.put(&pip_id, &subject, PipAttributeSet::new());
        }

        assert_eq!(cache.size(), 3);
        assert!(cache.stats().evictions >= 2);
    }
}
