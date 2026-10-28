// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Address derivation for XEPAY (Legacy and Native SegWit).

use xepay_crypto::sha3_256_hex;

use crate::base58check;
use crate::bech32;
use crate::blockchain;

use super::PUBLIC_KEY_SIZE;

pub fn address_from_public_key(public_key: &[u8]) -> Result<String, String> {
    if public_key.len() != PUBLIC_KEY_SIZE {
        return Err(format!(
            "public key must be {} bytes, got {}",
            PUBLIC_KEY_SIZE,
            public_key.len()
        ));
    }

    let payload = [public_key, b"addr"].concat();
    let hash_hex = sha3_256_hex(&payload);
    let hash_bytes = hex::decode(&hash_hex).expect("sha3 hex is always valid");
    let hash20 = &hash_bytes[..20];

    let prefix = blockchain::legacy_prefix_byte();
    let mut body = vec![prefix];
    body.extend_from_slice(hash20);

    Ok(base58check::encode(&body))
}

pub fn verify_legacy_address(address: &str) -> bool {
    let payload = match base58check::decode(address) {
        Some(p) => p,
        None => return false,
    };
    if payload.len() != 21 {
        return false;
    }
    let expected_prefix = blockchain::legacy_prefix_byte();
    payload[0] == expected_prefix
}

pub fn address_from_public_key_bech32(public_key: &[u8]) -> Result<String, String> {
    if public_key.len() != PUBLIC_KEY_SIZE {
        return Err(format!(
            "public key must be {} bytes, got {}",
            PUBLIC_KEY_SIZE,
            public_key.len()
        ));
    }

    let payload = [public_key, b"addr"].concat();
    let hash_hex = sha3_256_hex(&payload);
    let hash_bytes = hex::decode(&hash_hex).expect("sha3 hex is always valid");
    let hash20 = &hash_bytes[..20];

    let hrp = blockchain::bech32_hrp();
    bech32::encode(hrp, hash20)
}

pub fn verify_bech32_address(address: &str) -> bool {
    let hrp = blockchain::bech32_hrp();
    match bech32::decode(address) {
        Some((decoded_hrp, _data)) => decoded_hrp == hrp,
        None => false,
    }
}

/// Derive a deterministic contract address from `sender`, `nonce`, and
/// optional `salt` + `init_code` (for CREATE2). Uses SHA3-256, consistent
/// with the rest of XEPAY's hashing.
///
/// CREATE:  hash20 = SHA3-256(sender_bytes ++ nonce_be)[0..20]
/// CREATE2: hash20 = SHA3-256(0xff ++ sender_bytes ++ salt ++ SHA3-256(init_code))[0..20]
pub fn derive_contract_address(
    sender: &str,
    nonce: u64,
    salt: Option<[u8; 32]>,
    init_code: &[u8],
) -> Result<String, String> {
    let sender_bytes = decode_sender_address(sender)?;

    let mut payload = Vec::new();
    match salt {
        Some(s) => {
            payload.push(0xff);
            payload.extend_from_slice(&sender_bytes);
            payload.extend_from_slice(&s);
            let init_hash_hex = sha3_256_hex(init_code);
            let init_hash = hex::decode(&init_hash_hex).expect("sha3 hex is always valid");
            payload.extend_from_slice(&init_hash);
        }
        None => {
            payload.extend_from_slice(&sender_bytes);
            payload.extend_from_slice(&nonce.to_be_bytes());
        }
    }

    let hash_hex = sha3_256_hex(&payload);
    let hash_bytes = hex::decode(&hash_hex).expect("sha3 hex is always valid");
    let hash20 = &hash_bytes[..20];

    let prefix = blockchain::legacy_prefix_byte();
    let mut body = vec![prefix];
    body.extend_from_slice(hash20);
    Ok(base58check::encode(&body))
}

/// Same as `derive_contract_address` but returns a Bech32 address.
pub fn derive_contract_address_bech32(
    sender: &str,
    nonce: u64,
    salt: Option<[u8; 32]>,
    init_code: &[u8],
) -> Result<String, String> {
    let sender_bytes = decode_sender_address(sender)?;

    let mut payload = Vec::new();
    match salt {
        Some(s) => {
            payload.push(0xff);
            payload.extend_from_slice(&sender_bytes);
            payload.extend_from_slice(&s);
            let init_hash_hex = sha3_256_hex(init_code);
            let init_hash = hex::decode(&init_hash_hex).expect("sha3 hex is always valid");
            payload.extend_from_slice(&init_hash);
        }
        None => {
            payload.extend_from_slice(&sender_bytes);
            payload.extend_from_slice(&nonce.to_be_bytes());
        }
    }

    let hash_hex = sha3_256_hex(&payload);
    let hash_bytes = hex::decode(&hash_hex).expect("sha3 hex is always valid");
    let hash20 = &hash_bytes[..20];

    let hrp = blockchain::bech32_hrp();
    bech32::encode(hrp, hash20)
}

fn decode_sender_address(sender: &str) -> Result<Vec<u8>, String> {
    if sender.starts_with(&format!("{}1", blockchain::bech32_hrp())) {
        match bech32::decode(sender) {
            Some((_, data)) => Ok(data),
            None => Err("invalid bech32 sender".into()),
        }
    } else {
        match base58check::decode(sender) {
            Some(mut body) => {
                body.remove(0); // strip prefix byte
                Ok(body)
            }
            None => Err("invalid legacy sender".into()),
        }
    }
}
