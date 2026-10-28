# XEPAY Architecture

XEPAY is a Layer-1 payment network built in Rust. It uses an account-based model with a custom consensus mechanism powered by SHA3-256. This document explains the internal architecture of the node.

## Core Components

1. **Consensus Layer (`src/consensus/`):**
   Defines the economic parameters (supply, halving, block time) and the genesis block for Mainnet, Testnet, and Regtest. It also handles the Proof-of-Work algorithm and the difficulty adjustment.

2. **Storage Layer (`src/storage/` & `src/storage_legacy.rs`):**
   Uses a Merkle Patricia Trie backed by LevelDB for state management. Blocks are serialized using Bincode for maximum I/O efficiency. Includes a manual reindex mechanism similar to Bitcoin Core.
   - `storage/account.rs`: Defines the `Account` structure with a `BTreeMap` for deterministic state roots.
   - `storage/state.rs`: Manages the `StateDB`, including account updates, rewards, and rollbacks. Uses strict `checked_add` math to prevent integer overflows.
   - `storage_legacy.rs`: Handles block serialization (Bincode) and state rebuilding from history.

3. **Cryptography (`src/secp256k1/` & `crypto/`):**
   Uses secp256k1 for ECDSA signatures and SHA3-256 for hashing. Addresses support both Legacy (Base58Check) and Native SegWit (Bech32) formats.
   - `crypto/encryption.rs`: AES-256-GCM and PBKDF2 for wallet encryption.
   - `secp256k1/address.rs`: Derivation of Legacy and Bech32 addresses.

4. **P2P Network (`src/p2p/`):**
   A custom TCP-based peer-to-peer network. It includes anti-eclipse measures (anchors, banman, asmap) and a manual sync state for block propagation.
   - `p2p/framing.rs`: Message framing with magic bytes and checksums.
   - `p2p/handshake.rs`: Version/Verack handshake.
   - `p2p/sync.rs`: Chain synchronization and reorg handling.

5. **RPC & CLI (`src/rpc.rs` & `src/xepay.rs`):**
   Provides an HTTP JSON-RPC interface for nodes and a command-line interface for users.

## Data Flow

1. A user creates a transaction via CLI or RPC.
2. The transaction is signed using the private key (secp256k1).
3. The signed transaction enters the Mempool.
4. Miners pick transactions from the Mempool, create a block, and solve the Proof-of-Work.
5. The new block is broadcasted via P2P to other nodes.
6. Nodes validate the block and update their local state (Merkle Trie).
