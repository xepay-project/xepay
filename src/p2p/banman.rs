// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Ban and discouragement management, modeled after Bitcoin Core's BanMan.
//!
//! Banning is manual and persistent (banlist.json). Discouragement is
//! automatic and ephemeral (rolling bloom filter). A discouraged peer is
//! still accepted as inbound but is preferred for eviction.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::datadir;

/// Default ban duration: 24 hours.
pub const DEFAULT_BAN_TIME_SECS: u64 = 60 * 60 * 24;

/// Ban database file name.
pub const BANLIST_DATABASE_FILENAME: &str = "banlist.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BanEntry {
    pub subnet: String,
    pub ban_created: u64,
    pub banned_until: u64,
    pub ban_duration: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BanList {
    pub banned: HashMap<String, BanEntry>,
}

pub struct BanMan {
    banlist: BanList,
    /// Ephemeral discourage filter. In a full implementation this would
    /// be a rolling bloom filter. For simplicity we use a HashSet with
    /// bounded size.
    discouraged: std::collections::HashSet<IpAddr>,
    max_discouraged: usize,
}

impl BanMan {
    pub fn new() -> Self {
        let banlist = load_banlist().unwrap_or_default();
        BanMan {
            banlist,
            discouraged: std::collections::HashSet::new(),
            // Bitcoin Core uses a 50,000-entry rolling bloom filter with
            // a false positive rate of 1e-6.
            max_discouraged: 50_000,
        }
    }

    /// Return whether the given address is banned.
    pub fn is_banned(&self, addr: &SocketAddr) -> bool {
        let ip = addr.ip();
        for entry in self.banlist.banned.values() {
            if subnet_matches(&entry.subnet, &ip) {
                let now = now_secs();
                if entry.banned_until == 0 || entry.banned_until > now {
                    return true;
                }
            }
        }
        false
    }

    /// Return whether the given address is discouraged.
    pub fn is_discouraged(&self, ip: &IpAddr) -> bool {
        self.discouraged.contains(ip)
    }

    /// Add an address to the ban list with the default duration.
    pub fn ban(&mut self, addr: &SocketAddr, duration_secs: Option<u64>) {
        let ip = addr.ip();
        let subnet = if ip.is_ipv4() {
            format!("{}/32", ip)
        } else {
            format!("{}/128", ip)
        };
        let now = now_secs();
        let duration = duration_secs.unwrap_or(DEFAULT_BAN_TIME_SECS);
        let until = now.saturating_add(duration);
        self.banlist.banned.insert(
            subnet.clone(),
            BanEntry {
                subnet,
                ban_created: now,
                banned_until: until,
                ban_duration: duration,
            },
        );
    }

    /// Remove a ban entry.
    pub fn unban(&mut self, subnet: &str) {
        self.banlist.banned.remove(subnet);
    }

    /// Clear all bans.
    pub fn clear_banned(&mut self) {
        self.banlist.banned.clear();
    }

    /// Mark an address as discouraged. This is called when a peer
    /// misbehaves. The peer is not disconnected but is preferred for
    /// eviction on the next inbound connection.
    pub fn discourage(&mut self, ip: IpAddr) {
        if self.discouraged.len() >= self.max_discouraged {
            // Remove an arbitrary entry to bound memory usage.
            if let Some(&victim) = self.discouraged.iter().next() {
                self.discouraged.remove(&victim);
            }
        }
        self.discouraged.insert(ip);
    }

    /// Persist the ban list to disk. Bitcoin Core dumps the ban list
    /// every 15 minutes and on shutdown.
    pub fn dump_banlist(&self) -> std::io::Result<()> {
        datadir::ensure_dirs()?;
        let path = banlist_path();
        let json = serde_json::to_string_pretty(&self.banlist).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    /// Return a list of active bans for the RPC layer.
    pub fn list_banned(&self) -> Vec<&BanEntry> {
        let now = now_secs();
        self.banlist
            .banned
            .values()
            .filter(|e| e.banned_until == 0 || e.banned_until > now)
            .collect()
    }
}

fn load_banlist() -> Option<BanList> {
    let path = banlist_path();
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

fn banlist_path() -> PathBuf {
    datadir::datadir().join(BANLIST_DATABASE_FILENAME)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Check whether an IP address matches a subnet string such as "1.2.3.0/24".
fn subnet_matches(subnet: &str, ip: &IpAddr) -> bool {
    let parts: Vec<&str> = subnet.split('/').collect();
    if parts.len() != 2 {
        return subnet == &ip.to_string();
    }
    let net_ip: IpAddr = match parts[0].parse() {
        Ok(ip) => ip,
        Err(_) => return false,
    };
    let prefix_len: u32 = parts[1].parse().unwrap_or(0);

    match (net_ip, ip) {
        (IpAddr::V4(net), IpAddr::V4(addr)) => {
            let net_bits = u32::from(net);
            let addr_bits = u32::from(*addr);
            if prefix_len >= 32 {
                net_bits == addr_bits
            } else {
                let mask = !0u32 << (32 - prefix_len);
                (net_bits & mask) == (addr_bits & mask)
            }
        }
        (IpAddr::V6(net), IpAddr::V6(addr)) => {
            let net_bits = u128::from(net);
            let addr_bits = u128::from(*addr);
            if prefix_len >= 128 {
                net_bits == addr_bits
            } else {
                let mask = !0u128 << (128 - prefix_len);
                (net_bits & mask) == (addr_bits & mask)
            }
        }
        _ => false,
    }
}
