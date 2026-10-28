// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// block.rs
// Block and BlockHeader for XEPAY (ticker: XEPY) v0.2.
//
// v0.2 changes:
//   - Transactions are now signed `Transaction` values, not strings.
//   - merkle_root is a real binary Merkle root, not a flat hash.
//   - Each block carries exactly one coinbase transaction (reward + fees).

use serde::{Serialize, Deserialize};
use chrono::Utc;
use crate::crypto::{merkle_root, sha3_256_hash};
use crate::transaction::Transaction;

/// The coinbase is the first transaction in every block. It has no sender
/// (from = "") and pays the miner's reward plus all transaction fees.
pub fn coinbase_tx(miner_address: &str, reward: u64, fees: u64) -> Transaction {
    Transaction {
        from: String::new(),
        to: miner_address.to_string(),
        amount: reward + fees,
        fee: 0,
        nonce: 0,
        signature: String::new(), // coinbase is exempt from signature checks
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockHeader {
    pub version: u32,
    pub timestamp: i64,
    pub prev_block_hash: String,
    pub merkle_root: String,
    pub difficulty: u32,
    pub nonce: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    pub hash: String,
}

impl Block {
    /// Construct a new (unmined) block pointing to `prev_hash`.
    /// Transactions[0] MUST be the coinbase.
    pub fn new(
        prev_hash: String,
        transactions: Vec<Transaction>,
        difficulty: u32,
    ) -> Self {
        let timestamp = Utc::now().timestamp();

        // Real Merkle root over per-transaction hashes.
        let leaves: Vec<String> = transactions.iter().map(sha3_256_hash).collect();
        let root = merkle_root(&leaves);

        let header = BlockHeader {
            version: 2,
            timestamp,
            prev_block_hash: prev_hash,
            merkle_root: root,
            difficulty,
            nonce: 0,
        };

        let mut block = Block {
            header,
            transactions,
            hash: String::new(),
        };
        block.hash = sha3_256_hash(&block.header);
        block
    }

    pub fn compute_hash(&self) -> String {
        sha3_256_hash(&self.header)
    }

    pub fn mine(&mut self) {
        let target = "0".repeat(self.header.difficulty as usize);
        loop {
            let hash = self.compute_hash();
            if hash.starts_with(&target) {
                self.hash = hash;
                break;
            }
            self.header.nonce = self.header.nonce.wrapping_add(1);
        }
    }

    /// Recompute the Merkle root from the current transaction list.
    pub fn compute_merkle_root(&self) -> String {
        let leaves: Vec<String> = self.transactions.iter().map(sha3_256_hash).collect();
        merkle_root(&leaves)
    }

    pub fn is_valid(&self) -> bool {
        if self.transactions.is_empty() {
            return false;
        }
        // First transaction must be a coinbase (empty `from`).
        if !self.transactions[0].from.is_empty() {
            return false;
        }
        // All non-coinbase transactions must verify.
        for tx in &self.transactions[1..] {
            if !tx.verify() {
                return false;
            }
        }
        // Merkle root must match.
        if self.compute_merkle_root() != self.header.merkle_root {
            return false;
        }
        // Stored hash must match recomputation and meet difficulty.
        let recomputed = self.compute_hash();
        if recomputed != self.hash {
            return false;
        }
        let target = "0".repeat(self.header.difficulty as usize);
        recomputed.starts_with(&target)
    }

    /// Sum of all fees paid by non-coinbase transactions.
    pub fn total_fees(&self) -> u64 {
        self.transactions[1..].iter().map(|t| t.fee).sum()
    }
}
