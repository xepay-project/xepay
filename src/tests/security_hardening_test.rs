// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Regression tests for security hardening fixes.
//!
//! These tests verify that:
//! - `compute_work` handles extreme `nbits` values safely.
//! - The mempool eviction strategy respects absolute fee value.
//! - The reorg handler validates the state root before rolling back.

use serial_test::serial;
use std::sync::Mutex;
use xepay::block::Block;
use xepay::blockchain::{self, Blockchain};

static ENV_MUTEX: Mutex<()> = Mutex::new(());

fn with_temp_datadir<F: FnOnce()>(f: F) {
    let _guard = ENV_MUTEX.lock().unwrap();
    let dir = tempfile::tempdir().expect("tempdir failed");
    std::env::set_var("XEPC_DATADIR", dir.path());
    f();
    std::env::remove_var("XEPC_DATADIR");
}

/// `compute_work` must not return `u128::MAX` for a valid target.
#[test]
fn compute_work_is_bounded_for_valid_nbits() {
    let nbits = blockchain::genesis_nbits();
    let work = Block::compute_work(nbits);
    assert!(
        work < u128::MAX,
        "compute_work must be bounded for valid nbits"
    );
    assert!(work > 0, "compute_work must be positive for valid nbits");
}

/// `compute_work` must return `u128::MAX` only for a zero target.
#[test]
fn compute_work_returns_max_for_zero_target() {
    // nbits with zero target is invalid, but compute_work must handle it safely.
    let work = Block::compute_work(0x00000000);
    assert_eq!(
        work,
        u128::MAX,
        "compute_work must return u128::MAX for a zero target"
    );
}

/// The mempool must keep the transaction with the highest absolute fee
/// when eviction is triggered.
#[test]
#[serial]
fn mempool_eviction_keeps_high_total_fee() {
    with_temp_datadir(|| {
        let mut chain = Blockchain::new();
        chain.mempool.clear();

        // Small transaction with high fee rate (100 fee per byte).
        let mut small_tx = xepay::transaction::Transaction::new(
            "xreg1qsmall",
            "xreg1qdest1",
            1,
            10_000,
            0,
        );
        small_tx.txid = small_tx.compute_txid();

        // Large transaction with lower fee rate but higher total fee.
        let mut large_tx = xepay::transaction::Transaction::new(
            "xreg1qlarge",
            "xreg1qdest2",
            1,
            100_000,
            0,
        );
        large_tx.txid = large_tx.compute_txid();

        chain.mempool.push(small_tx.clone());
        chain.mempool.push(large_tx.clone());

        // Force eviction by requesting more space than available.
        let needed = chain.mempool_size_bytes() + 1;
        chain.evict_lowest_fee_transactions(needed);

        // The large transaction (higher total fee) should be kept.
        assert!(
            chain.mempool.iter().any(|tx| tx.txid == large_tx.txid),
            "high total fee transaction must be kept"
        );
    });
}

/// A reorg must be rejected if the ancestor block has no state root.
#[test]
#[serial]
fn reorg_with_missing_state_root_is_rejected() {
    with_temp_datadir(|| {
        let mut chain = Blockchain::new();

        // Create a fake new block that claims a reorg.
        let mut fake_block = Block::new_for(
            2,
            chain.chain[0].hash.clone(),
            Blockchain::calculate_reward(2),
            0,
            chain.nbits,
            Vec::new(),
            "xreg1q4t3vtam8l8lz8hzr9fwakycgrm2c2z89yg7n0n".to_string(),
            String::new(),
            chain.chain[0].chainwork,
        )
        .expect("failed to create fake block");

        // Corrupt the ancestor's state root to simulate disk corruption.
        chain.chain[0].state_root = String::new();

        // Give the fake block more chainwork to force the reorg attempt.
        fake_block.chainwork = u128::MAX;
        fake_block.hash = fake_block.calculate_hash();

        let result = xepay::p2p::sync::SyncState::handle_reorg(&mut chain, fake_block);
        assert!(
            result.is_err(),
            "reorg with missing state root must be rejected"
        );
        assert!(
            result.unwrap_err().contains("state root"),
            "error must mention state root"
        );
    });
}
