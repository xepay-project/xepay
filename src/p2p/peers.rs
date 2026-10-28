// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Peer bootstrap parameters for XEPAY.
//!
//! The default values in this module are used when `xepay.conf`
//! does not provide an override. Change the values through the
//! configuration file, not by editing this module.

pub const DEFAULT_LISTEN_ADDR: &str = "0.0.0.0:7555";
pub const DEFAULT_BOOTSTRAP: &[&str] = &[];

pub const MAX_PEERS: usize = 32;
pub const MAX_OUTBOUND: usize = 8;

// Set the maximum number of headers per request to 5,000 for faster
// testnet synchronization. For mainnet, this should be lowered to 2000.
pub const HEADERS_PER_REQUEST: u64 = 5_000;

pub const GENESIS_HEIGHT: u64 = 0;
