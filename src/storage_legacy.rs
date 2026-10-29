// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Legacy storage functions (blocks, mempool, state snapshot).

use std::collections::BTreeMap;
use std::io::{Error, Read, Write};

use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::datadir;
use crate::storage::Account;
use crate::validation;

pub const BLOCKS_PER_FILE: usize = 1000;

pub fn save_chain(chain: &Blockchain) -> std::io::Result<()> {
    datadir::ensure_dirs()?;
    save_blocks(chain)?;
    save_mempool(&chain.mempool)?;
    save_state_snapshot(chain)?;
    Ok(())
}

pub fn load_chain() -> std::io::Result<Blockchain> {
    let db_path = datadir::state_db_path();

    let blocks = load_blocks()?;
    if blocks.is_empty() {
        return Ok(Blockchain::new());
    }

    if blocks[0].index != 0 {
        return Err(Error::other(
            "consensus failure: the first block on disk is not the genesis block.",
        ));
    }
    if blocks[0].previous_hash != crate::consensus::rules::ZERO_HASH {
        return Err(Error::other(
            "consensus failure: the genesis block previous_hash is not zero.",
        ));
    }

    let mempool = load_mempool().unwrap_or_default();

    // Check the reindex mode.
    let reindex_requested = std::env::var("XEPAY_REINDEX").is_ok();

    if reindex_requested {
        eprintln!("[node] Manual reindex requested. Deleting state database...");
        let _ = std::fs::remove_dir_all(&db_path);

        // Delete root_hash.txt so that the trie starts from empty.
        let root_path = datadir::chainstate_dir().join("root_hash.txt");
        let _ = std::fs::remove_file(root_path);
    }

    let state = crate::storage::StateDB::open(&db_path)
        .map_err(|e| Error::other(format!("failed to open state db: {}", e)))?;

    let mut chain = Blockchain {
        chain: blocks,
        nbits: crate::consensus::spec::current_network().genesis_nbits(),
        mempool,
        balances: std::collections::HashMap::new(),
        nonces: std::collections::HashMap::new(),
        mempool_timestamps: std::collections::HashMap::new(),
        addresses: Vec::new(),
        state,
    };

    if reindex_requested {
        eprintln!("[node] Rebuilding state from block history...");
        if let Err(e) = rebuild_state_from_blocks(&mut chain) {
            eprintln!("[node] FATAL: Reindex failed: {}", e);
            std::process::exit(1);
        }

        // Flush after reindex so that the root hash is persisted.
        if let Err(e) = chain.state.flush() {
            eprintln!("[node] FATAL: Failed to flush state after reindex: {}", e);
            std::process::exit(1);
        }

        eprintln!("[node] Reindex successful! State fully rebuilt from blocks.");
    } else {
        // Strict verification: the node refuses to start with an
        // inconsistent state. The caller (AppInit) decides whether to
        // exit or prompt the user to run with -reindex.
        if let Err(e) = verify_state_root(&mut chain) {
            return Err(Error::other(format!(
                "state root verification failed: {}. \
                 The state database is corrupted or out of sync. \
                 Run with -reindex to rebuild the state from the block history.",
                e
            )));
        }
    }

    Ok(chain)
}

fn rebuild_state_from_blocks(chain: &mut Blockchain) -> std::io::Result<()> {
    let blocks = chain.chain.clone();

    if let Some(genesis_block) = blocks.first() {
        chain.state.update_account(
            "GENESIS",
            &Account {
                nonce: 0,
                balance: genesis_block.reward,
                immature_rewards: Vec::new(),
                code: Vec::new(),
                storage: BTreeMap::new(),
            },
        ).map_err(Error::other)?;
    }

    for block in blocks.iter().skip(1) {
        for tx in &block.transactions {
            if !validation::is_special_tx(tx) {
                chain.state.apply_transaction(&tx.from, &tx.to, tx.amount, tx.fee, tx.nonce)
                    .map_err(|e| Error::other(format!("Failed to apply tx {}: {}", tx.txid, e)))?;
            }
        }
        chain.state.credit_reward(&block.mined_for, block.reward, block.index)
            .map_err(|e| Error::other(format!("Failed to credit reward block {}: {}", block.index, e)))?;
    }

    Ok(())
}

