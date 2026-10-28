// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// blockchain.rs
// The XEPAY (ticker: XEPY) chain, v0.2.
//
// v0.2 changes:
//   - Timestamp bounds via Median-Time-Past (MTP) rule.
//   - Moving-average difficulty adjustment (replaces integer ±1 step).
//   - Block reward with halving every HALVING_INTERVAL blocks.
//   - Coinbase automatically added to every block.
//   - Balance tracking for transaction validation.

use std::collections::HashMap;
use crate::block::{coinbase_tx, Block};
use crate::transaction::Transaction;

pub const TARGET_BLOCK_TIME_SECS: i64 = 15;
pub const ADJUSTMENT_INTERVAL: usize = 20;
pub const MIN_DIFFICULTY: u32 = 1;

/// Initial block reward, in XEPYsat (1 XEPY = 100_000_000 XEPYsat).
pub const INITIAL_REWARD: u64 = 50_0000_0000;
/// Reward halves every 210,000 blocks (Bitcoin-style cadence).
pub const HALVING_INTERVAL: u64 = 210_000;
/// How many past timestamps participate in the MTP median.
pub const MTP_WINDOW: usize = 11;

pub struct Blockchain {
    pub chain: Vec<Block>,
    pub difficulty: u32,
    /// Current balance per address (public key hex).
    pub balances: HashMap<String, u64>,
    /// Next expected nonce per sender (replay protection).
    pub nonces: HashMap<String, u64>,
}

impl Blockchain {
    pub fn new(initial_difficulty: u32) -> Self {
        let genesis_cb = coinbase_tx("genesis-miner", INITIAL_REWARD, 0);
        let mut genesis = Block::new(String::from("0"), vec![genesis_cb], initial_difficulty);
        genesis.mine();

        let mut chain = Blockchain {
            chain: vec![genesis],
            difficulty: initial_difficulty,
            balances: HashMap::new(),
            nonces: HashMap::new(),
        };
        chain.apply_block_balances(0);
        chain
    }

    pub fn tip(&self) -> &Block {
        self.chain.last().expect("non-empty")
    }

    pub fn height(&self) -> usize {
        self.chain.len()
    }

    /// Compute the block reward for a given height, applying halving.
    pub fn block_reward(&self) -> u64 {
        let halvings = (self.chain.len() as u64) / HALVING_INTERVAL;
        if halvings >= 64 {
            return 0;
        }
        INITIAL_REWARD >> halvings
    }

    /// Median-Time-Past check: reject timestamps ≤ median of last MTP_WINDOW.
    pub fn timestamp_passes_mtp(&self, ts: i64) -> bool {
        if self.chain.len() < MTP_WINDOW {
            return ts > self.tip().header.timestamp;
        }
        let mut recent: Vec<i64> = self
            .chain
            .iter()
            .rev()
            .take(MTP_WINDOW)
            .map(|b| b.header.timestamp)
            .collect();
        recent.sort();
        ts > recent[MTP_WINDOW / 2]
    }

    /// Check that every non-coinbase transaction is affordable given current
    /// balances and nonces. Does NOT mutate state.
    pub fn validate_transactions(&self, txs: &[Transaction]) -> bool {
        // Temporary deltas to account for intra-block spending.
        let mut deltas: HashMap<String, i128> = HashMap::new();
        let mut nonce_delta: HashMap<String, u64> = HashMap::new();

        for tx in txs {
            if tx.from.is_empty() {
                continue; // coinbase handled elsewhere
            }
            if !tx.verify() {
                return false;
            }
            let bal = *self.balances.get(&tx.from).unwrap_or(&0) as i128
                + deltas.get(&tx.from).copied().unwrap_or(0);
            if bal < tx.total_debit() as i128 {
                return false;
            }
            let expected = *self.nonces.get(&tx.from).unwrap_or(&0)
                + nonce_delta.get(&tx.from).copied().unwrap_or(0);
            if tx.nonce != expected {
                return false;
            }
            *deltas.entry(tx.from.clone()).or_insert(0) -= tx.total_debit() as i128;
            *deltas.entry(tx.to.clone()).or_insert(0) += tx.amount as i128;
            *nonce_delta.entry(tx.from.clone()).or_insert(0) += 1;
        }
        true
    }

