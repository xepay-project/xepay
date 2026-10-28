// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// crypto.rs
// Cryptographic helpers for XEPAY (XEPY).
// The sole hashing primitive is SHA3-256.

use sha3::{Digest, Sha3_256};
use serde::Serialize;
use bincode;

/// Compute a SHA3-256 hash over any serializable value.
/// Returns the digest as a lowercase hex string.
pub fn sha3_256_hash<T: Serialize>(data: &T) -> String {
    // Serialize deterministically, then hash the bytes.
    let encoded = bincode::serialize(data).expect("Failed to serialize data");
    let mut hasher = Sha3_256::new();
    hasher.update(&encoded);
    hex::encode(hasher.finalize())
}

/// Hash a raw byte slice with SHA3-256.
pub fn sha3_256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}
