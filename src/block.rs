// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Block structure and related operations.

use crate::blockchain::{self, Blockchain};
use crate::consensus::spec::{self, Network};
use crate::consensus::{genesis, rules};
use crate::datadir;
use crate::pow;
use crate::transaction::Transaction;
use crate::utils::{format_datetime, now_millis};
use serde::{Deserialize, Serialize};
use xepay_crypto::sha3_256_hex;

pub const BLOCK_VERSION: u32 = rules::BLOCK_VERSION;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub nversion: u32,
    pub index: u64,
    pub timestamp: u128,
    pub mined_at: String,
    pub mined_for: String,
    pub previous_hash: String,
    pub merkle_root: String,
    pub state_root: String,
    pub hash: String,
    pub nonce: u64,
    pub nbits: u32,
    pub reward: u64,
    pub subsidy: u64,
    pub total_fees: u64,
    pub coinbase_txid: String,
    pub blockchain_size: u64,
    pub genesis_output_script: String,
    pub miner_pubkey: String,
    pub miner_signature: String,
    #[serde(with = "crate::utils::u128_string")]
    pub chainwork: u128,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn compute_merkle_root(transactions: &[Transaction]) -> String {
        if transactions.is_empty() {
            return sha3_256_hex(b"empty");
        }

        let mut layer: Vec<[u8; 32]> = transactions
            .iter()
            .filter_map(|tx| {
                let bytes = hex::decode(&tx.txid).ok()?;
                if bytes.len() != 32 {
                    return None;
                }
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                Some(arr)
            })
            .collect();

        if layer.is_empty() {
            return sha3_256_hex(b"empty");
        }

        while layer.len() > 1 {
            if layer.len() % 2 == 1 {
                let last = *layer.last().unwrap();
                layer.push(last);
            }

            let mut next: Vec<[u8; 32]> = Vec::with_capacity(layer.len() / 2);
            for pair in layer.chunks(2) {
                let mut buf = [0u8; 64];
                buf[..32].copy_from_slice(&pair[0]);
                buf[32..].copy_from_slice(&pair[1]);
                let hash = sha3_256_hex(&buf);
                let bytes = hex::decode(&hash).unwrap();
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                next.push(arr);
            }
            layer = next;
        }

        hex::encode(layer[0])
    }

    pub fn compute_work(nbits: u32) -> u128 {
        let target = pow::nbits_to_target(nbits);

        // chainwork = 2^256 / (target + 1).
        //
        // Since chainwork is stored as u128 (not u256), we must handle
        // targets that exceed u128::MAX. A target larger than u128::MAX
        // means the difficulty is trivially low, so the work contributed
        // by the block is the minimum possible (1).
        //
        // The first 16 bytes of the target are the most significant.
        // If any of them is non-zero, the target is > u128::MAX.
        for &b in &target[0..16] {
            if b != 0 {
                return 1;
            }
        }

        // Read the lower 16 bytes of the target.
        let mut target_u128: u128 = 0;
        for &b in &target[16..32] {
            target_u128 = (target_u128 << 8) | (b as u128);
        }

        // If the target is zero, the work is maximal. This should never
        // happen for a valid nbits because validation rejects it.
        if target_u128 == 0 {
            return u128::MAX;
        }

        // chainwork = u128::MAX / (target + 1).
        u128::MAX / (target_u128.saturating_add(1))
    }

    pub fn new(
        index: u64,
        previous_hash: String,
        reward: u64,
        nbits: u32,
        transactions: Vec<Transaction>,
    ) -> Self {
        Self::new_for(
            index,
            previous_hash,
            reward,
            0,
            nbits,
            transactions,
            String::new(),
            String::new(),
            0,
        )
        .expect("failed to create block in Block::new")
    }

    pub fn new_for(
        index: u64,
        previous_hash: String,
        subsidy: u64,
        total_fees: u64,
        nbits: u32,
        transactions: Vec<Transaction>,
        mined_for: String,
        state_root: String,
        previous_chainwork: u128,
    ) -> Result<Self, String> {
        let ts = now_millis();
        let coinbase_txid = sha3_256_hex(format!("{}|{}|{}", index, mined_for, ts).as_bytes());
        let merkle_root = Self::compute_merkle_root(&transactions);

        let reward = subsidy
            .checked_add(total_fees)
            .ok_or_else(|| "block reward overflow".to_string())?;

        let size = (previous_hash.len()
            + merkle_root.len()
            + coinbase_txid.len()
            + transactions.iter().map(|t| t.txid.len()).sum::<usize>()
            + 256) as u64;

        let block_work = Self::compute_work(nbits);

        let chainwork = previous_chainwork
            .checked_add(block_work)
            .unwrap_or(u128::MAX);

        let mut b = Block {
            nversion: rules::BLOCK_VERSION,
            index,
            timestamp: ts,
            mined_at: format_datetime(ts),
            mined_for,
            previous_hash,
            merkle_root,
            state_root,
            hash: String::new(),
            nonce: 0,
            nbits,
            reward,
            subsidy,
            total_fees,
            coinbase_txid,
            blockchain_size: size,
            genesis_output_script: String::new(),
            miner_pubkey: String::new(),
            miner_signature: String::new(),
            chainwork,
            transactions,
        };
        b.hash = b.calculate_hash();
        Ok(b)
    }

    pub fn genesis_unchecked() -> Self {
        Self::genesis_mined_for(spec::current_network()).expect("genesis mining failed")
    }

    pub fn genesis_unchecked_for(network: Network) -> Self {
        Self::genesis_mined_for(network).expect("genesis mining failed")
    }

    fn genesis_mined_for(network: Network) -> Result<Self, String> {
        let ts = network.genesis_timestamp();
        let coinbase_txid = blockchain::genesis_coinbase_txid_for(network);
        let genesis_output_script = genesis::GENESIS_OUTPUT_SCRIPT.to_string();
        let nbits = network.genesis_nbits();

        let transactions = vec![
            Transaction::coinbase(genesis::GENESIS_MESSAGE.into()),
            Transaction::secret(genesis::GENESIS_SECRET.into()),
        ];
        let merkle_root = Self::compute_merkle_root(&transactions);

        let size = (genesis_output_script.len()
            + merkle_root.len()
            + coinbase_txid.len()
            + transactions.iter().map(|t| t.txid.len()).sum::<usize>()
            + 256) as u64;

        let subsidy = crate::consensus::spec::INITIAL_REWARD;
        let total_fees = 0;

        let block_work = Self::compute_work(nbits);

        let mut b = Block {
            nversion: rules::BLOCK_VERSION,
            index: 0,
            timestamp: ts,
            mined_at: format_datetime(ts),
            mined_for: "GENESIS".to_string(),
            previous_hash: "0".repeat(64),
            merkle_root,
            state_root: String::new(),
            hash: String::new(),
            nonce: 0,
            nbits,
            reward: subsidy,
            subsidy,
            total_fees,
            coinbase_txid,
            blockchain_size: size,
            genesis_output_script,
            miner_pubkey: String::new(),
            miner_signature: String::new(),
            chainwork: block_work,
            transactions,
        };

        if !pow::mine(&mut b, nbits) {
            return Err("genesis mining failed: nonce exhausted".to_string());
        }

        Ok(b)
    }

    pub fn genesis() -> Self {
        Self::genesis_for(spec::current_network())
    }

    #[cfg(feature = "skip-pow")]
    fn is_test_mode() -> bool {
        true
    }

    pub fn genesis_for(network: Network) -> Self {
        #[cfg(feature = "skip-pow")]
        {
            if Self::is_test_mode() {
                return Self::genesis_precomputed_for(network);
            }
        }

        let b = Self::genesis_mined_for(network).unwrap_or_else(|e| {
            eprintln!("FATAL: genesis mining failed: {}", e);
            std::process::exit(1);
        });

        let hash_ok = b.hash == genesis::expected_genesis_hash_for(network);
        let merkle_ok = b.merkle_root == genesis::expected_genesis_merkle_root_for(network);
        let ts_ok = b.timestamp == genesis::expected_genesis_timestamp_for(network);
        let nonce_ok = b.nonce == genesis::expected_genesis_nonce_for(network);

        if hash_ok && merkle_ok && ts_ok && nonce_ok {
            return b;
        }

        eprintln!();
        eprintln!("genesis mismatch for network '{}'", network.name());
        eprintln!();
        eprintln!("pub const EXPECTED_GENESIS_HASH: &str =");
        eprintln!("    \"{}\";", b.hash);
        eprintln!();
        eprintln!("pub const EXPECTED_GENESIS_MERKLE_ROOT: &str =");
        eprintln!("    \"{}\";", b.merkle_root);
        eprintln!();
        eprintln!(
            "pub const EXPECTED_GENESIS_TIMESTAMP: u128 = {};",
            b.timestamp
        );
        eprintln!();
        eprintln!("pub const EXPECTED_GENESIS_NONCE: u64 = {};", b.nonce);
        eprintln!();

        std::process::exit(1);
    }

    #[cfg(feature = "skip-pow")]
    fn genesis_precomputed_for(network: Network) -> Self {
        let ts = network.genesis_timestamp();
        let coinbase_txid = blockchain::genesis_coinbase_txid_for(network);
        let genesis_output_script = genesis::GENESIS_OUTPUT_SCRIPT.to_string();
        let nbits = network.genesis_nbits();

        let transactions = vec![
            Transaction::coinbase(genesis::GENESIS_MESSAGE.into()),
            Transaction::secret(genesis::GENESIS_SECRET.into()),
        ];

        let merkle_root = genesis::expected_genesis_merkle_root_for(network).to_string();
        let hash = genesis::expected_genesis_hash_for(network).to_string();
        let nonce = genesis::expected_genesis_nonce_for(network);

        let size = (genesis_output_script.len()
            + merkle_root.len()
            + coinbase_txid.len()
            + transactions.iter().map(|t| t.txid.len()).sum::<usize>()
            + 256) as u64;

        let subsidy = crate::consensus::spec::INITIAL_REWARD;
        let total_fees = 0;

        let block_work = Self::compute_work(nbits);

        Block {
            nversion: rules::BLOCK_VERSION,
            index: 0,
            timestamp: ts,
            mined_at: format_datetime(ts),
            mined_for: "GENESIS".to_string(),
            previous_hash: "0".repeat(64),
            merkle_root,
            state_root: String::new(),
            hash,
            nonce,
            nbits,
            reward: subsidy,
            subsidy,
            total_fees,
            coinbase_txid,
            blockchain_size: size,
            genesis_output_script,
            miner_pubkey: String::new(),
            miner_signature: String::new(),
            chainwork: block_work,
            transactions,
        }
    }

    pub fn calculate_hash(&self) -> String {
        let mut payload = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.nversion,
            self.index,
            self.timestamp,
            self.mined_at,
            self.mined_for,
            self.previous_hash,
            self.merkle_root,
            self.state_root,
            self.nonce,
            self.nbits,
            self.reward,
            self.subsidy,
            self.total_fees,
            self.coinbase_txid,
            self.blockchain_size,
            self.genesis_output_script,
            self.miner_pubkey,
            self.miner_signature,
            self.chainwork,
            self.transactions.len()
        );
        for tx in &self.transactions {
            payload.push_str(&format!(
                "\n{}|{}|{}|{}|{}|{}|{}|{}|{}",
                tx.from,
                tx.to,
                tx.amount,
                tx.fee,
                tx.nonce,
                tx.note,
                tx.output_script,
                tx.public_key,
                tx.signature
            ));
        }
        sha3_256_hex(payload.as_bytes())
    }

    pub fn reward_at(height: u64) -> u64 {
        Blockchain::calculate_reward(height)
    }

    pub fn log_add_to_wallet(&self) {
        let entry = format!(
            "AddToWallet\n  mined_at : {}\n  txid     : {}\n  hash     : {}",
            self.mined_at, self.coinbase_txid, self.hash
        );

        println!("{}", entry);
        datadir::append_debug_log(&entry);
    }
}
