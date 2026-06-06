//! # oxide-tombstone
//!
//! Tombstone-based deletion for GPU distributed data.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryState { Alive = 1, Tombstoned = 0, Purged = -1 }

#[derive(Debug, Clone)]
pub struct Entry {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub state: EntryState,
    pub version: u64,
}

pub struct TombstoneStore {
    entries: HashMap<Vec<u8>, Entry>,
    tombstone_count: u64,
    purge_count: u64,
    version: u64,
}

impl TombstoneStore {
    pub fn new() -> Self {
        Self { entries: HashMap::new(), tombstone_count: 0, purge_count: 0, version: 0 }
    }

    pub fn put(&mut self, key: &[u8], value: &[u8]) -> u64 {
        self.version += 1;
        self.entries.insert(key.to_vec(), Entry {
            key: key.to_vec(), value: value.to_vec(), state: EntryState::Alive, version: self.version
        });
        self.version
    }

    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        self.entries.get(key).filter(|e| e.state == EntryState::Alive).map(|e| e.value.as_slice())
    }

    pub fn get_any(&self, key: &[u8]) -> Option<&Entry> { self.entries.get(key) }

    /// Tombstone: mark as deleted but keep for replication.
    pub fn delete(&mut self, key: &[u8]) -> bool {
        if let Some(entry) = self.entries.get_mut(key) {
            if entry.state == EntryState::Alive {
                entry.state = EntryState::Tombstoned;
                self.tombstone_count += 1;
                self.version += 1;
                return true;
            }
        }
        false
    }

    /// Purge: remove tombstoned entries older than watermark.
    pub fn purge(&mut self, watermark_version: u64) -> usize {
        let before = self.entries.len();
        self.entries.retain(|_, e| {
            !(e.state == EntryState::Tombstoned && e.version <= watermark_version)
        });
        let purged = before - self.entries.len();
        self.purge_count += purged as u64;
        purged
    }

    /// Compact: remove all tombstoned entries.
    pub fn compact(&mut self) -> usize {
        let before = self.entries.len();
        self.entries.retain(|_, e| e.state != EntryState::Tombstoned);
        let removed = before - self.entries.len();
        self.purge_count += removed as u64;
        self.tombstone_count = self.entries.values().filter(|e| e.state == EntryState::Tombstoned).count() as u64;
        removed
    }

    pub fn state_of(&self, key: &[u8]) -> EntryState {
        self.entries.get(key).map(|e| e.state).unwrap_or(EntryState::Purged)
    }

    pub fn alive_count(&self) -> usize { self.entries.values().filter(|e| e.state == EntryState::Alive).count() }
    pub fn tombstone_count(&self) -> u64 { self.tombstone_count }
    pub fn purge_count(&self) -> u64 { self.purge_count }
    pub fn version(&self) -> u64 { self.version }
}

impl Default for TombstoneStore { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_get() {
        let mut store = TombstoneStore::new();
        store.put(b"key", b"value");
        assert_eq!(store.get(b"key"), Some(&b"value"[..]));
    }

    #[test]
    fn test_delete_tombstones() {
        let mut store = TombstoneStore::new();
        store.put(b"k", b"v");
        store.delete(b"k");
        assert_eq!(store.state_of(b"k"), EntryState::Tombstoned);
        assert!(store.get(b"k").is_none()); // tombstoned, not readable
    }

    #[test]
    fn test_purge() {
        let mut store = TombstoneStore::new();
        store.put(b"k1", b"v1");
        let v = store.version();
        store.delete(b"k1");
        let purged = store.purge(v);
        assert_eq!(purged, 1);
        assert_eq!(store.state_of(b"k1"), EntryState::Purged);
    }

    #[test]
    fn test_compact() {
        let mut store = TombstoneStore::new();
        store.put(b"a", b"1");
        store.put(b"b", b"2");
        store.delete(b"a");
        let removed = store.compact();
        assert_eq!(removed, 1);
        assert_eq!(store.alive_count(), 1);
    }

    #[test]
    fn test_double_delete() {
        let mut store = TombstoneStore::new();
        store.put(b"k", b"v");
        assert!(store.delete(b"k"));
        assert!(!store.delete(b"k")); // already tombstoned
    }

    #[test]
    fn test_missing_key() {
        let store = TombstoneStore::new();
        assert_eq!(store.state_of(b"missing"), EntryState::Purged);
    }

    #[test]
    fn test_overwrite_revives() {
        let mut store = TombstoneStore::new();
        store.put(b"k", b"v1");
        store.delete(b"k");
        store.put(b"k", b"v2"); // overwrites tombstone
        assert_eq!(store.state_of(b"k"), EntryState::Alive);
        assert_eq!(store.get(b"k"), Some(&b"v2"[..]));
    }

    #[test]
    fn test_version_increments() {
        let mut store = TombstoneStore::new();
        store.put(b"k", b"v");
        let v1 = store.version();
        store.delete(b"k");
        assert!(store.version() > v1);
    }
}
