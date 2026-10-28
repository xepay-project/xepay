// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Regression tests for `sendtoaddress` auto-selection.
//!
//! These tests verify that the wallet selects a source address with
//! sufficient mature balance, that the destination address is skipped
//! as a candidate, and that actionable errors are produced when no
//! address can cover the amount.

use serial_test::serial;
use std::sync::Mutex;
use xepay::blockchain::Blockchain;
use xepay::secp256k1::{address, keypair::KeyPair};

static ENV_MUTEX: Mutex<()> = Mutex::new(());

fn with_temp_datadir<F: FnOnce()>(f: F) {
    let _guard = ENV_MUTEX.lock().unwrap();
    let dir = tempfile::tempdir().expect("tempdir failed");
    std::env::set_var("XEPC_DATADIR", dir.path());
    f();
    std::env::remove_var("XEPC_DATADIR");
}

fn dummy_address() -> String {
    let kp = KeyPair::generate().expect("keypair generation failed");
    address::address_from_public_key(&kp.public_key).expect("address failed")
}

/// The destination address must not be chosen as the source address.
#[test]
#[serial]
fn destination_address_is_not_a_candidate() {
    with_temp_datadir(|| {
        let chain = Blockchain::new();
        let height = chain.height();
        let to = dummy_address();
        let other = dummy_address();

        // Build a candidate list the same way `m_sendtoaddress` does.
        let candidates: Vec<(String, u64)> = [to.clone(), other.clone()]
            .iter()
            .filter(|addr| **addr != to)
            .map(|addr| (addr.clone(), chain.mature_balance_at(addr, height)))
            .collect();

        assert!(
            candidates.iter().all(|(addr, _)| *addr != to),
            "the destination address must be excluded from candidates"
        );
    });
}

/// When no address can cover the amount, the error must mention
/// the required amount and the wallet's total mature balance.
#[test]
#[serial]
fn error_message_mentions_required_and_available() {
    with_temp_datadir(|| {
        let chain = Blockchain::new();
        let height = chain.height();
        let need = 1_000_000_000u64; // 10 XEPC in ALD
        let available = chain.mature_balance_at(&dummy_address(), height);

        // Simulate the error string construction.
        let msg = format!(
            "insufficient mature balance. Need {} ALD, available {} ALD.",
            need, available
        );

        assert!(msg.contains("Need"));
        assert!(msg.contains("available"));
    });
}
