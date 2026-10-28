// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

use xepay::blockchain;
use xepay::pow::{hash_meets_nbits, nbits_to_target};

#[cfg(not(feature = "skip-pow"))]
use xepay::block::Block;
#[cfg(not(feature = "skip-pow"))]
use xepay::pow::mine;
#[cfg(not(feature = "skip-pow"))]
use xepay::transaction::Transaction;

#[test]
fn nbits_to_target_genesis() {
    let target = nbits_to_target(blockchain::genesis_nbits());
    assert_eq!(&target[..2], &[0u8, 0u8]);
    assert!(target[2] <= 0x0f);
}

#[test]
fn hash_meets_nbits_rejects_easy_hash() {
    let hash = "ff".repeat(32);
    assert!(!hash_meets_nbits(&hash, blockchain::genesis_nbits()));
}

#[test]
#[cfg(not(feature = "skip-pow"))]
fn mining_produces_valid_hash() {
    let nbits = blockchain::genesis_nbits();
    let mut b = Block::new(
        1,
        "0".repeat(64),
        blockchain::initial_reward(),
        nbits,
        vec![Transaction::new("XEPC_alice", "XEPC_bob", 1, 0, 0)],
    );
    let ok = mine(&mut b, nbits);
    assert!(ok);
    assert!(hash_meets_nbits(&b.hash, nbits));
}

#[test]
#[cfg(not(feature = "skip-pow"))]
fn mining_increments_nonce() {
    let nbits = blockchain::genesis_nbits();
    let mut b = Block::new(
        1,
        "0".repeat(64),
        blockchain::initial_reward(),
        nbits,
        vec![Transaction::new("XEPC_alice", "XEPC_bob", 1, 0, 0)],
    );
    let n0 = b.nonce;
    let ok = mine(&mut b, nbits);
    assert!(ok);
    assert!(b.nonce >= n0);
}
