// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Proof-of-work primitives.

use crate::block::Block;
use std::sync::atomic::{AtomicBool, Ordering};

pub use crate::consensus::pow::GENESIS_NBITS;

/// Convert a compact nBits value into a 256-bit big-endian target.
pub fn nbits_to_target(nbits: u32) -> [u8; 32] {
    let exponent = (nbits >> 24) as usize;
    let mantissa = nbits & 0x007fffff;
    let negative = (nbits & 0x00800000) != 0;

    let mut target = [0u8; 32];

    if negative {
        return target;
    }

    if exponent <= 3 {
        let value = mantissa >> (8 * (3 - exponent));
        let be = value.to_be_bytes();
        target[29] = be[1];
        target[30] = be[2];
        target[31] = be[3];
        return target;
    }

    let shift = 32 - exponent;
    if shift + 3 > 32 {
        return target;
    }
    let be = mantissa.to_be_bytes();
    target[shift] = be[1];
    target[shift + 1] = be[2];
    target[shift + 2] = be[3];
    target
}

/// Convert nBits to a floating-point difficulty value (similar to Bitcoin).
pub fn nbits_to_difficulty(nbits: u32) -> f64 {
    let max_target = nbits_to_target(0x1d00ffff);
    let target = nbits_to_target(nbits);

    let mut max_val = 0.0f64;
    for &b in max_target.iter() {
        max_val = max_val * 256.0 + b as f64;
    }

    let mut cur_val = 0.0f64;
    for &b in target.iter() {
        cur_val = cur_val * 256.0 + b as f64;
    }

    if cur_val == 0.0 {
        return 0.0;
    }

    max_val / cur_val
}

/// Return whether `hash_hex` is less than or equal to `target`.
pub fn hash_le_target(hash_hex: &str, target: &[u8; 32]) -> bool {
    let hash_bytes = match hex::decode(hash_hex) {
        Ok(b) if b.len() == 32 => b,
        _ => return false,
    };

    for i in 0..32 {
        if hash_bytes[i] < target[i] {
            return true;
        }
        if hash_bytes[i] > target[i] {
            return false;
        }
    }
    true
}

/// Return whether `hash_hex` meets the target encoded by `nbits`.
pub fn hash_meets_nbits(hash_hex: &str, nbits: u32) -> bool {
    let target = nbits_to_target(nbits);
    hash_le_target(hash_hex, &target)
}

/// Check whether proof-of-work mining should be skipped.
/// This function is only compiled when the `skip-pow` feature is enabled.
/// In release builds, it always returns `false`, ensuring that production
/// binaries (Mainnet, Testnet, Regtest) never skip mining.
#[cfg(feature = "skip-pow")]
fn should_skip_pow() -> bool {
    true
}

/// In non-test builds, mining is never skipped.
#[cfg(not(feature = "skip-pow"))]
fn should_skip_pow() -> bool {
    false
}

/// Mine the block until the hash meets the target or the nonce is exhausted.
pub fn mine(block: &mut Block, nbits: u32) -> bool {
    mine_interruptible(block, nbits, None)
}

/// Mine the block until the hash meets the target, the nonce is exhausted,
/// or the interrupt flag is set. Return `true` on success.
pub fn mine_interruptible(block: &mut Block, nbits: u32, interrupt: Option<&AtomicBool>) -> bool {
    // In test builds with the `skip-pow` feature, skip the mining process.
    // In release builds, `should_skip_pow()` always returns `false`.
    if should_skip_pow() {
        return true;
    }

    let target = nbits_to_target(nbits);
    loop {
        if let Some(flag) = interrupt {
            if flag.load(Ordering::SeqCst) {
                return false;
            }
        }
        block.hash = block.calculate_hash();
        if hash_le_target(&block.hash, &target) {
            return true;
        }
        if block.nonce == u64::MAX {
            return false;
        }
        block.nonce = block.nonce.wrapping_add(1);
    }
}
