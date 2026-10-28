// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Hierarchical Deterministic (HD) Wallet for XEPAY.
//!
//! This module follows the XEPAY HD Wallet Standard, which uses:
//! - Mnemonic phrases (12 or 24 words) for seed generation.
//! - HMAC-SHA3-256 based key derivation (extended to 64 bytes).
//! - Derivation paths:
//!   - Legacy: m/44'/9999'/0'/0/{index}
//!   - Bech32: m/84'/9999'/0'/0/{index}
//! - Separate wallet file: hdwallet.dat

use bip39::{Language, Mnemonic};
use hmac::{Hmac, Mac};
use sha3::Sha3_256;
use std::path::PathBuf;

use crate::datadir;
use crate::secp256k1::address;
use crate::secp256k1::keypair::KeyPair;
use crate::utils::now_millis;

type HmacSha3_256 = Hmac<Sha3_256>;

/// HD Wallet file name.
pub const HD_WALLET_FILE: &str = "hdwallet.dat";

/// Coin type used in derivation paths.
pub const COIN_TYPE: u32 = 9999;

/// Address type for derivation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AddressType {
    Legacy,
    Bech32,
}

impl AddressType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "legacy" => Some(AddressType::Legacy),
            "bech32" => Some(AddressType::Bech32),
            _ => None,
        }
    }

    /// Return the purpose number for this address type.
    pub fn purpose(&self) -> u32 {
        match self {
            AddressType::Legacy => 44,
            AddressType::Bech32 => 84,
        }
    }
}

/// A derived HD address entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HdAddressEntry {
    pub address: String,
    pub public_key: String,
    pub secret: String,
    pub path: String,
    pub index: u32,
    pub address_type: AddressType,
    pub created_at: u128,
}

/// HD Wallet structure.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HdWallet {
    pub mnemonic: String,
    pub passphrase: String,
    pub created_at: u128,
    pub entries: Vec<HdAddressEntry>,
}

impl HdWallet {
    /// Generate a new HD wallet with the given word count (12 or 24).
    pub fn generate(word_count: usize) -> Result<Self, String> {
        if word_count != 12 && word_count != 24 {
            return Err("word count must be 12 or 24".into());
        }

        let mnemonic = Mnemonic::generate_in(Language::English, word_count)
            .map_err(|e| format!("failed to generate mnemonic: {}", e))?;

        Ok(HdWallet {
            mnemonic: mnemonic.to_string(),
            passphrase: String::new(),
            created_at: now_millis(),
            entries: Vec::new(),
        })
    }

    /// Restore an HD wallet from an existing mnemonic.
    pub fn from_mnemonic(phrase: &str, passphrase: Option<&str>) -> Result<Self, String> {
        let mnemonic = Mnemonic::parse_in(Language::English, phrase)
            .map_err(|e| format!("invalid mnemonic: {}", e))?;

        Ok(HdWallet {
            mnemonic: mnemonic.to_string(),
            passphrase: passphrase.unwrap_or("").to_string(),
            created_at: now_millis(),
            entries: Vec::new(),
        })
    }

    /// Import an existing address entry into the HD wallet.
    pub fn import_entry(&mut self, entry: HdAddressEntry) -> Result<(), String> {
        if self.entries.iter().any(|e| e.address == entry.address) {
            return Err(format!("address {} already exists in HD wallet", entry.address));
        }
        self.entries.push(entry);
        Ok(())
    }

    /// Derive the seed from the mnemonic and passphrase.
    fn seed(&self) -> [u8; 64] {
        let mnemonic = Mnemonic::parse_in(Language::English, &self.mnemonic)
            .expect("stored mnemonic is always valid");
        mnemonic.to_seed(&self.passphrase)
    }

    /// Derive 64 bytes of key material using HMAC-SHA3-256.
    /// Since HMAC-SHA3-256 produces 32 bytes, we call it twice with
    /// different counters to obtain 64 bytes.
    fn hmac_sha3_256_64(key: &[u8], data: &[u8]) -> [u8; 64] {
        let mut mac = HmacSha3_256::new_from_slice(key)
            .expect("HMAC can take key of any size");
        mac.update(data);
        mac.update(&[0x00]);
        let first = mac.finalize().into_bytes();

        let mut mac = HmacSha3_256::new_from_slice(key)
            .expect("HMAC can take key of any size");
        mac.update(data);
        mac.update(&[0x01]);
        let second = mac.finalize().into_bytes();

        let mut out = [0u8; 64];
        out[..32].copy_from_slice(&first);
        out[32..].copy_from_slice(&second);
        out
    }

