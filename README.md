
---

#  Time-Series Buffer Manager 

Traditional database buffer managers (like those in Postgres or MySQL) are optimized for general-purpose workloads using **Least Recently Used (LRU)** policies. However, Time-Series data—characterized by high-velocity ingestion and temporal queries—requires a specialized approach.

This project implements a **Tiered-Temporal Buffer Manager** in Rust, designed to handle "Line-Speed" ingestion while protecting "Hot" (recent) data from being evicted by large historical scans.

---

## 🏗️ Architectural Overview

The system moves away from a monolithic cache to a **Three-Tier Architecture**:

1. **Tier 0: Ingestion Ring Buffer (The "Now" Window)**
* **Mechanism:** Lock-free `ArrayQueue`.
* **Purpose:** Absorbs millions of writes per second without blocking the storage engine. It provides a "zero-copy" landing zone for incoming sensor or financial data.


2. **Tier 1: Temporal Cache (Semantic-Aware)**
* **Mechanism:** Concurrent Hash Map (`DashMap`) with **SLRU-TS** replacement.
* **Purpose:** Keeps data from the last  window (e.g., 5 minutes) in memory. Unlike LRU, it uses a **Decay Function** to weigh relevance.


3. **Tier 2: Compressed Cold Pool**
* **Mechanism:** Delta-Delta encoding / Gorilla Compression.
* **Purpose:** Instead of evicting to disk, pages are compressed. This allows for a 10x higher "effective RAM capacity."



---

## 🧠 Key Algorithm: SLRU-TS

**Segmented Least Recently Used - Time Series** (SLRU-TS) solves the "Scan Pollution" problem. In a standard LRU, a query for 1 year of data would wipe out the "Hot" data from the last 10 seconds.

Our implementation uses a **Decay Function** to calculate a page's importance score ():

* ** (Frequency):** How many times the page was accessed.
* ** (Temporal Distance):** The difference between "Now" and the page's timestamp.

A page with a high frequency but an old timestamp will eventually have a lower score than a brand-new page, allowing for natural "aging out" of data.

---

## 🛠️ Implementation Details (Rust)

### High-Concurrency with `DashMap` & `Crossbeam`

Standard Rust `Mutex<HashMap>` creates a bottleneck where only one thread can access the buffer at a time. This project uses:

* **`crossbeam::queue::ArrayQueue`**: For O(1) lock-free ingestion.
* **`dashmap::DashMap`**: For sharded concurrency, allowing multiple CPU cores to query the buffer simultaneously.

### Memory Safety & Performance

The implementation utilizes Rust's **Ownership model** to ensure that once a page is moved from Tier 0 to Tier 1, no data races can occur. The use of `[u8; 4096]` (Page Size) ensures memory is aligned for **SIMD (Single Instruction, Multiple Data)** operations, which are critical for high-speed temporal scanning.

---

## 🚀 Getting Started

### Prerequisites

Ensure you have the Rust toolchain installed.

```bash
# Add required dependencies
cargo add crossbeam dashmap

```

### Running the System

```bash
# Run the demonstration (Ingestion -> Promotion -> Eviction)
cargo run

# Run the temporal scoring and logic tests
cargo test

```

