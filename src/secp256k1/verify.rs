// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! ECDSA verification for XEPAY.

use secp256k1::{ecdsa::Signature, Message, PublicKey, Secp256k1};

/// Verify a DER-encoded ECDSA signature.
pub fn verify(public_key: &[u8], message_hash: &[u8], signature: &[u8]) -> bool {
    if public_key.len() != super::PUBLIC_KEY_SIZE {
        return false;
    }
    if message_hash.len() != 32 {
        return false;
    }

    let secp = Secp256k1::verification_only();

    let pk = match PublicKey::from_slice(public_key) {
        Ok(p) => p,
        Err(_) => return false,
    };
    let sig = match Signature::from_der(signature) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let msg = match Message::from_digest_slice(message_hash) {
        Ok(m) => m,
        Err(_) => return false,
    };

    secp.verify_ecdsa(&msg, &sig, &pk).is_ok()
}
