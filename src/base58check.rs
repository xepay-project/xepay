// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Base58Check encoding/decoding for XEPAY Legacy addresses.

use xepay_crypto::sha3_256_hex;

pub fn encode(payload: &[u8]) -> String {
    let mut body = payload.to_vec();
    let checksum = compute_checksum(payload);
    body.extend_from_slice(&checksum);
    bs58::encode(body).into_string()
}

pub fn decode(address: &str) -> Option<Vec<u8>> {
    let decoded = bs58::decode(address).into_vec().ok()?;
    if decoded.len() < 4 {
        return None;
    }
    let (payload, checksum) = decoded.split_at(decoded.len() - 4);
    let expected = compute_checksum(payload);
    if checksum != expected {
        return None;
    }
    Some(payload.to_vec())
}

fn compute_checksum(payload: &[u8]) -> [u8; 4] {
    let first_hash = sha3_256_hex(payload);
    let first_bytes = hex::decode(&first_hash).expect("sha3 hex is always valid");
    let second_hash = sha3_256_hex(&first_bytes);
    let second_bytes = hex::decode(&second_hash).expect("sha3 hex is always valid");
    let mut checksum = [0u8; 4];
    checksum.copy_from_slice(&second_bytes[..4]);
    checksum
}
