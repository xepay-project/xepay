// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// mempool.rs
// In-memory mempool for XEPAY (ticker: XEPY).
//
// Stores verified-but-unmined transactions. Miners select the highest-fee
// transactions first, which is the simplest possible fee market model.

use crate::transaction::Transaction;

/// Maximum number of transactions held in the mempool.
const MEMPOOL_CAP: usize = 10_000;

pub struct Mempool {
    txs: Vec<Transaction>,
}

impl Mempool {
    pub fn new() -> Self {
        Mempool { txs: Vec::new() }
    }

    /// Add a transaction if it verifies, is not a duplicate, and there is room.
    /// Returns true if the transaction was accepted.
    pub fn add(&mut self, tx: Transaction) -> bool {
        if self.txs.len() >= MEMPOOL_CAP {
            return false;
        }
        if !tx.verify() {
            return false;
        }
        // Reject duplicates by (from, nonce) — replay protection.
        for existing in &self.txs {
            if existing.from == tx.from && existing.nonce == tx.nonce {
                return false;
            }
        }
        self.txs.push(tx);
        true
    }

    /// Select up to `limit` transactions, ordered by fee descending.
    /// Removes the selected transactions from the mempool.
    pub fn drain_highest_fee(&mut self, limit: usize) -> Vec<Transaction> {
        self.txs.sort_by(|a, b| b.fee.cmp(&a.fee));
        let take = limit.min(self.txs.len());
        self.txs.drain(0..take).collect()
    }

    pub fn len(&self) -> usize {
        self.txs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.txs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::Wallet;

    #[test]
    fn rejects_invalid_signature() {
        let a = Wallet::generate();
        let b = Wallet::generate();
        let mut tx = Transaction::new_signed(&a.signing_key(), &b.address(), 100, 1, 0);
        tx.amount = 999; // tamper
        let mut pool = Mempool::new();
        assert!(!pool.add(tx));
    }

    #[test]
    fn drain_sorts_by_fee() {
        let a = Wallet::generate();
        let b = Wallet::generate();
        let mut pool = Mempool::new();
        pool.add(Transaction::new_signed(
            &a.signing_key(),
            &b.address(),
            10,
            1,
            0,
        ));
        pool.add(Transaction::new_signed(
            &a.signing_key(),
            &b.address(),
            10,
            5,
            1,
        ));
        pool.add(Transaction::new_signed(
            &a.signing_key(),
            &b.address(),
            10,
            3,
            2,
        ));
        let picked = pool.drain_highest_fee(2);
        assert_eq!(picked[0].fee, 5);
        assert_eq!(picked[1].fee, 3);
    }
}
