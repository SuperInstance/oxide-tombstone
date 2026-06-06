# oxide-tombstone

*Tombstone-based deletion for distributed GPU data. Mark deleted, propagate, garbage collect.*

## Why This Exists

In distributed GPU computing, you can't just free memory — other devices might have pointers to it. Tombstone deletion marks data as deleted (-1), propagates that state across devices, and only garbage collects when all devices acknowledge.

## Architecture

### Key Types

Tombstone (deletion marker with timestamp), TombstoneMap (tracks deleted keys), GarbageCollector (reclaims tombstoned data after acknowledgment)

### State Machine

```
+1 (Active/Arrived/Allocated)
  ↓ transition event
 0 (Grace/InTransit/Fragmented)
  ↓ transition event
-1 (Reclaimable/NotStarted/Free)
```

## Usage

```rust
use oxide_tombstone::*;

let mut map = TombstoneMap::new(); map.insert(key, value); map.tombstone(key); map.acknowledge(device_id, key); map.gc(); // only after all acks
```

## The Deeper Idea

Tombstones are the distributed systems equivalent of grace periods. The same pattern appears in CRDTs (mark then merge) and in the agent-semiosis crate (sign death before replacement).

## Related Crates

- `oxide-fleet` — Fleet-level orchestration using these primitives
- `oxide-sandbox` — Safe execution environment built on oxide primitives
- `oxide-slotmap` — Slot-based memory management (complementary allocation strategy)
