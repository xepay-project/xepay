// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Peer state for the XEPAY peer-to-peer network.

use std::net::SocketAddr;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerState {
    Disconnected,
    Connecting,
    Handshaking,
    Connected,
    Banned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    OutboundFullRelay,
    OutboundBlockRelayOnly,
    Inbound,
    Feeler,
}

#[derive(Debug, Clone)]
pub struct Peer {
    pub id: u64,
    pub addr: SocketAddr,
    pub state: PeerState,
    pub conn_type: ConnectionType,
    pub outbound: bool,
    pub mapped_as: u32,
    pub local_nonce: u64,
    pub remote_nonce: u64,
    pub protocol_version: u32,
    pub user_agent: String,
    pub start_height: u64,
    pub last_recv: Instant,
    pub last_send: Instant,
    pub last_ping: Option<Instant>,
    pub pending_ping: Option<u64>,
    pub misbehavior: u32,
    pub read_buf: Vec<u8>,
    pub write_buf: Vec<u8>,
}

impl Peer {
    pub fn new(id: u64, addr: SocketAddr, conn_type: ConnectionType, mapped_as: u32) -> Self {
        let now = Instant::now();
        let outbound = matches!(
            conn_type,
            ConnectionType::OutboundFullRelay
                | ConnectionType::OutboundBlockRelayOnly
                | ConnectionType::Feeler
        );
        Peer {
            id,
            addr,
            state: if outbound {
                PeerState::Connecting
            } else {
                PeerState::Handshaking
            },
            conn_type,
            outbound,
            mapped_as,
            local_nonce: rand_nonce(),
            remote_nonce: 0,
            protocol_version: 0,
            user_agent: String::new(),
            start_height: 0,
            last_recv: now,
            last_send: now,
            last_ping: None,
            pending_ping: None,
            misbehavior: 0,
            read_buf: Vec::new(),
            write_buf: Vec::new(),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.state == PeerState::Connected
    }

    pub fn is_handshaking(&self) -> bool {
        self.state == PeerState::Handshaking
    }

    pub fn add_misbehavior(&mut self, score: u32) -> bool {
        self.misbehavior = self.misbehavior.saturating_add(score);
        if self.misbehavior >= MAX_MISBEHAVIOR {
            self.state = PeerState::Banned;
            true
        } else {
            false
        }
    }

    pub fn is_timed_out(&self, timeout: Duration) -> bool {
        self.last_recv.elapsed() > timeout
    }

    pub fn should_ping(&self, interval: Duration) -> bool {
        match self.last_ping {
            Some(t) => t.elapsed() > interval,
            None => true,
        }
    }

    pub fn mark_ping_sent(&mut self, nonce: u64) {
        self.last_ping = Some(Instant::now());
        self.pending_ping = Some(nonce);
    }

    pub fn mark_pong_received(&mut self, nonce: u64) -> bool {
        if self.pending_ping == Some(nonce) {
            self.pending_ping = None;
            true
        } else {
            false
        }
    }

    pub fn is_ping_timed_out(&self, timeout: Duration) -> bool {
        match self.last_ping {
            Some(t) => self.pending_ping.is_some() && t.elapsed() > timeout,
            None => false,
        }
    }
}

pub const MAX_MISBEHAVIOR: u32 = 100;

pub fn rand_nonce() -> u64 {
    use rand::RngCore;
    let mut rng = rand::thread_rng();
    rng.next_u64()
}