/// Verify that the state database matches the state root committed in the
/// tip block. Returns an error if the roots do not match.
///
/// This function does NOT call `std::process::exit`. It returns an error
/// to the caller so that the caller can decide how to handle the
/// inconsistency (e.g. print a message and exit, or retry with reindex).
fn verify_state_root(chain: &mut Blockchain) -> std::io::Result<()> {
    if chain.chain.is_empty() {
        return Ok(());
    }

    let tip = chain.chain.last().unwrap();

    if tip.state_root.is_empty() {
        return Ok(());
    }

    let current_root = chain
        .state
        .root_hash()
        .map_err(|e| Error::other(format!("failed to compute state root: {}", e)))?;

    let current_root_hex = hex::encode(current_root);

    if current_root_hex != tip.state_root {
        return Err(Error::other(format!(
            "state root mismatch at height {}.\nExpected: {}\nGot:      {}",
            tip.index, tip.state_root, current_root_hex
        )));
    }

    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct AccountSnapshot {
    nonce: u64,
    balance: u64,
    immature_rewards: Vec<(u64, u64)>,
    code: Vec<u8>,
    storage: Vec<(String, String)>,
}

pub fn save_state_snapshot(chain: &Blockchain) -> std::io::Result<()> {
    if chain.addresses.len() <= 1 {
        return Ok(());
    }

    let mut accounts: std::collections::HashMap<String, AccountSnapshot> = std::collections::HashMap::new();
    for addr in &chain.addresses {
        if let Ok(Some(acc)) = chain.state.get_account(addr) {
            let storage_hex: Vec<(String, String)> = acc
                .storage
                .iter()
                .map(|(k, v)| (hex::encode(k), hex::encode(v)))
                .collect();
            accounts.insert(
                addr.clone(),
                AccountSnapshot {
                    nonce: acc.nonce,
                    balance: acc.balance,
                    immature_rewards: acc.immature_rewards,
                    code: acc.code,
                    storage: storage_hex,
                },
            );
        }
    }
    let json = serde_json::to_string_pretty(&accounts).map_err(Error::other)?;
    std::fs::write(datadir::chainstate_dir().join("state_snapshot.json"), json)
}

pub fn save_blocks(chain: &Blockchain) -> std::io::Result<()> {
    let total = chain.chain.len();
    let mut file_index = 0u32;
    let mut written = 0usize;

    while written < total {
        let end = (written + BLOCKS_PER_FILE).min(total);
        let slice = &chain.chain[written..end];
        let path = datadir::block_file(file_index);

        let encoded = bincode::serialize(slice)
            .map_err(|e| Error::other(format!("bincode serialize error: {}", e)))?;

        let mut file = std::fs::File::create(path)?;
        file.write_all(&encoded)?;
        file.flush()?;

        written = end;
        file_index += 1;
    }
    Ok(())
}

pub fn load_blocks() -> std::io::Result<Vec<Block>> {
    let mut all: Vec<Block> = Vec::new();
    let mut i = 0u32;

    loop {
        let path = datadir::block_file(i);
        if !path.exists() {
            break;
        }

        let mut file = std::fs::File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let mut chunk: Vec<Block> = bincode::deserialize(&buffer)
            .map_err(|e| Error::other(format!("bincode deserialize error: {}", e)))?;

        all.append(&mut chunk);
        i += 1;
    }
    Ok(all)
}

pub fn save_mempool(mempool: &[crate::transaction::Transaction]) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(mempool).map_err(Error::other)?;
    std::fs::write(datadir::mempool_path(), json)
}

pub fn load_mempool() -> std::io::Result<Vec<crate::transaction::Transaction>> {
    let data = std::fs::read_to_string(datadir::mempool_path())?;
    let v: Vec<crate::transaction::Transaction> =
        serde_json::from_str(&data).map_err(Error::other)?;
    Ok(v)
}
