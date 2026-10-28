// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! XEPAY standalone miner.
//!
//! This binary requests a block template from the node, mines the block,
//! and submits it back to the node through the RPC interface.

use xepay::block::Block;
use xepay::rpc;
use xepay::transaction::Transaction;

fn main() {
    println!("[miner] xepay-miner started");
    println!(
        "[miner] connecting to the node at {}",
        rpc::DEFAULT_RPC_ADDR
    );

    loop {
        let template = match rpc::call("getblocktemplate", vec![]) {
            Ok(r) if r.ok => r.result,
            Ok(r) => {
                println!("[miner] node rejected the request: {}", r.error);
                std::thread::sleep(std::time::Duration::from_secs(5));
                continue;
            }
            Err(e) => {
                println!("[miner] cannot reach the node: {}", e);
                std::thread::sleep(std::time::Duration::from_secs(5));
                continue;
            }
        };

        let height = template["height"].as_u64().unwrap_or(0);
        let previous_hash = template["previous_hash"].as_str().unwrap_or("").to_string();
        let previous_chainwork = template["previous_chainwork"]
            .as_str()
            .unwrap_or("0")
            .parse::<u128>()
            .unwrap_or(0);
        let nbits = match template["nbits"]
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
        {
            Some(v) => v,
            None => {
                println!("[miner] invalid nbits in template");
                std::thread::sleep(std::time::Duration::from_secs(5));
                continue;
            }
        };

        let subsidy_satoshi = template["subsidy_satoshi"].as_u64().unwrap_or(0);
        let total_fees_satoshi = template["total_fees_satoshi"].as_u64().unwrap_or(0);
        let reward_satoshi = template["reward_satoshi"].as_u64().unwrap_or(0);
        let state_root = template["state_root"].as_str().unwrap_or("").to_string();

        let mempool: Vec<Transaction> =
            serde_json::from_value(template["mempool"].clone()).unwrap_or_default();

        let target_address =
            std::env::var("XEPC_MINE_FOR").unwrap_or_else(|_| "UNKNOWN".to_string());

        if target_address == "UNKNOWN" {
            println!("[miner] XEPC_MINE_FOR is not set. Set it to a valid XEPAY address.");
            std::thread::sleep(std::time::Duration::from_secs(10));
            continue;
        }

        // BITCOIN-GRADE: Handle Result from Block::new_for
        let mut block = match Block::new_for(
            height,
            previous_hash,
            subsidy_satoshi,
            total_fees_satoshi,
            nbits,
            mempool,
            target_address.clone(),
            state_root,
            previous_chainwork,
        ) {
            Ok(b) => b,
            Err(e) => {
                println!("[miner] failed to create block: {}", e);
                std::thread::sleep(std::time::Duration::from_secs(1));
                continue;
            }
        };

        if block.reward != reward_satoshi {
            println!(
                "[miner] WARNING: reward mismatch. Block reward: {}, Template reward: {}",
                block.reward, reward_satoshi
            );
        }

        if !xepay::pow::mine(&mut block, nbits) {
            println!("[miner] mining failed: nonce exhausted");
            std::thread::sleep(std::time::Duration::from_secs(1));
            continue;
        }

        let block_json = serde_json::to_value(&block).unwrap();
        match rpc::call("submitblock", vec![block_json]) {
            Ok(r) if r.ok => {
                println!(
                    "[miner] block accepted: {}",
                    serde_json::to_string(&r.result).unwrap()
                );
            }
            Ok(r) => {
                println!("[miner] block rejected: {}", r.error);
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
            Err(e) => {
                println!("[miner] submission failed: {}", e);
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        }
    }
}
