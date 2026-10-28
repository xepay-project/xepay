// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// cli.rs
// Command-line interface for XEPAY (ticker: XEPY).
//
// All output is structured key=value through the logging module. Every
// command opens ./xepay.db, performs its action, and persists the result.

use crate::blockchain::Blockchain;
use crate::logging::{self, fmt_amount, short};
use crate::mempool::Mempool;
use crate::storage::Storage;
use crate::transaction::Transaction;
use crate::wallet::Wallet;

const DB_PATH: &str = "xepay.db";
const DEFAULT_DIFFICULTY: u32 = 3;

pub fn run(args: &[String]) {
    if args.is_empty() {
        print_help();
        return;
    }
    let result = match (args[0].as_str(), args.get(1).map(String::as_str)) {
        ("help", _) | ("--help", _) | ("-h", _) => { print_help(); Ok(()) }
        ("wallet", Some("new"))   => cmd_wallet_new(),
        ("wallet", Some("from"))  => cmd_wallet_from(args),
        ("wallet", Some("info"))  => cmd_wallet_info(args),
        ("chain", Some("info"))   => cmd_chain_info(),
        ("chain", Some("blocks")) => cmd_chain_blocks(args),
        ("chain", Some("block"))  => cmd_chain_block(args),
        ("balance", _)            => cmd_balance(args),
        ("mempool", Some("list")) => cmd_mempool_list(),
        ("tx", Some("send"))      => cmd_tx_send(args),
        ("tx", Some("history"))   => cmd_tx_history(args),
        ("mine", _)               => cmd_mine(args),
        _ => { print_help(); Ok(()) }
    };
    if let Err(e) = result {
        logging::error("cli_error", &[("message", e)]);
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------
// Wallet
// ---------------------------------------------------------------

fn cmd_wallet_new() -> Result<(), String> {
    let w = Wallet::generate();
    println!("address:    {}", w.address());
    println!("secret_hex: {}", w.export_secret());
    println!();
    println!("Store the secret_hex securely. Anyone with it controls the funds.");
    Ok(())
}

fn cmd_wallet_from(args: &[String]) -> Result<(), String> {
    let secret = args.get(2).ok_or("usage: xepay wallet from <secret_hex>")?;
    let w = Wallet::from_secret_hex(secret)?;
    println!("address: {}", w.address());
    Ok(())
}

fn cmd_wallet_info(args: &[String]) -> Result<(), String> {
    let secret = args.get(2).ok_or("usage: xepay wallet info <secret_hex>")?;
    let w = Wallet::from_secret_hex(secret)?;
    let chain = load_chain()?;
    let addr = w.address();
    let bal = chain.balances.get(&addr).copied().unwrap_or(0);
    let nonce = chain.nonces.get(&addr).copied().unwrap_or(0);
    println!("address:    {}", addr);
    println!("balance:    {}", fmt_amount(bal));
    println!("next_nonce: {}", nonce);
    Ok(())
}

// ---------------------------------------------------------------
// Chain
// ---------------------------------------------------------------

fn cmd_chain_info() -> Result<(), String> {
    let chain = load_chain()?;
    println!("network:         XEPAY ({})", crate::blockchain::NETWORK);
    println!("chain_id:        {}", crate::blockchain::CHAIN_ID);
    println!("protocol:        v{}", crate::blockchain::PROTOCOL_VERSION);
    println!("consensus:       PoW-SHA3-256");
    println!("height:          {}", chain.height());
    println!("difficulty:      {}", chain.difficulty);
    println!("avg_block_time:  {:.2}s", chain.average_block_time_secs());
    println!("target_block:    {}s", crate::blockchain::TARGET_BLOCK_TIME_SECS);
    println!("block_reward:    {}", fmt_amount(chain.block_reward()));
    println!("tip:             {}", chain.tip().hash);
    Ok(())
}

fn cmd_chain_blocks(args: &[String]) -> Result<(), String> {
    let n: usize = args
        .get(2)
        .map(|s| s.parse().map_err(|_| "invalid count".to_string()))
        .transpose()?
        .unwrap_or(10);
    let chain = load_chain()?;
    let start = chain.chain.len().saturating_sub(n);
    println!("count={}", chain.chain.len() - start);
    for (i, b) in chain.chain.iter().enumerate().skip(start) {
        println!(
            "  height={} hash={} diff={} nonce={} txs={}",
            i,
            short(&b.hash),
            b.header.difficulty,
            b.header.nonce,
            b.transactions.len(),
        );
    }
    Ok(())
}

fn cmd_chain_block(args: &[String]) -> Result<(), String> {
    let h: usize = args
        .get(2)
        .ok_or("usage: xepay chain block <height>")?
        .parse()
        .map_err(|_| "invalid height".to_string())?;
    let chain = load_chain()?;
    let b = chain
        .chain
        .get(h)
        .ok_or_else(|| format!("no block at height {}", h))?;

    println!("height:          {}", h);
    println!("version:         {}", b.header.version);
    println!("timestamp:       {}", b.header.timestamp);
    println!("prev_block_hash: {}", b.header.prev_block_hash);
    println!("merkle_root:     {}", b.header.merkle_root);
    println!("difficulty:      {}", b.header.difficulty);
    println!("nonce:           {}", b.header.nonce);
    println!("hash:            {}", b.hash);
    println!("transactions:    {}", b.transactions.len());
    for (i, tx) in b.transactions.iter().enumerate() {
        let from = if tx.from.is_empty() {
            "COINBASE".to_string()
        } else {
            short(&tx.from)
        };
        println!(
            "  [{}] from={} to={} amount={} fee={} nonce={}",
            i,
            from,
            short(&tx.to),
            fmt_amount(tx.amount),
            tx.fee,
            tx.nonce,
        );
    }
    Ok(())
}

fn cmd_balance(args: &[String]) -> Result<(), String> {
    let addr = args.get(1).ok_or("usage: xepay balance <address>")?;
    let chain = load_chain()?;
    let bal = chain.balances.get(addr).copied().unwrap_or(0);
    println!("address: {}", addr);
    println!("balance: {}", fmt_amount(bal));
    Ok(())
}

// ---------------------------------------------------------------
// Mempool
// ---------------------------------------------------------------

fn cmd_mempool_list() -> Result<(), String> {
    let store = Storage::open(DB_PATH)?;
    let pool = store.load_mempool()?;
    println!("mempool_size: {}", pool.len());
    for (i, tx) in pool.iter().enumerate() {
        println!(
            "  [{}] from={} to={} amount={} fee={} nonce={}",
            i,
            short(&tx.from),
            short(&tx.to),
            fmt_amount(tx.amount),
            tx.fee,
            tx.nonce,
        );
    }
    Ok(())
}

// ---------------------------------------------------------------
// Transactions
// ---------------------------------------------------------------

fn cmd_tx_send(args: &[String]) -> Result<(), String> {
    if args.len() < 6 {
        return Err("usage: xepay tx send <secret_hex> <to_address> <amount_xepysat> <fee_xepysat>".into());
    }
    let secret = &args[2];
    let to = &args[3];
    let amount: u64 = args[4].parse().map_err(|_| "invalid amount".to_string())?;
    let fee: u64 = args[5].parse().map_err(|_| "invalid fee".to_string())?;

    let w = Wallet::from_secret_hex(secret)?;
    let chain = load_chain()?;
    let from = w.address();

    let nonce = chain.nonces.get(&from).copied().unwrap_or(0);
    if let Some(existing) = chain.balances.get(&from) {
        if *existing < amount + fee {
            return Err(format!(
                "insufficient_balance have={} need={}",
                existing,
                amount + fee
            ));
        }
    } else {
        return Err(format!("address_has_no_balance address={}", from));
    }

    let tx = Transaction::new_signed(&w.signing_key(), to, amount, fee, nonce);
    if !tx.verify() {
        return Err("signature_verification_failed".into());
    }

    // Persist into the mempool.
    let store = Storage::open(DB_PATH)?;
    let mut pool = store.load_mempool()?;
    if !pool.add(tx.clone()) {
        return Err("mempool_rejected_transaction (duplicate or full)".into());
    }
    store.save_mempool(&pool)?;

    logging::info(
        "tx_submitted",
        &[
            ("from", from),
            ("to", to.to_string()),
            ("amount", fmt_amount(amount)),
            ("fee", fmt_amount(fee)),
            ("nonce", nonce.to_string()),
            ("mempool_size", pool.len().to_string()),
        ],
    );
    Ok(())
}

fn cmd_tx_history(args: &[String]) -> Result<(), String> {
    if args.len() < 3 {
        return Err("usage: xepay tx history <address> [n]".into());
    }
    let addr = &args[2];
    let n: usize = args
        .get(3)
        .map(|s| s.parse().map_err(|_| "invalid count".to_string()))
        .transpose()?
        .unwrap_or(10);

    let chain = load_chain()?;
    let mut found = 0;
    for (h, b) in chain.chain.iter().enumerate().rev() {
        for tx in &b.transactions {
            if tx.from == *addr || tx.to == *addr {
                let dir = if tx.from == *addr { "OUT" } else { "IN " };
                let counterpart = if tx.from == *addr { &tx.to } else { &tx.from };
                println!(
                    "height={} dir={} amount={} fee={} nonce={} counterpart={}",
                    h,
                    dir,
                    fmt_amount(tx.amount),
                    tx.fee,
                    tx.nonce,
                    short(counterpart),
                );
                found += 1;
                if found >= n {
                    return Ok(());
                }
            }
        }
    }
    if found == 0 {
        println!("no_transactions address={}", addr);
    }
    Ok(())
}

// ---------------------------------------------------------------
// Mining
// ---------------------------------------------------------------

fn cmd_mine(args: &[String]) -> Result<(), String> {
    let secret = args
        .get(1)
        .ok_or("usage: xepay mine <secret_hex> [max_tx]")?;
    let max_tx: usize = args
        .get(2)
        .map(|s| s.parse().map_err(|_| "invalid max_tx".to_string()))
        .transpose()?
        .unwrap_or(100);

    let w = Wallet::from_secret_hex(secret)?;
    let miner = w.address();

    let mut chain = load_chain()?;
    let store = Storage::open(DB_PATH)?;
    let mut pool = store.load_mempool()?;

    let height_before = chain.height();
    let selected = pool.drain_highest_fee(max_tx);
    chain.add_block(selected.clone(), &miner)?;
    let new_block = chain.tip().clone();

    // Persist the new block, the updated chain, and the reduced mempool.
    store.put_block(chain.chain.len() - 1, &new_block)?;
    store.save_chain(&chain)?;
    store.save_mempool(&pool)?;

    logging::info(
        "block_mined",
        &[
            ("height", height_before.to_string()),
            ("hash", short(&new_block.hash)),
            ("nonce", new_block.header.nonce.to_string()),
            ("txs", new_block.transactions.len().to_string()),
            ("fees", fmt_amount(new_block.total_fees())),
            ("reward", fmt_amount(new_block.transactions[0].amount)),
            ("miner", short(&miner)),
        ],
    );
    Ok(())
}

// ---------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------

fn load_chain() -> Result<Blockchain, String> {
    let store = Storage::open(DB_PATH)?;
    store.check_chain_id()?;
    match store.restore_chain(DEFAULT_DIFFICULTY)? {
        Some(c) => Ok(c),
        None => {
            let chain = Blockchain::new(DEFAULT_DIFFICULTY);
            store.save_chain(&chain)?;
            store.save_meta(&crate::storage::ChainMeta {
                chain_id: crate::blockchain::CHAIN_ID.to_string(),
                network: crate::blockchain::NETWORK.to_string(),
                protocol_version: crate::blockchain::PROTOCOL_VERSION.to_string(),
                difficulty: DEFAULT_DIFFICULTY,
            })?;
            Ok(chain)
        }
    }
}

// ---------------------------------------------------------------
// Help
// ---------------------------------------------------------------

fn print_help() {
    println!("XEPAY (ticker: XEPY) — CLI");
    println!();
    println!("Usage: xepay <command> [args]");
    println!();
    println!("WALLET");
    println!("  wallet new");
    println!("      Generate an ed25519 keypair; print address and secret_hex.");
    println!("  wallet from <secret_hex>");
    println!("      Recover the address from an existing secret_hex.");
    println!("  wallet info <secret_hex>");
    println!("      Print address, balance, and next nonce for the wallet.");
    println!();
    println!("CHAIN");
    println!("  chain info");
    println!("      Print network, chain_id, height, difficulty, and tip hash.");
    println!("  chain blocks [n]");
    println!("      List the last N blocks (default: 10).");
    println!("  chain block <height>");
    println!("      Print full details of one block.");
    println!("  balance <address>");
    println!("      Print the balance for an address, in XEPY and XEPYsat.");
    println!();
    println!("MEMPOOL");
    println!("  mempool list");
    println!("      Print pending transactions with their fees and nonces.");
    println!();
    println!("TRANSACTIONS");
    println!("  tx send <secret_hex> <to_address> <amount_xepysat> <fee_xepysat>");
    println!("      Sign a transaction and enqueue it in the mempool.");
    println!("  tx history <address> [n]");
    println!("      Print the last N on-chain transactions for an address.");
    println!();
    println!("MINING");
    println!("  mine <secret_hex> [max_tx]");
    println!("      Mine one block from the mempool, credit the coinbase to");
    println!("      the secret_hex's address, and persist the result.");
    println!();
    println!("UNITS");
    println!("  1 XEPY = 100000000 XEPYsat");
    println!("  All <amount> and <fee> arguments are in XEPYsat.");
    println!();
    println!("LOGGING");
    println!("  Set XEPAY_LOG=debug|info|warn|error|quiet to control verbosity.");
}