    /// Derive the master private key from the seed using HMAC-SHA3-256.
    fn master_key(&self) -> ([u8; 32], [u8; 32]) {
        let seed = self.seed();
        let result = Self::hmac_sha3_256_64(b"XEPAY seed", &seed);

        let mut private_key = [0u8; 32];
        let mut chain_code = [0u8; 32];
        private_key.copy_from_slice(&result[..32]);
        chain_code.copy_from_slice(&result[32..]);
        (private_key, chain_code)
    }

    /// Derive a child private key at the given index using HMAC-SHA3-256.
    fn derive_child_key(
        &self,
        private_key: &[u8; 32],
        chain_code: &[u8; 32],
        index: u32,
        hardened: bool,
    ) -> Result<([u8; 32], [u8; 32]), String> {
        let mut data = Vec::new();
        if hardened {
            data.push(0x00);
            data.extend_from_slice(private_key);
        } else {
            let kp = KeyPair::from_secret(private_key)?;
            data.extend_from_slice(&kp.public_key);
        }
        data.extend_from_slice(&(index | if hardened { 0x80000000 } else { 0 }).to_be_bytes());

        let result = Self::hmac_sha3_256_64(chain_code, &data);

        let mut child_key = [0u8; 32];
        let mut child_chain = [0u8; 32];
        child_key.copy_from_slice(&result[..32]);
        child_chain.copy_from_slice(&result[32..]);

        Ok((child_key, child_chain))
    }

    /// Derive a private key at the given path (e.g., m/44'/9999'/0'/0/0).
    fn derive_path(&self, path: &str) -> Result<[u8; 32], String> {
        let (mut key, mut chain) = self.master_key();

        let parts: Vec<&str> = path.trim_start_matches("m/").split('/').collect();
        for part in parts {
            if part.is_empty() {
                continue;
            }
            let (index_str, hardened) = if part.ends_with('\'') {
                (&part[..part.len() - 1], true)
            } else {
                (part, false)
            };
            let index: u32 = index_str
                .parse()
                .map_err(|_| format!("invalid path component: {}", part))?;
            let (new_key, new_chain) = self.derive_child_key(&key, &chain, index, hardened)?;
            key = new_key;
            chain = new_chain;
        }
        Ok(key)
    }

    /// Derive an address at the given index and type.
    pub fn derive_address(
        &mut self,
        index: u32,
        address_type: AddressType,
    ) -> Result<HdAddressEntry, String> {
        let purpose = address_type.purpose();
        let path = format!("m/{}'/{}'/0'/0/{}", purpose, COIN_TYPE, index);

        let secret_key = self.derive_path(&path)?;
        let kp = KeyPair::from_secret(&secret_key)?;

        let address = match address_type {
            AddressType::Legacy => address::address_from_public_key(&kp.public_key)?,
            AddressType::Bech32 => address::address_from_public_key_bech32(&kp.public_key)?,
        };

        let entry = HdAddressEntry {
            address,
            public_key: kp.public_key_hex(),
            secret: kp.secret_key_hex(),
            path,
            index,
            address_type,
            created_at: now_millis(),
        };

        self.entries.push(entry.clone());
        Ok(entry)
    }

    /// Return the default file path for the HD wallet.
    pub fn default_path() -> PathBuf {
        datadir::datadir().join(HD_WALLET_FILE)
    }

    /// Save the HD wallet to disk.
    pub fn save_default(&self) -> std::io::Result<()> {
        datadir::ensure_dirs()?;
        let path = Self::default_path();
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    /// Load the HD wallet from disk.
    pub fn load_default() -> std::io::Result<Self> {
        let path = Self::default_path();
        let data = std::fs::read_to_string(path)?;
        let wallet: HdWallet = serde_json::from_str(&data).map_err(std::io::Error::other)?;
        Ok(wallet)
    }

    /// Check if an HD wallet file exists.
    pub fn exists_default() -> bool {
        Self::default_path().exists()
    }
}
