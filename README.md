# oxide-tombstone

*Tombstone-based deletion for distributed GPU data. Don't delete — mark deleted, propagate the mark, garbage collect after acknowledgment. Because in a distributed system, free isn't free until everyone agrees.*

## Why This Exists

On a single device, freeing memory is simple: call `free()`. In a distributed GPU system, other devices might have cached references to your data. If you free it while they're reading it, you get use-after-free corruption. If you wait for everyone to stop reading, you wait forever.

Tombstone deletion solves this: instead of freeing memory, you mark it with a tombstone (-1). The tombstone propagates to all devices. Once every device acknowledges the tombstone, you know it's safe to free. No coordination overhead during normal reads — only during deletion.

## Architecture

```
Device A: Data[key=42] = value
                ↓ tombstone(42)
Device A: Data[key=42] = TOMBSTONE (-1)
                ↓ propagate
Device B: sees tombstone, sends ACK
Device C: sees tombstone, sends ACK
                ↓ all ACKs received
Garbage Collector: free(key=42)
```

### Key Types

- **`Tombstone`** — Deletion marker with timestamp, origin device, and acknowledgment bitmap.
- **`TombstoneMap`** — Tracks which keys are tombstoned. O(1) lookup: is this key alive or dead?
- **`GarbageCollector`** — Reclaims tombstoned data only after all devices acknowledge. Configurable sweep interval.
- **`TombstoneStats`** — Active tombstones, pending ACKs, bytes reclaimable, oldest uncollected tombstone.

## Usage

```rust
use oxide_tombstone::*;

let mut map = TombstoneMap::new(3); // 3 devices in cluster

// Insert data
map.insert(42, my_data);

// Delete via tombstone
map.tombstone(42, DeviceId(0)); // Device 0 initiates deletion

// Other devices acknowledge
map.acknowledge(42, DeviceId(1));
map.acknowledge(42, DeviceId(2));

// Now safe to garbage collect
let stats = map.gc();
println!("Reclaimed {} bytes", stats.bytes_reclaimed);

// Check if key is usable
if map.is_alive(42) {
    // Key is available for new data
}
```

## The Deeper Idea

Tombstones are the distributed systems equivalent of grace periods. The same pattern appears in:
- CRDTs: mark-then-merge semantics
- `oxide-epoch`: Grace period before memory reclamation
- `agent-semiosis`: Sign death before replacement (the old sign isn't removed until all agents have seen the replacement)
- Git: Deleted branches still exist as unreachable objects until `git gc`

The ternary state of a key (Alive/+1, Tombstoned/0, Reclaimed/-1) is the same lifecycle as every other resource in the ecosystem. Deletion is a process, not an event.

## Related Crates

- `oxide-epoch` — Epoch-based reclamation (complementary approach to safe freeing)
- `oxide-journal` — Write-ahead log (tombstones are journaled for crash recovery)
- `oxide-federation` — Cross-cluster tombstone propagation
- `smartcrdt` — CRDT-based conflict resolution using tombstone semantics
