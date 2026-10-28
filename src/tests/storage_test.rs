// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

use serial_test::serial;
use std::sync::Mutex;
use xepay::blockchain::Blockchain;
use xepay::storage;

static ENV_MUTEX: Mutex<()> = Mutex::new(());

fn with_temp_datadir<F: FnOnce()>(f: F) {
    let _guard = ENV_MUTEX.lock().unwrap();
    let dir = tempfile::tempdir().expect("tempdir failed");
    std::env::set_var("XEPC_DATADIR", dir.path());
    f();
    std::env::remove_var("XEPC_DATADIR");
}

#[test]
#[serial]
fn save_and_load_chain() {
    with_temp_datadir(|| {
        {
            let chain = Blockchain::new();
            storage::save_chain(&chain).expect("save failed");
        }

        let loaded = storage::load_chain().expect("load failed");
        assert_eq!(loaded.height(), 0);
        assert_eq!(loaded.chain.len(), 1);
    });
}

#[test]
#[serial]
fn load_chain_returns_new_when_empty() {
    with_temp_datadir(|| {
        let loaded = storage::load_chain().expect("load failed");
        assert_eq!(loaded.chain.len(), 1);
        assert_eq!(loaded.height(), 0);
    });
}
