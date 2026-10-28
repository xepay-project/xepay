// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Proof-of-work consensus parameters for XEPAY.

/// Genesis nBits. This value is the same as Litecoin: 0x1e0ffff0.
pub const GENESIS_NBITS: u32 = 0x1e0ffff0;

/// The proof-of-work limit for Mainnet and Testnet.
/// This value is the same as Litecoin: 00000fffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
pub const POW_LIMIT_MAIN: &str = "00000fffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

/// The proof-of-work limit for Regtest.
/// This value is very easy: 7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
pub const POW_LIMIT_REGTEST: &str =
    "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

pub fn genesis_nbits() -> u32 {
    GENESIS_NBITS
}

pub fn pow_limit() -> &'static str {
    match crate::consensus::spec::current_network() {
        crate::consensus::spec::Network::Main => POW_LIMIT_MAIN,
        crate::consensus::spec::Network::Test => POW_LIMIT_MAIN,
        crate::consensus::spec::Network::Regtest => POW_LIMIT_REGTEST,
    }
}
