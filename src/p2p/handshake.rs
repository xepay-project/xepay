// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Version and Verack handshaking for the XEPAY peer-to-peer network.

use std::net::SocketAddr;

use crate::blockchain::Blockchain;
use crate::p2p::message::{Message, NetAddr, VersionMessage, PROTOCOL_VERSION, USER_AGENT};
use crate::p2p::peer::{Peer, PeerState};

pub fn build_version(chain: &Blockchain, peer: &Peer, local_addr: SocketAddr) -> Message {
    build_version_with_relay(chain, peer, local_addr, true)
}

pub fn build_version_with_relay(
    chain: &Blockchain,
    peer: &Peer,
    local_addr: SocketAddr,
    relay: bool,
) -> Message {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    Message::Version(VersionMessage {
        protocol_version: PROTOCOL_VERSION,
        services: 0,
        timestamp: now,
        addr_recv: NetAddr::from_socket(peer.addr),
        addr_from: NetAddr::from_socket(local_addr),
        nonce: peer.local_nonce,
        user_agent: USER_AGENT.to_string(),
        start_height: chain.height(),
        relay,
    })
}

pub fn handle_version(peer: &mut Peer, msg: &VersionMessage) -> Result<(), String> {
    if peer.state != PeerState::Handshaking {
        return Err("version received in the wrong state".into());
    }

    if msg.protocol_version < PROTOCOL_VERSION {
        return Err(format!(
            "peer protocol version {} is below the minimum {}",
            msg.protocol_version, PROTOCOL_VERSION
        ));
    }

    if msg.nonce == peer.local_nonce {
        return Err("self-connection detected".into());
    }

    peer.protocol_version = msg.protocol_version;
    peer.user_agent = msg.user_agent.clone();
    peer.start_height = msg.start_height;
    peer.remote_nonce = msg.nonce;

    Ok(())
}

pub fn handle_verack(peer: &mut Peer) -> Result<bool, String> {
    if peer.state != PeerState::Handshaking {
        return Err("verack received in the wrong state".into());
    }
    if peer.protocol_version == 0 {
        return Err("verack received before version".into());
    }
    peer.state = PeerState::Connected;
    Ok(true)
}

pub fn validate_magic(_peer: &Peer) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::peer::Peer;

    fn dummy_peer() -> Peer {
        let addr: SocketAddr = "127.0.0.1:8333".parse().unwrap();
        // The `mapped_as` parameter is set to 0 for tests.
        Peer::new(1, addr, crate::p2p::peer::ConnectionType::Inbound, 0)
    }

    #[test]
    fn valid_version() {
        let mut peer = dummy_peer();
        let msg = VersionMessage {
            protocol_version: PROTOCOL_VERSION,
            services: 0,
            timestamp: 0,
            addr_recv: NetAddr::from_socket(peer.addr),
            addr_from: NetAddr::from_socket(peer.addr),
            nonce: peer.local_nonce.wrapping_add(1),
            user_agent: "/test/".into(),
            start_height: 0,
            relay: true,
        };
        assert!(handle_version(&mut peer, &msg).is_ok());
        assert_eq!(peer.protocol_version, PROTOCOL_VERSION);
    }

    #[test]
    fn self_connection_is_rejected() {
        let mut peer = dummy_peer();
        let nonce = peer.local_nonce;
        let msg = VersionMessage {
            protocol_version: PROTOCOL_VERSION,
            services: 0,
            timestamp: 0,
            addr_recv: NetAddr::from_socket(peer.addr),
            addr_from: NetAddr::from_socket(peer.addr),
            nonce,
            user_agent: "/test/".into(),
            start_height: 0,
            relay: true,
        };
        assert!(handle_version(&mut peer, &msg).is_err());
    }

    #[test]
    fn verack_after_version() {
        let mut peer = dummy_peer();
        let msg = VersionMessage {
            protocol_version: PROTOCOL_VERSION,
            services: 0,
            timestamp: 0,
            addr_recv: NetAddr::from_socket(peer.addr),
            addr_from: NetAddr::from_socket(peer.addr),
            nonce: peer.local_nonce.wrapping_add(1),
            user_agent: "/test/".into(),
            start_height: 0,
            relay: true,
        };
        handle_version(&mut peer, &msg).unwrap();
        assert!(handle_verack(&mut peer).unwrap());
        assert_eq!(peer.state, PeerState::Connected);
    }

    #[test]
    fn verack_without_version_is_rejected() {
        let mut peer = dummy_peer();
        assert!(handle_verack(&mut peer).is_err());
    }
}
