// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// wallet.rs
//
// A wallet is a signing key plus a nonce counter. In a full deployment,
// keys would be persisted encrypted to disk; for v0.2 they live in memory
// and can be exported/imported as hex.

use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Wallet {
    /// Secret key bytes (hex). In production, encrypt at rest.
    secret_hex: String,
    /// Next nonce this wallet should use.
    pub next_nonce: u64,
}

impl Wallet {
    /// Generate a fresh wallet with a cryptographically secure keypair.
    pub fn generate() -> Self {
        let sk = SigningKey::generate(&mut OsRng);
        Wallet {
            secret_hex: hex::encode(sk.to_bytes()),
            next_nonce: 0,
        }
    }

    /// Import an existing wallet from a hex-encoded 32-byte secret key.
    pub fn from_secret_hex(secret_hex: &str) -> Result<Self, String> {
        let bytes = hex::decode(secret_hex).map_err(|e| e.to_string())?;
        if bytes.len() != 32 {
            return Err("secret key must be 32 bytes".into());
        }
        SigningKey::from_bytes(&bytes.try_into().unwrap());
        Ok(Wallet {
            secret_hex: secret_hex.to_string(),
            next_nonce: 0,
        })
    }

    /// Recover the signing key.
    pub fn signing_key(&self) -> SigningKey {
        let bytes = hex::decode(&self.secret_hex).unwrap();
        SigningKey::from_bytes(&bytes.try_into().unwrap())
    }

    /// Hex-encoded ed25519 public key — the wallet's on-chain address.
    pub fn address(&self) -> String {
        hex::encode(self.signing_key().verifying_key().to_bytes())
    }

    /// Export the secret key (for backup).
    pub fn export_secret(&self) -> String {
        self.secret_hex.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_is_32_byte_hex() {
        let w = Wallet::generate();
        let addr = w.address();
        assert_eq!(addr.len(), 64);
        assert!(addr.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn export_import_roundtrip() {
        let w = Wallet::generate();
        let exported = w.export_secret();
        let w2 = Wallet::from_secret_hex(&exported).unwrap();
        assert_eq!(w.address(), w2.address());
    }
}
