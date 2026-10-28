# XEPAY (ticker: XEPY)

> A serious, fast-block Layer-1 blockchain written in Rust.
> **SHA3-256** as the sole cryptographic primitive.
> **~15-second target block time**, enforced by an adaptive difficulty algorithm.

![status](https://img.shields.io/badge/status-v0.2%20pre--testnet-yellow)
![language](https://img.shields.io/badge/language-Rust-orange)
![hash](https://img.shields.io/badge/hash-SHA3--256-green)
![block time](https://img.shields.io/badge/target%20block%20time-15s-blue)
![consensus](https://img.shields.io/badge/consensus-Proof%20of%20Work-purple)
![signatures](https://img.shields.io/badge/signatures-ed25519-lightblue)
![license](https://img.shields.io/badge/license-Apache--2.0-lightgrey)

---

## Overview

Indonesian, 28/10/2026 🇮🇩,

**XEPAY** (ticker: **XEPY**) is a Layer-1 proof-of-work blockchain engineered
around three design commitments:

1. **A fast, predictable block time.** XEPAY targets an average of **15 seconds
   per block**, sustained by a moving-average difficulty adjustment algorithm
   rather than a fixed parameter. The chain remains responsive as hashrate
   changes, hardware evolves, and the network grows.
2. **A modern, standardized hash.** XEPAY commits to **SHA3-256** (FIPS 202,
   Keccak) as its sole hashing primitive — a deliberate choice for a distinct
   hash family, clean specifications, and a well-audited Rust implementation.
3. **Verifiability by construction.** Every block is a cryptographic
   commitment to its predecessor. Every hash is recomputable. Every validation
   step is deterministic. The chain does not trust peers; it recomputes.

XEPAY is not a fork of Bitcoin, Ethereum, or any existing network. It is an
independent implementation, and it is being built in stages toward a
production-grade Layer-1.

---

## Project Status — v0.2

| Milestone | Status | Notes |
|-----------|--------|-------|
| Block structure & SHA3-256 hashing | Complete | `block.rs`, `crypto.rs` |
| Proof-of-work mining | Complete | Exponential difficulty, standard PoW loop |
| Chain linkage & validation | Complete | `blockchain.rs::is_valid` |
| Moving-average DAA | Complete | Scales difficulty smoothly toward 15 s |
| Median-Time-Past (MTP) | Complete | Rejects timestamps ≤ median of last 11 blocks |
| Real binary Merkle tree | Complete | `crypto.rs::merkle_root` over per-tx hashes |
| Signed transactions (ed25519) | Complete | `transaction.rs` |
| Per-sender replay protection | Complete | Monotonic `nonce` per sender |
| Mempool with fee selection | Complete | `mempool.rs::drain_highest_fee` |
| Block reward with halving | Complete | 50 XEPY → halves every 210,000 blocks |
| Coinbase transaction | Complete | First tx in every block, exempt from signature |
| Balance tracking | Complete | `Blockchain::balances` |
| On-disk persistence (sled) | Complete | `storage/storage.rs` |
| CLI wallet | Complete | `xepay wallet new \| from \| info` |
| P2P networking | Planned | `tokio` / `libp2p`, v0.3 |
| Light client (SPV) | Planned | Merkle inclusion proofs, v0.4 |
| JSON-RPC + explorer | Planned | v0.5 |
| Test suite & benchmarks | In progress | Unit tests present; integration/bench planned |
| Security audit | Planned | Pre-mainnet |

> **Current state:** the consensus core plus the transaction model,
> persistence, and CLI wallet are complete and exercised end-to-end. P2P
> networking and light-client support are scheduled for the next milestones.
> XEPAY remains **pre-testnet**.

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
**moving-average DAA**: every 20 blocks it measures actual elapsed time across
the window and scales difficulty by `target / actual`. Without a DAA, a
chain's block time drifts the moment hashrate changes. With one, block time
is a *governed property* of the network — the same principle every serious
PoW chain relies on.

### 3. Modern cryptographic primitive

SHA3-256 belongs to the Keccak family, standardized in FIPS 202. Its sponge
construction is a fundamentally different design from SHA-2, giving XEPAY a
distinct hash identity and insulation from any future cryptanalytic surprises
affecting SHA-2. The Rust `sha3` crate is a well-audited, `no_std`-compatible
implementation.

### 4. Real cryptographic ownership

Every value transfer on XEPAY is a **signed transaction**. The `ed25519`
signature scheme provides fast verification, compact 64-byte signatures, and
strong security guarantees. Transactions are replay-protected by a per-sender
monotonic `nonce`, and validated against current balances before inclusion.

### 5. Verifiable by construction

XEPAY's correctness model is recomputation. A node does not ask peers whether
a block is valid; it hashes the header and checks. With a real binary Merkle
tree, each transaction's inclusion is independently provable — the foundation
for lightweight (SPV) clients in v0.4.

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

- It is **fast enough** for interactive applications: a wallet can display a
  pending transaction as confirmed in seconds rather than minutes.
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
│              (orchestration + CLI dispatch)                  │
└──────────────────────────────┬──────────────────────────────┘
                               │
        ┌──────────┬───────────┼───────────┬──────────┐
        ▼          ▼           ▼           ▼          ▼
   ┌────────┐ ┌────────┐ ┌──────────┐ ┌────────┐ ┌────────┐
   │crypto  │ │trans-  │ │  block   │ │  chain │ │  cli   │
   │        │ │ action │ │          │ │        │ │        │
   │SHA3-256│ │signed  │ │Header    │ │DAA     │ │wallet  │
   │Merkle  │ │ed25519 │ │PoW       │ │MTP     │ │new     │
   │        │ │nonce   │ │is_valid  │ │reward  │ │from    │
   │        │ │        │ │coinbase  │ │balance │ │info    │
   └────────┘ └────────┘ └──────────┘ └────────┘ └────────┘
        ▲          ▲           ▲           ▲          ▲
        └──────────┴───────────┴───────────┴──────────┘
                               │
                    ┌──────────┴──────────┐
                    ▼                     ▼
              ┌──────────┐          ┌──────────┐
              │ mempool  │          │ storage/ │
              │          │          │          │
              │fee-sorted│          │  sled    │
              │dedup     │          │  verify  │
              └──────────┘          └──────────┘
```

| Module | Responsibility |
|--------|----------------|
| `crypto.rs` | SHA3-256 hashing, binary Merkle root construction |
| `transaction.rs` | Signed `Transaction`, ed25519 signing/verification |
| `wallet.rs` | In-memory wallet: keypair, address, secret export |
| `mempool.rs` | Pending transactions, fee-sorted selection, replay dedup |
| `block.rs` | Block/header structures, PoW, coinbase, per-block validation |
| `blockchain.rs` | Chain, moving-average DAA, MTP, rewards, balances |
| `storage/storage.rs` | On-disk persistence (sled) with re-verification on load |
| `cli.rs` | CLI wallet subcommands |
| `xepay.rs` | Entry point; dispatches to demo or CLI |

---

## Technical Specifications

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Project name | XEPAY | Network identity |
| Ticker | XEPY | Currency symbol |
| Base unit | XEPYsat | 1 XEPY = 100,000,000 XEPYsat |
| Hash function | SHA3-256 | Modern, standardized (FIPS 202) |
| Signature scheme | ed25519 | Fast, compact, well-audited |
| Consensus | Proof of Work | Trustless, permissionless, battle-tested |
| Target block time | 15 s | Fast UX, still PoW-honest |
| Difficulty adjustment interval | 20 blocks | ~5 min at target; responsive to hashrate |
| Difficulty adjustment rule | Moving average | `difficulty *= target / actual` |
| Difficulty floor | 1 | Minimum required prefix |
| Initial block reward | 50 XEPY | Bitcoin-style cadence |
| Halving interval | 210,000 blocks | Reward halves at each interval |
| Merkle tree | Binary, no dup-last | Avoids Bitcoin's CVE-2012-2459 class |
| MTP window | 11 blocks | Standard median-time-past |
| Replay protection | Per-sender nonce | Monotonic, validated on inclusion |
| Mempool cap | 10,000 txs | Bounded memory |
| Persistence | sled | Embedded KV, crash-safe |

---

## How the 15-Second Target Is Enforced

### Constants (`blockchain.rs`)

```rust
pub const TARGET_BLOCK_TIME_SECS: i64 = 15;
pub const ADJUSTMENT_INTERVAL: usize = 20;
pub const MIN_DIFFICULTY: u32 = 1;
pub const MTP_WINDOW: usize = 11;
```

### Adjustment Rule

Every `ADJUSTMENT_INTERVAL` blocks:

```
elapsed  = timestamp(last) - timestamp(first-in-window)
actual   = elapsed / (ADJUSTMENT_INTERVAL - 1)
ratio    = TARGET_BLOCK_TIME_SECS / max(actual, 0.001)
new_diff = round(difficulty * ratio), clamped to >= MIN_DIFFICULTY
```

This is a multiplicative controller. Compared to integer-step DAA, it
converges faster and is less sensitive to transient variance. It still
exposes a known weakness — a sustained hashrate drop can overshoot downward —
which the v0.3 roadmap addresses with an EMA-based variant and
outlier-resistant windows.

### Why a DAA at All?

A fixed difficulty produces a fixed *work* requirement per block, not a fixed
*time* requirement. As hashrate rises, block time falls; as it falls, block
time rises. A DAA turns block time into a *governed property*: the network
continuously measures, compares, and corrects.

---

## Transaction Model

Every value transfer on XEPAY is a `Transaction` with the following fields:

| Field | Type | Notes |
|-------|------|-------|
| `from` | string | Hex-encoded ed25519 public key; empty for coinbase |
| `to` | string | Hex-encoded ed25519 public key |
| `amount` | u64 | Value in XEPYsat |
| `fee` | u64 | Miner fee in XEPYsat |
| `nonce` | u64 | Per-sender monotonic counter (replay protection) |
| `signature` | string | Hex-encoded 64-byte ed25519 signature |

### Sighash

The sighash is a SHA3-256 over the serialized tuple
`(from, to, amount, fee, nonce)`. `signature` itself is excluded, so signing
and verifying are symmetric.

### Coinbase

The first transaction in every block is the coinbase:

- `from` is empty.
- `signature` is empty.
- `amount = block_reward + sum(fees)`.
- `to` is the miner's address.
- Coinbase is exempt from signature verification by design.

### Mempool

The mempool holds verified, unmined transactions. When mining, transactions
are selected in descending `fee` order, bounded by a per-block limit.
Duplicates (same `from` and `nonce`) are rejected, which prevents double
inclusion of a replayed transaction.

### Balance Tracking

`Blockchain::balances` maintains a `HashMap<address, u64>`. Every block update
applies `-amount - fee` to the sender and `+amount` to the recipient. This is
an account model, not a UTXO model; a UTXO migration is out of scope for
v0.2.

---

## Persistence

Chains are persisted with `sled`, an embedded crash-safe key-value store.
Blocks are keyed by big-endian height and stored as bincode blobs.

On `restore_chain`, every block is deserialized and re-verified through
`Blockchain::is_valid` before being trusted. Corrupted or tampered on-disk
state is rejected, not silently accepted.

---

## Getting Started

### Prerequisites

- Rust ≥ 1.70 (stable) via [rustup](https://rustup.rs)
- Cargo (bundled with rustup)
- A C linker (usually preinstalled on Linux/macOS; MSVC build tools on Windows)

### Build & Run

```bash
# Demo mode (default)
cargo run --release -- run

# CLI wallet
cargo run --release -- wallet new
cargo run --release -- wallet from <secret_hex>
cargo run --release -- wallet info <secret_hex>
```

### Build Only

```bash
cargo build --release
```

### Test

```bash
cargo test
```

### Lint & Format

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
```

---

## Hands-On: Observing the DAA

### Observe convergence

Increase the number of blocks mined in `xepay.rs` by calling `add_block` in a
loop. Watch difficulty rise at each adjustment until block times converge
toward 15 s (or hit the practical ceiling of your hardware).

### Change the target

Edit `TARGET_BLOCK_TIME_SECS` in `blockchain.rs` from `15` to `30`. The DAA
will target 30-second blocks; because actual is far below expected, difficulty
will drop at each adjustment.

### Verify tamper-evidence

```rust
let mut b = chain.chain[1].clone();
b.transactions[0].amount += 1_0000_0000;
assert!(!b.is_valid()); // coinbase amount change breaks the merkle root
```

### Verify replay protection

```rust
let tx_a = Transaction::new_signed(&alice.signing_key(), &bob.address(), 100, 1, 0);
let tx_b = Transaction::new_signed(&alice.signing_key(), &carol.address(), 100, 1, 0);
// Both use nonce = 0, so the second is a replay and must be rejected
assert!(!chain.validate_transactions(&[tx_a, tx_b]));
```

---

## CLI Wallet

```bash
# Create a fresh wallet
cargo run --release -- wallet new
# Output:
#   Address:    <64 hex chars>
#   Secret hex: <64 hex chars>

# Recover a wallet from a secret hex
cargo run --release -- wallet from <secret_hex>

# Inspect a wallet
cargo run --release -- wallet info <secret_hex>
# Output:
#   Address:    <64 hex chars>
#   Next nonce: 0
```

The secret hex is the raw 32-byte ed25519 signing key. **Anyone with it
controls the funds.** For v0.2, secrets are stored in plaintext in memory;
encrypted at-rest storage is scheduled for v0.3.

---

## Roadmap

XEPAY is developed in explicit milestones. Each is independently testable and
auditable.

### v0.1 — Consensus core (done)
- Block structure and SHA3-256 hashing
- Proof-of-work mining and per-block validation
- Chain linkage and chain-wide validation
- Interval-based adaptive difficulty
- Block-time statistics

### v0.2 — Transaction model & persistence (current)
- ed25519-dalek signature scheme
- Signed transactions with per-sender nonce replay protection
- Real binary Merkle tree over per-transaction hashes
- Mempool with fee-based selection and dedup
- Block reward with Bitcoin-style halving (210,000 blocks)
- Coinbase transaction in every block
- Median-Time-Past timestamp rule
- Moving-average difficulty adjustment
- On-disk persistence via sled, with re-verification on load
- CLI wallet (`xepay wallet new | from | info`)
- Balance tracking for transaction validation

### v0.3 — Networking (planned)
- P2P gossip over TCP (`tokio`)
- Peer discovery and handshake
- Block and transaction propagation
- Reorg handling and fork choice
- Encrypted wallet secrets at rest
- EMA-based DAA with outlier-resistant windows

### v0.4 — Light client (planned)
- SPV proofs using Merkle inclusion paths
- Wallet without full chain
- Bloom filters for transaction filtering

### v0.5 — API & tooling (planned)
- JSON-RPC server
- Block explorer web UI
- Prometheus metrics endpoint

### v1.0 — Production readiness (planned)
- Comprehensive test suite (unit, integration, property, fuzz)
- Criterion benchmarks
- External security audit
- Mainnet deployment plan

---

## Design Decisions

**Why not `src/main.rs`?** The consensus core is small and flat by design.
`[[bin]] path = "xepay.rs"` plus `#[path = "..."] mod ...;` keeps the entire
core readable in one sitting — a deliberate choice for auditability during
pre-testnet development. `storage/storage.rs` demonstrates the same technique
for subdirectories. In v1.0, the code will be reorganized into `src/` with
proper module boundaries.

**Why SHA3-256 and not SHA-256?** Both are secure. SHA3-256 belongs to a
different construction family (Keccak sponge vs. Merkle–Damgård), giving XEPAY
a distinct hash identity and insulation from any future cryptanalytic
surprises affecting SHA-2. The choice is deliberate, not cosmetic.

**Why ed25519 and not secp256k1?** Ed25519 offers faster verification,
smaller keys, and simpler constant-time implementations. secp256k1's primary
advantage is Bitcoin compatibility, which is not a goal for XEPAY.

**Why binary Merkle tree without duplicate-last-leaf?** Bitcoin's
duplicate-last-leaf rule enables a known block-level malleability attack
(CVE-2012-2459). XEPAY instead promotes the odd leaf unchanged, which
eliminates the class of bug at the cost of ~1 extra branch in the tree.

**Why an account model and not UTXO?** The account model is simpler to reason
about, exposes balances directly to the wallet CLI, and is the natural fit for
a chain with per-sender nonces. A UTXO migration remains possible but is out
of scope for v0.2.

**Why proof of work and not proof of stake?** PoW is trustless,
permissionless, and its security properties are well-understood after 15 years
of production use. PoS introduces validator-set management, finality gadgets,
and slashing conditions — significant additional complexity that XEPAY's first
generation deliberately excludes.

---

## FAQ

**Q: Is XEPAY a real blockchain?**
A: Yes. The consensus core, signed transaction model, mempool, persistence
   layer, and CLI wallet are all real and exercised end-to-end. What is not
   yet implemented is networking (v0.3) and light-client support (v0.4).
   XEPAY is a **pre-testnet Layer-1 in active development**.

**Q: Is 15 seconds really achieved?**
A: The mechanism is real; the tuning depends on hashrate. On commodity
   hardware, the demo reaches block times well under 15 s at modest
   difficulty. In a production deployment, the moving-average DAA converges
   naturally to the difficulty that yields 15 s on the current network
   hashrate.

**Q: What is "finality" and is it 15 seconds?**
A: Finality is irreversibility. XEPAY, like Bitcoin, provides *probabilistic*
   finality: the more blocks stacked on a transaction, the harder to reverse.
   15 seconds is the *block time*, not the finality time. A common
   conservative heuristic is 6 confirmations ≈ 90 s.

**Q: Can I use XEPAY in production today?**
A: No. XEPAY is pre-testnet. It has no network, wallet secrets are stored in
   plaintext in memory, and no audit has been performed. Do not transfer
   anything of value on it.

**Q: How does XEPAY differ from other fast chains?**
A: XEPAY is an independent implementation, not a fork. Its distinguishing
   properties are (a) SHA3-256 as the sole hash, (b) a 15-second target
   enforced by moving-average DAA, (c) ed25519 signed transactions with
   per-sender nonce replay protection, and (d) a deliberately minimal
   consensus core designed for auditability.

**Q: Where does the chain persist?**
A: To a `sled` database at the path passed to `Storage::open` (in the demo,
   `./xepay.db`). On reload, every block is re-verified before being trusted.
   Delete the directory to reset the node.

**Q: Can I contribute?**
A: Yes. Contributions that advance the roadmap, add tests, or fix correctness
   bugs are welcome. Open an issue to discuss before submitting a large PR.

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

<sub>XEPAY (XEPY) — Fast blocks. Modern hashes. Real signatures. Verifiable by construction.</sub>
