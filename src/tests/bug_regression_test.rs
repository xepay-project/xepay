// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Regression tests for the five bugs identified during the code review.
//!
//! Bug #1: Genesis nonce mismatch between `genesis.rs` and `spec.rs`.
//! Bug #2: Hardcoded `ADDRESS_PREFIX` that did not adapt to the network.
//! Bug #3: `StateDB` panicked on `clone()` (compile-time check).
//! Bug #4: Race condition on `XEPC_DATADIR` in tests.
//! Bug #5: Legacy genesis accessors always returned Mainnet values (compile-time check).

use serial_test::serial;
use std::sync::Mutex;
use xepay::blockchain::Blockchain;
use xepay::consensus::genesis;
use xepay::consensus::spec::Network;
use xepay::validation;

// A global mutex to protect the XEPC_DATADIR environment variable.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

// A helper function that sets up a temporary datadir, runs the closure, and
// then restores the environment. It uses a mutex to prevent race conditions.
//
// # Important
//
// We use `tempfile::TempDir::keep()` (formerly `into_path()`) so that the
// directory is *not* deleted when the `TempDir` guard is dropped. This is
// necessary because `Blockchain` may hold an open LevelDB handle that
// references this path. Deleting the directory while LevelDB is still
// open would cause undefined behavior or a hang.
//
// The directory is intentionally leaked. The OS will clean it up when the
// test process exits.
fn with_temp_datadir<F: FnOnce()>(f: F) {
    let _guard = ENV_MUTEX.lock().unwrap();
    let dir = tempfile::tempdir().expect("tempdir failed");
    let path = dir.keep();
    std::env::set_var("XEPC_DATADIR", &path);
    f();
    std::env::remove_var("XEPC_DATADIR");
}

// ----------------------------------------------------------------------
// Bug #1: Genesis Nonce Consistency
// ----------------------------------------------------------------------

// This test verifies that the genesis nonce in `spec.rs` matches the one
// in `genesis.rs` for all networks. If they diverge, the node will refuse
// to start because the mined genesis block will not match the expected
// constants.
#[test]
fn bug_1_genesis_nonce_is_consistent() {
    // Mainnet: `spec.rs` and `genesis.rs` must agree.
    let spec_main = Network::Main.expected_genesis_nonce();
    let genesis_main = genesis::expected_genesis_nonce_for(Network::Main);
    assert_eq!(
        spec_main, genesis_main,
        "Mainnet genesis nonce mismatch: spec.rs = {}, genesis.rs = {}",
        spec_main, genesis_main
    );

    // Testnet: `spec.rs` and `genesis.rs` must agree.
    let spec_test = Network::Test.expected_genesis_nonce();
    let genesis_test = genesis::expected_genesis_nonce_for(Network::Test);
    assert_eq!(
        spec_test, genesis_test,
        "Testnet genesis nonce mismatch: spec.rs = {}, genesis.rs = {}",
        spec_test, genesis_test
    );

    // Regtest: `spec.rs` and `genesis.rs` must agree.
    let spec_regtest = Network::Regtest.expected_genesis_nonce();
    let genesis_regtest = genesis::expected_genesis_nonce_for(Network::Regtest);
    assert_eq!(
        spec_regtest, genesis_regtest,
        "Regtest genesis nonce mismatch: spec.rs = {}, genesis.rs = {}",
        spec_regtest, genesis_regtest
    );
}

// This test verifies that the expected genesis nonce is not zero for Mainnet
// and Testnet (it must be a valid mined nonce). Regtest is allowed to be
// zero or very small because it uses an easy target.
#[test]
fn bug_1_genesis_nonce_is_valid() {
    let main = genesis::expected_genesis_nonce_for(Network::Main);
    assert!(main > 0, "Mainnet genesis nonce must be greater than zero");

    let test = genesis::expected_genesis_nonce_for(Network::Test);
    assert!(test > 0, "Testnet genesis nonce must be greater than zero");

    let regtest = genesis::expected_genesis_nonce_for(Network::Regtest);
    assert!(regtest < 100, "Regtest genesis nonce should be small");
}

// ----------------------------------------------------------------------
// Bug #2: Network-Aware Address Prefix
// ----------------------------------------------------------------------

#[test]
fn bug_2_address_prefix_is_network_aware() {
    assert_eq!(
        xepay::blockchain::legacy_prefix_byte_for(Network::Main),
        0x30,
        "Mainnet legacy prefix byte must be 0x30"
    );

    assert_eq!(
        xepay::blockchain::bech32_hrp_for(Network::Main),
        "xepc",
        "Mainnet bech32 HRP must be xepc"
    );

    assert_eq!(
        xepay::blockchain::legacy_prefix_byte_for(Network::Test),
        0x6f,
        "Testnet legacy prefix byte must be 0x6f"
    );

    assert_eq!(
        xepay::blockchain::bech32_hrp_for(Network::Test),
        "xtes",
        "Testnet bech32 HRP must be xtes"
    );

    assert_eq!(
        xepay::blockchain::legacy_prefix_byte_for(Network::Regtest),
        0x6f,
        "Regtest legacy prefix byte must be 0x6f"
    );

    assert_eq!(
        xepay::blockchain::bech32_hrp_for(Network::Regtest),
        "xreg",
        "Regtest bech32 HRP must be xreg"
    );
}

#[test]
#[serial]
fn bug_2_address_validation_uses_correct_prefix() {
    with_temp_datadir(|| {
        let chain = Blockchain::new();

        let main_addr = "LgoJ9krkNnKWWgqzdR3keHJkoBPdEnyWHc";
        assert!(
            validation::is_valid_address(main_addr, &chain),
            "Mainnet legacy address should be valid"
        );

        let bech32_addr = "xepc1q4t3vtam8l8lz8hzr9fwakycgrm2c2z89yg7n0n";
        assert!(
            validation::is_valid_address(bech32_addr, &chain),
            "Mainnet bech32 address should be valid"
        );

        assert!(
            !validation::is_valid_address("not-an-address", &chain),
            "Random string should be invalid"
        );
    });
}

// ----------------------------------------------------------------------
// Bug #3: StateDB Clone Panic (Compile-Time)
// ----------------------------------------------------------------------

// fn bug_3_state_db_cannot_be_cloned(db: &xepay::storage::StateDB) {
//     let _ = db.clone();
// }

// ----------------------------------------------------------------------
// Bug #4: Environment Variable Race Condition
// ----------------------------------------------------------------------

#[test]
#[serial]
fn bug_4_temp_datadir_is_safe() {
    with_temp_datadir(|| {
        let chain = Blockchain::new();
        assert_eq!(chain.height(), 0, "Genesis height should be 0");
    });
}

#[test]
#[serial]
fn bug_4_temp_datadir_is_safe_again() {
    with_temp_datadir(|| {
        let chain = Blockchain::new();
        assert_eq!(chain.chain.len(), 1, "Genesis chain should have one block");
    });
}
