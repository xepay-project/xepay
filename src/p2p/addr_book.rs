// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Address book for the XEPAY peer-to-peer network.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// A single address book entry.
#[derive(Debug, Clone)]
pub struct AddrEntry {
    pub addr: SocketAddr,
    pub last_seen: Instant,
    pub last_try: Option<Instant>,
    pub attempts: u32,
    pub failures: u32,
}

/// The address book.
#[derive(Debug, Default)]
pub struct AddrBook {
    entries: HashMap<SocketAddr, AddrEntry>,
    max_entries: usize,
}

impl AddrBook {
    pub fn new(max_entries: usize) -> Self {
        AddrBook {
            entries: HashMap::new(),
            max_entries,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Add an address. Return `true` if the address is new.
    pub fn add(&mut self, addr: SocketAddr) -> bool {
        if self.entries.contains_key(&addr) {
            if let Some(entry) = self.entries.get_mut(&addr) {
                entry.last_seen = Instant::now();
            }
            return false;
        }

        if self.entries.len() >= self.max_entries {
            self.evict_oldest();
        }

        self.entries.insert(
            addr,
            AddrEntry {
                addr,
                last_seen: Instant::now(),
                last_try: None,
                attempts: 0,
                failures: 0,
            },
        );
        true
    }

    /// Add multiple addresses. Return the number of new addresses.
    pub fn add_many(&mut self, addrs: &[SocketAddr]) -> usize {
        let mut added = 0;
        for addr in addrs {
            if self.add(*addr) {
                added += 1;
            }
        }
        added
    }

    /// Mark an address as attempted.
    pub fn mark_tried(&mut self, addr: &SocketAddr) {
        if let Some(entry) = self.entries.get_mut(addr) {
            entry.last_try = Some(Instant::now());
            entry.attempts = entry.attempts.saturating_add(1);
        }
    }

    /// Mark an address as failed. Return `true` if the address should be removed.
    pub fn mark_failed(&mut self, addr: &SocketAddr) -> bool {
        if let Some(entry) = self.entries.get_mut(addr) {
            entry.failures = entry.failures.saturating_add(1);
            if entry.failures >= 3 {
                return true;
            }
        }
        false
    }

    /// Return candidate addresses that have not been attempted recently.
    pub fn candidates(&self, min_interval: Duration, limit: usize) -> Vec<SocketAddr> {
        let now = Instant::now();
        let mut out: Vec<SocketAddr> = self
            .entries
            .values()
            .filter(|e| {
                if e.failures >= 3 {
                    return false;
                }
                match e.last_try {
                    None => true,
                    Some(t) => now.duration_since(t) > min_interval,
                }
            })
            .map(|e| e.addr)
            .collect();

        out.sort_by_key(|addr| {
            self.entries
                .get(addr)
                .and_then(|e| e.last_try)
                .map(|t| now.duration_since(t))
                .unwrap_or(Duration::MAX)
        });

        out.truncate(limit);
        out
    }

    /// Remove entries that have not been seen for longer than `max_age`.
    pub fn prune(&mut self, max_age: Duration) {
        let now = Instant::now();
        self.entries
            .retain(|_, e| now.duration_since(e.last_seen) <= max_age);
    }

    fn evict_oldest(&mut self) {
        if let Some(oldest) = self
            .entries
            .iter()
            .min_by_key(|(_, e)| e.last_seen)
            .map(|(addr, _)| *addr)
        {
            self.entries.remove(&oldest);
        }
    }

    pub fn all_addrs(&self) -> Vec<SocketAddr> {
        self.entries.keys().copied().collect()
    }
}
