// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Economic consensus parameters for XEPAY.

use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Main,
    Test,
    Regtest,
}

impl Network {
    pub fn name(&self) -> &'static str {
        match self {
            Network::Main => "main",
            Network::Test => "test",
            Network::Regtest => "regtest",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "main" => Some(Network::Main),
            "test" | "testnet" => Some(Network::Test),
            "regtest" => Some(Network::Regtest),
            _ => None,
        }
    }

    pub fn magic(&self) -> [u8; 4] {
        match self {
            Network::Main => NETWORK_MAGIC_MAIN,
            Network::Test => NETWORK_MAGIC_TEST,
            Network::Regtest => NETWORK_MAGIC_REGTEST,
        }
    }

    pub fn magic_hex(&self) -> String {
        let m = self.magic();
        format!("{:02x}{:02x}{:02x}{:02x}", m[0], m[1], m[2], m[3])
    }

    pub fn default_port(&self) -> u16 {
        match self {
            Network::Main => DEFAULT_PORT_MAIN,
            Network::Test => DEFAULT_PORT_TEST,
            Network::Regtest => DEFAULT_PORT_REGTEST,
        }
    }

    pub fn default_rpc_port(&self) -> u16 {
        match self {
            Network::Main => DEFAULT_RPC_PORT_MAIN,
            Network::Test => DEFAULT_RPC_PORT_TEST,
            Network::Regtest => DEFAULT_RPC_PORT_REGTEST,
        }
    }

    pub fn datadir_name(&self) -> &'static str {
        match self {
            Network::Main => DATADIR_NAME_MAIN,
            Network::Test => DATADIR_NAME_TEST,
            Network::Regtest => DATADIR_NAME_REGTEST,
        }
    }

    pub fn genesis_timestamp(&self) -> u128 {
        match self {
            Network::Main => GENESIS_TIMESTAMP_MAIN,
            Network::Test => GENESIS_TIMESTAMP_TEST,
            Network::Regtest => GENESIS_TIMESTAMP_REGTEST,
        }
    }

    pub fn genesis_nbits(&self) -> u32 {
        match self {
            Network::Main => GENESIS_NBITS_MAIN,
            Network::Test => GENESIS_NBITS_TEST,
            Network::Regtest => GENESIS_NBITS_REGTEST,
        }
    }

    pub fn expected_genesis_nonce(&self) -> u64 {
        match self {
            Network::Main => EXPECTED_GENESIS_NONCE_MAIN,
            Network::Test => EXPECTED_GENESIS_NONCE_TEST,
            Network::Regtest => EXPECTED_GENESIS_NONCE_REGTEST,
        }
    }
}

impl Default for Network {
    fn default() -> Self {
        Network::Main
    }
}

static CURRENT_NETWORK: OnceLock<Network> = OnceLock::new();

pub fn current_network() -> Network {
    *CURRENT_NETWORK.get_or_init(|| {
        if let Ok(v) = std::env::var("XEPAY_NETWORK") {
            if let Some(n) = Network::from_name(&v) {
                return n;
            }
        }
        Network::Main
    })
}

pub fn set_current_network(network: Network) {
    let _ = CURRENT_NETWORK.set(network);
}

// ----------------------------------------------------------------------
// Coin identity (shared by all networks)
// ----------------------------------------------------------------------

pub const COIN_NAME: &str = "XEPAY";
pub const SYMBOL: &str = "XEPC";

/// Name of the smallest unit (like "satoshi" in Bitcoin).
pub const UNIT_NAME: &str = "Aldian";
/// Symbol of the smallest unit.
pub const UNIT_SYMBOL: &str = "ALD";

pub const VERSION: &str = "0.1.0";
pub const PROTOCOL_VERSION: u32 = 1;

pub const HASH_ALGO: &str = "SHA3-256";
pub const SIGNATURE_ALGO: &str = "secp256k1-ECDSA";
pub const DECIMALS: u32 = 8;

/// The number of Aldian in one XEPC.
/// 1 XEPC = 100_000_000 Aldian (8 decimals).
pub const ALD_PER_XEPC: u64 = 100_000_000;

// === ECONOMIC PARAMETERS ===
// Total supply: 485,000,000 XEPC
// Initial reward: 15 XEPC
// Halving interval: 16,166,666 blocks
pub const MAX_SUPPLY_XEPC: u64 = 485_000_000;
pub const MAX_SUPPLY: u64 = MAX_SUPPLY_XEPC * ALD_PER_XEPC;

