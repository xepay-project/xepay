// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! AES-256-GCM encryption and PBKDF2 key derivation for wallet security.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

/// Derive a 32-byte key from a password and salt using PBKDF2-HMAC-SHA256.
/// The iteration count is set to 600,000 as recommended by OWASP for 2023.
pub fn derive_key(password: &str, salt_hex: &str) -> [u8; 32] {
    let salt = hex::decode(salt_hex).unwrap_or_default();
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, 600_000, &mut key);
    key
}

/// Encrypt a string using AES-256-GCM. Returns base64 ciphertext.
pub fn aes_encrypt(key: &[u8; 32], nonce_bytes: &[u8], plaintext: &str) -> Result<String, String> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("encryption failed: {}", e))?;
    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &ciphertext,
    ))
}

/// Decrypt a base64 ciphertext using AES-256-GCM.
pub fn aes_decrypt(
    key: &[u8; 32],
    nonce_bytes: &[u8],
    ciphertext_b64: &str,
) -> Result<String, String> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(nonce_bytes);
    let ciphertext =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, ciphertext_b64)
            .map_err(|e| format!("base64 decode failed: {}", e))?;
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| format!("decryption failed: {}", e))?;
    String::from_utf8(plaintext).map_err(|e| e.to_string())
}
