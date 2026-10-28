// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// blockchain.rs
// The XEPAY (ticker: XEPY) chain with adaptive difficulty.
//
// Design goal: maintain an average block time of approximately
// TARGET_BLOCK_TIME_SECS (15 seconds). Every `adjustment_interval` blocks,
// the chain compares the actual elapsed time against the expected time and
// nudges difficulty up or down accordingly.

use crate::block::Block;

/// Target average block time, in seconds. XEPY aims for ~15 s.
pub const TARGET_BLOCK_TIME_SECS: i64 = 15;

/// How many blocks between difficulty adjustments.
/// A short interval is chosen because the target block time is short.
pub const ADJUSTMENT_INTERVAL: usize = 20;

/// Clamp bounds for difficulty to keep the demo tractable.
pub const MIN_DIFFICULTY: u32 = 1;
pub const MAX_DIFFICULTY: u32 = 8;

pub struct Blockchain {
    pub chain: Vec<Block>,
    pub difficulty: u32,
}

impl Blockchain {
    /// Create a new chain with a freshly mined genesis block.
    pub fn new(initial_difficulty: u32) -> Self {
        let mut genesis = Block::new(
            String::from("0"),
            vec!["XEPY Genesis Block".to_string()],
            initial_difficulty,
        );
        genesis.mine();

        Blockchain {
            chain: vec![genesis],
            difficulty: initial_difficulty,
        }
    }

    /// Return the most recent block.
    pub fn tip(&self) -> &Block {
        self.chain.last().expect("Chain is always non-empty")
    }

    /// Append a new block containing `transactions`, mining it first.
    /// If the adjustment interval has been reached, difficulty is recalculated
    /// for the *next* block.
    pub fn add_block(&mut self, transactions: Vec<String>) {
        let prev_hash = self.tip().hash.clone();
        let mut new_block = Block::new(prev_hash, transactions, self.difficulty);
        new_block.mine();

        assert!(new_block.is_valid(), "New block failed validation");
        self.chain.push(new_block);

        // Recalculate difficulty for the next block, if needed.
        if self.chain.len() % ADJUSTMENT_INTERVAL == 0 {
            self.adjust_difficulty();
        }
    }

    /// Adjust difficulty based on the actual elapsed time of the last
    /// `ADJUSTMENT_INTERVAL` blocks.
    pub fn adjust_difficulty(&mut self) {
        if self.chain.len() < ADJUSTMENT_INTERVAL {
            return;
        }

        let last = self.chain.len() - 1;
        let first = last - (ADJUSTMENT_INTERVAL - 1);

        let actual_secs =
            self.chain[last].header.timestamp - self.chain[first].header.timestamp;

        let expected_secs =
            (ADJUSTMENT_INTERVAL as i64 - 1) * TARGET_BLOCK_TIME_SECS;

        // Guard against zero/negative elapsed time (clock skew).
        let actual_secs = actual_secs.max(1);

        let old = self.difficulty;

        // If blocks came in too fast, raise difficulty.
        if actual_secs * 2 < expected_secs {
            self.difficulty = (self.difficulty + 1).min(MAX_DIFFICULTY);
        }
        // If blocks came in too slowly, lower difficulty.
        else if actual_secs > expected_secs * 2 {
            self.difficulty = self.difficulty.saturating_sub(1).max(MIN_DIFFICULTY);
        }

        if old != self.difficulty {
            println!(
                "[XEPY] Difficulty adjusted: {} -> {} (actual={}s, expected={}s)",
                old, self.difficulty, actual_secs, expected_secs
            );
        } else {
            println!(
                "[XEPY] Difficulty unchanged at {} (actual={}s, expected={}s)",
                self.difficulty, actual_secs, expected_secs
            );
        }
    }

    /// Validate the entire chain.
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
            }
        }
        true
    }

    /// Number of blocks in the chain.
    pub fn height(&self) -> usize {
        self.chain.len()
    }

    /// Average block time over the whole chain, in seconds.
    /// Returns 0.0 if the chain has fewer than two blocks.
    pub fn average_block_time_secs(&self) -> f64 {
        if self.chain.len() < 2 {
            return 0.0;
        }
        let first = self.chain.first().unwrap().header.timestamp;
        let last = self.chain.last().unwrap().header.timestamp;
        let intervals = self.chain.len() as f64 - 1.0;
        (last - first) as f64 / intervals
    }
}
