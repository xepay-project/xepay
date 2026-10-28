// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Data directory layout and path helpers.

use std::path::{Path, PathBuf};

use crate::consensus::spec;

pub const BLOCKS_DIR: &str = "blocks";
pub const CHAINSTATE_DIR: &str = "chainstate";
pub const WALLET_FILE: &str = "wallet.dat";
pub const CONFIG_FILE: &str = "xepay.conf";
pub const PEERS_FILE: &str = "peers.json";
pub const MEMPOOL_FILE: &str = "mempool.json";
pub const DEBUG_LOG: &str = "debug.log";
pub const BLOCK_FILE_PREFIX: &str = "blk";
pub const PEER_ID_FILE: &str = "peer_id";
pub const COOKIE_FILE: &str = ".cookie";
pub const STATE_DB_DIR: &str = "state_db";

pub fn datadir() -> PathBuf {
    if let Ok(custom) = std::env::var("XEPC_DATADIR") {
        return PathBuf::from(custom);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(spec::current_network().datadir_name())
}

pub fn blocks_dir() -> PathBuf {
    datadir().join(BLOCKS_DIR)
}
pub fn chainstate_dir() -> PathBuf {
    datadir().join(CHAINSTATE_DIR)
}
pub fn state_db_path() -> PathBuf {
    chainstate_dir().join(STATE_DB_DIR)
}
pub fn wallet_path() -> PathBuf {
    datadir().join(WALLET_FILE)
}
pub fn config_path() -> PathBuf {
    datadir().join(CONFIG_FILE)
}
pub fn peers_path() -> PathBuf {
    datadir().join(PEERS_FILE)
}
pub fn mempool_path() -> PathBuf {
    datadir().join(MEMPOOL_FILE)
}
pub fn debug_log_path() -> PathBuf {
    datadir().join(DEBUG_LOG)
}
pub fn peer_id_path() -> PathBuf {
    datadir().join(PEER_ID_FILE)
}
pub fn cookie_path() -> PathBuf {
    datadir().join(COOKIE_FILE)
}

pub fn block_file(index: u32) -> PathBuf {
    blocks_dir().join(format!("{}{:05}.dat", BLOCK_FILE_PREFIX, index))
}

pub fn chainstate_file() -> PathBuf {
    chainstate_dir().join("state.json")
}

pub fn ensure_dirs() -> std::io::Result<()> {
    std::fs::create_dir_all(datadir())?;
    std::fs::create_dir_all(blocks_dir())?;
    std::fs::create_dir_all(chainstate_dir())?;
    std::fs::create_dir_all(state_db_path())?;
    Ok(())
}

pub fn exists() -> bool {
    datadir().exists()
}

pub fn as_str(p: &Path) -> String {
    p.to_string_lossy().to_string()
}

pub fn append_debug_log(line: &str) {
    let _ = ensure_dirs();
    let path = debug_log_path();
    let ts = crate::utils::now_millis();
    let entry = format!("{} {}\n", ts, line);
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = f.write_all(entry.as_bytes());
    }
}
