// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// storage/storage.rs
// On-disk persistence for XEPAY (ticker: XEPY) using sled.
//
// Block tree keys:  b"b" + height_be     → bincode(Block)
// Metadata key:     b"meta"              → bincode(ChainMeta)
// Mempool key:      b"mempool"           → bincode(Mempool)

use serde::{Serialize, Deserialize};
use sled::Db;
use crate::block::Block;
use crate::blockchain::{Blockchain, CHAIN_ID};
use crate::mempool::Mempool;

const PREFIX_BLOCK: &[u8] = b"b";
const KEY_MEMPOOL: &[u8] = b"mempool";
const KEY_META: &[u8] = b"meta";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChainMeta {
    pub chain_id: String,
    pub network: String,
    pub protocol_version: String,
    pub difficulty: u32,
}

pub struct Storage {
    db: Db,
}

impl Storage {
    pub fn open(path: &str) -> Result<Self, String> {
        let db = sled::open(path).map_err(|e| e.to_string())?;
        Ok(Storage { db })
    }

    fn block_key(height: usize) -> Vec<u8> {
        let mut k = PREFIX_BLOCK.to_vec();
        k.extend_from_slice(&(height as u64).to_be_bytes());
        k
    }

    // ---------------------------------------------------------------
    // Metadata
    // ---------------------------------------------------------------

    pub fn save_meta(&self, meta: &ChainMeta) -> Result<(), String> {
        let encoded = bincode::serialize(meta).map_err(|e| e.to_string())?;
        self.db.insert(KEY_META, encoded).map_err(|e| e.to_string())?;
        self.db.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load_meta(&self) -> Result<Option<ChainMeta>, String> {
        match self.db.get(KEY_META).map_err(|e| e.to_string())? {
            Some(bytes) => {
                let meta: ChainMeta =
                    bincode::deserialize(&bytes).map_err(|e| e.to_string())?;
                Ok(Some(meta))
            }
            None => Ok(None),
        }
    }

    /// Verify that the on-disk chain belongs to the expected chain ID.
    /// Returns Ok(()) if no metadata exists yet (fresh DB) or if it matches.
    pub fn check_chain_id(&self) -> Result<(), String> {
        if let Some(meta) = self.load_meta()? {
            if meta.chain_id != CHAIN_ID {
                return Err(format!(
                    "chain id mismatch: on-disk={} expected={}",
                    meta.chain_id, CHAIN_ID
                ));
            }
        }
        Ok(())
    }

    // ---------------------------------------------------------------
    // Block persistence
    // ---------------------------------------------------------------

    pub fn put_block(&self, height: usize, block: &Block) -> Result<(), String> {
        let encoded = bincode::serialize(block).map_err(|e| e.to_string())?;
        self.db
            .insert(Self::block_key(height), encoded)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load_chain(&self) -> Result<Vec<Block>, String> {
        let mut blocks = Vec::new();
        for item in self.db.scan_prefix(PREFIX_BLOCK) {
            let (_, v) = item.map_err(|e| e.to_string())?;
            let block: Block = bincode::deserialize(&v).map_err(|e| e.to_string())?;
            blocks.push(block);
        }
        Ok(blocks)
    }

    pub fn save_chain(&self, chain: &Blockchain) -> Result<(), String> {
        for (i, block) in chain.chain.iter().enumerate() {
            self.put_block(i, block)?;
        }
        self.db.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

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

    // ---------------------------------------------------------------
    // Mempool persistence
    // ---------------------------------------------------------------

    pub fn save_mempool(&self, pool: &Mempool) -> Result<(), String> {
        let encoded = bincode::serialize(pool).map_err(|e| e.to_string())?;
        self.db
            .insert(KEY_MEMPOOL, encoded)
            .map_err(|e| e.to_string())?;
        self.db.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load_mempool(&self) -> Result<Mempool, String> {
        match self.db.get(KEY_MEMPOOL).map_err(|e| e.to_string())? {
            Some(bytes) => {
                let pool: Mempool =
                    bincode::deserialize(&bytes).map_err(|e| e.to_string())?;
                Ok(pool)
            }
            None => Ok(Mempool::new()),
        }
    }
}
