// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Peer-to-peer message types for XEPAY, modeled after Bitcoin.
//!
//! Serialization is performed manually: integers are encoded in
//! little-endian byte order, array and string lengths use a varint
//! encoding, and hashes are encoded as 32 raw bytes.

use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::transaction::Transaction;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u32 = 1;
pub const USER_AGENT: &str = "/xepay:0.1.0/";

// The maximum payload length has been increased to 32 MB to
// accommodate larger header batches (5,000 headers) during sync.
pub const MAX_PAYLOAD_LEN: usize = 32 * 1024 * 1024;

/// Inventory type: transaction or block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InvType {
    Transaction = 1,
    Block = 2,
}

impl InvType {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            1 => Some(InvType::Transaction),
            2 => Some(InvType::Block),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Inventory {
    pub inv_type: InvType,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetAddr {
    pub services: u64,
    pub ip: [u8; 16],
    pub port: u16,
}

impl NetAddr {
    pub fn from_socket(addr: std::net::SocketAddr) -> Self {
        let ip = match addr.ip() {
            std::net::IpAddr::V4(v4) => v4.to_ipv6_mapped().octets(),
            std::net::IpAddr::V6(v6) => v6.octets(),
        };
        NetAddr {
            services: 0,
            ip,
            port: addr.port(),
        }
    }

    pub fn to_socket(&self) -> std::net::SocketAddr {
        let ip = std::net::Ipv6Addr::from(self.ip);
        if let Some(v4) = ip.to_ipv4_mapped() {
            std::net::SocketAddr::new(std::net::IpAddr::V4(v4), self.port)
        } else {
            std::net::SocketAddr::new(std::net::IpAddr::V6(ip), self.port)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionMessage {
    pub protocol_version: u32,
    pub services: u64,
    pub timestamp: i64,
    pub addr_recv: NetAddr,
    pub addr_from: NetAddr,
    pub nonce: u64,
    pub user_agent: String,
    pub start_height: u64,
    pub relay: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockLocator {
    pub version: u32,
    pub locator_hashes: Vec<String>,
    pub hash_stop: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Version(VersionMessage),
    Verack,
    Inv(Vec<Inventory>),
    GetData(Vec<Inventory>),
    GetHeaders(BlockLocator),
    Headers(Vec<Block>),
    Block(Block),
    Tx(Transaction),
    Addr(Vec<NetAddr>),
    GetAddr,
    Ping(u64),
    Pong(u64),
    Reject {
        command: String,
        code: u8,
        reason: String,
    },
}

impl Message {
    pub fn command(&self) -> &'static str {
        match self {
            Message::Version(_) => "version",
            Message::Verack => "verack",
            Message::Inv(_) => "inv",
            Message::GetData(_) => "getdata",
            Message::GetHeaders(_) => "getheaders",
            Message::Headers(_) => "headers",
            Message::Block(_) => "block",
            Message::Tx(_) => "tx",
            Message::Addr(_) => "addr",
            Message::GetAddr => "getaddr",
            Message::Ping(_) => "ping",
            Message::Pong(_) => "pong",
            Message::Reject { .. } => "reject",
        }
    }
}

fn write_varint(out: &mut Vec<u8>, n: u64) {
    if n < 0xfd {
        out.push(n as u8);
    } else if n <= 0xffff {
        out.push(0xfd);
        out.extend_from_slice(&(n as u16).to_le_bytes());
    } else if n <= 0xffff_ffff {
        out.push(0xfe);
        out.extend_from_slice(&(n as u32).to_le_bytes());
    } else {
        out.push(0xff);
        out.extend_from_slice(&n.to_le_bytes());
    }
}

fn read_varint(input: &mut &[u8]) -> Result<u64, String> {
    if input.is_empty() {
        return Err("varint: input is empty".into());
    }
    let first = input[0];
    *input = &input[1..];
    match first {
        0xfd => {
            if input.len() < 2 {
                return Err("varint: missing 2 bytes".into());
            }
            let v = u16::from_le_bytes([input[0], input[1]]) as u64;
            *input = &input[2..];
            Ok(v)
        }
        0xfe => {
            if input.len() < 4 {
                return Err("varint: missing 4 bytes".into());
            }
            let v = u32::from_le_bytes([input[0], input[1], input[2], input[3]]) as u64;
            *input = &input[4..];
            Ok(v)
        }
        0xff => {
            if input.len() < 8 {
                return Err("varint: missing 8 bytes".into());
            }
            let mut b = [0u8; 8];
            b.copy_from_slice(&input[..8]);
            *input = &input[8..];
            Ok(u64::from_le_bytes(b))
        }
        v => Ok(v as u64),
    }
}

fn write_string(out: &mut Vec<u8>, s: &str) {
    write_varint(out, s.len() as u64);
    out.extend_from_slice(s.as_bytes());
}

fn read_string(input: &mut &[u8]) -> Result<String, String> {
    let len = read_varint(input)? as usize;
    if input.len() < len {
        return Err("string: not enough bytes".into());
    }
    let s = String::from_utf8(input[..len].to_vec()).map_err(|e| format!("string: {}", e))?;
    *input = &input[len..];
    Ok(s)
}

fn write_netaddr(out: &mut Vec<u8>, addr: &NetAddr) {
    out.extend_from_slice(&addr.services.to_le_bytes());
    out.extend_from_slice(&addr.ip);
    out.extend_from_slice(&addr.port.to_be_bytes());
}

fn read_netaddr(input: &mut &[u8]) -> Result<NetAddr, String> {
    if input.len() < 8 + 16 + 2 {
        return Err("netaddr: not enough bytes".into());
    }
    let services = u64::from_le_bytes(input[..8].try_into().unwrap());
    let mut ip = [0u8; 16];
    ip.copy_from_slice(&input[8..24]);
    let port = u16::from_be_bytes([input[24], input[25]]);
    *input = &input[26..];
    Ok(NetAddr { services, ip, port })
}

impl Message {
    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::new();
        match self {
            Message::Version(v) => {
                out.extend_from_slice(&v.protocol_version.to_le_bytes());
                out.extend_from_slice(&v.services.to_le_bytes());
                out.extend_from_slice(&v.timestamp.to_le_bytes());
                write_netaddr(&mut out, &v.addr_recv);
                write_netaddr(&mut out, &v.addr_from);
                out.extend_from_slice(&v.nonce.to_le_bytes());
                write_string(&mut out, &v.user_agent);
                out.extend_from_slice(&v.start_height.to_le_bytes());
                out.push(if v.relay { 1 } else { 0 });
            }
            Message::Verack => {}
            Message::Inv(items) | Message::GetData(items) => {
                write_varint(&mut out, items.len() as u64);
                for inv in items {
                    out.extend_from_slice(&(inv.inv_type as u32).to_le_bytes());
                    let hash_bytes = hex::decode(&inv.hash).unwrap_or_else(|_| vec![0u8; 32]);
                    out.extend_from_slice(&hash_bytes);
                }
            }
            Message::GetHeaders(loc) => {
                out.extend_from_slice(&loc.version.to_le_bytes());
                write_varint(&mut out, loc.locator_hashes.len() as u64);
                for h in &loc.locator_hashes {
                    let b = hex::decode(h).unwrap_or_else(|_| vec![0u8; 32]);
                    out.extend_from_slice(&b);
                }
                let b = hex::decode(&loc.hash_stop).unwrap_or_else(|_| vec![0u8; 32]);
                out.extend_from_slice(&b);
            }
            Message::Headers(headers) => {
                write_varint(&mut out, headers.len() as u64);
                for h in headers {
                    out.extend_from_slice(&h.nversion.to_le_bytes());
                    out.extend_from_slice(&h.index.to_le_bytes());
                    out.extend_from_slice(&h.timestamp.to_le_bytes());
                    let prev = hex::decode(&h.previous_hash).unwrap_or_else(|_| vec![0u8; 32]);
                    out.extend_from_slice(&prev);
                    let merkle = hex::decode(&h.merkle_root).unwrap_or_else(|_| vec![0u8; 32]);
                    out.extend_from_slice(&merkle);
                    out.extend_from_slice(&h.nonce.to_le_bytes());
                    out.extend_from_slice(&h.nbits.to_le_bytes());
                    out.extend_from_slice(&h.reward.to_le_bytes());
                    let hash = hex::decode(&h.hash).unwrap_or_else(|_| vec![0u8; 32]);
                    out.extend_from_slice(&hash);
                    write_varint(&mut out, 0);
                }
            }
            Message::Block(b) => {
                out.extend_from_slice(&serde_json::to_vec(b).unwrap_or_default());
            }
            Message::Tx(tx) => {
                out.extend_from_slice(&serde_json::to_vec(tx).unwrap_or_default());
            }
            Message::Addr(addrs) => {
                write_varint(&mut out, addrs.len() as u64);
                for a in addrs {
                    write_netaddr(&mut out, a);
                }
            }
            Message::GetAddr => {}
            Message::Ping(nonce) | Message::Pong(nonce) => {
                out.extend_from_slice(&nonce.to_le_bytes());
            }
            Message::Reject {
                command,
                code,
                reason,
            } => {
                write_string(&mut out, command);
                out.push(*code);
                write_string(&mut out, reason);
            }
        }
        out
    }

    pub fn deserialize(command: &str, payload: &[u8]) -> Result<Self, String> {
        let mut input = payload;
        match command {
            "version" => {
                if input.len() < 4 + 8 + 8 + 26 + 26 + 8 {
                    return Err("version: payload too short".into());
                }
                let protocol_version = u32::from_le_bytes(input[..4].try_into().unwrap());
                input = &input[4..];
                let services = u64::from_le_bytes(input[..8].try_into().unwrap());
                input = &input[8..];
                let timestamp = i64::from_le_bytes(input[..8].try_into().unwrap());
                input = &input[8..];
                let addr_recv = read_netaddr(&mut input)?;
                let addr_from = read_netaddr(&mut input)?;
                let nonce = u64::from_le_bytes(input[..8].try_into().unwrap());
                input = &input[8..];
                let user_agent = read_string(&mut input)?;
                if input.len() < 8 {
                    return Err("version: start_height missing".into());
                }
                let start_height = u64::from_le_bytes(input[..8].try_into().unwrap());
                input = &input[8..];
                let relay = input.first().map(|&b| b != 0).unwrap_or(false);
                Ok(Message::Version(VersionMessage {
                    protocol_version,
                    services,
                    timestamp,
                    addr_recv,
                    addr_from,
                    nonce,
                    user_agent,
                    start_height,
                    relay,
                }))
            }
            "verack" => Ok(Message::Verack),
            "inv" | "getdata" => {
                let count = read_varint(&mut input)? as usize;
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    if input.len() < 4 + 32 {
                        return Err("inv: item missing bytes".into());
                    }
                    let t = u32::from_le_bytes(input[..4].try_into().unwrap());
                    input = &input[4..];
                    let inv_type = InvType::from_u32(t).ok_or("inv: unknown type")?;
                    let hash = hex::encode(&input[..32]);
                    input = &input[32..];
                    items.push(Inventory { inv_type, hash });
                }
                if command == "inv" {
                    Ok(Message::Inv(items))
                } else {
                    Ok(Message::GetData(items))
                }
            }
            "getheaders" => {
                if input.len() < 4 {
                    return Err("getheaders: version missing".into());
                }
                let version = u32::from_le_bytes(input[..4].try_into().unwrap());
                input = &input[4..];
                let count = read_varint(&mut input)? as usize;
                let mut locator_hashes = Vec::with_capacity(count);
                for _ in 0..count {
                    if input.len() < 32 {
                        return Err("getheaders: locator missing".into());
                    }
                    locator_hashes.push(hex::encode(&input[..32]));
                    input = &input[32..];
                }
                if input.len() < 32 {
                    return Err("getheaders: hash_stop missing".into());
                }
                let hash_stop = hex::encode(&input[..32]);
                Ok(Message::GetHeaders(BlockLocator {
                    version,
                    locator_hashes,
                    hash_stop,
                }))
            }
            "headers" => {
                let count = read_varint(&mut input)? as usize;
                let mut headers = Vec::with_capacity(count);
                for _ in 0..count {
                    if input.len() < 4 + 8 + 16 + 32 + 32 + 8 + 4 + 8 + 32 + 1 {
                        return Err("headers: header missing bytes".into());
                    }
                    let nversion = u32::from_le_bytes(input[..4].try_into().unwrap());
                    input = &input[4..];
                    let index = u64::from_le_bytes(input[..8].try_into().unwrap());
                    input = &input[8..];
                    let timestamp = u128::from_le_bytes(input[..16].try_into().unwrap());
                    input = &input[16..];
                    let previous_hash = hex::encode(&input[..32]);
                    input = &input[32..];
                    let merkle_root = hex::encode(&input[..32]);
                    input = &input[32..];
                    let nonce = u64::from_le_bytes(input[..8].try_into().unwrap());
                    input = &input[8..];
                    let nbits = u32::from_le_bytes(input[..4].try_into().unwrap());
                    input = &input[4..];
                    let reward = u64::from_le_bytes(input[..8].try_into().unwrap());
                    input = &input[8..];
                    let hash = hex::encode(&input[..32]);
                    input = &input[32..];
                    let _tx_count = read_varint(&mut input)?;

                    let mut b = Block::new(index, previous_hash.clone(), reward, nbits, Vec::new());
                    b.nversion = nversion;
                    b.timestamp = timestamp;
                    b.previous_hash = previous_hash;
                    b.merkle_root = merkle_root;
                    b.nonce = nonce;
                    b.nbits = nbits;
                    b.hash = hash;
                    b.reward = reward;
                    b.subsidy = Blockchain::calculate_reward(index);
                    headers.push(b);
                }
                Ok(Message::Headers(headers))
            }
            "block" => {
                let b: Block =
                    serde_json::from_slice(payload).map_err(|e| format!("block: {}", e))?;
                Ok(Message::Block(b))
            }
            "tx" => {
                let tx: Transaction =
                    serde_json::from_slice(payload).map_err(|e| format!("tx: {}", e))?;
                Ok(Message::Tx(tx))
            }
            "addr" => {
                let count = read_varint(&mut input)? as usize;
                let mut addrs = Vec::with_capacity(count);
                for _ in 0..count {
                    addrs.push(read_netaddr(&mut input)?);
                }
                Ok(Message::Addr(addrs))
            }
            "getaddr" => Ok(Message::GetAddr),
            "ping" => {
                if input.len() < 8 {
                    return Err("ping: nonce missing".into());
                }
                Ok(Message::Ping(u64::from_le_bytes(
                    input[..8].try_into().unwrap(),
                )))
            }
            "pong" => {
                if input.len() < 8 {
                    return Err("pong: nonce missing".into());
                }
                Ok(Message::Pong(u64::from_le_bytes(
                    input[..8].try_into().unwrap(),
                )))
            }
            "reject" => {
                let command = read_string(&mut input)?;
                if input.is_empty() {
                    return Err("reject: code missing".into());
                }
                let code = input[0];
                input = &input[1..];
                let reason = read_string(&mut input)?;
                Ok(Message::Reject {
                    command,
                    code,
                    reason,
                })
            }
            other => Err(format!("unknown command: {}", other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_roundtrip() {
        let v = VersionMessage {
            protocol_version: PROTOCOL_VERSION,
            services: 0,
            timestamp: 1_792_012_800,
            addr_recv: NetAddr {
                services: 0,
                ip: [0u8; 16],
                port: 7555,
            },
            addr_from: NetAddr {
                services: 0,
                ip: [0u8; 16],
                port: 7555,
            },
            nonce: 42,
            user_agent: USER_AGENT.to_string(),
            start_height: 0,
            relay: true,
        };
        let msg = Message::Version(v);
        let bytes = msg.serialize();
        let decoded = Message::deserialize("version", &bytes).unwrap();
        match decoded {
            Message::Version(v2) => {
                assert_eq!(v2.protocol_version, PROTOCOL_VERSION);
                assert_eq!(v2.nonce, 42);
                assert_eq!(v2.user_agent, USER_AGENT);
                assert!(v2.relay);
            }
            _ => panic!("expected a Version message"),
        }
    }

    #[test]
    fn inv_roundtrip() {
        let items = vec![
            Inventory {
                inv_type: InvType::Block,
                hash: "aa".repeat(32),
            },
            Inventory {
                inv_type: InvType::Transaction,
                hash: "bb".repeat(32),
            },
        ];
        let msg = Message::Inv(items.clone());
        let bytes = msg.serialize();
        let decoded = Message::deserialize("inv", &bytes).unwrap();
        match decoded {
            Message::Inv(items2) => assert_eq!(items2, items),
            _ => panic!("expected an Inv message"),
        }
    }

    #[test]
    fn ping_roundtrip() {
        let msg = Message::Ping(12345);
        let bytes = msg.serialize();
        let decoded = Message::deserialize("ping", &bytes).unwrap();
        match decoded {
            Message::Ping(n) => assert_eq!(n, 12345),
            _ => panic!("expected a Ping message"),
        }
    }

    #[test]
    fn varint_boundaries() {
        for n in [
            0u64,
            1,
            0xfc,
            0xfd,
            0xffff,
            0x1_0000,
            u32::MAX as u64,
            u64::MAX,
        ] {
            let mut buf = Vec::new();
            write_varint(&mut buf, n);
            let mut slice = buf.as_slice();
            assert_eq!(read_varint(&mut slice).unwrap(), n);
        }
    }
}
