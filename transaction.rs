// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// transaction.rs
// Signed transactions for XEPAY (ticker: XEPY).
//
// Each transaction is signed by the sender's ed25519 private key. The
// signature covers all fields except `signature` itself, preventing any
// post-hoc mutation of from/to/amount/fee/nonce.

use serde::{Serialize, Deserialize};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use crate::crypto::sha3_256_hash;

/// A signed value transfer on XEPAY.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    /// Sender's public key (hex-encoded ed25519 VerifyingKey).
    pub from: String,
    /// Recipient's public key (hex-encoded ed25519 VerifyingKey).
    pub to: String,
    /// Amount in the smallest unit (satoshi-equivalent, "XEPYsat").
    pub amount: u64,
    /// Fee paid to the miner, same unit as `amount`.
    pub fee: u64,
    /// Monotonic per-sender counter to prevent replay.
    pub nonce: u64,
    /// Hex-encoded 64-byte ed25519 signature over the sighash.
    pub signature: String,
}

impl Transaction {
    /// Compute the sighash over every field except `signature`.
    pub fn sighash(&self) -> String {
        #[derive(Serialize)]
        struct Signable<'a> {
            from: &'a str,
            to: &'a str,
            amount: u64,
            fee: u64,
            nonce: u64,
        }
        sha3_256_hash(&Signable {
            from: &self.from,
            to: &self.to,
            amount: self.amount,
            fee: self.fee,
            nonce: self.nonce,
        })
    }

    /// Create and sign a new transaction.
    pub fn new_signed(
        signing_key: &SigningKey,
        to_pubkey_hex: &str,
        amount: u64,
        fee: u64,
        nonce: u64,
    ) -> Self {
        let mut tx = Transaction {
            from: hex::encode(signing_key.verifying_key().to_bytes()),
            to: to_pubkey_hex.to_string(),
            amount,
            fee,
            nonce,
            signature: String::new(),
        };
        let digest = tx.sighash();
        let sig: Signature = signing_key.sign(digest.as_bytes());
        tx.signature = hex::encode(sig.to_bytes());
        tx
    }

    /// Verify the signature and basic sanity of the transaction.
    pub fn verify(&self) -> bool {
        // Reject malformed inputs early.
        if self.from.is_empty() || self.to.is_empty() || self.amount == 0 {
            return false;
        }
        if self.from == self.to {
            return false;
        }

        // Decode the sender's public key.
        let from_bytes = match hex::decode(&self.from) {
            Ok(b) if b.len() == 32 => b,
            _ => return false,
        };
        let verifying_key = match VerifyingKey::from_bytes(
            &from_bytes.try_into().unwrap(),
        ) {
            Ok(k) => k,
            Err(_) => return false,
        };

        // Decode the signature.
        let sig_bytes = match hex::decode(&self.signature) {
            Ok(b) if b.len() == 64 => b,
            _ => return false,
        };
        let signature = Signature::from_bytes(&sig_bytes.try_into().unwrap());

        // Verify against the sighash.
        verifying_key
            .verify(self.sighash().as_bytes(), &signature)
            .is_ok()
    }

    /// Total amount debited from the sender (amount + fee).
    pub fn total_debit(&self) -> u64 {
        self.amount.saturating_add(self.fee)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    fn keypair() -> SigningKey {
        SigningKey::generate(&mut OsRng)
    }

    #[test]
    fn valid_signature_passes() {
        let a = keypair();
        let b = keypair();
        let tx = Transaction::new_signed(
            &a,
            &hex::encode(b.verifying_key().to_bytes()),
            100,
            1,
            0,
        );
        assert!(tx.verify());
    }

    #[test]
    fn tampered_amount_fails() {
        let a = keypair();
        let b = keypair();
        let mut tx = Transaction::new_signed(
            &a,
            &hex::encode(b.verifying_key().to_bytes()),
            100,
            1,
            0,
        );
        tx.amount = 999;
        assert!(!tx.verify());
    }

    #[test]
    fn tampered_fee_fails() {
        let a = keypair();
        let b = keypair();
        let mut tx = Transaction::new_signed(
            &a,
            &hex::encode(b.verifying_key().to_bytes()),
            100,
            1,
            0,
        );
        tx.fee = 999;
        assert!(!tx.verify());
    }
}
