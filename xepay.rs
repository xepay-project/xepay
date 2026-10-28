// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// xepay.rs
// Entry point for the XEPAY (ticker: XEPY) blockchain, v0.2.
//
// Two modes:
//   xepay run            — run the end-to-end demo (default)
//   xepay wallet <...>   — CLI wallet (see cli.rs)

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

    if args.first().map(String::as_str) == Some("wallet") {
        cli::run(&args);
        return;
    }

    run_demo();
}

fn run_demo() {
    println!("==============================================");
    println!("   XEPAY (ticker: XEPY) — v0.2 Demo Chain");
    println!("   Target block time: ~15 seconds");
    println!("==============================================\n");

    let difficulty = 3;
    let mut chain = Blockchain::new(difficulty);

    // Create a few wallets for the demo.
    let alice = Wallet::generate();
    let bob = Wallet::generate();
    let carol = Wallet::generate();
    println!("[XEPY] alice: {}", alice.address());
    println!("[XEPY] bob:   {}", bob.address());
    println!("[XEPY] carol: {}\n", carol.address());

    // Fund alice with a coinbase by mining the first block for her.
    chain.add_block(vec![], &alice.address()).unwrap();
    println!(
        "[XEPY] Alice funded; balance = {}\n",
        chain.balances.get(&alice.address()).copied().unwrap_or(0)
    );

    // Build a mempool with two fee-paying transactions from alice.
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

    // Mine one block selecting up to 10 txs, paying fees to bob.
    // The mempool emptiness check is a real guard: an empty mempool still
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

    // Persist the chain to disk.
    {
        let store = storage::Storage::open("xepay.db").unwrap();
        store.save_chain(&chain).unwrap();
        println!("[XEPY] Chain persisted to xepay.db");

        let restored = store.restore_chain(difficulty).unwrap().unwrap();
        println!("[XEPY] Restored chain height = {}\n", restored.height());
    }

    // Report.
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
