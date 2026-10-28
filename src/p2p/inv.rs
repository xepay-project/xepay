// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Inventory management for the XEPAY peer-to-peer network.

use std::collections::HashSet;

use crate::p2p::message::{InvType, Inventory};

/// Tracks block and transaction hashes that have already been seen.
#[derive(Debug, Default)]
pub struct InventoryManager {
    known_blocks: HashSet<String>,
    known_txs: HashSet<String>,
}

impl InventoryManager {
    pub fn new() -> Self {
        InventoryManager {
            known_blocks: HashSet::new(),
            known_txs: HashSet::new(),
        }
    }

    pub fn has_block(&self, hash: &str) -> bool {
        self.known_blocks.contains(hash)
    }

    pub fn has_tx(&self, txid: &str) -> bool {
        self.known_txs.contains(txid)
    }

    pub fn add_block(&mut self, hash: &str) -> bool {
        self.known_blocks.insert(hash.to_string())
    }

    pub fn add_tx(&mut self, txid: &str) -> bool {
        self.known_txs.insert(txid.to_string())
    }

    /// Add an inventory item. Return `true` if the item was not seen before.
    pub fn add(&mut self, inv: &Inventory) -> bool {
        match inv.inv_type {
            InvType::Block => self.add_block(&inv.hash),
            InvType::Transaction => self.add_tx(&inv.hash),
        }
    }

    /// Filter the given items and return only those that have not been seen.
    pub fn filter_new(&self, items: &[Inventory]) -> Vec<Inventory> {
        items
            .iter()
            .filter(|inv| match inv.inv_type {
                InvType::Block => !self.has_block(&inv.hash),
                InvType::Transaction => !self.has_tx(&inv.hash),
            })
            .cloned()
            .collect()
    }

    pub fn block_count(&self) -> usize {
        self.known_blocks.len()
    }

    pub fn tx_count(&self) -> usize {
        self.known_txs.len()
    }
}
