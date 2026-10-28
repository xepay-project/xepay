// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Anchor connections for eclipse attack resistance.
//!
//! Bitcoin Core maintains up to 2 block-relay-only anchor connections
//! that persist across restarts. These connections are stored in
//! anchors.dat and help prevent eclipse attacks [citation:1][citation:5].

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;

use crate::datadir;

/// Maximum number of block-relay-only anchor connections.
pub const MAX_BLOCK_RELAY_ONLY_ANCHORS: usize = 2;

/// Anchor database file name.
pub const ANCHORS_DATABASE_FILENAME: &str = "anchors.dat";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anchor {
    pub addr: SocketAddr,
}

/// Load anchors from disk. Returns an empty vector if the file does not
/// exist or cannot be parsed.
pub fn load_anchors() -> Vec<Anchor> {
    let path = anchors_path();
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    let mut anchors: Vec<Anchor> = serde_json::from_str(&data).unwrap_or_default();
    // Cap the number of anchors to the maximum allowed.
    if anchors.len() > MAX_BLOCK_RELAY_ONLY_ANCHORS {
        anchors.truncate(MAX_BLOCK_RELAY_ONLY_ANCHORS);
    }
    anchors
}

/// Save anchors to disk. This function is called only during a clean
/// shutdown. On an unclean shutdown, the file is not written, which
/// prevents a malicious anchor from crashing the node and being
/// reconnected on the next start [citation:5].
pub fn save_anchors(anchors: &[Anchor]) -> std::io::Result<()> {
    datadir::ensure_dirs()?;
    let path = anchors_path();
    let capped: Vec<&Anchor> = anchors.iter().take(MAX_BLOCK_RELAY_ONLY_ANCHORS).collect();
    let json = serde_json::to_string_pretty(&capped).map_err(std::io::Error::other)?;
    std::fs::write(path, json)
}

/// Delete the anchors file. Bitcoin Core deletes anchors.dat after
/// loading to avoid reconnecting to anchors after an unclean shutdown
/// [citation:16].
pub fn delete_anchors_file() {
    let _ = std::fs::remove_file(anchors_path());
}

fn anchors_path() -> PathBuf {
    datadir::datadir().join(ANCHORS_DATABASE_FILENAME)
}
