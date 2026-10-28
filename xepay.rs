// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// xepay.rs
// Entry point for the XEPY (ticker: XEPY) demonstration blockchain.
// The implementation relies exclusively on SHA3-256 as its hashing primitive.
//
// This file serves as the orchestration layer of the demonstration. It does
// not implement cryptographic or consensus primitives itself; rather, it
// declares the sibling modules located at the project root, invokes the
// proof-of-work pipeline, and reports the resulting chain state in a
// deterministic, human-readable form.

// The project deliberately avoids the conventional `src/` directory in order
// to keep the demonstration compact and easily inspectable. Consequently,
// each module must be explicitly declared via the `#[path]` attribute so that
// the Rust compiler can resolve sibling files located at the repository root.
#[path = "crypto.rs"]
mod crypto;

#[path = "block.rs"]
mod block;

#[path = "blockchain.rs"]
mod blockchain;

use blockchain::Blockchain;

/// Program entry point.
///
/// The function performs the following sequence of operations:
///   1. Prints a banner identifying the network and its hashing algorithm.
///   2. Instantiates a blockchain with a predefined proof-of-work difficulty.
///   3. Appends two demonstration blocks containing illustrative transactions.
///   4. Validates the integrity of the entire chain.
///   5. Emits a concise summary of every block currently held in memory.
fn main() {
    print_banner();

    // A difficulty of three requires each block hash to begin with the
    // hexadecimal prefix "000". This level is intentionally modest so that
    // the demonstration completes within a reasonable amount of time on
    // ordinary hardware while still exercising the proof-of-work loop.
    let difficulty: u32 = 3;

    // Construct the chain. A genesis block is mined immediately upon
    // initialization, guaranteeing that the chain is never empty.
    let mut chain = Blockchain::new(difficulty);
    println!(
        "[XEPY] Genesis block has been created; current height = {}.\n",
        chain.height()
    );

    // Append the first demonstration block. Transactions are represented as
    // plain strings for the sake of brevity; a production-grade network would
    // instead employ signed, structured transaction records.
    chain.add_block(vec![
        "alice -> bob: 10 XEPY".to_string(),
        "bob -> carol: 2 XEPY".to_string(),
    ]);
    println!(
        "[XEPY] First demonstration block appended; current height = {}.\n",
        chain.height()
    );

    // Append the second demonstration block, again with illustrative payload.
    chain.add_block(vec!["carol -> dave: 5 XEPY".to_string()]);
    println!(
        "[XEPY] Second demonstration block appended; current height = {}.\n",
        chain.height()
    );

    // Validate the chain in its entirety before reporting the outcome. The
    // validation routine recomputes every block hash and verifies that each
    // block correctly references its predecessor.
    let validity = chain.is_valid();
    println!("Chain integrity verified: {}.", validity);
    println!("Total number of blocks: {}.", chain.height());

    // Emit a compact, line-per-block report. Hash fragments are truncated to
    // sixteen hexadecimal characters solely for readability; the underlying
    // values remain full-length inside the chain structure.
    println!("\n--- Chain contents ---");
    for (index, block) in chain.chain.iter().enumerate() {
        println!(
            "Block #{:>2} | hash={} | prev={} | nonce={} | txs={}",
            index,
            &block.hash[..16],
            &block.header.prev_block_hash[..16],
            block.header.nonce,
            block.transactions.len()
        );
    }

    // Concluding remark indicating that the demonstration has concluded.
    println!("\n[XEPY] Demonstration concluded successfully.");
}

/// Print the introductory banner for the demonstration.
///
/// The banner identifies the network name, the ticker symbol, and the
/// cryptographic primitive employed for block hashing. Keeping this output
/// separate from the main logic improves readability and makes it easier to
/// adjust the presentation in future revisions.
fn print_banner() {
    println!("==============================================");
    println!("   XEPY (ticker: XEPY) — SHA3-256 Demo Chain");
    println!("==============================================");
    println!();
    println!("This demonstration exercises a minimal blockchain");
    println!("implementation whose sole cryptographic primitive");
    println!("is SHA3-256. It is intended strictly for educational");
    println!("purposes and must not be deployed on any network");
    println!("that carries real economic value.");
    println!();
}
