// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// storage/storage.rs
//
// Stores the chain as bincode blobs keyed by block height. On load, blocks
// are re-verified end-to-end before being trusted.

use sled::Db;
use crate::block::Block;
use crate::blockchain::Blockchain;

pub struct Storage {
    db: Db,
}

impl Storage {
    pub fn open(path: &str) -> Result<Self, String> {
        let db = sled::open(path).map_err(|e| e.to_string())?;
        Ok(Storage { db })
    }

    /// Persist a single block at the given height.
    pub fn put_block(&self, height: usize, block: &Block) -> Result<(), String> {
        let encoded = bincode::serialize(block).map_err(|e| e.to_string())?;
        self.db
            .insert(height.to_be_bytes(), encoded)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Load every block from disk, ordered by height.
    pub fn load_chain(&self) -> Result<Vec<Block>, String> {
        let mut blocks = Vec::new();
        for item in self.db.iter() {
            let (_, v) = item.map_err(|e| e.to_string())?;
            let block: Block = bincode::deserialize(&v).map_err(|e| e.to_string())?;
            blocks.push(block);
        }
        Ok(blocks)
    }

    /// Save the current chain to disk.
    pub fn save_chain(&self, chain: &Blockchain) -> Result<(), String> {
        for (i, block) in chain.chain.iter().enumerate() {
            self.put_block(i, block)?;
        }
        self.db.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Restore a chain from disk and re-verify it. Returns None if empty.
    pub fn restore_chain(&self, difficulty: u32) -> Result<Option<Blockchain>, String> {
        let blocks = self.load_chain()?;
        if blocks.is_empty() {
            return Ok(None);
        }
        let mut chain = Blockchain::new(difficulty);
        chain.chain = blocks;
        if !chain.is_valid() {
            return Err("stored chain failed validation".into());
        }
        Ok(Some(chain))
    }
}
