// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Storage layer for XEPAY.

pub mod account;
pub mod adapter;
pub mod state;

pub use account::Account;
pub use state::StateDB;

// Fungsi khusus buat P2P: Cuma nulis block ke disk, nggak nulis state.
// Ini biar P2P nggak ganggu proses Mining.
pub fn save_block_only(block: &crate::block::Block) -> Result<(), String> {
    let path = crate::datadir::blocks_dir().join(format!("blk{:05}.dat", block.index / 1000));
    // ... (implementasi write append ke file blk) ...
    // Karena lo pake bincode, tinggal append aja.
    Ok(())
}

pub use crate::storage_legacy::*;
