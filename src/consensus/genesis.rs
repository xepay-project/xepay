// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Genesis consensus parameters for XEPAY.
//!
//! Each network has its own genesis block. The mainnet genesis is
//! locked forever; the testnet and regtest genesis blocks may be
//! regenerated when the consensus rules change.

use crate::consensus::spec::{self, Network}; // Network selector and parameters.

// ----------------------------------------------------------------------
// Shared genesis messages (same on all networks)
// ----------------------------------------------------------------------

// The message embedded in the genesis coinbase transaction.
pub const GENESIS_MESSAGE: &str = "XEPAY Genesis - 2026";
// The secret embedded in the genesis secret transaction.
pub const GENESIS_SECRET: &str = "ID Today 15/Oct/2026";
// The output script used in the genesis block.
pub const GENESIS_OUTPUT_SCRIPT: &str =
    "OP_DUP OP_HASH160 XEPAYGenesisOutput OP_EQUALVERIFY OP_CHECKSIG";

// ----------------------------------------------------------------------
// Mainnet genesis
// ----------------------------------------------------------------------

// The mainnet genesis timestamp (in milliseconds).
pub const GENESIS_TIMESTAMP: u128 = 1792303198445;

// The expected mainnet genesis hash.
pub const EXPECTED_GENESIS_HASH: &str =
    "0000050159a15b33b8a726aabce26cec6ef14f96340fa07d481d445d3ac25a1a";

// The expected mainnet genesis merkle root.
pub const EXPECTED_GENESIS_MERKLE_ROOT: &str =
    "8d76a0d9d58b20892687b8269e9626dfd007c9aae4e362ffac8407df75a23fc3";

// The expected mainnet genesis timestamp.
pub const EXPECTED_GENESIS_TIMESTAMP: u128 = 1792303198445;

// The expected mainnet genesis nonce.
pub const EXPECTED_GENESIS_NONCE: u64 = 835093;

// ----------------------------------------------------------------------
// Testnet genesis
// ----------------------------------------------------------------------
//
// The testnet genesis is mined at build time by the `xepay-genesis`
// binary with the `-testnet` flag. The resulting hash, merkle root,
// timestamp, and nonce must be pasted into this module.

// The testnet genesis timestamp (in milliseconds).
pub const TESTNET_GENESIS_TIMESTAMP: u128 = 1792303198446;

// The expected testnet genesis hash.
pub const TESTNET_EXPECTED_GENESIS_HASH: &str =
    "00000794f3bd67af183fd6b237b08d4e87ebd27758446fd1687ef9c4afd97dc9";

// The expected testnet genesis merkle root.
pub const TESTNET_EXPECTED_GENESIS_MERKLE_ROOT: &str =
    "1d6f1c25b8d4e95bb1fdd588c236480a0a3772d115a78bdcb728b457613d9b6e";

// The expected testnet genesis timestamp.
pub const TESTNET_EXPECTED_GENESIS_TIMESTAMP: u128 = 1792303198446;

// The expected testnet genesis nonce.
pub const TESTNET_EXPECTED_GENESIS_NONCE: u64 = 1892429;

// ----------------------------------------------------------------------
// Regtest genesis
// ----------------------------------------------------------------------
//
// The regtest genesis is mined at build time by the `xepay-genesis`
// binary with the `-regtest` flag. The resulting hash, merkle root,
// timestamp, and nonce must be pasted into this module.

// The regtest genesis timestamp (in milliseconds).
pub const REGTEST_GENESIS_TIMESTAMP: u128 = 1792303198447;

// The expected regtest genesis hash.
pub const REGTEST_EXPECTED_GENESIS_HASH: &str =
    "41015a249ae2cfd0c4f997f00115701be383bf8d3adc0e76fe0bad5c3413af53";

// The expected regtest genesis merkle root.
pub const REGTEST_EXPECTED_GENESIS_MERKLE_ROOT: &str =
    "8d76a0d9d58b20892687b8269e9626dfd007c9aae4e362ffac8407df75a23fc3";

// The expected regtest genesis timestamp.
pub const REGTEST_EXPECTED_GENESIS_TIMESTAMP: u128 = 1792303198447;

// The expected regtest genesis nonce.
pub const REGTEST_EXPECTED_GENESIS_NONCE: u64 = 0;

// ----------------------------------------------------------------------
// Network-aware accessors
// ----------------------------------------------------------------------

// Returns the shared genesis message.
pub fn genesis_message() -> &'static str {
    GENESIS_MESSAGE // Return the constant.
}

// Returns the shared genesis secret.
pub fn genesis_secret() -> &'static str {
    GENESIS_SECRET // Return the constant.
}

// Returns the genesis timestamp for the active network.
pub fn genesis_timestamp() -> u128 {
    spec::current_network().genesis_timestamp() // Fetch from the spec.
}

// Returns the shared genesis output script.
pub fn genesis_output_script() -> String {
    GENESIS_OUTPUT_SCRIPT.to_string() // Return the constant as a String.
}

// Returns the expected genesis hash for the given network.
pub fn expected_genesis_hash_for(network: Network) -> &'static str {
    match network {
        Network::Main => EXPECTED_GENESIS_HASH, // Mainnet hash.
        Network::Test => TESTNET_EXPECTED_GENESIS_HASH, // Testnet hash.
        Network::Regtest => REGTEST_EXPECTED_GENESIS_HASH, // Regtest hash.
    }
}

// Returns the expected genesis merkle root for the given network.
pub fn expected_genesis_merkle_root_for(network: Network) -> &'static str {
    match network {
        Network::Main => EXPECTED_GENESIS_MERKLE_ROOT, // Mainnet merkle root.
        Network::Test => TESTNET_EXPECTED_GENESIS_MERKLE_ROOT, // Testnet merkle root.
        Network::Regtest => REGTEST_EXPECTED_GENESIS_MERKLE_ROOT, // Regtest merkle root.
    }
}

// Returns the expected genesis timestamp for the given network.
pub fn expected_genesis_timestamp_for(network: Network) -> u128 {
    match network {
        Network::Main => EXPECTED_GENESIS_TIMESTAMP, // Mainnet timestamp.
        Network::Test => TESTNET_EXPECTED_GENESIS_TIMESTAMP, // Testnet timestamp.
        Network::Regtest => REGTEST_EXPECTED_GENESIS_TIMESTAMP, // Regtest timestamp.
    }
}

// Returns the expected genesis nonce for the given network.
pub fn expected_genesis_nonce_for(network: Network) -> u64 {
    match network {
        Network::Main => EXPECTED_GENESIS_NONCE, // Mainnet nonce.
        Network::Test => TESTNET_EXPECTED_GENESIS_NONCE, // Testnet nonce.
        Network::Regtest => REGTEST_EXPECTED_GENESIS_NONCE, // Regtest nonce.
    }
}

// NOTE: The legacy accessors `expected_genesis_hash()`,
// `expected_genesis_merkle_root()`, and `expected_genesis_nonce()` have
// been removed. They always returned Mainnet values and were a footgun
// for Testnet/Regtest code. Use the `_for(network)` variants instead,
// or call `expected_genesis_*_for(spec::current_network())` if you
// need the value for the active network.
