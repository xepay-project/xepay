// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Consensus validation rule constants for XEPAY.

// The number of blocks a coinbase reward must mature before it can be spent.
pub const COINBASE_MATURITY: u64 = 100;

// The maximum number of transactions allowed in a single block.
pub const MAX_TX_PER_BLOCK: usize = 10_000;

// The maximum number of inputs per transaction (unused in the account-based model).
pub const MAX_INPUTS_PER_TX: usize = 1_000;

// The maximum number of outputs per transaction (unused in the account-based model).
pub const MAX_OUTPUTS_PER_TX: usize = 1_000;

// The maximum length of a transaction note, in bytes.
// Increased from 256 to 4096 to accommodate smart contract call data.
pub const MAX_NOTE_LEN: usize = 4096;

// The zero hash used as the `previous_hash` of the genesis block.
pub const ZERO_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

// The block version supported by the current consensus rules.
pub const BLOCK_VERSION: u32 = 1;

// Helper function to return the maximum number of transactions per block.
pub fn max_tx_per_block() -> usize {
    MAX_TX_PER_BLOCK // Return the constant value.
}
