// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Hashing functions used throughout the blockchain.

use sha3::{Digest, Sha3_256};

/// Compute the SHA3-256 digest of `data` and return it as a lowercase hex string.
pub fn sha3_256_hex(data: &[u8]) -> String {
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}
