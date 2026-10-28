// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

use xepay::block::Block;
use xepay::consensus::spec::{self, Network};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut selected_network = Network::Main;

    for arg in &args[1..] {
        match arg.as_str() {
            "-testnet" | "--testnet" => selected_network = Network::Test,
            "-regtest" | "--regtest" => selected_network = Network::Regtest,
            "--help" | "-h" => {
                println!("Usage: xepay-genesis [-testnet|--testnet] [-regtest|--regtest]");
                return;
            }
            other => {
                eprintln!("error: unknown option: {}", other);
                std::process::exit(1);
            }
        }
    }

    // The network MUST be set before calling any consensus-dependent
    // function. This ensures that the signing hash (which includes the
    // network magic bytes as a chain ID) and all other network-specific
    // parameters are correct for the genesis block.
    spec::set_current_network(selected_network);

    let b = Block::genesis_unchecked();

    println!();
    println!("============================================================");
    println!("  XEPCHAIN GENESIS ({})", selected_network.name());
    println!("  (copy into src/consensus/genesis.rs)");
    println!("============================================================");
    println!();

    match selected_network {
        Network::Main => {
            println!("  pub const EXPECTED_GENESIS_HASH: &str =");
            println!("      \"{}\";", b.hash);
            println!();
            println!("  pub const EXPECTED_GENESIS_MERKLE_ROOT: &str =");
            println!("      \"{}\";", b.merkle_root);
            println!();
            println!(
                "  pub const EXPECTED_GENESIS_TIMESTAMP: u128 = {};",
                b.timestamp
            );
            println!();
            println!("  pub const EXPECTED_GENESIS_NONCE: u64 = {};", b.nonce);
        }
        Network::Test => {
            println!("  pub const TESTNET_EXPECTED_GENESIS_HASH: &str =");
            println!("      \"{}\";", b.hash);
            println!();
            println!("  pub const TESTNET_EXPECTED_GENESIS_MERKLE_ROOT: &str =");
            println!("      \"{}\";", b.merkle_root);
            println!();
            println!(
                "  pub const TESTNET_EXPECTED_GENESIS_TIMESTAMP: u128 = {};",
                b.timestamp
            );
            println!();
            println!(
                "  pub const TESTNET_EXPECTED_GENESIS_NONCE: u64 = {};",
                b.nonce
            );
        }
        Network::Regtest => {
            println!("  pub const REGTEST_EXPECTED_GENESIS_HASH: &str =");
            println!("      \"{}\";", b.hash);
            println!();
            println!("  pub const REGTEST_EXPECTED_GENESIS_MERKLE_ROOT: &str =");
            println!("      \"{}\";", b.merkle_root);
            println!();
            println!(
                "  pub const REGTEST_EXPECTED_GENESIS_TIMESTAMP: u128 = {};",
                b.timestamp
            );
            println!();
            println!(
                "  pub const REGTEST_EXPECTED_GENESIS_NONCE: u64 = {};",
                b.nonce
            );
        }
    }
    println!("============================================================");
    println!();
}
