// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Integration tests for the XEPAY HD wallet module.

use serial_test::serial;
use std::sync::Mutex;

use xepay::hdwallet::{AddressType, HdWallet};

static ENV_MUTEX: Mutex<()> = Mutex::new(());

fn with_temp_datadir<F: FnOnce()>(f: F) {
    let _guard = ENV_MUTEX.lock().unwrap();
    let dir = tempfile::tempdir().expect("tempdir failed");
    std::env::set_var("XEPC_DATADIR", dir.path());
    f();
    std::env::remove_var("XEPC_DATADIR");
}

#[test]
fn generate_12_word_mnemonic() {
    let wallet = HdWallet::generate(12).expect("generation failed");
    let word_count = wallet.mnemonic.split_whitespace().count();
    assert_eq!(word_count, 12);
}

#[test]
fn generate_24_word_mnemonic() {
    let wallet = HdWallet::generate(24).expect("generation failed");
    let word_count = wallet.mnemonic.split_whitespace().count();
    assert_eq!(word_count, 24);
}

#[test]
fn invalid_word_count_is_rejected() {
    assert!(HdWallet::generate(15).is_err());
    assert!(HdWallet::generate(0).is_err());
}

#[test]
fn restore_from_mnemonic_is_deterministic() {
    let wallet1 = HdWallet::generate(12).expect("generation failed");
    let wallet2 = HdWallet::from_mnemonic(&wallet1.mnemonic, None).expect("restore failed");

    let mut w1 = wallet1;
    let mut w2 = wallet2;

    let addr1 = w1.derive_address(0, AddressType::Legacy).expect("derive failed");
    let addr2 = w2.derive_address(0, AddressType::Legacy).expect("derive failed");

    assert_eq!(addr1.address, addr2.address);
    assert_eq!(addr1.public_key, addr2.public_key);
}

#[test]
fn different_address_types_produce_different_addresses() {
    let mut wallet = HdWallet::generate(12).expect("generation failed");
    let legacy = wallet.derive_address(0, AddressType::Legacy).expect("derive failed");
    let bech32 = wallet.derive_address(0, AddressType::Bech32).expect("derive failed");

    assert_ne!(legacy.address, bech32.address);
}

#[test]
fn different_indices_produce_different_addresses() {
    let mut wallet = HdWallet::generate(12).expect("generation failed");
    let addr0 = wallet.derive_address(0, AddressType::Legacy).expect("derive failed");
    let addr1 = wallet.derive_address(1, AddressType::Legacy).expect("derive failed");

    assert_ne!(addr0.address, addr1.address);
}

#[test]
#[serial]
fn save_and_load_hd_wallet() {
    with_temp_datadir(|| {
        let mut wallet = HdWallet::generate(12).expect("generation failed");
        wallet.derive_address(0, AddressType::Legacy).expect("derive failed");
        wallet.save_default().expect("save failed");

        let loaded = HdWallet::load_default().expect("load failed");
        assert_eq!(loaded.mnemonic, wallet.mnemonic);
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].address, wallet.entries[0].address);
    });
}

#[test]
fn path_format_is_correct() {
    let mut wallet = HdWallet::generate(12).expect("generation failed");
    let legacy = wallet.derive_address(5, AddressType::Legacy).expect("derive failed");
    let bech32 = wallet.derive_address(5, AddressType::Bech32).expect("derive failed");

    assert_eq!(legacy.path, "m/44'/9999'/0'/0/5");
    assert_eq!(bech32.path, "m/84'/9999'/0'/0/5");
}
