// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Consensus module for XEPAY.

pub mod genesis;
pub mod pow;
pub mod rules;
pub mod spec;

use crate::blockchain;
use crate::consensus::spec::Network;
use xepay_crypto::sha3_256_hex;

pub fn consensus_fingerprint_for(network: Network) -> String {
    let softfork_fingerprint: String = blockchain::SOFT_FORK_DEPLOYMENTS
        .iter()
        .map(|(name, d)| format!("{}:{}:{}:{}:{}:{}:{}", name, d.bit, d.start_time, d.timeout, d.min_activation_height, d.period, d.threshold))
        .collect::<Vec<_>>()
        .join(",");

    let payload = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        network.name(),
        spec::HASH_ALGO,
        spec::SIGNATURE_ALGO,
        spec::DECIMALS,
        spec::ALD_PER_XEPC,
        spec::MAX_SUPPLY_XEPC,
        spec::MAX_SUPPLY,
        spec::INITIAL_REWARD_XEPC,
        spec::INITIAL_REWARD,
        spec::SUBSIDY_HALVING_INTERVAL,
        spec::MAX_HALVINGS,
        spec::POW_TARGET_SPACING_SECS,
        spec::POW_TARGET_TIMESPAN_SECS,
        spec::MINER_CONFIRMATION_WINDOW,
        spec::DIFFICULTY_ADJUST_INTERVAL,
        genesis::GENESIS_MESSAGE,
        genesis::GENESIS_SECRET,
        network.genesis_timestamp(),
        genesis::GENESIS_OUTPUT_SCRIPT,
        network.genesis_nbits(),
        pow::pow_limit(),
        rules::COINBASE_MATURITY,
        blockchain::bech32_hrp_for(network),
        rules::MAX_TX_PER_BLOCK,
        rules::MAX_NOTE_LEN,
        rules::BLOCK_VERSION,
        rules::ZERO_HASH,
        softfork_fingerprint,
    );
    sha3_256_hex(payload.as_bytes())
}

pub fn consensus_fingerprint() -> String {
    consensus_fingerprint_for(spec::current_network())
}

pub fn parameter_fingerprint_for(network: Network) -> String {
    let softfork_fingerprint: String = blockchain::SOFT_FORK_DEPLOYMENTS
        .iter()
        .map(|(name, d)| format!("{}:{}:{}:{}:{}:{}:{}", name, d.bit, d.start_time, d.timeout, d.min_activation_height, d.period, d.threshold))
        .collect::<Vec<_>>()
        .join(",");

    let payload = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        network.name(),
        spec::HASH_ALGO,
        spec::SIGNATURE_ALGO,
        spec::DECIMALS,
        spec::ALD_PER_XEPC,
        spec::MAX_SUPPLY_XEPC,
        spec::MAX_SUPPLY,
        spec::INITIAL_REWARD_XEPC,
        spec::INITIAL_REWARD,
        spec::SUBSIDY_HALVING_INTERVAL,
        spec::MAX_HALVINGS,
        spec::POW_TARGET_SPACING_SECS,
        spec::POW_TARGET_TIMESPAN_SECS,
        spec::MINER_CONFIRMATION_WINDOW,
        spec::DIFFICULTY_ADJUST_INTERVAL,
        genesis::GENESIS_MESSAGE,
        genesis::GENESIS_SECRET,
        network.genesis_timestamp(),
        genesis::GENESIS_OUTPUT_SCRIPT,
        network.genesis_nbits(),
        pow::pow_limit(),
        rules::COINBASE_MATURITY,
        blockchain::bech32_hrp_for(network),
        rules::MAX_TX_PER_BLOCK,
        rules::MAX_NOTE_LEN,
        rules::BLOCK_VERSION,
        rules::ZERO_HASH,
        softfork_fingerprint,
    );
    sha3_256_hex(payload.as_bytes())
}

pub fn parameter_fingerprint() -> String {
    parameter_fingerprint_for(spec::current_network())
}

pub fn genesis_coinbase_txid_for(network: Network) -> String {
    let fp = parameter_fingerprint_for(network);
    let payload = format!(
        "genesis|{}|{}|{}",
        fp,
        network.genesis_timestamp(),
        network.genesis_nbits()
    );
    sha3_256_hex(payload.as_bytes())
}

pub fn genesis_coinbase_txid() -> String {
    genesis_coinbase_txid_for(spec::current_network())
}

pub use genesis::genesis_message;
pub use genesis::genesis_output_script;
pub use genesis::genesis_secret;
pub use genesis::genesis_timestamp;
pub use pow::genesis_nbits;
pub use pow::pow_limit;
pub use rules::max_tx_per_block;
pub use spec::coin_name;
pub use spec::decimals;
pub use spec::difficulty_adjust_interval;
pub use spec::hash_algo;
pub use spec::initial_reward;
pub use spec::initial_reward_xepc;
pub use spec::max_halvings;
pub use spec::max_supply;
pub use spec::max_supply_xepc;
pub use spec::miner_confirmation_window;
pub use spec::pow_target_spacing_secs;
pub use spec::pow_target_timespan_secs;
pub use spec::protocol_version;
pub use spec::signature_algo;
pub use spec::subsidy_halving_interval;
pub use spec::symbol;
pub use spec::version;

// Re-export the new unit-related constants.
pub use spec::ald_per_xepc;
pub use spec::UNIT_NAME;
pub use spec::UNIT_SYMBOL;
pub use spec::ALD_PER_XEPC;
