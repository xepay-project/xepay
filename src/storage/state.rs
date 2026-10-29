// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! State database using Merkle Patricia Trie backed by LevelDB.

use alloy_primitives::B256;
use eth_trie::{EthTrie, Trie, DB};
use keccak_hash::keccak;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use super::account::Account;
use super::adapter::LevelDbAdapter;

pub struct StateDB {
    trie: EthTrie<LevelDbAdapter>,
    adapter: Arc<LevelDbAdapter>,
}

impl StateDB {
    pub fn open(path: &Path) -> Result<Self, String> {
        let adapter = Arc::new(LevelDbAdapter::open(path)?);

        let root_path = crate::datadir::chainstate_dir().join("root_hash.txt");
        if let Ok(hex_str) = std::fs::read_to_string(&root_path) {
            let hex_str = hex_str.trim();
            if let Ok(bytes) = hex::decode(hex_str) {
                if bytes.len() == 32 {
                    let root = B256::from_slice(&bytes);
                    if let Ok(trie) = EthTrie::from(adapter.clone(), root) {
                        return Ok(StateDB { trie, adapter });
                    }
                }
            }
        }

        let trie = EthTrie::new(adapter.clone());
        Ok(StateDB { trie, adapter })
    }

    fn address_key(address: &str) -> [u8; 32] {
        keccak(address.as_bytes()).0
    }