    /// Mine and append a block containing `txs`. Adds coinbase automatically.
    pub fn add_block(&mut self, mut txs: Vec<Transaction>, miner_address: &str) -> Result<(), String> {
        if !self.validate_transactions(&txs) {
            return Err("transaction validation failed".into());
        }

        // Apply nonce increments for intra-block sequencing.
        for tx in &txs {
            if !tx.from.is_empty() {
                *self.nonces.entry(tx.from.clone()).or_insert(0) += 1;
            }
        }

        // Compute fees and build coinbase.
        let fees: u64 = txs.iter().map(|t| t.fee).sum();
        let reward = self.block_reward();
        let cb = coinbase_tx(miner_address, reward, fees);
        txs.insert(0, cb);

        let prev_hash = self.tip().hash.clone();
        let mut new_block = Block::new(prev_hash, txs, self.difficulty);

        if !self.timestamp_passes_mtp(new_block.header.timestamp) {
            return Err("timestamp failed median-time-past".into());
        }

        new_block.mine();
        if !new_block.is_valid() {
            return Err("block validation failed".into());
        }

        self.chain.push(new_block);
        let idx = self.chain.len() - 1;
        self.apply_block_balances(idx);

        if self.chain.len() % ADJUSTMENT_INTERVAL == 0 {
            self.adjust_difficulty();
        }
        Ok(())
    }

    /// Apply balance changes for the block at index `idx`.
    fn apply_block_balances(&mut self, idx: usize) {
        let block = self.chain[idx].clone();
        for tx in &block.transactions {
            if tx.from.is_empty() {
                // Coinbase: only credits.
                *self.balances.entry(tx.to.clone()).or_insert(0) += tx.amount;
            } else {
                let sender_bal = self.balances.entry(tx.from.clone()).or_insert(0);
                *sender_bal = sender_bal.saturating_sub(tx.total_debit());
                *self.balances.entry(tx.to.clone()).or_insert(0) += tx.amount;
            }
        }
    }

    /// Moving-average DAA: scale difficulty smoothly toward target.
    pub fn adjust_difficulty(&mut self) {
        if self.chain.len() < ADJUSTMENT_INTERVAL {
            return;
        }
        let last = self.chain.len() - 1;
        let first = last - (ADJUSTMENT_INTERVAL - 1);
        let elapsed = self.chain[last].header.timestamp
            - self.chain[first].header.timestamp;
        let actual_per_block = (elapsed as f64) / ((ADJUSTMENT_INTERVAL - 1) as f64);
        let actual = actual_per_block.max(0.001);

        let ratio = TARGET_BLOCK_TIME_SECS as f64 / actual;
        let new_d = (self.difficulty as f64 * ratio).round() as i64;
        let new_d = new_d.max(MIN_DIFFICULTY as i64) as u32;
        if new_d != self.difficulty {
            println!(
                "[XEPY] DAA: {} -> {} (avg {:.2}s/block, target {}s)",
                self.difficulty, new_d, actual, TARGET_BLOCK_TIME_SECS
            );
            self.difficulty = new_d;
        }
    }

    pub fn is_valid(&self) -> bool {
        for (i, block) in self.chain.iter().enumerate() {
            if !block.is_valid() {
                return false;
            }
            if i > 0 {
                let prev = &self.chain[i - 1];
                if block.header.prev_block_hash != prev.hash {
                    return false;
                }
                if block.header.timestamp <= prev.header.timestamp {
                    return false;
                }
            }
        }
        true
    }

    pub fn average_block_time_secs(&self) -> f64 {
        if self.chain.len() < 2 {
            return 0.0;
        }
        let first = self.chain.first().unwrap().header.timestamp;
        let last = self.chain.last().unwrap().header.timestamp;
        (last - first) as f64 / (self.chain.len() as f64 - 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::Wallet;

    #[test]
    fn genesis_is_valid() {
        let c = Blockchain::new(1);
        assert!(c.is_valid());
        assert_eq!(c.height(), 1);
    }

    #[test]
    fn reward_halves() {
        let mut c = Blockchain::new(1);
        // Fake height to test halving logic without mining 210k blocks.
        c.chain.resize(HALVING_INTERVAL as usize + 1, c.chain[0].clone());
        assert_eq!(c.block_reward(), INITIAL_REWARD >> 1);
    }

    #[test]
    fn rejects_overspend() {
        let mut c = Blockchain::new(1);
        let alice = Wallet::generate();
        let bob = Wallet::generate();
        let tx = Transaction::new_signed(&alice.signing_key(), &bob.address(), 1_000, 1, 0);
        assert!(c.validate_transactions(&[tx]).eq(&false));
    }
}
