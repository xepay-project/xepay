// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// cli.rs
// Command-line interface for XEPAY (ticker: XEPY).
//
// Every command opens the on-disk chain (./xepay.db), performs its action,
// and persists the result. Errors are reported without panicking so the CLI
// can be scripted safely.

use crate::blockchain::Blockchain;
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
        ("help", _) | ("--help", _) | ("-h", _) => {
            print_help();
            Ok(())
        }
        ("wallet", Some("new")) => cmd_wallet_new(),
        ("wallet", Some("from")) => cmd_wallet_from(args),
        ("wallet", Some("info")) => cmd_wallet_info(args),
        ("chain", Some("info")) => cmd_chain_info(),
        ("chain", Some("blocks")) => cmd_chain_blocks(args),
        ("chain", Some("block")) => cmd_chain_block(args),
        ("balance", _) => cmd_balance(args),
        ("mempool", Some("list")) => cmd_mempool_list(),
        ("tx", Some("send")) => cmd_tx_send(args),
        ("tx", Some("history")) => cmd_tx_history(args),
        ("mine", _) => cmd_mine(args),
        _ => {
            print_help();
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------
// Wallet commands
// ---------------------------------------------------------------

fn cmd_wallet_new() -> Result<(), String> {
    let w = Wallet::generate();
    println!("Address:    {}", w.address());
    println!("Secret hex: {}", w.export_secret());
    println!();
    println!("Save the secret hex securely. Anyone with it controls the funds.");
    Ok(())
}

fn cmd_wallet_from(args: &[String]) -> Result<(), String> {
    let secret = args.get(2).ok_or("usage: xepay wallet from <secret_hex>")?;
    let w = Wallet::from_secret_hex(secret)?;
    println!("Address: {}", w.address());
    Ok(())
}

fn cmd_wallet_info(args: &[String]) -> Result<(), String> {
    let secret = args.get(2).ok_or("usage: xepay wallet info <secret_hex>")?;
    let w = Wallet::from_secret_hex(secret)?;
    let chain = load_chain()?;
    let addr = w.address();
    let bal = chain.balances.get(&addr).copied().unwrap_or(0);
    let nonce = chain.nonces.get(&addr).copied().unwrap_or(0);
    println!("Address:    {}", addr);
    println!("Balance:    {} XEPYsat", bal);
    println!("Next nonce: {}", nonce);
    Ok(())
}

// ---------------------------------------------------------------
// Chain commands
// ---------------------------------------------------------------

fn cmd_chain_info() -> Result<(), String> {
    let chain = load_chain()?;
    println!("Network:        XEPAY (XEPY)");
    println!("Height:         {}", chain.height());
    println!("Difficulty:     {}", chain.difficulty);
    println!("Avg block time: {:.2}s", chain.average_block_time_secs());
    println!("Tip hash:       {}", chain.tip().hash);
    println!("Block reward:   {} XEPYsat", chain.block_reward());
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
    println!("Last {} block(s):", chain.chain.len() - start);
    for (i, b) in chain.chain.iter().enumerate().skip(start) {
        println!(
            "  #{:>4} | hash={} | diff={} | nonce={:>10} | txs={:>3}",
            i,
            &b.hash[..16],
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

    println!("Block #{}", h);
    println!("  version:         {}", b.header.version);
    println!("  timestamp:       {}", b.header.timestamp);
    println!("  prev_block_hash: {}", b.header.prev_block_hash);
    println!("  merkle_root:     {}", b.header.merkle_root);
    println!("  difficulty:      {}", b.header.difficulty);
    println!("  nonce:           {}", b.header.nonce);
    println!("  hash:            {}", b.hash);
    println!("  transactions:    {}", b.transactions.len());
    for (i, tx) in b.transactions.iter().enumerate() {
        let from = if tx.from.is_empty() { "COINBASE".to_string() } else { tx.from.clone() };
        println!(
            "    [{}] {} -> {} | amount={} fee={} nonce={}",
            i,
            &from[..from.len().min(16)],
            &tx.to[..tx.to.len().min(16)],
            tx.amount,
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
    println!("Address: {}", addr);
    println!("Balance: {} XEPYsat", bal);
    Ok(())
}

// ---------------------------------------------------------------
// Mempool commands
// ---------------------------------------------------------------

fn cmd_mempool_list() -> Result<(), String> {
    let _chain = load_chain()?;
    // The mempool is not persisted in v0.2, so we report it as empty.
    // A future version will persist mempool alongside the chain.
    let pool = Mempool::new();
    println!("Mempool size: {}", pool.len());
    if pool.is_empty() {
        println!("(mempool is not persisted in v0.2; new txs go here per-session)");
    }
    Ok(())
}

// ---------------------------------------------------------------
// Transaction commands
// ---------------------------------------------------------------

fn cmd_tx_send(args: &[String]) -> Result<(), String> {
    if args.len() < 6 {
        return Err("usage: xepay tx send <secret_hex> <to_address> <amount> <fee>".into());
    }
    let secret = &args[2];
    let to = &args[3];
    let amount: u64 = args[4].parse().map_err(|_| "invalid amount".to_string())?;
    let fee: u64 = args[5].parse().map_err(|_| "invalid fee".to_string())?;

    let w = Wallet::from_secret_hex(secret)?;
    let chain = load_chain()?;
    let from = w.address();

    // Determine the next nonce for this sender.
    let mut nonce = chain.nonces.get(&from).copied().unwrap_or(0);
    if let Some(existing) = chain.balances.get(&from) {
        if *existing < amount + fee {
            return Err(format!(
                "insufficient balance: have {}, need {}",
                existing,
                amount + fee
            ));
        }
    } else {
        return Err(format!("address {} has no balance", from));
    }
    nonce += 0; // placeholder, real increment happens when included in a block

    let tx = Transaction::new_signed(&w.signing_key(), to, amount, fee, nonce);
    if !tx.verify() {
        return Err("signature verification failed".into());
    }

    // The mempool is not persisted in v0.2. Print the signed transaction so
    // it can be included in a future block by the mining command in the
    // same process invocation.
    println!("Signed transaction:");
    println!("{}", serde_json_pretty(&tx)?);
    println!();
    println!("To include it in a block, use:");
    println!("  xepay mine <secret_hex>");
    println!();
    println!("Note: mempool persistence is scheduled for v0.2.1.");
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
                    "h={:>4} {} amount={:>12} fee={:>6} nonce={:>3} counterpart={}",
                    h,
                    dir,
                    tx.amount,
                    tx.fee,
                    tx.nonce,
                    &counterpart[..counterpart.len().min(16)],
                );
                found += 1;
                if found >= n {
                    return Ok(());
                }
            }
        }
    }
    if found == 0 {
        println!("No transactions found for {}", addr);
    }
    Ok(())
}

// ---------------------------------------------------------------
// Mining command
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

    // Since the mempool is not persisted, mining here simply mines an empty
    // block (coinbase only). A future version will load pending txs from a
    // persisted mempool.
    let txs: Vec<Transaction> = Vec::new();

    let height_before = chain.height();
    chain.add_block(txs, &miner)?;
    let new_block = chain.tip().clone();

    // Persist.
    let store = Storage::open(DB_PATH)?;
    store.put_block(chain.chain.len() - 1, &new_block)?;

    println!("Block mined at height {}", height_before);
    println!("  hash:   {}", new_block.hash);
    println!("  nonce:  {}", new_block.header.nonce);
    println!("  txs:    {}", new_block.transactions.len());
    println!("  fees:   {}", new_block.total_fees());
    println!("  reward: {} XEPYsat to {}", new_block.transactions[0].amount, miner);
    Ok(())
}

