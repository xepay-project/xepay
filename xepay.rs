// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// xepay.rs
// Entry point for the XEPAY (ticker: XEPY) blockchain, v0.2.1.
//
// Two modes:
//   xepay                — run the end-to-end smoke test
//   xepay run            — run the end-to-end smoke test
//   xepay <command> ...  — dispatch a CLI command (see cli.rs)

#[path = "logging.rs"]
mod logging;
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
use logging::{fmt_amount, info, short};
use mempool::Mempool;
use storage::Storage;
use transaction::Transaction;
use wallet::Wallet;

fn main() {
    logging::init();
    let args: Vec<String> = std::env::args().skip(1).collect();

    let is_smoke = args.is_empty() || args.first().map(String::as_str) == Some("run");
    if !is_smoke {
        cli::run(&args);
        return;
    }

    run_smoke_test();
}

/// Run the end-to-end smoke test.
///
/// Sequence:
///   1. Print node banner with network, chain_id, protocol version.
///   2. Generate three wallets: miner, treasury, recipient.
///   3. Mine a coinbase block paying the miner, report credit.
///   4. Build a mempool with two signed transactions from the miner.
///   5. Mine a block selecting the highest-fee transactions, paying the miner.
///   6. Persist chain and mempool to ./xepay.db.
///   7. Restore the chain and verify integrity.
///   8. Print final balances and average block time.
fn run_smoke_test() {
    let started = std::time::Instant::now();
    print_banner();

    let difficulty = 3;
    let mut chain = Blockchain::new(difficulty);
    let store = Storage::open("xepay.db").expect("open db");

    // Persist metadata on first run so future invocations can verify chain_id.
    if store.load_meta().ok().flatten().is_none() {
        let _ = store.save_meta(&storage::ChainMeta {
            chain_id: blockchain::CHAIN_ID.to_string(),
            network: blockchain::NETWORK.to_string(),
            protocol_version: blockchain::PROTOCOL_VERSION.to_string(),
            difficulty,
        });
    }

    info("genesis_initialized", &[
        ("height", chain.height().to_string()),
        ("hash", short(&chain.tip().hash)),
    ]);

    // Create three wallets with functional roles.
    let miner = Wallet::generate();
    let treasury = Wallet::generate();
    let recipient = Wallet::generate();
    info("wallet_generated", &[("role", "miner".into()), ("address", miner.address())]);
    info("wallet_generated", &[("role", "treasury".into()), ("address", treasury.address())]);
    info("wallet_generated", &[("role", "recipient".into()), ("address", recipient.address())]);

    // Mine a coinbase block paying the miner.
    chain.add_block(vec![], &miner.address()).unwrap();
    store.put_block(chain.chain.len() - 1, chain.tip()).unwrap();
    let miner_bal = chain.balances.get(&miner.address()).copied().unwrap_or(0);
    info("coinbase_credited", &[
        ("address", short(&miner.address())),
        ("amount", fmt_amount(miner_bal)),
    ]);

    // Build a mempool with two signed transactions from the miner.
    let mut pool = Mempool::new();
    let tx1 = Transaction::new_signed(
        &miner.signing_key(), &treasury.address(), 10_0000_0000, 100, 0,
    );
    let tx2 = Transaction::new_signed(
        &miner.signing_key(), &recipient.address(), 5_0000_0000, 500, 1,
    );
    assert!(pool.add(tx1));
    assert!(pool.add(tx2));
    info("mempool_size", &[("count", pool.len().to_string())]);

    // Mine a block selecting the highest-fee transactions, paying the miner.
    let picked = pool.drain_highest_fee(10);
    chain.add_block(picked, &miner.address()).unwrap();
    let new_block = chain.tip().clone();
    store.put_block(chain.chain.len() - 1, &new_block).unwrap();
    store.save_chain(&chain).unwrap();
    store.save_mempool(&pool).unwrap();

    info("block_mined", &[
        ("height", (chain.height() - 1).to_string()),
        ("hash", short(&new_block.hash)),
        ("nonce", new_block.header.nonce.to_string()),
        ("txs", new_block.transactions.len().to_string()),
        ("fees", fmt_amount(new_block.total_fees())),
        ("reward", fmt_amount(new_block.transactions[0].amount)),
    ]);

    // Restore and verify.
    match store.restore_chain(difficulty) {
        Ok(Some(restored)) => {
            info("chain_restored", &[
                ("height", restored.height().to_string()),
                ("valid", restored.is_valid().to_string()),
            ]);
        }
        _ => {
            logging::error("chain_restore_failed", &[("path", "xepay.db".into())]);
        }
    }

    // Final report.
    info("chain_state", &[
        ("height", chain.height().to_string()),
        ("valid", chain.is_valid().to_string()),
        ("avg_block_time_s", format!("{:.2}", chain.average_block_time_secs())),
        ("target_s", blockchain::TARGET_BLOCK_TIME_SECS.to_string()),
    ]);

    println!();
    println!("balances:");
    let mut addrs: Vec<_> = chain.balances.iter().collect();
    addrs.sort_by_key(|(_, b)| std::cmp::Reverse(**b));
    for (addr, bal) in addrs {
        println!("  {}  {}", &addr[..16], fmt_amount(*bal));
    }

    println!();
    info("smoke_test_completed", &[
        ("duration_s", format!("{:.2}", started.elapsed().as_secs_f64())),
    ]);
}

/// Print the node banner with network identity and runtime parameters.
fn print_banner() {
    println!("==============================================================");
    println!("  XEPAY TESTNET NODE");
    println!("  version:     v{}", blockchain::PROTOCOL_VERSION);
    println!("  network:     {}", blockchain::NETWORK);
    println!("  chain_id:    {}", blockchain::CHAIN_ID);
    println!("  ticker:      XEPY");
    println!("  consensus:   PoW-SHA3-256");
    println!("  base_unit:   XEPYsat (1 XEPY = 100000000 XEPYsat)");
    println!("  block_time:  {}s (target)", blockchain::TARGET_BLOCK_TIME_SECS);
    println!("  difficulty:  3");
    println!("==============================================================");
    println!();
}
