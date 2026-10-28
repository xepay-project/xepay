// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// cli.rs
// Command-line wallet for XEPAY (ticker: XEPY).
//
// Usage:
//   xepay wallet new
//   xepay wallet from <secret_hex>
//   xepay wallet info <secret_hex>

use crate::wallet::Wallet;

pub fn run(args: &[String]) {
    if args.len() < 2 {
        print_help();
        return;
    }
    match (args[0].as_str(), args[1].as_str()) {
        ("wallet", "new") => {
            let w = Wallet::generate();
            println!("Address:    {}", w.address());
            println!("Secret hex: {}", w.export_secret());
            println!();
            println!("Save the secret hex securely. Anyone with it controls the funds.");
        }
        ("wallet", "from") if args.len() >= 3 => match Wallet::from_secret_hex(&args[2]) {
            Ok(w) => {
                println!("Address:    {}", w.address());
            }
            Err(e) => eprintln!("Error: {}", e),
        },
        ("wallet", "info") if args.len() >= 3 => match Wallet::from_secret_hex(&args[2]) {
            Ok(w) => {
                println!("Address:    {}", w.address());
                println!("Next nonce: {}", w.next_nonce);
            }
            Err(e) => eprintln!("Error: {}", e),
        },
        _ => print_help(),
    }
}

fn print_help() {
    println!("XEPAY CLI — ticker XEPY");
    println!();
    println!("Commands:");
    println!("  wallet new               Generate a new wallet");
    println!("  wallet from <secret_hex> Import a wallet from a secret hex");
    println!("  wallet info <secret_hex> Show address and nonce for a wallet");
}