// ---------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------

/// Load the chain from disk, or create a fresh one if no chain exists.
fn load_chain() -> Result<Blockchain, String> {
    let store = Storage::open(DB_PATH)?;
    match store.restore_chain(DEFAULT_DIFFICULTY)? {
        Some(c) => Ok(c),
        None => {
            let chain = Blockchain::new(DEFAULT_DIFFICULTY);
            store.save_chain(&chain)?;
            Ok(chain)
        }
    }
}

/// Minimal pretty JSON output for a Transaction (no external crate).
fn serde_json_pretty(tx: &Transaction) -> Result<String, String> {
    Ok(format!(
        "{{\n  \"from\": \"{}\",\n  \"to\": \"{}\",\n  \"amount\": {},\n  \"fee\": {},\n  \"nonce\": {},\n  \"signature\": \"{}\"\n}}",
        tx.from, tx.to, tx.amount, tx.fee, tx.nonce, tx.signature
    ))
}

// ---------------------------------------------------------------
// Help text
// ---------------------------------------------------------------

fn print_help() {
    println!("XEPAY CLI — ticker XEPY");
    println!();
    println!("Usage:");
    println!("  xepay <command> [args]");
    println!();
    println!("Wallet:");
    println!("  wallet new                       Generate a new wallet");
    println!("  wallet from <secret_hex>         Import from a secret hex");
    println!("  wallet info <secret_hex>         Show address, balance, nonce");
    println!();
    println!("Chain:");
    println!("  chain info                       Show height, difficulty, tip");
    println!("  chain blocks [n]                 Show last N blocks (default 10)");
    println!("  chain block <height>             Show one block's details");
    println!("  balance <address>                Show balance for an address");
    println!();
    println!("Mempool:");
    println!("  mempool list                     Show mempool contents");
    println!();
    println!("Transactions:");
    println!("  tx send <secret> <to> <amount> <fee>   Sign a transaction");
    println!("  tx history <address> [n]               Show recent history");
    println!();
    println!("Mining:");
    println!("  mine <secret_hex> [max_tx]       Mine a block, reward to address");
    println!();
    println!("Examples:");
    println!("  xepay wallet new");
    println!("  xepay chain info");
    println!("  xepay chain blocks 5");
    println!("  xepay mine <secret_hex>");
}
