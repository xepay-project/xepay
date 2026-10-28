// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// block.rs

use serde::{Serialize, Deserialize};
use chrono::Utc;
use crate::crypto::sha3_256_hash;

/// The header contains the metadata that gets hashed.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockHeader {
    pub version: u32,
    pub timestamp: i64,
    pub prev_block_hash: String,
    pub merkle_root: String,
    pub difficulty: u32,
    pub nonce: u64,
}

/// A block contains a header plus the list of transactions.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<String>, // Simplified: transactions as strings
    pub hash: String,
}

impl Block {
    /// Create a new (unmined) block pointing to `prev_hash`.
    pub fn new(prev_hash: String, transactions: Vec<String>, difficulty: u32) -> Self {
        let timestamp = Utc::now().timestamp();

        // Simplified Merkle root: hash of the full transaction list.
        let merkle_root = sha3_256_hash(&transactions);

        let header = BlockHeader {
            version: 1,
            timestamp,
            prev_block_hash: prev_hash,
            merkle_root,
            difficulty,
            nonce: 0,
        };

        let mut block = Block {
            header,
            transactions,
            hash: String::new(),
        };

        // Initial hash (before mining)
        block.hash = sha3_256_hash(&block.header);
        block
    }

    /// Recompute the block hash from its current header.
    pub fn compute_hash(&self) -> String {
        sha3_256_hash(&self.header)
    }

    /// Proof-of-Work: find a nonce such that the hash starts with `difficulty` zeros.
    pub fn mine(&mut self) {
        let target = "0".repeat(self.header.difficulty as usize);
        loop {
            let hash = self.compute_hash();
            if hash.starts_with(&target) {
                self.hash = hash;
                println!(
                    "[XEPY] Block mined! nonce={} hash={}",
                    self.header.nonce, self.hash
                );
                break;
            }
            self.header.nonce = self.header.nonce.wrapping_add(1);
        }
    }

    /// Verify that this block's stored hash matches its header and meets difficulty.
    pub fn is_valid(&self) -> bool {
        let recomputed = self.compute_hash();
        if recomputed != self.hash {
            return false;
        }
        let target = "0".repeat(self.header.difficulty as usize);
        recomputed.starts_with(&target)
    }
}
