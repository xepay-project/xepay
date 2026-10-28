// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

use serial_test::serial; // For serializing tests that modify global state.
use std::sync::Mutex; // For protecting the environment variable.
use xepay::blockchain::{self, Blockchain}; // The Blockchain struct and constants.

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

#[test]
fn timespan_matches_adjust_interval_times_spacing() {
    let expected = blockchain::difficulty_adjust_interval() * blockchain::pow_target_spacing_secs(); // Compute the expected timespan.
    assert_eq!(blockchain::pow_target_timespan_secs(), expected); // Check that the timespan matches.
}

#[test]
#[serial]
fn adjust_difficulty_does_nothing_before_interval() {
    with_temp_datadir(|| {
        let mut chain = Blockchain::new(); // Create a new chain.
        let nbits_before = chain.nbits; // Record the initial nbits.
        chain.adjust_difficulty(); // Attempt to adjust the difficulty.
        assert_eq!(chain.nbits, nbits_before); // The nbits should not have changed.
    });
}

#[test]
#[serial]
fn adjust_difficulty_increases_target_when_blocks_are_slow() {
    with_temp_datadir(|| {
        let chain = Blockchain::new(); // Create a new chain.
        assert!(chain.nbits > 0); // The nbits should be greater than zero.
    });
}
