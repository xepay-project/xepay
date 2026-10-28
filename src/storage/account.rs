// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Account structure for the Ethereum-style state trie.

use rlp::{Decodable, DecoderError, Encodable, Rlp, RlpStream};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Account {
    pub nonce: u64,
    pub balance: u64,
    pub immature_rewards: Vec<(u64, u64)>,
    pub code: Vec<u8>,
    // Changed from HashMap to BTreeMap for deterministic RLP encoding.
    pub storage: BTreeMap<[u8; 32], [u8; 32]>,
}

impl Encodable for Account {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(5);
        s.append(&self.nonce);
        s.append(&self.balance);
        s.begin_list(self.immature_rewards.len());
        for (height, amount) in &self.immature_rewards {
            s.begin_list(2);
            s.append(height);
            s.append(amount);
        }
        s.append(&self.code);
        s.begin_list(self.storage.len());
        for (k, v) in &self.storage {
            s.begin_list(2);
            s.append(&k.to_vec());
            s.append(&v.to_vec());
        }
    }
}

impl Decodable for Account {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        let nonce: u64 = rlp.val_at(0)?;
        let balance: u64 = rlp.val_at(1)?;
        let immature_list = rlp.at(2)?;
        let mut immature_rewards = Vec::new();
        for i in 0..immature_list.item_count()? {
            let item = immature_list.at(i)?;
            let height: u64 = item.val_at(0)?;
            let amount: u64 = item.val_at(1)?;
            immature_rewards.push((height, amount));
        }
        let code: Vec<u8> = rlp.val_at(3)?;
        let storage_list = rlp.at(4)?;
        let mut storage = BTreeMap::new();
        for i in 0..storage_list.item_count()? {
            let item = storage_list.at(i)?;
            let key_vec: Vec<u8> = item.val_at(0)?;
            let value_vec: Vec<u8> = item.val_at(1)?;
            
            // Prevent panic if data from disk is corrupted or length is wrong.
            if key_vec.len() != 32 || value_vec.len() != 32 {
                return Err(DecoderError::Custom("invalid storage key/value length"));
            }

            let mut key = [0u8; 32];
            let mut value = [0u8; 32];
            key.copy_from_slice(&key_vec);
            value.copy_from_slice(&value_vec);
            storage.insert(key, value);
        }
        Ok(Account {
            nonce,
            balance,
            immature_rewards,
            code,
            storage,
        })
    }
}
