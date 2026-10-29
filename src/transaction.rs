// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Account-based transaction model with per-address nonces.

use serde::{Deserialize, Serialize};
use xepay_crypto::sha3_256_hex;

use crate::consensus::spec;
use crate::secp256k1::address;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub txid: String,
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub fee: u64,
    pub nonce: u64,
    pub note: String,
    pub public_key: String,
    pub signature: String,
    pub output_script: String,
}

impl Transaction {
    pub fn new(from: &str, to: &str, amount: u64, fee: u64, nonce: u64) -> Self {
        Self::new_with_note(from, to, amount, fee, nonce, String::new())
    }

    pub fn new_with_note(
        from: &str,
        to: &str,
        amount: u64,
        fee: u64,
        nonce: u64,
        note: String,
    ) -> Self {
        Self {
            txid: String::new(),
            from: from.into(),
            to: to.into(),
            amount,
            fee,
            nonce,
            note,
            public_key: String::new(),
            signature: String::new(),
            output_script: String::new(),
        }
        .finalize()
    }

    fn finalize(mut self) -> Self {
        self.txid = self.compute_txid();
        self
    }

    pub fn coinbase(note: String) -> Self {
        Self {
            txid: String::new(),
            from: "COINBASE".into(),
            to: "MINER".into(),
            amount: 0,
            fee: 0,
            nonce: 0,
            note,
            public_key: String::new(),
            signature: String::new(),
            output_script: String::new(),
        }
        .finalize()
    }

    pub fn secret(note: String) -> Self {
        Self {
            txid: String::new(),
            from: "GENESIS".into(),
            to: "XEPC".into(),
            amount: 0,
            fee: 0,
            nonce: 0,
            note,
            public_key: String::new(),
            signature: String::new(),
            output_script: String::new(),
        }
        .finalize()
    }

    /// Compute the transaction ID.
    ///
    /// The txid is a hash of every field that affects the transaction's
    /// validity, including `public_key` and `output_script`. This ensures
    /// that any change to the transaction produces a different txid, which
    /// prevents txid malleability. Reference node implementations follow
    /// the same principle: the txid commits to the full transaction data,
    /// including the script that carries the public key.
    pub fn compute_txid(&self) -> String {
        let payload = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.from,
            self.to,
            self.amount,
            self.fee,
            self.nonce,
            self.note,
            self.public_key,
            self.signature,
            self.output_script,
        );
        sha3_256_hex(payload.as_bytes())
    }

    pub fn signing_hash(&self) -> String {
        let chain_id = spec::current_network().magic_hex();
        let payload = format!(
            "{}|{}|{}|{}|{}|{}|{}",
            chain_id,
            self.from,
            self.to,
            self.amount,
            self.fee,
            self.nonce,
            self.note,
        );
        sha3_256_hex(payload.as_bytes())
    }

    pub fn sign(&mut self, secret_hex: &str) -> Result<(), String> {
        use crate::secp256k1::{keypair::KeyPair, sign};

        let sk = hex::decode(secret_hex).map_err(|e| e.to_string())?;
        let kp = KeyPair::from_secret(&sk)?;

        self.public_key = kp.public_key_hex();
        self.txid = self.compute_txid();

        let msg = hex::decode(self.signing_hash()).map_err(|e| e.to_string())?;
        let sig = sign::sign(&kp.secret_key, &msg)?;
        self.signature = hex::encode(sig);

        Ok(())
    }

    pub fn verify_signature(&self) -> bool {
        crate::validation::rule_signature(self).is_ok()
    }

    pub fn expected_address(&self) -> Option<String> {
        if self.public_key.is_empty() {
            return None;
        }
        let pk = hex::decode(&self.public_key).ok()?;
        address::address_from_public_key(&pk).ok()
    }

    pub fn is_valid(&self) -> bool {
        self.txid == self.compute_txid()
            && (address::verify_legacy_address(&self.from)
                || address::verify_bech32_address(&self.from))
            && (address::verify_legacy_address(&self.to)
                || address::verify_bech32_address(&self.to))
            && self.from != self.to
            && self.amount > 0
            && self.note.len() <= crate::consensus::rules::MAX_NOTE_LEN
            && self.verify_signature()
            && self
                .expected_address()
                .map(|a| a == self.from)
                .unwrap_or(false)
    }
}
