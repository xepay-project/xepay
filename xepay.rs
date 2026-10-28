// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// xepay.rs
// Entry point for the XEPAY (ticker: XEPY) blockchain, v0.2.
//
// Two modes:
//   xepay                — run the end-to-end demo (default, no arguments)
//   xepay run            — run the end-to-end demo
//   xepay <command> ...  — dispatch a CLI command (see cli.rs)

#[path = "crypto.rs"]
mod crypto;
#[path = "transaction.rs"]
mod transaction;
#[path = "wallet.rs"]
mod wallet;
#[path = "mempool.rs"]
mod mempool;
#[path = "block.rs"]
mod block;
#[path = "blockchain.rs"]
mod blockchain;
#[path = "storage/storage.rs"]
mod storage;
#[path = "cli.rs"]
mod cli;

use blockchain::Blockchain;
use mempool::Mempool;
use transaction::Transaction;
use wallet::Wallet;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Route to the CLI unless the caller explicitly asked for the demo.
    // No arguments or the literal `run` triggers the demo; anything else
    // is forwarded to the CLI dispatcher.
    let is_demo = args.is_empty() || args.first().map(String::as_str) == Some("run");
    if !is_demo {
        cli::run(&args);
        return;
    }

    run_demo();
}

/// Run the end-to-end demonstration.
///
/// The demonstration performs the following sequence:
///   1. Prints a banner identifying the network and target block time.
///   2. Creates three wallets (alice, bob, carol) for illustrative purposes.
///   3. Mines a coinbase block paying alice, then reports her balance.
///   4. Builds a mempool with two signed transactions from alice.
///   5. Mines a block selecting the highest-fee transactions, paying bob.
///   6. Persists the chain to ./xepay.db and restores it to verify integrity.
///   7. Prints a summary of chain integrity, height, average block time,
///      and final balances.
fn run_demo() {
    print_banner();

    let difficulty = 3;
    let mut chain = Blockchain::new(difficulty);

    // Create three wallets for the demonstration.
    let alice = Wallet::generate();
    let bob = Wallet::generate();
    let carol = Wallet::generate();
    println!("[XEPY] alice: {}", alice.address());
    println!("[XEPY] bob:   {}", bob.address());
    println!("[XEPY] carol: {}\n", carol.address());

    // Fund alice by mining a coinbase block addressed to her.
    chain.add_block(vec![], &alice.address()).unwrap();
    println!(
        "[XEPY] Alice funded; balance = {}\n",
        chain.balances.get(&alice.address()).copied().unwrap_or(0)
    );

    // Build a mempool containing two fee-paying transactions from alice.
    // The fees differ so the fee-based selection is observable in the output.
    let mut pool = Mempool::new();
    let tx1 = Transaction::new_signed(
        &alice.signing_key(),
        &bob.address(),
        10_0000_0000,
        100,
        0,
    );
    let tx2 = Transaction::new_signed(
        &alice.signing_key(),
        &carol.address(),
        5_0000_0000,
        500,
        1,
    );
    assert!(pool.add(tx1));
    assert!(pool.add(tx2));

    // Mine a block selecting up to ten transactions, paying fees to bob.
    // The emptiness check is a genuine guard: an empty mempool still
    // produces a valid block containing only the coinbase.
    if pool.is_empty() {
        println!("[XEPY] Mempool empty; mining empty block.");
    } else {
        println!("[XEPY] Mempool size = {}", pool.len());
    }

    let picked = pool.drain_highest_fee(10);
    chain.add_block(picked, &bob.address()).unwrap();
    println!(
        "[XEPY] Block mined; height = {}; fees = {}\n",
        chain.height(),
        chain.tip().total_fees()
    );

    // Persist the chain to disk and immediately restore it. Restoration
    // re-verifies every block, so a successful restore is a functional
    // integrity check as well as a persistence check.
    {
        let store = storage::Storage::open("xepay.db").unwrap();
        store.save_chain(&chain).unwrap();
        println!("[XEPY] Chain persisted to xepay.db");

        let restored = store.restore_chain(difficulty).unwrap().unwrap();
        println!("[XEPY] Restored chain height = {}\n", restored.height());
    }

    // Emit the final report.
    println!("Chain integrity: {}", chain.is_valid());
    println!("Total blocks:    {}", chain.height());
    println!("Avg block time:  {:.2}s", chain.average_block_time_secs());
    println!();
    println!("Balances:");
    for (addr, bal) in &chain.balances {
        println!("  {}: {}", &addr[..16], bal);
    }
    println!();
    println!("[XEPY] Demo concluded.");
}

/// Print the banner and the educational disclaimer.
fn print_banner() {
    println!("==============================================");
    println!("   XEPAY (ticker: XEPY) — v0.2 Demo Chain");
    println!("   Target block time: ~15 seconds");
    println!("==============================================\n");
}
