// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

use crate::block::Block;

pub struct Blockchain {
    pub chain: Vec<Block>,
    pub difficulty: u32,
}

impl Blockchain {
    /// Create a new chain with a freshly mined genesis block.
    pub fn new(difficulty: u32) -> Self {
        let mut genesis = Block::new(
            String::from("0"),
            vec!["XEPY Genesis Block".to_string()],
            difficulty,
        );
        genesis.mine();

        Blockchain {
            chain: vec![genesis],
            difficulty,
        }
    }

    /// Return the tip (most recent block) of the chain.
    pub fn tip(&self) -> &Block {
        self.chain.last().expect("Chain is always non-empty")
    }

    /// Add a new block containing the given transactions.
    pub fn add_block(&mut self, transactions: Vec<String>) {
        let prev_hash = self.tip().hash.clone();
        let mut new_block = Block::new(prev_hash, transactions, self.difficulty);
        new_block.mine();

        // Sanity check before appending
        assert!(new_block.is_valid(), "New block failed validation");
        self.chain.push(new_block);
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

    /// Total number of blocks in the chain.
    pub fn height(&self) -> usize {
        self.chain.len()
    }
}
