// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

use std::sync::OnceLock;
use xepay::block::Block;
use xepay::blockchain;
use xepay::consensus;

fn cached_genesis() -> &'static Block {
    static GENESIS: OnceLock<Block> = OnceLock::new();
    GENESIS.get_or_init(|| Block::genesis())
}

#[test]
fn genesis_secret_is_embedded() {
    let g = cached_genesis();
    let secret_tx = g
        .transactions
        .iter()
        .find(|tx| tx.from == "GENESIS")
        .expect("the secret transaction is missing");
    assert_eq!(secret_tx.note, blockchain::genesis_secret());
}

#[test]
fn genesis_hash_binds_message() {
    let mut g1 = cached_genesis().clone();
    let h1 = g1.calculate_hash();
    g1.transactions[1].note = "CHANGED".into();
    let h2 = g1.calculate_hash();
    assert_ne!(h1, h2);
}

#[test]
fn genesis_index_is_zero() {
    let g = cached_genesis();
    assert_eq!(g.index, 0);
}

#[test]
fn genesis_previous_hash_is_zero() {
    let g = cached_genesis();
    assert_eq!(g.previous_hash, consensus::rules::ZERO_HASH);
}

#[test]
fn genesis_hash_matches_expected() {
    let g = cached_genesis();
    assert_eq!(g.hash, consensus::genesis::EXPECTED_GENESIS_HASH);
}

#[test]
fn genesis_merkle_root_matches_expected() {
    let g = cached_genesis();
    assert_eq!(
        g.merkle_root,
        consensus::genesis::EXPECTED_GENESIS_MERKLE_ROOT
    );
}

#[test]
fn genesis_nonce_matches_expected() {
    let g = cached_genesis();
    assert_eq!(g.nonce, consensus::genesis::EXPECTED_GENESIS_NONCE);
}
