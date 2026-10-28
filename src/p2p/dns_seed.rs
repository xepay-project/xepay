// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! DNS seed resolution for peer discovery.

use std::net::SocketAddr;

use crate::blockchain::dns_seed;
use crate::consensus::spec;

/// Resolve every DNS seed for the current network and return the
/// resulting socket addresses.
pub async fn resolve_seeds() -> Vec<SocketAddr> {
    let mut out = Vec::new();
    let port = spec::current_network().default_port();
    for seed in dns_seed::seeds_for_current_network() {
        // `lookup_host` requires a port in the string. Append the
        // network's default P2P port if it is not already present.
        let host = if seed.contains(':') {
            seed.to_string()
        } else {
            format!("{}:{}", seed, port)
        };

        // Bind the result to a local variable so that the borrow of
        // `host` ends before `host` is dropped.
        let lookup_result = tokio::net::lookup_host(&host).await;
        match lookup_result {
            Ok(iter) => {
                for addr in iter {
                    out.push(addr);
                }
            }
            Err(e) => {
                eprintln!("[dns_seed] failed to resolve {}: {}", host, e);
            }
        }
    }
    out
}

/// Resolve a single DNS seed.
pub async fn resolve_one(seed: &str) -> Vec<SocketAddr> {
    let port = spec::current_network().default_port();
    let host = if seed.contains(':') {
        seed.to_string()
    } else {
        format!("{}:{}", seed, port)
    };

    // Bind the result to a local variable to avoid the borrow
    // outliving `host`.
    let lookup_result = tokio::net::lookup_host(&host).await;
    match lookup_result {
        Ok(iter) => iter.collect(),
        Err(e) => {
            eprintln!("[dns_seed] failed to resolve {}: {}", host, e);
            Vec::new()
        }
    }
}
