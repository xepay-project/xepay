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
const DEFAULT_FEE_XEPYSAT: u64 = 100;
const XEPYSAT_PER_XEPY: u64 = 100_000_000;

pub fn run(args: &[String]) {
    if args.is_empty() {
        print_help();
        return;
    }
    let result = match (args[0].as_str(), args.get(1).map(String::as_str)) {
        ("help", _) | ("--help", _) | ("-h", _) => { print_help(); Ok(()) }
        ("wallet", Some("new"))    => cmd_wallet_new(),
        ("wallet", Some("from"))   => cmd_wallet_from(args),
        ("wallet", Some("info"))   => cmd_wallet_info(args),
        ("wallet", Some("use"))    => cmd_wallet_use(args),
        ("wallet", Some("active")) => cmd_wallet_active(),
        ("wallet", Some("clear"))  => cmd_wallet_clear(),
        ("sendtoaddress", _)       => cmd_sendtoaddress(args),
        ("chain", Some("info"))    => cmd_chain_info(),
        ("chain", Some("blocks"))  => cmd_chain_blocks(args),
        ("chain", Some("block"))   => cmd_chain_block(args),
        ("balance", _)             => cmd_balance(args),
        ("mempool", Some("list"))  => cmd_mempool_list(),
        ("tx", Some("send"))       => cmd_tx_send(args),
        ("tx", Some("history"))    => cmd_tx_history(args),
        ("mine", _)                => cmd_mine(args),
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

fn cmd_wallet_use(args: &[String]) -> Result<(), String> {
    let secret = args.get(2).ok_or("usage: xepay wallet use <secret_hex>")?;
    let w = Wallet::from_secret_hex(secret)?;
    let store = Storage::open(DB_PATH)?;
    store.save_active_wallet(secret)?;
    logging::info("wallet_active_set", &[("address", w.address())]);
    Ok(())
}

fn cmd_wallet_active() -> Result<(), String> {
    let store = Storage::open(DB_PATH)?;
    match store.load_active_wallet()? {
        Some(secret) => {
            let w = Wallet::from_secret_hex(&secret)?;
            let chain = load_chain()?;
            let addr = w.address();
            let bal = chain.balances.get(&addr).copied().unwrap_or(0);
            println!("address: {}", addr);
            println!("balance: {}", fmt_amount(bal));
        }
        None => {
            println!("no_active_wallet");
            println!("(run `xepay wallet use <secret_hex>` to set one)");
        }
    }
    Ok(())
}

fn cmd_wallet_clear() -> Result<(), String> {
    let store = Storage::open(DB_PATH)?;
    store.clear_active_wallet()?;
    logging::info("wallet_active_cleared", &[]);
    Ok(())
}

// ---------------------------------------------------------------
// sendtoaddress — one-shot transfer with auto-mine
// ---------------------------------------------------------------

fn cmd_sendtoaddress(args: &[String]) -> Result<(), String> {
    // Parse: xepay sendtoaddress [--from <secret>] [--unit xepy|xepysat]
    //        [--fee <n>] [--no-mine] <amount> <to_address>
    let mut from_secret: Option<String> = None;
    let mut unit = "xepy".to_string();
    let mut fee: u64 = DEFAULT_FEE_XEPYSAT;
    let mut auto_mine = true;
    let mut positional: Vec<String> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--from" => {
                from_secret = Some(
                    args.get(i + 1)
                        .ok_or("--from requires a secret_hex argument")?
                        .clone(),
                );
                i += 2;
            }
            "--unit" => {
                unit = args
                    .get(i + 1)
                    .ok_or("--unit requires 'xepy' or 'xepysat'")?
                    .clone();
                i += 2;
            }
            "--fee" => {
                fee = args
                    .get(i + 1)
                    .ok_or("--fee requires a number (XEPYsat)")?
                    .parse()
                    .map_err(|_| "invalid --fee".to_string())?;
                i += 2;
            }
            "--no-mine" => {
                auto_mine = false;
                i += 1;
            }
            other => {
                positional.push(other.to_string());
                i += 1;
            }
        }
    }

    if positional.len() < 2 {
        return Err(
            "usage: xepay sendtoaddress [--from <secret>] [--unit xepy|xepysat] \
             [--fee <xepysat>] [--no-mine] <amount> <to_address>"
                .into(),
        );
    }
    let amount_str = &positional[0];
    let to = &positional[1];

    // Resolve the sender wallet.
    let secret = match from_secret {
        Some(s) => s,
        None => {
            let store = Storage::open(DB_PATH)?;
            store
                .load_active_wallet()?
                .ok_or("no_wallet: pass --from <secret_hex> or run `xepay wallet use <secret_hex>`")?
        }
    };
    let sender = Wallet::from_secret_hex(&secret)?;
    let from_addr = sender.address();

    // Resolve amount into XEPYsat.
    let amount_xepysat: u64 = match unit.as_str() {
        "xepy" => {
            let xepy: f64 = amount_str
                .parse()
                .map_err(|_| "invalid amount (XEPY)".to_string())?;
            if xepy <= 0.0 {
                return Err("amount must be positive".into());
            }
            (xepy * XEPYSAT_PER_XEPY as f64).round() as u64
        }
        "xepysat" => amount_str
            .parse()
            .map_err(|_| "invalid amount (XEPYsat)".to_string())?,
        other => return Err(format!("unknown --unit '{}'", other)),
    };
    if amount_xepysat == 0 {
        return Err("amount must be greater than zero".into());
    }

    // Load chain and check balance.
    let mut chain = load_chain()?;
    let store = Storage::open(DB_PATH)?;
    let available = chain.balances.get(&from_addr).copied().unwrap_or(0);
    let needed = amount_xepysat.saturating_add(fee);
    if available < needed {
        return Err(format!(
            "insufficient_balance available={} needed={}",
            available, needed
        ));
    }

    // Build, sign, verify.
    let nonce = chain.nonces.get(&from_addr).copied().unwrap_or(0);
    let tx = Transaction::new_signed(&sender.signing_key(), to, amount_xepysat, fee, nonce);
    if !tx.verify() {
        return Err("signature_verification_failed".into());
    }

    // Enqueue in the mempool.
    let mut pool = store.load_mempool()?;
    if !pool.add(tx.clone()) {
        return Err("mempool_rejected_transaction (duplicate or full)".into());
    }
    store.save_mempool(&pool)?;

    logging::info(
        "tx_submitted",
        &[
            ("from", from_addr.clone()),
            ("to", to.to_string()),
            ("amount", fmt_amount(amount_xepysat)),
            ("fee", fmt_amount(fee)),
            ("nonce", nonce.to_string()),
            ("mempool_size", pool.len().to_string()),
        ],
    );

    if !auto_mine {
        logging::info("tx_in_mempool", &[
            ("note", "run `xepay mine <secret_hex>` to confirm".to_string()),
        ]);
        return Ok(());
    }

    // Auto-mine: select all mempool txs, mine a block, pay the miner.
    let height_before = chain.height();
    let selected = pool.drain_highest_fee(10_000);
    chain.add_block(selected, &from_addr)?;
    let new_block = chain.tip().clone();

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
            ("miner", short(&from_addr)),
        ],
    );

    let from_after = chain.balances.get(&from_addr).copied().unwrap_or(0);
    let to_after = chain.balances.get(to).copied().unwrap_or(0);
    logging::info(
        "transfer_confirmed",
        &[
            ("txid", short(&new_block.hash)),
            ("height", height_before.to_string()),
            ("confirmations", "1".to_string()),
        ],
    );
    println!("  from={} balance_after={}", short(&from_addr), fmt_amount(from_after));
    println!("  to={}   balance_after={}", short(to), fmt_amount(to_after));
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
    println!("  wallet use <secret_hex>");
    println!("      Set the active wallet in ./xepay.db (used by sendtoaddress).");
    println!("  wallet active");
    println!("      Print the active wallet's address and balance.");
    println!("  wallet clear");
    println!("      Forget the active wallet.");
    println!();
    println!("SEND");
    println!("  sendtoaddress [options] <amount> <to_address>");
    println!("      Sign, enqueue, and (by default) mine a block containing");
    println!("      the transfer. Default unit is XEPY; use --unit xepysat");
    println!("      for raw smallest units.");
    println!("      Options:");
    println!("        --from <secret_hex>   use this wallet instead of the active one");
    println!("        --unit xepy|xepysat   unit for <amount> (default: xepy)");
    println!("        --fee <xepysat>       miner fee (default: {})", DEFAULT_FEE_XEPYSAT);
    println!("        --no-mine             leave the tx in the mempool");
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
    println!("LOW-LEVEL TRANSACTIONS");
    println!("  tx send <secret_hex> <to_address> <amount_xepysat> <fee_xepysat>");
    println!("      Sign a transaction and enqueue it (no auto-mine).");
    println!("  tx history <address> [n]");
    println!("      Print the last N on-chain transactions for an address.");
    println!();
    println!("MINING");
    println!("  mine <secret_hex> [max_tx]");
    println!("      Mine one block from the mempool, credit the coinbase to");
    println!("      the secret_hex's address, and persist the result.");
    println!();
    println!("UNITS");
    println!("  1 XEPY = {} XEPYsat", XEPYSAT_PER_XEPY);
    println!("  `tx send` and `--fee` always use XEPYsat.");
    println!("  `sendtoaddress` defaults to XEPY; use --unit xepysat for raw.");
    println!();
    println!("LOGGING");
    println!("  Set XEPAY_LOG=debug|info|warn|error|quiet to control verbosity.");
}
