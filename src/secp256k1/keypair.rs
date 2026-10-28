// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! secp256k1 key pair generation and reconstruction.

use rand::rngs::OsRng;
use secp256k1::{PublicKey, Secp256k1, SecretKey};

use super::{PUBLIC_KEY_SIZE, SECRET_KEY_SIZE};

/// A secp256k1 key pair.
#[derive(Debug, Clone)]
pub struct KeyPair {
    pub public_key: Vec<u8>,
    pub secret_key: Vec<u8>,
}

impl KeyPair {
    /// Generate a new key pair using the operating system random number generator.
    pub fn generate() -> Result<Self, String> {
        let secp = Secp256k1::new();
        let (secret, public) = secp.generate_keypair(&mut OsRng);

        Ok(KeyPair {
            public_key: public.serialize().to_vec(),
            secret_key: secret.secret_bytes().to_vec(),
        })
    }

    /// Construct a key pair from a public key and a secret key.
    /// The function verifies that the public key matches the secret key.
    pub fn from_bytes(public_key: &[u8], secret_key: &[u8]) -> Result<Self, String> {
        if public_key.len() != PUBLIC_KEY_SIZE {
            return Err(format!(
                "public key must be {} bytes, got {}",
                PUBLIC_KEY_SIZE,
                public_key.len()
            ));
        }
        if secret_key.len() != SECRET_KEY_SIZE {
            return Err(format!(
                "secret key must be {} bytes, got {}",
                SECRET_KEY_SIZE,
                secret_key.len()
            ));
        }

        let derived = Self::from_secret(secret_key)?;
        if derived.public_key != public_key {
            return Err("public key does not match the secret key".to_string());
        }
        Ok(derived)
    }

    /// Reconstruct a key pair from a secret key.
    pub fn from_secret(secret_bytes: &[u8]) -> Result<Self, String> {
        if secret_bytes.len() != SECRET_KEY_SIZE {
            return Err(format!(
                "secret key must be {} bytes, got {}",
                SECRET_KEY_SIZE,
                secret_bytes.len()
            ));
        }

        let secp = Secp256k1::new();
        let secret = SecretKey::from_slice(secret_bytes)
            .map_err(|e| format!("invalid secret key: {}", e))?;
        let public = PublicKey::from_secret_key(&secp, &secret);

        Ok(KeyPair {
            public_key: public.serialize().to_vec(),
            secret_key: secret.secret_bytes().to_vec(),
        })
    }

    /// Return the public key as a lowercase hexadecimal string.
    pub fn public_key_hex(&self) -> String {
        hex::encode(&self.public_key)
    }

    /// Return the secret key as a lowercase hexadecimal string.
    pub fn secret_key_hex(&self) -> String {
        hex::encode(&self.secret_key)
    }
}