pub const INITIAL_REWARD_XEPC: u64 = 15;
pub const INITIAL_REWARD: u64 = INITIAL_REWARD_XEPC * ALD_PER_XEPC;

pub const SUBSIDY_HALVING_INTERVAL: u64 = 16_166_666;
pub const MAX_HALVINGS: u32 = 64;

pub const POW_TARGET_SPACING_SECS: u64 = 15;
pub const POW_TARGET_TIMESPAN_SECS: u64 = DIFFICULTY_ADJUST_INTERVAL * POW_TARGET_SPACING_SECS;
pub const MINER_CONFIRMATION_WINDOW: u64 = 69_120;
pub const DIFFICULTY_ADJUST_INTERVAL: u64 = 69_120;

// ----------------------------------------------------------------------
// Mainnet parameters
// ----------------------------------------------------------------------

pub const NETWORK_MAGIC_MAIN: [u8; 4] = [0x58, 0x45, 0x50, 0x43]; // "XEPC"
pub const DEFAULT_PORT_MAIN: u16 = 7555;
pub const DEFAULT_RPC_PORT_MAIN: u16 = 7554;
pub const DATADIR_NAME_MAIN: &str = ".xepay";
pub const GENESIS_TIMESTAMP_MAIN: u128 = 1792303198445;
pub const GENESIS_NBITS_MAIN: u32 = 0x1e0ffff0;
pub const EXPECTED_GENESIS_NONCE_MAIN: u64 = 849719;

// ----------------------------------------------------------------------
// Testnet parameters
// ----------------------------------------------------------------------

pub const NETWORK_MAGIC_TEST: [u8; 4] = [0x5a, 0x54, 0x45, 0x53]; // "ZTES"
pub const DEFAULT_PORT_TEST: u16 = 12255;
pub const DEFAULT_RPC_PORT_TEST: u16 = 12254;
pub const DATADIR_NAME_TEST: &str = ".xepay-testnet";
pub const GENESIS_TIMESTAMP_TEST: u128 = 1792303198446;
pub const GENESIS_NBITS_TEST: u32 = 0x1e0ffff0;
pub const EXPECTED_GENESIS_NONCE_TEST: u64 = 1892429;

// ----------------------------------------------------------------------
// Regtest parameters
// ----------------------------------------------------------------------

pub const NETWORK_MAGIC_REGTEST: [u8; 4] = [0x5a, 0x52, 0x45, 0x47]; // "ZREG"
pub const DEFAULT_PORT_REGTEST: u16 = 18444;
pub const DEFAULT_RPC_PORT_REGTEST: u16 = 18443;
pub const DATADIR_NAME_REGTEST: &str = ".xepay-regtest";
pub const GENESIS_TIMESTAMP_REGTEST: u128 = 1792303198447;
pub const GENESIS_NBITS_REGTEST: u32 = 0x207fffff;
pub const EXPECTED_GENESIS_NONCE_REGTEST: u64 = 2;

// ----------------------------------------------------------------------
// Legacy accessors
// ----------------------------------------------------------------------

pub fn coin_name() -> &'static str { COIN_NAME }
pub fn symbol() -> &'static str { SYMBOL }
pub fn version() -> &'static str { VERSION }
pub fn protocol_version() -> u32 { PROTOCOL_VERSION }
pub fn hash_algo() -> &'static str { HASH_ALGO }
pub fn signature_algo() -> &'static str { SIGNATURE_ALGO }
pub fn decimals() -> u32 { DECIMALS }
pub fn ald_per_xepc() -> u64 { ALD_PER_XEPC }
pub fn max_supply_xepc() -> u64 { MAX_SUPPLY_XEPC }
pub fn max_supply() -> u64 { MAX_SUPPLY }
pub fn initial_reward_xepc() -> u64 { INITIAL_REWARD_XEPC }
pub fn initial_reward() -> u64 { INITIAL_REWARD }
pub fn subsidy_halving_interval() -> u64 { SUBSIDY_HALVING_INTERVAL }
pub fn max_halvings() -> u32 { MAX_HALVINGS }
pub fn pow_target_spacing_secs() -> u64 { POW_TARGET_SPACING_SECS }
pub fn pow_target_timespan_secs() -> u64 { POW_TARGET_TIMESPAN_SECS }
pub fn miner_confirmation_window() -> u64 { MINER_CONFIRMATION_WINDOW }
pub fn difficulty_adjust_interval() -> u64 { DIFFICULTY_ADJUST_INTERVAL }
