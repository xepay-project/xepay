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
use xepay::blockchain::Blockchain;
use xepay::consensus::spec::Network;

static ENV_MUTEX: Mutex<()> = Mutex::new(());

fn with_temp_datadir<F: FnOnce()>(f: F) {
    let _guard = ENV_MUTEX.lock().unwrap();
    let dir = tempfile::tempdir().expect("tempdir failed");
    let path = dir.keep();
    std::env::set_var("XEPC_DATADIR", &path);
    f();
    std::env::remove_var("XEPC_DATADIR");
}

/// `compute_work` must be bounded for any valid nbits.
///
/// A target that exceeds `u128::MAX` yields the minimum work (1), because
/// the difficulty is trivially low. This mirrors the behaviour of
/// reference node implementations, which clamp chainwork to 1 when the
/// target is larger than the chainwork word size.
#[test]
fn compute_work_is_bounded_for_valid_nbits() {
    // Mainnet nbits (0x1e0ffff0) has a target that exceeds u128::MAX,
    // so compute_work must return the minimum possible work (1).
    let nbits = Network::Main.genesis_nbits();
    let work = Block::compute_work(nbits);
    assert_eq!(
        work, 1,
        "compute_work must return 1 for a target that exceeds u128::MAX"
    );

    // Regtest nbits (0x207fffff) also has a target that exceeds u128::MAX.
    let regtest_nbits = Network::Regtest.genesis_nbits();
    let regtest_work = Block::compute_work(regtest_nbits);
    assert_eq!(
        regtest_work, 1,
        "compute_work must return 1 for Regtest nbits"
    );

    // A nbits with a target that fits in u128 must return work > 1.
    // 0x1d00ffff has a target of 0x00000000ffff... which fits in u128.
    let small_nbits = 0x1d00ffffu32;
    let small_work = Block::compute_work(small_nbits);
    assert!(
        small_work > 1,
        "compute_work must return work > 1 for a target that fits in u128"
    );
}

/// `compute_work` must return `u128::MAX` only for a zero target.
#[test]
fn compute_work_returns_max_for_zero_target() {
    let work = Block::compute_work(0x00000000);
    assert_eq!(
        work,
        u128::MAX,
        "compute_work must return u128::MAX for a zero target"
    );
}

/// The mempool must keep the transaction with the highest absolute fee
/// when eviction is triggered.
///
/// # Design
///
/// We request exactly enough space to evict one small transaction. If the
/// eviction logic is correct, the transaction with the lower total fee is
/// evicted and the one with the higher total fee is kept.
#[test]
#[serial]
fn mempool_eviction_keeps_high_total_fee() {
    with_temp_datadir(|| {
        let mut chain = Blockchain::new();
        chain.mempool.clear();

        // Small transaction with high fee rate but low total fee.
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

        let small_size = Blockchain::tx_size_bytes(&small_tx);

        chain.mempool.push(small_tx.clone());
        chain.mempool.push(large_tx.clone());

        // Request exactly enough space to evict one small transaction.
        // This ensures only the lower-fee transaction is removed.
        let needed = small_size;
        chain.evict_lowest_fee_transactions(needed);

        // The large transaction (higher total fee) should be kept.
        assert!(
            chain.mempool.iter().any(|tx| tx.txid == large_tx.txid),
            "high total fee transaction must be kept"
        );

        // The small transaction (lower total fee) should have been evicted.
        assert!(
            !chain.mempool.iter().any(|tx| tx.txid == small_tx.txid),
            "low total fee transaction must be evicted"
        );
    });
}

/// A reorg must be rejected if the ancestor block has no state root.
///
/// # Design
///
/// The genesis block always has an empty `state_root` by design. To test
/// the reorg handler's state-root validation, we build a *second* block
/// on top of genesis, corrupt its `state_root`, and then attempt a reorg
/// from a *third* block whose `previous_hash` points to the corrupted
/// block.
#[test]
#[serial]
fn reorg_with_missing_state_root_is_rejected() {
    with_temp_datadir(|| {
        use xepay::p2p::sync::SyncState;

        let mut chain = Blockchain::new();
        let genesis = chain.chain[0].clone();

        // Build block 1 on top of genesis with a non-empty state_root.
        let mut block1 = Block::new_for(
            1,
            genesis.hash.clone(),
            Blockchain::calculate_reward(1),
            0,
            chain.nbits,
            Vec::new(),
            "xreg1q4t3vtam8l8lz8hzr9fwakycgrm2c2z89yg7n0n".to_string(),
            "00".repeat(32),
            genesis.chainwork,
        )
        .expect("failed to create block1");
        block1.hash = block1.calculate_hash();
        chain.chain.push(block1.clone());

        // Build block 2 on top of block1.
        let mut block2 = Block::new_for(
            2,
            block1.hash.clone(),
            Blockchain::calculate_reward(2),
            0,
            chain.nbits,
            Vec::new(),
            "xreg1q4t3vtam8l8lz8hzr9fwakycgrm2c2z89yg7n0n".to_string(),
            "00".repeat(32),
            block1.chainwork,
        )
        .expect("failed to create block2");

        // Corrupt block1's state_root to simulate disk corruption.
        chain.chain[1].state_root = String::new();

        // Give block2 more chainwork to force the reorg attempt.
        block2.chainwork = u128::MAX;
        block2.hash = block2.calculate_hash();

        let result = SyncState::handle_reorg(&mut chain, block2);
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
