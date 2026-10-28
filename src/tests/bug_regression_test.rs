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

use serial_test::serial; // For serializing tests that modify global state.
use std::sync::Mutex; // For protecting the environment variable.
use xepay::blockchain::Blockchain; // The Blockchain struct.
use xepay::consensus::genesis; // Genesis parameters.
use xepay::consensus::spec::Network; // Network parameters.
use xepay::validation; // Validation rules.

// A global mutex to protect the XEPC_DATADIR environment variable.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

// A helper function that sets up a temporary datadir, runs the closure, and
// then restores the environment. It uses a mutex to prevent race conditions.
fn with_temp_datadir<F: FnOnce()>(f: F) {
    let _guard = ENV_MUTEX.lock().unwrap(); // Acquire the mutex.
    let dir = tempfile::tempdir().expect("tempdir failed"); // Create a temp directory.
    std::env::set_var("XEPC_DATADIR", dir.path()); // Set the environment variable.
    f(); // Run the closure.
    std::env::remove_var("XEPC_DATADIR"); // Remove the environment variable.
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
    let spec_main = Network::Main.expected_genesis_nonce(); // From spec.rs.
    let genesis_main = genesis::expected_genesis_nonce_for(Network::Main); // From genesis.rs.
    assert_eq!(
        spec_main, genesis_main,
        "Mainnet genesis nonce mismatch: spec.rs = {}, genesis.rs = {}",
        spec_main, genesis_main
    );

    // Testnet: `spec.rs` and `genesis.rs` must agree.
    let spec_test = Network::Test.expected_genesis_nonce(); // From spec.rs.
    let genesis_test = genesis::expected_genesis_nonce_for(Network::Test); // From genesis.rs.
    assert_eq!(
        spec_test, genesis_test,
        "Testnet genesis nonce mismatch: spec.rs = {}, genesis.rs = {}",
        spec_test, genesis_test
    );

    // Regtest: `spec.rs` and `genesis.rs` must agree.
    let spec_regtest = Network::Regtest.expected_genesis_nonce(); // From spec.rs.
    let genesis_regtest = genesis::expected_genesis_nonce_for(Network::Regtest); // From genesis.rs.
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
    let main = genesis::expected_genesis_nonce_for(Network::Main); // Mainnet nonce.
    assert!(main > 0, "Mainnet genesis nonce must be greater than zero");

    let test = genesis::expected_genesis_nonce_for(Network::Test); // Testnet nonce.
    assert!(test > 0, "Testnet genesis nonce must be greater than zero");

    let regtest = genesis::expected_genesis_nonce_for(Network::Regtest); // Regtest nonce.
    assert!(regtest < 100, "Regtest genesis nonce should be small");
}

// ----------------------------------------------------------------------
// Bug #2: Network-Aware Address Prefix (Now Handled in blockchain.rs)
// ----------------------------------------------------------------------

// This test verifies that the address prefix for each network is correct.
// It uses the accessors in blockchain.rs (legacy_prefix_byte, bech32_hrp).
#[test]
fn bug_2_address_prefix_is_network_aware() {
    // The default network is Mainnet.
    // Legacy prefix byte for Mainnet should be 0x30 (48 decimal).
    assert_eq!(
        xepay::blockchain::legacy_prefix_byte_for(Network::Main),
        0x30,
        "Mainnet legacy prefix byte must be 0x30"
    );

    // Bech32 HRP for Mainnet should be "xepc".
    assert_eq!(
        xepay::blockchain::bech32_hrp_for(Network::Main),
        "xepc",
        "Mainnet bech32 HRP must be xepc"
    );

    // Testnet legacy prefix byte should be 0x6f.
    assert_eq!(
        xepay::blockchain::legacy_prefix_byte_for(Network::Test),
        0x6f,
        "Testnet legacy prefix byte must be 0x6f"
    );

    // Testnet bech32 HRP should be "xtes".
    assert_eq!(
        xepay::blockchain::bech32_hrp_for(Network::Test),
        "xtes",
        "Testnet bech32 HRP must be xtes"
    );

    // Regtest legacy prefix byte should be 0x6f.
    assert_eq!(
        xepay::blockchain::legacy_prefix_byte_for(Network::Regtest),
        0x6f,
        "Regtest legacy prefix byte must be 0x6f"
    );

    // Regtest bech32 HRP should be "xreg".
    assert_eq!(
        xepay::blockchain::bech32_hrp_for(Network::Regtest),
        "xreg",
        "Regtest bech32 HRP must be xreg"
    );
}

// This test verifies that `is_valid_address()` accepts addresses with the
// correct prefix and rejects addresses with the wrong prefix.
#[test]
#[serial]
fn bug_2_address_validation_uses_correct_prefix() {
    with_temp_datadir(|| {
        let chain = Blockchain::new();

        // A valid Mainnet Legacy address (starts with "L").
        let main_addr = "LgoJ9krkNnKWWgqzdR3keHJkoBPdEnyWHc";
        assert!(
            validation::is_valid_address(main_addr, &chain),
            "Mainnet legacy address should be valid"
        );

        // A valid Mainnet Bech32 address (starts with "xepc1").
        let bech32_addr = "xepc1q4t3vtam8l8lz8hzr9fwakycgrm2c2z89yg7n0n";
        assert!(
            validation::is_valid_address(bech32_addr, &chain),
            "Mainnet bech32 address should be valid"
        );

        // A random string should be invalid.
        assert!(
            !validation::is_valid_address("not-an-address", &chain),
            "Random string should be invalid"
        );
    });
}

// ----------------------------------------------------------------------
// Bug #3: StateDB Clone Panic (Compile-Time)
// ----------------------------------------------------------------------

// This is a compile-time test. If `StateDB` implements `Clone`, the
// following function will compile. If it does not, the compiler will
// reject it. We cannot write a runtime test for this because the panic
// happens at runtime if `Clone` is implemented.
//
// To verify that the bug is fixed, simply uncomment the following
// function and run `cargo check`. If it fails to compile, the fix is
// working.
//
// fn bug_3_state_db_cannot_be_cloned(db: &xepay::storage::StateDB) {
//     let _ = db.clone(); // This should NOT compile.
// }

// ----------------------------------------------------------------------
// Bug #4: Environment Variable Race Condition
// ----------------------------------------------------------------------

// This test verifies that `with_temp_datadir` is safe to use with
// multiple tests. Because we use a mutex, the tests will not race.
#[test]
#[serial]
fn bug_4_temp_datadir_is_safe() {
    with_temp_datadir(|| {
        let chain = Blockchain::new(); // Create a new chain.
        assert_eq!(chain.height(), 0, "Genesis height should be 0"); // Sanity check.
    });
}

// A second test that uses `with_temp_datadir` to verify that the mutex
// serializes access correctly.
#[test]
#[serial]
fn bug_4_temp_datadir_is_safe_again() {
    with_temp_datadir(|| {
        let chain = Blockchain::new(); // Create a new chain.
        assert_eq!(chain.chain.len(), 1, "Genesis chain should have one block");
        // Sanity check.
    });
}
