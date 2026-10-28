// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// crypto.rs
// Cryptographic helpers for XEPY (ticker: XEPY).
// Uses SHA3-256 as the base hashing algorithm.

use sha3::{Digest, Sha3_256};
use serde::Serialize;
use bincode;

/// Compute a SHA3-256 hash over any serializable data.
/// Returns the hash as a lowercase hex string.
pub fn sha3_256_hash<T: Serialize>(data: &T) -> String {
    // Serialize the input into bytes
    let encoded = bincode::serialize(data).expect("Failed to serialize data");

    // Feed the bytes into SHA3-256
    let mut hasher = Sha3_256::new();
    hasher.update(&encoded);
    let result = hasher.finalize();

    // Return the digest as a hex string
    hex::encode(result)
}

/// Hash a raw byte slice with SHA3-256 (useful for Merkle trees later).
pub fn sha3_256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}
