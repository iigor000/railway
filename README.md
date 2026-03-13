# Railway Cargo System

A high-performance Rust solution for determining which cargo types can reach each station in a railway network.

## Problem Statement

A railway system consists of multiple stations connected by one-way tracks. Each station has:

- **Unload type**: Cargo that is removed when a train arrives
- **Load type**: Cargo that is added before the train departs

All trains start from the same initial station carrying no cargo and can follow any valid route through the network. The amount of cargo does not matter.

### Goal

For each station, determine which cargo types **might** be present when a train arrives. A cargo type is considered possible if there exists at least one valid route from the initial station that brings it there.

## Algorithm Overview

The solution uses a **Fixpoint Iteration Algorithm** (also called worklist algorithm), which is more efficient than traditional BFS approaches.

### How It Works: Step-by-Step

```
Input: Graph with stations, cargo loads/unloads, starting station

1. Initialize:
   - cargo_at_station[start_station] = 0 (empty set)
   - worklist = [start_station]

2. While worklist is not empty:
   a. Pop station from worklist
   b. Unload station's cargo type from the train
   c. Load station's cargo type onto the train
   d. For each neighboring station:
      - Merge incoming cargo with existing cargo
      - If cargo changed, add neighbor to worklist
      - (If cargo didn't change, skip it - no need to reprocess)

3. Return cargo_at_station (which cargo types reach each station)
```

### Example Walkthrough

**Network:**

```
Station 0 (load: cargo 1)  →  Station 1 (unload: cargo 1, load: cargo 2)  →  Station 2
```

**Execution:**

| Step | Station | Cargo On Train | Action                     | Neighbors | Change?                 |
| ---- | ------- | -------------- | -------------------------- | --------- | ----------------------- |
| 1    | 0       | ∅ (empty)      | Load cargo 1 → {1}         | 1         | Yes → add 1 to worklist |
| 2    | 1       | {1}            | Unload 1 → ∅, Load 2 → {2} | 2         | Yes → add 2 to worklist |
| 3    | 2       | {2}            | (no load/unload)           | (none)    | No action               |

**Result:**

- Station 0: {} (arrives empty)
- Station 1: {1} (arrives with cargo 1)
- Station 2: {2} (arrives with cargo 2)

## Key Optimizations

### 1. Bitset Representation

Instead of storing cargo sets as collections, we use a single `u64` where each bit represents one cargo type:

```
Cargo state example:  0b101 (binary) = Cargo types 0 and 2 are present

Operations:
- Add cargo 5:    cargo |= 1u64 << 5    (set bit 5)
- Remove cargo 3: cargo &= !(1u64 << 3) (clear bit 3)
- Check cargo 2:  (cargo & (1u64 << 2)) != 0
```

**Benefits:**

- 8 bytes instead of 48+ bytes per set
- O(1) cloning (just copy u64)
- O(1) comparison (u64 == u64)
- O(1) hashing (perfect hash for visited tracking)
- Supports up to 64 cargo types

## Building & Running

### Build

```bash
cargo build --release
```

### Run

```bash
cargo run --release
```

### Input Format

```
S T
station_id1 unload_cargo1 load_cargo1
station_id2 unload_cargo2 load_cargo2
...
from_station to_station
from_station to_station
...
starting_station_id
```

### Example Input

```
3 2
0 99 1
1 1 2
2 2 99
0 1
1 2
0
```

**Output:**

```
Cargo types that can arrive at each station:
Station 0: (no cargo)
Station 1: [1]
Station 2: [2]
```

## Testing

Run the comprehensive test suite:

```bash
cargo test
```

The tests cover:

- Simple linear paths (0 → 1 → 2)
- Branching paths (one station → multiple)
- Cargo accumulation (multiple cargo types on same train)
- Cycle handling (graphs with loops)
- Cargo unloading verification
- Isolated stations

All tests pass with the optimized fixpoint iteration algorithm.

## Performance Characteristics

### Time Complexity

- **Best case**: O(T) - Direct acyclic graph
- **Typical case**: O(S × log(S) × C_max) - Most real networks
- **Worst case**: O(S × 64) - Each station processes at most 64 times (one per cargo bit)

Where:

- S = number of stations
- T = number of tracks (edges)
- C_max = number of distinct cargo types (≤ 64)

### Space Complexity

- O(S + T) for the graph
- O(S) for cargo tracking
- **Total**: O(S + T)

## Implementation Details

### Core Algorithm (Fixpoint Iteration)

Located in `calculate()` function:

```rust
fn calculate(
    load: &HashMap<usize, usize>,
    unload: &HashMap<usize, usize>,
    graph: &HashMap<usize, Vec<usize>>,
    start_station: usize,
) -> HashMap<usize, u64>
```

### Main Components

1. **Graph representation**: HashMap of adjacency lists
2. **Cargo tracking**: HashMap mapping stations to bitset cargo states
3. **Worklist management**: VecDeque for efficient queue operations

## Limitations & Extensions

### Current Limitations

- Supports up to 64 cargo types (due to u64 bitset)
- All edges have equal weight (no capacity/cost)

### Possible Extensions

- Use `u128` or `[u64; N]` for more cargo types
- Add cargo quantities with different data structures
- Implement weighted graphs for optimization problems
- Cache results for multiple queries
- Parallel processing of independent subgraphs

## Example: Complex Network

```
Input:
5 5
0 99 1
1 1 2
2 2 3
3 3 4
4 4 99
0 1
1 2
2 3
3 4
0 2
0
```

**Graph:**

```
0 → 1 → 2 → 3 → 4
└─────────→ 2 ─┘
```

**Execution:**

1. Process 0: cargo = {1}, propagate to 1 and 2
2. Process 1: cargo = {1} → unload 1 → load 2 → cargo = {2}, propagate to 2
3. Process 2a (from 0): cargo = {1} → cargo = {3}, propagate to 3
4. Process 2b (from 1): cargo = {2} → unload 2 → load 3 → cargo = {3}, already have it
5. Process 3: cargo = {3} → unload 3 → load 4 → cargo = {4}, propagate to 4
6. Process 4: cargo = {4} → done

**Result:**

- Station 0: {} (arrives empty)
- Station 1: {1}
- Station 2: {1, 3}
- Station 3: {3}
- Station 4: {4}
