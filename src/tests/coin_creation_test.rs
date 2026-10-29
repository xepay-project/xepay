// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Regression tests for coin creation via reward manipulation.
//!
//! These tests ensure that a malicious miner cannot create coins from
//! thin air by submitting a block with an inflated reward, fake fees,
//! or incorrect subsidy.

use serial_test::serial;
use std::sync::Mutex;
use xepay::block::Block;
use xepay::blockchain::{self, Blockchain};
use xepay::validation;

static ENV_MUTEX: Mutex<()> = Mutex::new(());

fn with_temp_datadir<F: FnOnce()>(f: F) {
    let _guard = ENV_MUTEX.lock().unwrap();
    let dir = tempfile::tempdir().expect("tempdir failed");
    std::env::set_var("XEPC_DATADIR", dir.path());
    f();
    std::env::remove_var("XEPC_DATADIR");
}

fn dummy_address() -> String {
    // A valid regtest bech32 address for testing purposes.
    "xreg1q4t3vtam8l8lz8hzr9fwakycgrm2c2z89yg7n0n".to_string()
}

/// A block with `reward` greater than `subsidy + total_fees` must be rejected.
/// This prevents a miner from creating coins from thin air.
#[test]
#[serial]
fn block_with_inflated_reward_is_rejected() {
    with_temp_datadir(|| {
        let mut chain = Blockchain::new();
        let prev = chain.chain.last().unwrap().clone();
        let next_index = prev.index + 1;
        let subsidy = Blockchain::calculate_reward(next_index);

        let mut b = Block::new_for(
            next_index,
            prev.hash.clone(),
            subsidy,
            0, // total_fees
            chain.nbits,
            Vec::new(),
            dummy_address(),
            String::new(),
            prev.chainwork,
        )
        .expect("failed to create block");

        // Manipulate the reward to be the maximum possible value.
        b.reward = u64::MAX;
        b.hash = b.calculate_hash();

        let err = validation::validate_block(&mut chain, &prev, &b);
        assert!(
            err.is_err(),
            "block with inflated reward should be rejected"
        );
        assert_eq!(
            err.unwrap_err().code,
            "E046",
            "should fail with E046 (reward mismatch)"
        );
    });
}

/// A block with `total_fees` that does not match the sum of transaction fees
/// must be rejected. This prevents a miner from faking fees to inflate the reward.
#[test]
#[serial]
fn block_with_fake_total_fees_is_rejected() {
    with_temp_datadir(|| {
        let mut chain = Blockchain::new();
        let prev = chain.chain.last().unwrap().clone();
        let next_index = prev.index + 1;
        let subsidy = Blockchain::calculate_reward(next_index);

        let mut b = Block::new_for(
            next_index,
            prev.hash.clone(),
            subsidy,
            0, // real total_fees is 0
            chain.nbits,
            Vec::new(),
            dummy_address(),
            String::new(),
            prev.chainwork,
        )
        .expect("failed to create block");

        // Manipulate total_fees to a non-zero value and adjust the reward.
        b.total_fees = 1_000_000;
        b.reward = b.subsidy.saturating_add(b.total_fees);
        b.hash = b.calculate_hash();

        let err = validation::validate_block(&mut chain, &prev, &b);
        assert!(
            err.is_err(),
            "block with fake total_fees should be rejected"
        );
        assert_eq!(
            err.unwrap_err().code,
            "E047",
            "should fail with E047 (total_fees mismatch)"
        );
    });
}

/// A block with a `subsidy` that does not match the current era must be rejected.
/// This prevents a miner from inflating the subsidy beyond the halving schedule.
#[test]
#[serial]
fn block_with_wrong_subsidy_is_rejected() {
    with_temp_datadir(|| {
        let mut chain = Blockchain::new();
        let prev = chain.chain.last().unwrap().clone();
        let next_index = prev.index + 1;

        let mut b = Block::new_for(
            next_index,
            prev.hash.clone(),
            blockchain::initial_reward(),
            0,
            chain.nbits,
            Vec::new(),
            dummy_address(),
            String::new(),
            prev.chainwork,
        )
        .expect("failed to create block");

        // Manipulate the subsidy to double the correct value.
        b.subsidy = blockchain::initial_reward() * 2;
        b.reward = b.subsidy;
        b.hash = b.calculate_hash();

        let err = validation::validate_block(&mut chain, &prev, &b);
        assert!(
            err.is_err(),
            "block with wrong subsidy should be rejected"
        );
        assert_eq!(
            err.unwrap_err().code,
            "E031",
            "should fail with E031 (subsidy mismatch)"
        );
    });
}

/// A block with a `reward` lower than `subsidy + total_fees` must also be rejected.
/// This ensures the miner cannot burn coins or create an inconsistent ledger.
#[test]
#[serial]
fn block_with_deflated_reward_is_rejected() {
    with_temp_datadir(|| {
        let mut chain = Blockchain::new();
        let prev = chain.chain.last().unwrap().clone();
        let next_index = prev.index + 1;
        let subsidy = Blockchain::calculate_reward(next_index);

        let mut b = Block::new_for(
            next_index,
            prev.hash.clone(),
            subsidy,
            0,
            chain.nbits,
            Vec::new(),
            dummy_address(),
            String::new(),
            prev.chainwork,
        )
        .expect("failed to create block");

        // Manipulate the reward to be less than the correct value.
        b.reward = subsidy.saturating_sub(1);
        b.hash = b.calculate_hash();

        let err = validation::validate_block(&mut chain, &prev, &b);
        assert!(
            err.is_err(),
            "block with deflated reward should be rejected"
        );
        assert_eq!(
            err.unwrap_err().code,
            "E046",
            "should fail with E046 (reward mismatch)"
        );
    });
}
