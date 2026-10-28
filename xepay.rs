// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// xepay.rs
// Entry point for the XEPAY (ticker: XEPY) demonstration blockchain.
//
// XEPY's design goal is a fast-block chain with an average block time of
// approximately fifteen seconds. This demonstration instantiates the chain,
// appends a number of blocks, exercises the difficulty adjustment algorithm,
// and reports both per-block and aggregate timing statistics.

#[path = "crypto.rs"]
mod crypto;

#[path = "block.rs"]
mod block;

#[path = "blockchain.rs"]
mod blockchain;

use blockchain::Blockchain;

fn main() {
    print_banner();

    // A modest starting difficulty keeps the demo fast on modest hardware.
    // The difficulty adjustment algorithm will adapt from here.
    let initial_difficulty: u32 = 3;

    let mut chain = Blockchain::new(initial_difficulty);
    println!(
        "[XEPY] Genesis block mined; height = {}; difficulty = {}.",
        chain.height(),
        chain.difficulty
    );
    println!();

    // Append enough blocks to trigger several difficulty adjustments.
    // With ADJUSTMENT_INTERVAL = 20, mining ~45 blocks triggers two
    // adjustments and yields a meaningful average block time.
    let blocks_to_mine: usize = 45;
    for i in 1..=blocks_to_mine {
        chain.add_block(vec![format!("tx-{}: alice -> bob: 1 XEPY", i)]);
        let avg = chain.average_block_time_secs();
        println!(
            "[XEPY] Block {:>3} appended; height={}; difficulty={}; avg_block_time={:.2}s",
            i,
            chain.height(),
            chain.difficulty,
            avg
        );
    }
    println!();

    // Final report.
    let validity = chain.is_valid();
    let avg = chain.average_block_time_secs();
    println!("Chain integrity verified: {}.", validity);
    println!("Total blocks: {}.", chain.height());
    println!("Average block time: {:.2} seconds.", avg);
    println!(
        "Target block time: {} seconds (drift: {:+.2} s).",
        blockchain::TARGET_BLOCK_TIME_SECS,
        avg - blockchain::TARGET_BLOCK_TIME_SECS as f64
    );
    println!();

    println!("--- Chain summary (last 10 blocks) ---");
    let start = chain.chain.len().saturating_sub(10);
    for (index, block) in chain.chain.iter().enumerate().skip(start) {
        println!(
            "Block #{:>3} | hash={} | prev={} | nonce={} | diff={} | txs={}",
            index,
            &block.hash[..16],
            &block.header.prev_block_hash[..16],
            block.header.nonce,
            block.header.difficulty,
            block.transactions.len()
        );
    }

    println!();
    println!("[XEPY] Demonstration concluded.");
}

/// Print the banner and educational disclaimer.
fn print_banner() {
    println!("==============================================");
    println!("   XEPY (ticker: XEPY) — SHA3-256 Fast Chain");
    println!("   Target block time: ~15 seconds");
    println!("==============================================");
    println!();
    println!("This demonstration exercises a minimal blockchain");
    println!("whose sole cryptographic primitive is SHA3-256.");
    println!("It includes an adaptive difficulty mechanism that");
    println!("aims to hold the average block time near fifteen");
    println!("seconds as hashrate fluctuates.");
    println!();
    println!("It is strictly an educational artifact and must");
    println!("not be deployed on any network carrying real");
    println!("economic value.");
    println!();
}
