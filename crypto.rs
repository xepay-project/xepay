// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// crypto.rs
// Cryptographic helpers for XEPAY (ticker: XEPY).
// Sole hashing primitive: SHA3-256. Also provides Merkle root construction.

use sha3::{Digest, Sha3_256};
use serde::Serialize;
use bincode;

/// Compute a SHA3-256 hash over any serializable value.
/// Returns the digest as a lowercase hex string.
pub fn sha3_256_hash<T: Serialize>(data: &T) -> String {
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

/// Build a real binary Merkle root over a list of transaction hashes.
/// Uses the standard "hash pairs upward" construction (not Bitcoin's
/// duplicate-last-block quirk, which is a known CVE source).
///
/// Empty list → hash of empty string. Single item → that item's hash.
pub fn merkle_root(leaves: &[String]) -> String {
    if leaves.is_empty() {
        return sha3_256_bytes(b"");
    }
    if leaves.len() == 1 {
        return leaves[0].clone();
    }

    let mut current: Vec<String> = leaves.to_vec();
    while current.len() > 1 {
        let mut next = Vec::with_capacity((current.len() + 1) / 2);
        for chunk in current.chunks(2) {
            if chunk.len() == 2 {
                // Concatenate two child hex strings, then hash the result
                let combined = format!("{}{}", chunk[0], chunk[1]);
                next.push(sha3_256_bytes(combined.as_bytes()));
            } else {
                // Odd number: promote the last leaf unchanged
                next.push(chunk[0].clone());
            }
        }
        current = next;
    }
    current.into_iter().next().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_deterministic() {
        assert_eq!(sha3_256_hash(&"hello"), sha3_256_hash(&"hello"));
    }

    #[test]
    fn hash_is_64_hex_chars() {
        let h = sha3_256_hash(&"hello");
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn merkle_single_leaf_is_identity() {
        let leaf = sha3_256_bytes(b"a");
        assert_eq!(merkle_root(&[leaf.clone()]), leaf);
    }

    #[test]
    fn merkle_two_leaves_differ_from_concat() {
        let a = sha3_256_bytes(b"a");
        let b = sha3_256_bytes(b"b");
        let root = merkle_root(&[a.clone(), b.clone()]);
        // Root must not equal the naive concatenation of raw hashes
        assert_ne!(root, format!("{}{}", a, b));
    }
}
