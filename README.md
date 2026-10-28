# XEPAY (ticker: XEPY)

> A serious, fast-block Layer-1 blockchain written in Rust.
> **SHA3-256** as the sole cryptographic primitive.
> **~15-second target block time**, enforced by an adaptive difficulty algorithm.

![status](https://img.shields.io/badge/status-v0.3%20pre--testnet-yellow)
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
   per block**, sustained by a moving-average difficulty adjustment algorithm.
2. **A modern, standardized hash.** XEPAY commits to **SHA3-256** (FIPS 202,
   Keccak) as its sole hashing primitive.
3. **Verifiability by construction.** Every block is a cryptographic commitment
   to its predecessor. Every hash is recomputable. Every validation step is
   deterministic.

XEPAY is not a fork of Bitcoin, Ethereum, or any existing network. It is an
independent implementation, built in stages toward a production-grade Layer-1.

---

## Install

### From source

```bash
git clone https://github.com/<username>/xepay.git
cd xepay
cargo build --release
```

Then either:

**Option A — use the binary directly (recommended for development):**

```bash
./target/release/xepay help
```

**Option B — symlink at the project root (so `./xepay` works):**

```bash
ln -sf target/release/xepay ./xepay
./xepay help
```

**Option C — install to `~/.cargo/bin` (global, on PATH):**

```bash
cargo install --path .
xepay help
```

All examples in this README use **Option B** (`./xepay`). Choose whichever
fits your workflow; the binary behaves identically in all three cases.

---

## Quick Start

```bash
# 1. Create a wallet (store the secret_hex securely)
./xepay wallet new
# address:    <64 hex>
# secret_hex: <64 hex>

# 2. Set it as the active wallet
./xepay wallet use <secret_hex>

# 3. Mine a coinbase block — you earn 50 XEPY
./xepay mine <secret_hex>

# 4. Check your balance
./xepay balance <address>
# balance: 50.00000000 XEPY (5000000000 XEPYsat)

# 5. Create a second wallet to receive funds
./xepay wallet new

# 6. Send 10 XEPY to the second wallet — one command, auto-mined
./xepay sendtoaddress 10 <recipient_address>
# [INFO] transfer_confirmed ...
#   from=... balance_after=90.00000100 XEPY
#   to=...   balance_after=10.00000000 XEPY

# 7. Verify
./xepay balance <recipient_address>
./xepay tx history <recipient_address>
```

---

## CLI Reference

### Wallet

```bash
./xepay wallet new                    # generate a new ed25519 keypair
./xepay wallet from <secret_hex>      # recover address from a secret
./xepay wallet info <secret_hex>      # print address, balance, next nonce
./xepay wallet use <secret_hex>       # set the active wallet in xepay.db
./xepay wallet active                 # print the active wallet's address and balance
./xepay wallet clear                  # forget the active wallet
```

### Sending funds

```bash
./xepay sendtoaddress <amount> <to_address> [options]
```

By default, `sendtoaddress` signs a transaction, enqueues it in the mempool,
**and immediately mines a block** containing it. The miner reward is paid to
the sender's address.

Options:

| Flag | Default | Effect |
|------|---------|--------|
| `--from <secret_hex>` | active wallet | use this wallet instead of the active one |
| `--unit xepy\|xepysat` | `xepy` | unit for `<amount>` |
| `--fee <xepysat>` | `100` | miner fee in smallest units |
| `--no-mine` | off | leave the transaction in the mempool instead of mining |

Examples:

```bash
# Send 10 XEPY from the active wallet
./xepay sendtoaddress 10 <to>

# Send 5 XEPY from a specific wallet
./xepay sendtoaddress --from <secret> 5 <to>

# Send 0.5 XEPY with a higher fee (1000 XEPYsat)
./xepay sendtoaddress --fee 1000 0.5 <to>

# Send 1000000000 XEPYsat (= 10 XEPY), no auto-mine
./xepay sendtoaddress --unit xepysat --no-mine 1000000000 <to>
./xepay mine <secret>                  # confirm it later
```

### Chain

```bash
./xepay chain info                    # network, chain_id, height, difficulty, tip
./xepay chain blocks [n]              # last N blocks (default 10)
./xepay chain block <height>          # full details of one block
./xepay balance <address>             # balance in XEPY and XEPYsat
```

### Mempool

```bash
./xepay mempool list                  # pending transactions with fees and nonces
```

### Low-level transactions

```bash
./xepay tx send <secret> <to> <amount_xepysat> <fee_xepysat>
./xepay tx history <address> [n]
```

### Mining

```bash
./xepay mine <secret_hex> [max_tx]
```

Mines one block from the mempool, credits the coinbase to `<secret_hex>`'s
address, and persists the result.

### Logging

Set `XEPAY_LOG` to control verbosity:

```bash
XEPAY_LOG=debug ./xepay run     # verbose
XEPAY_LOG=warn  ./xepay run     # warnings only
XEPAY_LOG=quiet ./xepay run     # silent
```

---

## Units

```
1 XEPY = 100,000,000 XEPYaldian
```

- `tx send`, `--fee`, and `--unit xepyaldian` use **XEPYsat** (smallest unit).
- `sendtoaddress` defaults to **XEPY** (fractional amounts allowed, e.g. `0.5`).

---

## Project Status — v0.3

| Milestone | Status |
|-----------|--------|
| Block structure & SHA3-256 hashing | Complete |
| Proof-of-work mining | Complete |
| Chain linkage & validation | Complete |
| Moving-average DAA | Complete |
| Median-Time-Past (MTP) | Complete |
| Real binary Merkle tree | Complete |
| Signed transactions (ed25519) | Complete |
| Per-sender replay protection | Complete |
| Mempool with fee selection | Complete |
| Block reward with halving | Complete |
| Coinbase transaction | Complete |
| Balance tracking | Complete |
| On-disk persistence (sled) | Complete |
| CLI wallet | Complete |
| `sendtoaddress` with auto-mine | Complete |
| Active wallet in DB | Complete (plaintext; encryption planned v0.4) |
| P2P networking | Planned v0.4 |
| Light client (SPV) | Planned v0.5 |
| JSON-RPC + explorer | Planned v0.6 |
| Security audit | Planned pre-mainnet |

> **Current state:** consensus core, transaction model, persistence, CLI wallet,
> and one-shot `sendtoaddress` are complete and exercised end-to-end.
> XEPAY remains **pre-testnet**.

---

## Design Notes

**Why SHA3-256?** Keccak's sponge construction gives XEPAY a distinct hash
identity from SHA-2 and insulation from future cryptanalytic surprises
affecting SHA-2.

**Why ed25519?** Faster verification, smaller keys, simpler constant-time
implementations than secp256k1.

**Why an account model?** Simpler to reason about, exposes balances directly
to the wallet CLI, and fits per-sender nonces naturally. UTXO migration is
out of scope for v0.3.

**Why `./xepay`?** Because a serious CLI should be invocable as a single
binary, not through a build tool. `cargo run --release --` is for development;
`./xepay` (or a PATH-installed `xepay`) is how users interact with the node.

---

## Safety Notes

- `wallet use` stores the secret hex **in plaintext** in `./xepay.db`. This is
  acceptable for testnet only. Encrypted at-rest storage is scheduled for v0.4.
- XEPAY has no network yet. Every node is a single-process single-chain
  instance. Do not treat any balance as real.

---

## Roadmap

### v0.4 — Encrypted wallet & EMA-based DAA
- Password-protected secret storage (Argon2 + XChaCha20-Poly1305)
- EMA difficulty adjustment with outlier-resistant windows
- P2P gossip over TCP (`tokio`)

### v0.5 — Light client
- SPV proofs using Merkle inclusion paths
- Bloom filters for transaction filtering

### v0.6 — API & tooling
- JSON-RPC server
- Block explorer web UI
- Prometheus metrics

### v1.0 — Production readiness
- Comprehensive test suite, fuzzing, benchmarks
- External security audit
- Mainnet deployment plan

---

## Development

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --release
cargo build --release
```

### Smoke test

```bash
rm -rf xepay.db
./target/release/xepay run
```

Prints a full demo: three wallets, two mined blocks, persistence check,
final balances, and average block time.

---

## License

Copyright (c) 2026 **Aldianokto**.

Licensed under the **Apache License, Version 2.0**.
See the [`LICENSE`](./LICENSE) file for the full text.

---

<sub>XEPAY (XEPY) — Fast blocks. Modern hashes. Real signatures. Verifiable by construction.</sub>
