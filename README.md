# oxide-tombstone

Tombstone-based deletion for GPU distributed data with ternary states. {+1=alive, 0=tombstoned, -1=purged}. Lazy deletion, compaction, gc.

## Overview

# oxide-tombstone

Tombstone-based deletion for GPU distributed data.

## Architecture

This crate sits within the **five-layer Oxide Stack**:

| Layer | Crate | Role |
|-------|-------|------|
| 1 | open-parallel | Async runtime (tokio fork) |
| 2 | pincher | "Vector DB as runtime, LLM as compiler" |
| 3 | flux-core | Bytecode VM + A2A agent protocol |
| 4 | cuda-oxide | Flux→MIR→Pliron→NVVM→PTX compiler |
| 5 | cudaclaw | Persistent GPU kernels, warp consensus, SmartCRDT |

The key insight: **ternary values {-1, 0, +1} map directly to GPU compute**. They pack 16× denser than FP32, enable XNOR+popcount matmul, and conservation laws become compile-time checks.

## Stats

| Metric | Value |
|--------|-------|
| Tests | 8 |
| Lines of Code | 164 |
| Public API Surface | 15 items |
| License | Apache-2.0 |

## Installation

```toml
[dependencies]
oxide-tombstone = "0.1.0"
```

## Usage

```rust
use oxide_tombstone::*;
// See src/lib.rs tests for complete working examples
```

### Key Types

```
- pub enum EntryState { Alive = 1, Tombstoned = 0, Purged = -1 }
- pub struct Entry {
- pub struct TombstoneStore {
    pub fn new() -> Self {
    pub fn put(&mut self, key: &[u8], value: &[u8]) -> u64 {
    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
    pub fn get_any(&self, key: &[u8]) -> Option<&Entry> { self.entries.get(key) }
    pub fn delete(&mut self, key: &[u8]) -> bool {
    pub fn purge(&mut self, watermark_version: u64) -> usize {
    pub fn compact(&mut self) -> usize {
```

## Design Philosophy

This crate uses **ternary algebra** (Z₃) where every value is {-1, 0, +1}:

- **+1** → positive signal (healthy, allocated, converged, ready)
- **0** → neutral (pending, balanced, monitoring, degraded)
- **-1** → negative signal (failed, free, diverged, overloaded)

This isn't arbitrary — ternary is the natural encoding for:
1. **BitNet b1.58** (Microsoft) — ternary neural networks at 60% less power
2. **GPU warp voting** — hardware ballot instructions return ternary consensus
3. **Conservation laws** — {-1, 0, +1} preserves quantity (what goes in must come out)

## Testing

```bash
git clone https://github.com/SuperInstance/oxide-tombstone.git
cd oxide-tombstone
cargo test
```

## License

Apache-2.0
