# Oxide Tombstone

**Oxide Tombstone** provides tombstone-based deletion for GPU distributed data with ternary entry states — `+1` (alive), `0` (tombstoned), `-1` (purged) — implementing lazy deletion, compaction, and garbage collection for concurrent data stores.

## Why It Matters

Distributed key-value stores cannot synchronously delete data that may be replicated across nodes or actively read by concurrent threads. Tombstones solve this: a deleted key is marked as "tombstoned" (invisible to reads) but not physically removed. Later, a compaction pass purges tombstones once all replicas acknowledge the deletion. This is exactly how Cassandra, LevelDB, and RocksDB handle deletion. Oxide Tombstone brings this pattern to GPU memory management with version tracking and ternary lifecycle states.

## How It Works

### Entry Lifecycle

```
put(key, value)         → Entry { state: Alive (+1), version: V }
tombstone(key)          → Entry { state: Tombstoned (0), version: V+1 }
purge(tombstone_key)    → Entry { state: Purged (-1) }  // physical removal
```

State transitions: `Alive → Tombstoned → Purged` (one-way). No transition can revive a purged entry.

### Visibility Semantics

```
get(key):
  if entry.state == Alive (+1):
    return Some(entry.value)
  else:
    return None    // Tombstoned and Purged are invisible
```

Read cost: **O(1)** via HashMap lookup.

### Versioning

Each write operation increments a global version counter:

```
version: u64 (monotonic)

put(key, value)   → version += 1, entry.version = version
tombstone(key)    → version += 1, entry.version = version
```

Versioning enables MVCC (Multi-Version Concurrency Control): readers at version V see only entries with `version ≤ V` and `state = Alive`.

### Compaction

The compaction pass physically removes Purged entries and optionally reclaims space from old Tombstoned entries:

```
compact():
  for each entry where state == Purged:
    remove from HashMap    // O(1) per entry
  for each entry where state == Tombstoned and age > threshold:
    mark as Purged → remove
```

Compaction cost: **O(N)** where N = total entries. Should run periodically during low-load periods.

### Conservation Tracking

```
alive_count      = count(state == Alive)
tombstone_count  = count(state == Tombstoned)
purge_count      = count(state == Purged)

conservation_check: alive + tombstoned + purged = total_entries
```

The tombstone_count and purge_count are tracked as **O(1)** running counters.

## Quick Start

```rust
use oxide_tombstone::{TombstoneStore, EntryState};

let mut store = TombstoneStore::new();

let v1 = store.put(b"key1", b"value1");
store.put(b"key2", b"value2");

assert_eq!(store.get(b"key1"), Some(&b"value1"[..]));

store.tombstone(b"key1");
assert_eq!(store.get(b"key1"), None); // Tombstoned — invisible
assert_eq!(store.tombstone_count(), 1);

store.compact(); // Physically removes tombstoned entries past threshold
```

## API

| Type | Description |
|------|-------------|
| `TombstoneStore` | Key-value store with ternary entry lifecycle |
| `Entry` | key, value, state, version |
| `EntryState` | `Alive (+1)`, `Tombstoned (0)`, `Purged (-1)` |

Key methods: `put(k,v)`, `get(k)`, `tombstone(k)`, `purge(k)`, `compact()`, `version()`.

## Architecture Notes

Oxide Tombstone provides safe deletion for GPU data structures in the oxide-* stack. In γ + η = C, tombstoning is η (avoidance — marking data as deleted without risking concurrent readers) while compaction is γ (growth — reclaiming physical space for new allocations). Works with `oxide-epoch` for safe reclamation timing and `oxide-chunk` for memory pool management.

See [ARCHITECTURE.md](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md) for GPU data management architecture.

## References

1. Lakshman, A. & Malik, P. (2010). "Cassandra: A Decentralized Structured Storage System." *ACM SIGOPS*.
2. Dong, S. et al. (2017). "RocksDB: Evolution of a Key-Value Store." *VLDB*.
3. Baker, J. et al. (2011). "Megastore: Providing Scalable, Highly Available Storage." *CIDR*.

## License

MIT
