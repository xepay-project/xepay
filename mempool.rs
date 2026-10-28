// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// mempool.rs
// In-memory mempool for XEPAY (ticker: XEPY).
//
// The mempool is serializable so the storage layer can persist it between
// CLI invocations. On load it is re-validated transaction by transaction.

use serde::{Serialize, Deserialize};
use crate::transaction::Transaction;

const MEMPOOL_CAP: usize = 10_000;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Mempool {
    txs: Vec<Transaction>,
}

impl Mempool {
    pub fn new() -> Self {
        Mempool { txs: Vec::new() }
    }

    pub fn add(&mut self, tx: Transaction) -> bool {
        if self.txs.len() >= MEMPOOL_CAP {
            return false;
        }
        if !tx.verify() {
            return false;
        }
        for existing in &self.txs {
            if existing.from == tx.from && existing.nonce == tx.nonce {
                return false;
            }
        }
        self.txs.push(tx);
        true
    }

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

    pub fn iter(&self) -> impl Iterator<Item = &Transaction> {
        self.txs.iter()
    }

    /// Remove every transaction from this mempool that is present in `block`.
    /// A transaction is matched by (from, nonce). Call after a block is mined.
    pub fn remove_mined(&mut self, mined: &[Transaction]) {
        self.txs.retain(|tx| {
            !mined
                .iter()
                .any(|m| m.from == tx.from && m.nonce == tx.nonce)
        });
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
        let mut tx = Transaction::new_signed(
            &a.signing_key(), &b.address(), 100, 1, 0,
        );
        tx.amount = 999;
        let mut pool = Mempool::new();
        assert!(!pool.add(tx));
    }

    #[test]
    fn drain_sorts_by_fee() {
        let a = Wallet::generate();
        let b = Wallet::generate();
        let mut pool = Mempool::new();
        pool.add(Transaction::new_signed(&a.signing_key(), &b.address(), 10, 1, 0));
        pool.add(Transaction::new_signed(&a.signing_key(), &b.address(), 10, 5, 1));
        pool.add(Transaction::new_signed(&a.signing_key(), &b.address(), 10, 3, 2));
        let picked = pool.drain_highest_fee(2);
        assert_eq!(picked[0].fee, 5);
        assert_eq!(picked[1].fee, 3);
    }

    #[test]
    fn remove_mined_clears_matching() {
        let a = Wallet::generate();
        let b = Wallet::generate();
        let mut pool = Mempool::new();
        let tx1 = Transaction::new_signed(&a.signing_key(), &b.address(), 10, 1, 0);
        let tx2 = Transaction::new_signed(&a.signing_key(), &b.address(), 10, 2, 1);
        pool.add(tx1.clone());
        pool.add(tx2.clone());
        pool.remove_mined(&[tx1]);
        assert_eq!(pool.len(), 1);
        assert!(pool.iter().any(|t| t.nonce == 1));
    }
}
