> A serious, fast-block Layer-1 payment network written in Rust.
> **SHA3-256** as the sole cryptographic primitive.
> **~15-second target block time**, enforced by an adaptive difficulty algorithm.

![language](https://img.shields.io/badge/language-Rust-orange)
![hash](https://img.shields.io/badge/hash-SHA3--256-green)
![consensus](https://img.shields.io/badge/consensus-Proof%20of%20Work-purple)

---

## Overview

**XEPAY** (**XEPC**) is a Layer-1 proof-of-work payment network written in Rust. It is an independent implementation, built in stages toward a production-grade Layer-1, and it uses an Account-Based model backed by a Merkle Patricia Trie and LevelDB. 

XEPAY is engineered around three core design commitments. First, it aims to provide a fast and predictable block time. The network targets an average of 15 seconds per block, which is sustained by a strict and adaptive difficulty adjustment algorithm. This ensures that the network remains responsive to sudden changes in hash rate while maintaining a consistent and reliable transaction finality. Second, XEPAY commits to a modern and standardized cryptographic hash. By using SHA3-256 as its sole hashing primitive, XEPAY ensures a high level of security and insulation from future cryptanalytic surprises that might affect older hash functions. Third, XEPAY is designed to be verifiable by construction. Every block in the chain is a cryptographic commitment to its predecessor, every hash is fully recomputable, and every validation step is deterministic. This design eliminates ambiguity and ensures that any node can independently verify the entire history of the network from the genesis block to the current tip.

Beyond these core commitments, XEPAY is designed to be a reliable, fast, and secure medium of exchange. It focuses entirely on peer-to-peer value transfer, avoiding unnecessary complexity and state bloat. The network maintains a strict state verification system, ensuring that every node operates on the exact same data without relying on external intermediaries. This strict verification process eliminates the risk of silent state corruption and ensures the integrity of the ledger at all times. The architecture is built to be robust, scalable, and ready for long-term sustainability, providing a solid foundation for a global payment network.

---

## Install

### From source
```bash
git clone https://github.com/xepay-project/xepay.git
cd xepay
cargo build --release
```
After building, copy the binaries to the project root using the provided script:
```bash
./build.sh
```
Now you can run the CLI directly:
```bash
./xepay help
```
---

## Networks

XEPAY supports three independent networks, each with its own genesis block and ports. You can select a network by passing the corresponding flag to the node and CLI.

| Network | Flag | P2P Port | RPC Port |
|---|---|---|---|
| Mainnet | (none) | 7555 | 7554 |
| Testnet | -testnet | 12255 | 12254 |
| Regtest | -regtest | 18444 | 18443 |

### What is Regtest?

Regtest (Regression Test) is a private, isolated network for local development. It uses a trivially low proof-of-work difficulty, allowing developers to mine blocks instantly. It is not connected to the public network and should only be used for testing.

---

## Running a Node

To run a node on the Mainnet:
```bash
./xepayd
```
To run a node on the Testnet:
```bash
./xepayd -testnet
```
To run a node on the Regtest network (for local development):
```bash
./xepayd -regtest
```
---

## License

Copyright (c) 2026 **Aldianokto**.

Licensed under the **Apache License, Version 2.0**.
See the LICENSE file for the full text.

---
