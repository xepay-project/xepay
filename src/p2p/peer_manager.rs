// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Peer management for the XEPAY peer-to-peer network.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use crate::p2p::peer::{rand_nonce, ConnectionType, Peer, PeerState, MAX_MISBEHAVIOR};

#[derive(Debug)]
pub enum PeerEvent {
    Connected(u64),
    HandshakeComplete(u64),
    Disconnected(u64, String),
    Banned(u64, String),
    PingDue(u64, u64),
}

pub struct PeerManager {
    peers: HashMap<u64, Peer>,
    banned: HashSet<SocketAddr>,
    next_id: u64,
    max_outbound_full_relay: usize,
    max_outbound_block_relay: usize,
    max_inbound: usize,
    handshake_timeout: Duration,
    idle_timeout: Duration,
    ping_interval: Duration,
    ping_timeout: Duration,
    events: Vec<PeerEvent>,
}

impl PeerManager {
    pub fn new(
        max_outbound_full_relay: usize,
        max_outbound_block_relay: usize,
        max_inbound: usize,
        handshake_timeout: Duration,
        idle_timeout: Duration,
        ping_interval: Duration,
        ping_timeout: Duration,
    ) -> Self {
        PeerManager {
            peers: HashMap::new(),
            banned: HashSet::new(),
            next_id: 1,
            max_outbound_full_relay,
            max_outbound_block_relay,
            max_inbound,
            handshake_timeout,
            idle_timeout,
            ping_interval,
            ping_timeout,
            events: Vec::new(),
        }
    }

    // ------------------------------------------------------------------
    // Capacity checks (only count active peers)
    // ------------------------------------------------------------------

    fn is_active(peer: &Peer) -> bool {
        peer.state != PeerState::Disconnected && peer.state != PeerState::Banned
    }

    pub fn outbound_full_relay_count(&self) -> usize {
        self.peers
            .values()
            .filter(|p| p.conn_type == ConnectionType::OutboundFullRelay && Self::is_active(p))
            .count()
    }

    pub fn outbound_block_relay_count(&self) -> usize {
        self.peers
            .values()
            .filter(|p| p.conn_type == ConnectionType::OutboundBlockRelayOnly && Self::is_active(p))
            .count()
    }

    pub fn inbound_count(&self) -> usize {
        self.peers
            .values()
            .filter(|p| p.conn_type == ConnectionType::Inbound && Self::is_active(p))
            .count()
    }

    pub fn can_add_outbound_full_relay(&self) -> bool {
        self.outbound_full_relay_count() < self.max_outbound_full_relay
    }

    pub fn can_add_outbound_block_relay(&self) -> bool {
        self.outbound_block_relay_count() < self.max_outbound_block_relay
    }

    pub fn can_accept_inbound(&self) -> bool {
        self.inbound_count() < self.max_inbound
    }

    // ------------------------------------------------------------------
    // Peer lifecycle
    // ------------------------------------------------------------------

    /// Return `true` if `addr` is banned.
    pub fn is_banned(&self, addr: &SocketAddr) -> bool {
        self.banned.contains(addr)
    }

    /// Add a peer. Returns `None` if the address is banned.
    pub fn add_peer(
        &mut self,
        addr: SocketAddr,
        conn_type: ConnectionType,
        mapped_as: u32,
    ) -> Option<u64> {
        if self.banned.contains(&addr) {
            return None;
        }
        let id = self.next_id;
        self.next_id += 1;
        let peer = Peer::new(id, addr, conn_type, mapped_as);
        self.peers.insert(id, peer);
        self.events.push(PeerEvent::Connected(id));
        Some(id)
    }

    pub fn get(&self, id: u64) -> Option<&Peer> {
        self.peers.get(&id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Peer> {
        self.peers.get_mut(&id)
    }

    pub fn all_peers(&self) -> Vec<&Peer> {
        self.peers.values().collect()
    }

    pub fn remove(&mut self, id: u64, reason: &str) {
        if self.peers.remove(&id).is_some() {
            self.events
                .push(PeerEvent::Disconnected(id, reason.to_string()));
        }
    }

    /// Ban a peer: mark it banned, add its address to the ban list,
    /// and remove it from the active set.
    pub fn ban(&mut self, id: u64, reason: &str) {
        if let Some(peer) = self.peers.get_mut(&id) {
            peer.state = PeerState::Banned;
            peer.add_misbehavior(MAX_MISBEHAVIOR);
            self.banned.insert(peer.addr);
            self.events.push(PeerEvent::Banned(id, reason.to_string()));
        }
    }

    pub fn drain_events(&mut self) -> Vec<PeerEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn push_event(&mut self, event: PeerEvent) {
        self.events.push(event);
    }

    // ------------------------------------------------------------------
    // Periodic maintenance
    // ------------------------------------------------------------------

    pub fn tick(&mut self) {
        let now = Instant::now();
        let mut to_remove = Vec::new();
        let mut to_ping = Vec::new();

        for (id, peer) in self.peers.iter_mut() {
            match peer.state {
                PeerState::Handshaking => {
                    if now.duration_since(peer.last_recv) > self.handshake_timeout {
                        to_remove.push((*id, "handshake timeout".to_string()));
                    }
                }
                PeerState::Connected => {
                    if peer.is_timed_out(self.idle_timeout) {
                        to_remove.push((*id, "idle timeout".to_string()));
                    } else if peer.is_ping_timed_out(self.ping_timeout) {
                        to_remove.push((*id, "ping timeout".to_string()));
                    } else if peer.should_ping(self.ping_interval) {
                        let nonce = rand_nonce();
                        peer.mark_ping_sent(nonce);
                        to_ping.push((*id, nonce));
                    }
                }
                _ => {}
            }
        }

        for (id, reason) in to_remove {
            self.remove(id, &reason);
        }
        for (id, nonce) in to_ping {
            self.events.push(PeerEvent::PingDue(id, nonce));
        }
    }

    pub fn connected_peers(&self) -> Vec<u64> {
        self.peers
            .iter()
            .filter(|(_, p)| p.is_connected())
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn connected_addrs(&self) -> Vec<SocketAddr> {
        self.peers
            .values()
            .filter(|p| p.is_connected())
            .map(|p| p.addr)
            .collect()
    }
}