    pub fn update_account(&mut self, address: &str, account: &Account) -> Result<(), String> {
        let key = Self::address_key(address);
        let value = rlp::encode(account).to_vec();
        self.trie.insert(&key, &value).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_account(&self, address: &str) -> Result<Option<Account>, String> {
        let key = Self::address_key(address);
        match self.trie.get(&key).map_err(|e| e.to_string())? {
            Some(value) => {
                let account: Account =
                    rlp::decode(&value).map_err(|e| format!("RLP decode error: {}", e))?;
                Ok(Some(account))
            }
            None => Ok(None),
        }
    }

    pub fn root_hash(&mut self) -> Result<[u8; 32], String> {
        let root = self.trie.root_hash().map_err(|e| e.to_string())?;
        let mut arr = [0u8; 32];
        arr.copy_from_slice(root.as_slice());
        Ok(arr)
    }

    pub fn rollback_to(&mut self, root_hash: &[u8; 32]) -> Result<(), String> {
        let root = B256::from_slice(root_hash);
        let new_trie = EthTrie::from(self.adapter.clone(), root)
            .map_err(|e| format!("failed to create trie from root: {}", e))?;

        // Verify that the root is valid by checking that the trie can
        // compute its own root hash and the result matches the input.
        let mut verification_trie = new_trie;
        let computed_root = verification_trie
            .root_hash()
            .map_err(|e| format!("failed to compute root hash during rollback: {}", e))?;

        if computed_root.as_slice() != root_hash {
            return Err(format!(
                "state root mismatch during rollback: expected {}, got {}",
                hex::encode(root_hash),
                hex::encode(computed_root)
            ));
        }

        self.trie = verification_trie;
        Ok(())
    }

    /// Create a temporary snapshot of the current state without
    /// modifying the original. This is used by `getblocktemplate`
    /// to simulate the effect of mempool transactions on the state root.
    ///
    /// # Bitcoin-Grade Note
    ///
    /// In Bitcoin Core, `getblocktemplate` acquires a lock and calls
    /// `TestBlockValidity`, which mutates a temporary `CCoinsViewCache`.
    /// Here, `rollback_snapshot()` is the equivalent: it takes `&mut self`
    /// because `root_hash()` requires mutable access to the trie. The
    /// caller (`m_getblocktemplate`) is responsible for holding a write
    /// lock on the chain while this snapshot is taken.
    pub fn rollback_snapshot(&mut self) -> Result<StateDB, String> {
        let root = self
            .trie
            .root_hash()
            .map_err(|e| format!("failed to read root hash: {}", e))?;
        let root_b256 = B256::from_slice(root.as_slice());

        let new_trie = EthTrie::from(self.adapter.clone(), root_b256)
            .map_err(|e| format!("failed to create snapshot trie: {}", e))?;

        Ok(StateDB {
            trie: new_trie,
            adapter: self.adapter.clone(),
        })
    }

    pub fn balance_of(&self, address: &str) -> u64 {
        match self.get_account(address) {
            Ok(Some(acc)) => {
                let immature: u64 = acc
                    .immature_rewards
                    .iter()
                    .map(|(_, a)| *a)
                    .fold(0u64, |acc, x| acc.saturating_add(x));
                acc.balance.saturating_add(immature)
            }
            _ => 0,
        }
    }

    pub fn mature_balance_at(&self, address: &str, current_height: u64) -> u64 {
        match self.get_account(address) {
            Ok(Some(acc)) => {
                let maturity = crate::consensus::rules::COINBASE_MATURITY;
                let mut mature = acc.balance;
                for (height, amount) in &acc.immature_rewards {
                    if current_height >= height.saturating_add(maturity) {
                        mature = mature.saturating_add(*amount);
                    }
                }
                mature
            }
            _ => 0,
        }
    }

    pub fn mature_balance_of(&self, address: &str) -> u64 {
        match self.get_account(address) {
            Ok(Some(acc)) => acc.balance,
            _ => 0,
        }
    }

    pub fn immature_balance_of(&self, address: &str) -> u64 {
        match self.get_account(address) {
            Ok(Some(acc)) => acc.immature_rewards.iter().map(|(_, a)| a).sum(),
            _ => 0,
        }
    }

    pub fn nonce_of(&self, address: &str) -> u64 {
        match self.get_account(address) {
            Ok(Some(acc)) => acc.nonce,
            _ => 0,
        }
    }

    pub fn get_code(&self, address: &str) -> Vec<u8> {
        match self.get_account(address) {
            Ok(Some(acc)) => acc.code.clone(),
            _ => Vec::new(),
        }
    }

    pub fn get_storage(&self, address: &str) -> BTreeMap<[u8; 32], [u8; 32]> {
        match self.get_account(address) {
            Ok(Some(acc)) => acc.storage.clone(),
            _ => BTreeMap::new(),
        }
    }

    pub fn set_storage(
        &mut self,
        address: &str,
        storage: BTreeMap<[u8; 32], [u8; 32]>,
    ) -> Result<(), String> {
        let mut acc = self.get_account(address)?.unwrap_or(Account {
            nonce: 0,
            balance: 0,
            immature_rewards: Vec::new(),
            code: Vec::new(),
            storage: BTreeMap::new(),
        });
        acc.storage = storage;
        self.update_account(address, &acc)
    }

    pub fn apply_transaction(
        &mut self,
        from: &str,
        to: &str,
        amount: u64,
        fee: u64,
        nonce: u64,
    ) -> Result<(), String> {
        let mut from_acc = self.get_account(from)?.unwrap_or(Account {
            nonce: 0,
            balance: 0,
            immature_rewards: Vec::new(),
            code: Vec::new(),
            storage: BTreeMap::new(),
        });

        if from_acc.nonce != nonce {
            return Err(format!(
                "invalid nonce: got {}, expected {}",
                nonce, from_acc.nonce
            ));
        }

        let total = amount
            .checked_add(fee)
            .ok_or_else(|| "amount + fee overflows u64".to_string())?;

        if from_acc.balance < total {
            return Err(format!(
                "insufficient mature balance: required {}, available {}",
                total, from_acc.balance
            ));
        }

        let mut to_acc = self.get_account(to)?.unwrap_or(Account {
            nonce: 0,
            balance: 0,
            immature_rewards: Vec::new(),
            code: Vec::new(),
            storage: BTreeMap::new(),
        });

        from_acc.balance -= total;
        from_acc.nonce = from_acc
            .nonce
            .checked_add(1)
            .ok_or_else(|| "sender nonce overflow".to_string())?;
        to_acc.balance = to_acc
            .balance
            .checked_add(amount)
            .ok_or_else(|| "recipient balance overflow".to_string())?;

        self.update_account(from, &from_acc)?;
        self.update_account(to, &to_acc)?;

        Ok(())
    }

    pub fn credit_reward(
        &mut self,
        miner: &str,
        reward: u64,
        block_height: u64,
    ) -> Result<(), String> {
        let mut acc = self.get_account(miner)?.unwrap_or(Account {
            nonce: 0,
            balance: 0,
            immature_rewards: Vec::new(),
            code: Vec::new(),
            storage: BTreeMap::new(),
        });

        let maturity = crate::consensus::rules::COINBASE_MATURITY;

        let mut still_immature = Vec::new();
        for (height, amount) in acc.immature_rewards.drain(..) {
            if block_height >= height.saturating_add(maturity) {
                acc.balance = acc.balance.checked_add(amount).ok_or_else(|| {
                    "balance overflow during maturity".to_string()
                })?;
            } else {
                still_immature.push((height, amount));
            }
        }
        acc.immature_rewards = still_immature;
        acc.immature_rewards.push((block_height, reward));

        self.update_account(miner, &acc)?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), String> {
        let root = self.trie.root_hash().map_err(|e| e.to_string())?;

        let path = crate::datadir::chainstate_dir().join("root_hash.txt");
        let _ = std::fs::write(path, hex::encode(root));

        self.adapter.flush().map_err(|e| e.to_string())
    }
}

impl std::fmt::Debug for StateDB {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateDB").finish()
    }
}
