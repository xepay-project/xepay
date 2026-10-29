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

// ----------------------------------------------------------------------
// Genesis output script (P2PK, same format as the reference implementation)
// ----------------------------------------------------------------------
//
// The genesis output script locks the genesis block reward to a specific
// public key. This mirrors the reference implementation's approach, which
// uses a P2PK script:
//
//     <public_key> OP_CHECKSIG
//
// The public key below is an uncompressed 65-byte secp256k1 public key.
// Its corresponding private key is unknown, so the genesis output can
// never be spent. This prevents any "pre-mine" from being claimed.
//
// Using a P2PK script (rather than the previous P2PKH placeholder) makes
// the script valid and parseable by a standard script interpreter. This
// is required if XEPAY later adds script execution or multi-signature
// support.
pub const GENESIS_OUTPUT_SCRIPT: &str =
    "04678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5f OP_CHECKSIG";

// ----------------------------------------------------------------------
// Mainnet genesis
// ----------------------------------------------------------------------

// The mainnet genesis timestamp (in milliseconds).
pub const GENESIS_TIMESTAMP: u128 = 1792303198445;

// The expected mainnet genesis hash.
//
// NOTE: This value must be regenerated after any change to
// GENESIS_OUTPUT_SCRIPT. Run `xepay-genesis` and paste the output here.
pub const EXPECTED_GENESIS_HASH: &str =
    "000004fba201c1e100435b85d52023062878e46b008bc4762f60cc6afbfba179";

// The expected mainnet genesis merkle root.
pub const EXPECTED_GENESIS_MERKLE_ROOT: &str =
    "8af01af7ca4b233428b7c2bf6ea9de4148272a5aea72dd16cda0870435aba720";

// The expected mainnet genesis timestamp.
pub const EXPECTED_GENESIS_TIMESTAMP: u128 = 1792303198445;

// The expected mainnet genesis nonce.
pub const EXPECTED_GENESIS_NONCE: u64 = 803050;

// ----------------------------------------------------------------------
// Testnet genesis
// ----------------------------------------------------------------------

// The testnet genesis timestamp (in milliseconds).
pub const TESTNET_GENESIS_TIMESTAMP: u128 = 1792303198446;

// The expected testnet genesis hash.
//
// NOTE: This value must be regenerated after any change to
// GENESIS_OUTPUT_SCRIPT. Run `xepay-genesis -testnet` and paste the
// output here.
pub const TESTNET_EXPECTED_GENESIS_HASH: &str =
    "PLACEHOLDER_RUN_XEPAY_GENESIS_TESTNET";

// The expected testnet genesis merkle root.
pub const TESTNET_EXPECTED_GENESIS_MERKLE_ROOT: &str =
    "PLACEHOLDER_RUN_XEPAY_GENESIS_TESTNET";

// The expected testnet genesis timestamp.
pub const TESTNET_EXPECTED_GENESIS_TIMESTAMP: u128 = 1792303198446;

// The expected testnet genesis nonce.
pub const TESTNET_EXPECTED_GENESIS_NONCE: u64 = 0;

// ----------------------------------------------------------------------
// Regtest genesis
// ----------------------------------------------------------------------

// The regtest genesis timestamp (in milliseconds).
pub const REGTEST_GENESIS_TIMESTAMP: u128 = 1792303198447;

// The expected regtest genesis hash.
//
// NOTE: This value must be regenerated after any change to
// GENESIS_OUTPUT_SCRIPT. Run `xepay-genesis -regtest` and paste the
// output here.
pub const REGTEST_EXPECTED_GENESIS_HASH: &str =
    "PLACEHOLDER_RUN_XEPAY_GENESIS_REGTEST";

// The expected regtest genesis merkle root.
pub const REGTEST_EXPECTED_GENESIS_MERKLE_ROOT: &str =
    "PLACEHOLDER_RUN_XEPAY_GENESIS_REGTEST";

// The expected regtest genesis timestamp.
pub const REGTEST_EXPECTED_GENESIS_TIMESTAMP: u128 = 1792303198447;

// The expected regtest genesis nonce.
pub const REGTEST_EXPECTED_GENESIS_NONCE: u64 = 0;

// ----------------------------------------------------------------------
// Network-aware accessors
// ----------------------------------------------------------------------

// Returns the shared genesis message.
pub fn genesis_message() -> &'static str {
    GENESIS_MESSAGE
}

// Returns the shared genesis secret.
pub fn genesis_secret() -> &'static str {
    GENESIS_SECRET
}

// Returns the genesis timestamp for the active network.
pub fn genesis_timestamp() -> u128 {
    spec::current_network().genesis_timestamp()
}

// Returns the shared genesis output script.
pub fn genesis_output_script() -> String {
    GENESIS_OUTPUT_SCRIPT.to_string()
}

// Returns the expected genesis hash for the given network.
pub fn expected_genesis_hash_for(network: Network) -> &'static str {
    match network {
        Network::Main => EXPECTED_GENESIS_HASH,
        Network::Test => TESTNET_EXPECTED_GENESIS_HASH,
        Network::Regtest => REGTEST_EXPECTED_GENESIS_HASH,
    }
}

// Returns the expected genesis merkle root for the given network.
pub fn expected_genesis_merkle_root_for(network: Network) -> &'static str {
    match network {
        Network::Main => EXPECTED_GENESIS_MERKLE_ROOT,
        Network::Test => TESTNET_EXPECTED_GENESIS_MERKLE_ROOT,
        Network::Regtest => REGTEST_EXPECTED_GENESIS_MERKLE_ROOT,
    }
}

// Returns the expected genesis timestamp for the given network.
pub fn expected_genesis_timestamp_for(network: Network) -> u128 {
    match network {
        Network::Main => EXPECTED_GENESIS_TIMESTAMP,
        Network::Test => TESTNET_EXPECTED_GENESIS_TIMESTAMP,
        Network::Regtest => REGTEST_EXPECTED_GENESIS_TIMESTAMP,
    }
}

// Returns the expected genesis nonce for the given network.
pub fn expected_genesis_nonce_for(network: Network) -> u64 {
    match network {
        Network::Main => EXPECTED_GENESIS_NONCE,
        Network::Test => TESTNET_EXPECTED_GENESIS_NONCE,
        Network::Regtest => REGTEST_EXPECTED_GENESIS_NONCE,
    }
}
