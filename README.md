# XEPAY (ticker: XEPY)

> A serious, fast-block Layer-1 blockchain written in Rust.
> **SHA3-256** as the sole cryptographic primitive.
> **~15-second target block time**, enforced by an adaptive difficulty algorithm.

![status](https://img.shields.io/badge/status-pre--testnet-yellow)
![language](https://img.shields.io/badge/language-Rust-orange)
![hash](https://img.shields.io/badge/hash-SHA3--256-green)
![block time](https://img.shields.io/badge/target%20block%20time-15s-blue)
![consensus](https://img.shields.io/badge/consensus-Proof%20of%20Work-purple)
![license](https://img.shields.io/badge/license-Apache--2.0-lightgrey)

---

## Overview

Indonesian, 28/10/2026 🇮🇩,
**XEPAY** (ticker: **XEPY**) is a Layer-1 proof-of-work blockchain engineered
around three design commitments:

1. **A fast, predictable block time.** XEPAY targets an average of **15 seconds
   per block**, sustained by an adaptive difficulty algorithm rather than a
   fixed parameter. The chain is designed to remain responsive as hashrate
   changes, hardware evolves, and the network grows.
2. **A modern, standardized hash.** XEPAY commits to **SHA3-256** (FIPS 202,
   Keccak) as its sole hashing primitive — a deliberate choice for a
   distinct hash family, clean specifications, and a well-audited Rust
   implementation.
3. **Verifiability by construction.** Every block is a cryptographic
   commitment to its predecessor. Every hash is recomputable. Every
   validation step is deterministic. The chain does not trust peers; it
   recomputes.

XEPAY is not a fork of Bitcoin, Ethereum, or any existing network. It is an
independent implementation, and it is being built in stages toward a
production-grade Layer-1.

---

## Project Status

| Milestone | Status | Notes |
|-----------|--------|-------|
| Block structure & SHA3-256 hashing | ✅ Complete | `block.rs`, `crypto.rs` |
| Proof-of-work mining | ✅ Complete | Exponential difficulty, standard PoW loop |
| Chain linkage & validation | ✅ Complete | `blockchain.rs::is_valid` |
| Adaptive difficulty (DAA) | ✅ Complete | Interval-based; targets 15 s |
| Block-time statistics | ✅ Complete | `average_block_time_secs()` |
| Signed transactions | 🚧 In progress | `ed25519-dalek`, planned for v0.3 |
| Merkle tree | 🚧 In progress | `merkle_light`, planned for v0.3 |
| P2P networking | 📋 Planned | `tokio` / `libp2p`, v0.4 |
| Persistence | 📋 Planned | `sled`, v0.5 |
| Wallet CLI | 📋 Planned | v0.6 |
| Test suite & benchmarks | 📋 Planned | v1.0 |
| Security audit | 📋 Planned | Pre-mainnet |

> **Current state:** the consensus core (blocks, PoW, DAA, validation) is
> complete and exercised end-to-end. The networking, wallet, and
> transaction-signing layers are in active development. XEPAY is **pre-testnet**.

---

## Design Goals

### 1. Fast, predictable block production

A 15-second target block time places XEPAY in the class of modern
high-throughput chains. Bitcoin's 10-minute interval is secure but
unresponsive for interactive applications; sub-second chains trade latency for
complexity. Fifteen seconds is the engineering compromise XEPAY makes: fast
enough for practical settlement UX, slow enough for proof-of-work to remain
meaningful on commodity hardware.

### 2. Stability under hashrate change

The 15-second target is **not** a hardcoded number. XEPAY implements a
**Difficulty Adjustment Algorithm (DAA)** that observes actual block
intervals and adjusts the required hash prefix accordingly. Without a DAA, a
chain's block time drifts the moment hashrate changes. With one, block time
is a *governed* property of the network — the same principle every serious
PoW chain relies on.

### 3. Modern cryptographic primitive

SHA3-256 belongs to the Keccak family, standardized in FIPS 202. It is not
merely "SHA-256 with a different name": its sponge construction is a
fundamentally different design, giving XEPAY a distinct hash identity and
insulation from any future cryptanalytic surprises affecting SHA-2. The Rust
`sha3` crate is a well-audited, `no_std`-compatible implementation.

### 4. Verifiable by construction

XEPAY's correctness model is recomputation. A node does not ask peers whether
a block is valid; it hashes the header and checks. This is the only
validation model that scales without trust, and every design decision in the
codebase follows from it.

---

## Why 15 Seconds?

| Chain | Target block time |
|-------|-------------------|
| Bitcoin | 600 s |
| Litecoin | 150 s |
| Dogecoin | 60 s |
| **XEPAY** | **15 s** |
| BSC | ~3 s |
| Polygon PoS | ~2 s |
| Solana (slot) | ~0.4 s |

XEPAY chooses 15 seconds because:

- It is **fast enough** for interactive applications: a wallet can display
  a pending transaction as confirmed in seconds rather than minutes.
- It is **slow enough** that proof-of-work remains a meaningful Sybil
  resistance mechanism on commodity hardware.
- It is **observable** — a 20-block DAA interval equals roughly 5 minutes of
  wall-clock time, allowing continuous monitoring and feedback.
- It is **defensible** — every parameter choice in XEPAY is documented and
  justified against a concrete engineering rationale.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        xepay.rs                              │
│                   (orchestration layer)                      │
└──────────────────────────────┬──────────────────────────────┘
                               │
              ┌────────────────┼────────────────┐
              ▼                ▼                ▼
       ┌────────────┐   ┌────────────┐   ┌──────────────┐
       │  crypto.rs │   │  block.rs  │   │blockchain.rs │
       │            │   │            │   │              │
       │ SHA3-256   │◄──│  Header    │◄──│  Chain       │
       │ hashing    │   │  Block     │   │  DAA         │
       │            │   │  PoW loop  │   │  Validation  │
       │            │   │  is_valid  │   │  Statistics  │
       └────────────┘   └────────────┘   └──────────────┘
```

| Module | Responsibility |
|--------|----------------|
| `crypto.rs` | SHA3-256 hashing over serialized values |
| `block.rs` | Block and header structures, proof-of-work, per-block validation |
| `blockchain.rs` | Chain construction, difficulty adjustment, chain-wide validation, timing statistics |
| `xepay.rs` | Entry point, demonstration orchestration, reporting |

Each module has a single responsibility and is independently testable.
The layout is intentionally flat (no `src/` directory) to make the entire
consensus core readable in one sitting — a deliberate choice for auditability
during pre-testnet development.

---

## Technical Specifications

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Project name | XEPAY | Network identity |
| Ticker | XEPY | Currency symbol |
| Hash function | SHA3-256 | Modern, standardized (FIPS 202), distinct family |
| Consensus | Proof of Work | Trustless, permissionless, battle-tested |
| Target block time | 15 s | Fast UX, still PoW-honest |
| Difficulty adjustment interval | 20 blocks | ~5 min at target; responsive to hashrate |
| Adjustment rule | ±1 per interval if 2× off target | Simple integer DAA; moving average planned |
| Difficulty bounds (demo) | 1 – 8 | Clamped for prototype tractability |
| Genesis block | Fixed | Bootstrap anchor |
| Merkle structure (v0.1) | Hash of transaction list | Placeholder; real Merkle tree in v0.3 |
| Signature scheme | None (v0.1) | `ed25519-dalek` planned for v0.3 |
| Networking | None (v0.1) | `tokio` / `libp2p` planned for v0.4 |

---

## How the 15-Second Target Is Enforced

### Constants (`blockchain.rs`)

```rust
pub const TARGET_BLOCK_TIME_SECS: i64 = 15;
pub const ADJUSTMENT_INTERVAL: usize = 20;
pub const MIN_DIFFICULTY: u32 = 1;
pub const MAX_DIFFICULTY: u32 = 8;
```

### Adjustment Rule

After every `ADJUSTMENT_INTERVAL` blocks:

```
actual   = timestamp(last) - timestamp(first-in-window)
expected = (ADJUSTMENT_INTERVAL - 1) * TARGET_BLOCK_TIME_SECS

if actual * 2 < expected:   difficulty += 1   // blocks too fast
if actual     > expected*2: difficulty -= 1   // blocks too slow
```

This is an integer-step feedback controller. It is a first-generation DAA:
simple, deterministic, and easy to audit. A moving-average DAA — which
smooths convergence and resists timestamp manipulation — is scheduled for
v0.2.

### Why a DAA at All?

A fixed difficulty produces a fixed *work* requirement per block, not a fixed
*time* requirement. As hashrate rises, block time falls; as it falls, block
time rises. A DAA turns block time into a *governed property*: the network
continuously measures, compares, and corrects. This is the fundamental
property that separates a production blockchain from a demo.

---

## Getting Started

### Prerequisites

- Rust ≥ 1.70 (stable) via [rustup](https://rustup.rs)
- Cargo (bundled with rustup)
- A C linker (usually preinstalled on Linux/macOS; MSVC build tools on Windows)

Verify:

```bash
rustc --version
cargo --version
```

### Build & Run

```bash
cargo run --release
```

The demo mines 45 blocks, triggering two difficulty adjustments, and reports
per-block timing plus aggregate statistics.

### Build Only

```bash
cargo build --release
```

### Lint & Format

```bash
cargo fmt
cargo clippy -- -D warnings
```

---

## Hands-On: Observing the DAA

### Observe convergence

Increase `blocks_to_mine` in `xepay.rs`:

```rust
let blocks_to_mine: usize = 200;
```

Watch `difficulty` increase at each adjustment interval until block times
converge toward 15 s (or hit `MAX_DIFFICULTY` on fast hardware — remove the
clamp to observe true convergence).

### Change the target

Edit `TARGET_BLOCK_TIME_SECS` in `blockchain.rs` from `15` to `30`. The DAA
will target 30-second blocks; because actual is far below expected,
difficulty will drop at each adjustment.

### Verify tamper-evidence

Add this at the end of `main()`:

```rust
chain.chain[1].header.nonce = 0;
println!("After tampering, valid = {}", chain.is_valid());
```

The output flips to `false` — a block's hash is a commitment to its entire
header, so any change invalidates the hash and every downstream block.

---

## Roadmap

XEPAY is developed in explicit milestones. Each is independently testable and
auditable.

### v0.1 — Consensus core (current)
- ✅ Block structure and SHA3-256 hashing
- ✅ Proof-of-work mining and per-block validation
- ✅ Chain linkage and chain-wide validation
- ✅ Adaptive difficulty (interval-based)
- ✅ Block-time statistics

### v0.2 — DAA refinement
- 🔜 Moving-average difficulty adjustment
- 🔜 Timestamp bounds (median-time-past rule)
- 🔜 Removal of `MAX_DIFFICULTY` clamp for production convergence
- 🔜 Property tests for DAA behavior

### v0.3 — Transaction model
- 🔜 `ed25519-dalek` signature scheme
- 🔜 Signed transactions
- 🔜 Real Merkle tree via `merkle_light`
- 🔜 UTXO or account model with balance tracking

### v0.4 — Networking
- 🔜 P2P gossip (`libp2p` or `tokio`)
- 🔜 Block and transaction propagation
- 🔜 Peer discovery and handshake

### v0.5 — Persistence
- 🔜 On-disk chain storage (`sled` or `rocksdb`)
- 🔜 Recovery and reorg handling

### v0.6 — Wallet
- 🔜 CLI wallet: address generation, signing, balance queries
- 🔜 HD key derivation (BIP-32-style)

### v1.0 — Production readiness
- 🔜 Comprehensive test suite
- 🔜 Criterion benchmarks
- 🔜 Fuzzing and property tests
- 🔜 External security audit

---

## Design Decisions

**Why SHA3-256 and not SHA-256?** Both are secure. SHA3-256 belongs to a
different construction family (Keccak sponge vs. Merkle–Damgård), giving
XEPAY a distinct hash identity and insulation from any future cryptanalytic
surprises affecting SHA-2. The choice is deliberate, not cosmetic.

**Why interval-based DAA and not per-block?** Interval-based DAA is simpler
to reason about, easier to audit, and sufficient for a chain targeting 15 s
per block. A per-block DAA (DigiByte-style) trades simplicity for
responsiveness; it is a valid choice that XEPAY may adopt in v0.2.

**Why proof of work and not proof of stake?** PoW is trustless,
permissionless, and its security properties are well-understood after 15
years of production use. PoS introduces validator-set management,
finality gadgets, and slashing conditions — significant additional
complexity that XEPAY's first generation deliberately excludes.

---

## FAQ

**Q: Is XEPAY a real blockchain?**
A: Yes. The consensus core is real: blocks, PoW, chain linkage, DAA, and
   validation are complete and exercised end-to-end. What is not yet
   implemented is the surrounding infrastructure — networking, wallets,
   signing, persistence. XEPAY is a **pre-testnet Layer-1 in active
   development**, not a finished product.

**Q: Is 15 seconds really achieved?**
A: The mechanism is real; the tuning is prototype-grade. On commodity
   hardware at difficulty 8, blocks still complete in well under a second,
   so the demo hits the `MAX_DIFFICULTY` clamp before reaching 15 s. In a
   production deployment, `MAX_DIFFICULTY` is removed and the DAA converges
   naturally to the difficulty that yields 15 s on current hashrate.

**Q: What is "finality" and is it 15 seconds?**
A: Finality is irreversibility. XEPAY, like Bitcoin, provides *probabilistic*
   finality: the more blocks stacked on a transaction, the harder to reverse.
   15 seconds is the *block time*, not the finality time. A common
   conservative heuristic is 6 confirmations ≈ 90 s. Explicit finality
   gadgets are out of scope for v0.1.

**Q: Can I use XEPAY in production today?**
A: No. XEPAY is pre-testnet. It has no network, no wallet, no signatures, and
   has not been audited. Do not transfer anything of value on it.

**Q: How does XEPAY differ from other fast chains?**
A: XEPAY is an independent implementation, not a fork. Its distinguishing
   properties are (a) SHA3-256 as the sole hash, (b) a 15-second target
   enforced by DAA rather than fixed parameters, and (c) a deliberately
   minimal consensus core designed for auditability.

**Q: Can I contribute?**
A: Yes. Contributions that advance the roadmap, add tests, or fix
   correctness bugs are welcome. Open an issue to discuss before submitting
   a large PR.

---

## License

Copyright (c) 2026 **Aldianokto**.

Licensed under the **Apache License, Version 2.0**.
You may obtain a copy of the License at:

<https://www.apache.org/licenses/LICENSE-2.0>

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an **"AS IS" BASIS, WITHOUT
WARRANTIES OR CONDITIONS OF ANY KIND**, either express or implied. See the
License for the specific language governing permissions and limitations under
the License.

See the [`LICENSE`](./LICENSE) file for the full text.

---

<sub>XEPAY (XEPY) — Fast blocks. Modern hashes. Verifiable by construction.</sub>
