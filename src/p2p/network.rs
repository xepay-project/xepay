// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Main network loop for the XEPAY peer-to-peer layer.

use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};

use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::p2p::addr_book::AddrBook;
use crate::p2p::anchors::{self, Anchor, MAX_BLOCK_RELAY_ONLY_ANCHORS};
use crate::p2p::banman::BanMan;
use crate::p2p::dns_seed;
use crate::p2p::framing::{read_message, write_message};
use crate::p2p::handshake;
use crate::p2p::inv::InventoryManager;
use crate::p2p::message::{BlockLocator, InvType, Inventory, Message};
use crate::p2p::peer::{ConnectionType, PeerState};
use crate::p2p::peer_manager::{PeerEvent, PeerManager};
use crate::p2p::peers;
use crate::p2p::sync::SyncState;
use crate::storage;
use crate::utils::{format_datetime, now_millis};
use crate::validation;

const ASMAP_JSON: &str = include_str!("asmap.json");
const MAX_WRITE_BUFFER: usize = 10 * 1024 * 1024;

// Rate limiting constants for incoming P2P messages.
const MAX_TX_PER_SECOND: u32 = 50;
const MAX_INV_PER_SECOND: u32 = 100;
const RATE_LIMIT_WINDOW_SECS: u64 = 1;

#[derive(Debug, Clone, serde::Serialize)]
pub struct PeerInfo {
    pub id: u64,
    pub addr: String,
    pub outbound: bool,
    pub conn_type: String,
    pub mapped_as: u32,
    pub state: String,
}

pub type PeerRegistry = Arc<Mutex<Vec<PeerInfo>>>;

#[derive(Debug, Clone)]
pub enum NetworkCommand {
    BroadcastBlock(String),
    BroadcastTransaction(String),
    Shutdown,
}

#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub listen_addr: SocketAddr,
    pub bootstrap: Vec<SocketAddr>,
    pub max_outbound_full_relay: usize,
    pub max_outbound_block_relay: usize,
    pub max_inbound: usize,
    pub handshake_timeout: Duration,
    pub idle_timeout: Duration,
    pub ping_interval: Duration,
    pub ping_timeout: Duration,
    pub connect_interval: Duration,
    pub feeler_interval: Duration,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        NetworkConfig {
            listen_addr: "0.0.0.0:7555".parse().unwrap(),
            bootstrap: Vec::new(),
            max_outbound_full_relay: 8,
            max_outbound_block_relay: 2,
            max_inbound: 115,
            handshake_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(120),
            ping_interval: Duration::from_secs(30),
            ping_timeout: Duration::from_secs(60),
            connect_interval: Duration::from_secs(2),
            feeler_interval: Duration::from_secs(120),
        }
    }
}

struct Connection {
    stream: TcpStream,
    addr: SocketAddr,
    read_buf: Vec<u8>,
    write_buf: Vec<u8>,
    // Rate limiting state per connection.
    tx_count: u32,
    inv_count: u32,
    last_reset: Instant,
}

pub struct Network {
    config: NetworkConfig,
    chain: Arc<RwLock<Blockchain>>,
    peer_manager: PeerManager,
    addr_book: AddrBook,
    inv: InventoryManager,
    connections: HashMap<u64, Connection>,
    listener: Option<TcpListener>,
    local_addr: Option<SocketAddr>,
    next_connect_at: Instant,
    next_feeler_at: Instant,
    peer_registry: PeerRegistry,
    command_rx: mpsc::Receiver<NetworkCommand>,
    asmap: HashMap<String, u32>,
    banman: BanMan,
    anchors: Vec<Anchor>,
    block_relay_peers: Vec<SocketAddr>,
    sync_state: SyncState,
}

impl Network {
    pub fn new(
        config: NetworkConfig,
        chain: Arc<RwLock<Blockchain>>,
        peer_registry: PeerRegistry,
        command_rx: mpsc::Receiver<NetworkCommand>,
    ) -> Self {
        let peer_manager = PeerManager::new(
            config.max_outbound_full_relay,
            config.max_outbound_block_relay,
            config.max_inbound,
            config.handshake_timeout,
            config.idle_timeout,
            config.ping_interval,
            config.ping_timeout,
        );

        let asmap: HashMap<String, u32> = serde_json::from_str(ASMAP_JSON).unwrap_or_else(|e| {
            eprintln!("[p2p] WARNING: failed to parse asmap.json: {}", e);
            HashMap::new()
        });
        println!("[p2p] loaded custom ASMap with {} entries", asmap.len());

        let anchors = anchors::load_anchors();
        if !anchors.is_empty() {
            println!(
                "[p2p] loaded {} anchor(s) from {}",
                anchors.len(),
                anchors::ANCHORS_DATABASE_FILENAME
            );
        }
        anchors::delete_anchors_file();

        Network {
            config,
            chain,
            peer_manager,
            addr_book: AddrBook::new(1024),
            inv: InventoryManager::new(),
            connections: HashMap::new(),
            listener: None,
            local_addr: None,
            next_connect_at: Instant::now(),
            next_feeler_at: Instant::now() + Duration::from_secs(10),
            peer_registry,
            command_rx,
            asmap,
            banman: BanMan::new(),
            anchors,
            block_relay_peers: Vec::new(),
            sync_state: SyncState::new(),
        }
    }

    pub async fn run(&mut self) -> Result<(), String> {
        let listener = TcpListener::bind(self.config.listen_addr)
            .await
            .map_err(|e| format!("failed to bind {}: {}", self.config.listen_addr, e))?;
        let local = listener
            .local_addr()
            .map_err(|e| format!("failed to get local address: {}", e))?;
        self.local_addr = Some(local);
        self.listener = Some(listener);
        println!("[p2p] listening on {}", local);

        for addr in &self.config.bootstrap {
            self.addr_book.add(*addr);
        }

        if self.addr_book.is_empty() {
            let seeds = dns_seed::resolve_seeds().await;
            let n = self.addr_book.add_many(&seeds);
            if n > 0 {
                println!("[p2p] {} peers discovered from DNS seeds", n);
            }
        }

        let anchors_to_connect: Vec<Anchor> = self.anchors.clone();
        for anchor in &anchors_to_connect {
            if !self.banman.is_banned(&anchor.addr) {
                let asn = self.ip_to_asn(anchor.addr.ip());
                println!("[p2p] connecting to anchor {}", anchor.addr);
                self.attempt_connect(anchor.addr, ConnectionType::OutboundBlockRelayOnly, asn)
                    .await;
            }
        }

        let tick_interval = Duration::from_millis(500);
        let mut last_tick = Instant::now();

        loop {
            let now = Instant::now();
            if now.duration_since(last_tick) >= tick_interval {
                self.on_tick().await;
                last_tick = now;
            }

            self.try_connect().await;
            self.try_feeler().await;

            let has_listener = self.listener.is_some();
            tokio::select! {
                accept = async {
                    if let Some(l) = &self.listener {
                        l.accept().await
                    } else {
                        std::future::pending().await
                    }
                }, if has_listener => {
                    match accept {
                        Ok((stream, addr)) => {
                            self.on_inbound(stream, addr).await;
                        }
                        Err(e) => {
                            eprintln!("[p2p] accept failed: {}", e);
                        }
                    }
                }
                cmd = self.command_rx.recv() => {
                    match cmd {
                        Some(NetworkCommand::Shutdown) => {
                            println!("[p2p] shutdown requested");
                            break;
                        }
                        Some(cmd) => self.handle_command(cmd).await,
                        None => break,
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(50)) => {}
            }

            self.process_connections().await;
            self.process_events().await;
            self.update_registry();
        }

        self.save_anchors_on_shutdown();
        if let Err(e) = self.banman.dump_banlist() {
            eprintln!("[p2p] failed to dump banlist: {}", e);
        }

        Ok(())
    }

    fn save_anchors_on_shutdown(&self) {
        let anchors: Vec<Anchor> = self
            .block_relay_peers
            .iter()
            .take(MAX_BLOCK_RELAY_ONLY_ANCHORS)
            .map(|addr| Anchor { addr: *addr })
            .collect();
        if let Err(e) = anchors::save_anchors(&anchors) {
            eprintln!("[p2p] failed to save anchors: {}", e);
        } else if !anchors.is_empty() {
            println!("[p2p] saved {} anchor(s)", anchors.len());
        }
    }

    async fn handle_command(&mut self, cmd: NetworkCommand) {
        match cmd {
            NetworkCommand::BroadcastBlock(hash) => {
                self.inv.add_block(&hash);
                println!("[p2p] broadcasting block {}", hash);
                self.broadcast_inv_to_full_relay(&[Inventory {
                    inv_type: InvType::Block,
                    hash,
                }]);
            }
            NetworkCommand::BroadcastTransaction(txid) => {
                self.inv.add_tx(&txid);
                self.broadcast_inv_to_full_relay(&[Inventory {
                    inv_type: InvType::Transaction,
                    hash: txid,
                }]);
            }
            NetworkCommand::Shutdown => {}
        }
    }

    fn update_registry(&self) {
        let mut reg = self.peer_registry.lock().unwrap();
        reg.clear();
        for peer in self.peer_manager.all_peers() {
            if peer.state == PeerState::Disconnected || peer.state == PeerState::Banned {
                continue;
            }
            let conn_type_str = match peer.conn_type {
                ConnectionType::OutboundFullRelay => "outbound-full-relay",
                ConnectionType::OutboundBlockRelayOnly => "block-relay-only",
                ConnectionType::Inbound => "inbound",
                ConnectionType::Feeler => "feeler",
            };
            reg.push(PeerInfo {
                id: peer.id,
                addr: peer.addr.to_string(),
                outbound: peer.outbound,
                conn_type: conn_type_str.to_string(),
                mapped_as: peer.mapped_as,
                state: format!("{:?}", peer.state),
            });
        }
    }

    async fn on_tick(&mut self) {
        self.peer_manager.tick();
        self.addr_book.prune(Duration::from_secs(3600));
        self.sync_state.expire_requests();
    }

    async fn try_connect(&mut self) {
        if Instant::now() < self.next_connect_at {
            return;
        }

        let mut connected_asns = HashSet::new();
        for peer in self.peer_manager.all_peers() {
            if peer.is_connected() {
                connected_asns.insert(peer.mapped_as);
            }
        }

        if self.peer_manager.can_add_outbound_full_relay() {
            if let Some(addr) = self
                .addr_book
                .candidates(Duration::from_secs(60), 10)
                .iter()
                .find(|addr| {
                    if self.banman.is_banned(addr) || self.banman.is_discouraged(&addr.ip()) {
                        return false;
                    }
                    let asn = self.ip_to_asn(addr.ip());
                    !connected_asns.contains(&asn)
                })
            {
                let asn = self.ip_to_asn(addr.ip());
                self.attempt_connect(*addr, ConnectionType::OutboundFullRelay, asn)
                    .await;
            }
        }

        if self.peer_manager.can_add_outbound_block_relay() {
            if let Some(addr) = self
                .addr_book
                .candidates(Duration::from_secs(60), 10)
                .iter()
                .find(|addr| {
                    if self.banman.is_banned(addr) || self.banman.is_discouraged(&addr.ip()) {
                        return false;
                    }
                    let asn = self.ip_to_asn(addr.ip());
                    !connected_asns.contains(&asn)
                })
            {
                let asn = self.ip_to_asn(addr.ip());
                self.attempt_connect(*addr, ConnectionType::OutboundBlockRelayOnly, asn)
                    .await;
            }
        }

        self.next_connect_at = Instant::now() + self.config.connect_interval;
    }

    async fn try_feeler(&mut self) {
        if Instant::now() < self.next_feeler_at {
            return;
        }
        let connected_addrs: HashSet<SocketAddr> = self
            .peer_manager
            .all_peers()
            .iter()
            .filter(|p| p.is_connected())
            .map(|p| p.addr)
            .collect();

        if let Some(addr) = self
            .addr_book
            .candidates(Duration::from_secs(120), 1)
            .first()
        {
            if self.banman.is_banned(addr) || self.banman.is_discouraged(&addr.ip()) {
                self.next_feeler_at = Instant::now() + self.config.feeler_interval;
                return;
            }
            if let Some(local) = self.local_addr {
                if local.ip().is_unspecified() {
                    if addr.port() == local.port() {
                        self.next_feeler_at = Instant::now() + self.config.feeler_interval;
                        return;
                    }
                } else if addr.ip() == local.ip() && addr.port() == local.port() {
                    self.next_feeler_at = Instant::now() + self.config.feeler_interval;
                    return;
                }
            }
            if connected_addrs.contains(addr) {
                self.next_feeler_at = Instant::now() + self.config.feeler_interval;
                return;
            }
            let asn = self.ip_to_asn(addr.ip());
            println!("[p2p] feeler connection to {} (AS{})", addr, asn);
            self.attempt_connect(*addr, ConnectionType::Feeler, asn)
                .await;
        }
        self.next_feeler_at = Instant::now() + self.config.feeler_interval;
    }

    async fn attempt_connect(
        &mut self,
        addr: SocketAddr,
        conn_type: ConnectionType,
        mapped_as: u32,
    ) {
        if self.banman.is_banned(&addr) {
            return;
        }
        if let Some(local) = self.local_addr {
            if local.ip().is_unspecified() {
                if addr.port() == local.port() {
                    return;
                }
            } else if addr.ip() == local.ip() && addr.port() == local.port() {
                return;
            }
        }

        self.addr_book.mark_tried(&addr);
        match TcpStream::connect(addr).await {
            Ok(stream) => {
                let _ = stream.set_nodelay(true);
                let id = match self.peer_manager.add_peer(addr, conn_type, mapped_as) {
                    Some(id) => id,
                    None => {
                        eprintln!("[p2p] refusing to connect to banned peer {}", addr);
                        return;
                    }
                };
                self.connections.insert(
                    id,
                    Connection {
                        stream,
                        addr,
                        read_buf: Vec::new(),
                        write_buf: Vec::new(),
                        tx_count: 0,
                        inv_count: 0,
                        last_reset: Instant::now(),
                    },
                );

                if let Some(peer) = self.peer_manager.get_mut(id) {
                    peer.state = PeerState::Handshaking;
                }

                if conn_type == ConnectionType::OutboundBlockRelayOnly {
                    self.block_relay_peers.push(addr);
                    if self.block_relay_peers.len() > MAX_BLOCK_RELAY_ONLY_ANCHORS {
                        self.block_relay_peers.remove(0);
                    }
                }

                self.send_version(id).await;
            }
            Err(e) => {
                eprintln!("[p2p] failed to connect to {}: {}", addr, e);
                let _ = self.addr_book.mark_failed(&addr);
            }
        }
    }

    async fn on_inbound(&mut self, stream: TcpStream, addr: SocketAddr) {
        if self.banman.is_banned(&addr) {
            eprintln!("[p2p] rejecting inbound from banned address {}", addr);
            return;
        }
        if !self.peer_manager.can_accept_inbound() {
            eprintln!("[p2p] rejecting inbound {}: peer limit reached", addr);
            return;
        }
        let _ = stream.set_nodelay(true);
        let asn = self.ip_to_asn(addr.ip());
        let id = match self
            .peer_manager
            .add_peer(addr, ConnectionType::Inbound, asn)
        {
            Some(id) => id,
            None => {
                eprintln!("[p2p] rejecting banned inbound {}", addr);
                return;
            }
        };
        self.connections.insert(
            id,
            Connection {
                stream,
                addr,
                read_buf: Vec::new(),
                write_buf: Vec::new(),
                tx_count: 0,
                inv_count: 0,
                last_reset: Instant::now(),
            },
        );
        println!("[p2p] inbound connection from {} (AS{})", addr, asn);

        if let Some(peer) = self.peer_manager.get_mut(id) {
            peer.state = PeerState::Handshaking;
        }

        self.send_version(id).await;
    }

    async fn send_version(&mut self, id: u64) {
        let peer = match self.peer_manager.get(id) {
            Some(p) => p.clone(),
            None => return,
        };
        let local = match self.local_addr {
            Some(a) => a,
            None => return,
        };
        let chain = self.chain.read().await;

        let relay = match peer.conn_type {
            ConnectionType::OutboundBlockRelayOnly | ConnectionType::Feeler => false,
            _ => true,
        };

        let msg = handshake::build_version_with_relay(&chain, &peer, local, relay);
        drop(chain);
        self.queue_message(id, &msg);
    }

    fn queue_message(&mut self, id: u64, msg: &Message) {
        let payload = msg.serialize();
        let cmd = msg.command();
        let mut buf = Vec::new();
        if write_message(&mut buf, cmd, &payload).is_err() {
            return;
        }
        if let Some(conn) = self.connections.get_mut(&id) {
            if conn.write_buf.len() + buf.len() > MAX_WRITE_BUFFER {
                eprintln!(
                    "[p2p] write buffer limit reached for peer {}, dropping connection",
                    id
                );
                conn.write_buf.clear();
                conn.read_buf.clear();
                return;
            }
            conn.write_buf.extend_from_slice(&buf);
        }
    }

    async fn process_connections(&mut self) {
        let ids: Vec<u64> = self.connections.keys().copied().collect();
        for id in ids {
            self.process_one(id).await;
        }
    }

    async fn process_one(&mut self, id: u64) {
        let (read_result, has_write) = {
            let conn = match self.connections.get_mut(&id) {
                Some(c) => c,
                None => return,
            };

            let mut buf = [0u8; 4096];
            let read_fut = conn.stream.read(&mut buf);
            let read_result = tokio::time::timeout(Duration::from_millis(10), read_fut).await;

            match read_result {
                Ok(Ok(0)) => (Err("peer closed the connection".to_string()), false),
                Ok(Ok(n)) => {
                    conn.read_buf.extend_from_slice(&buf[..n]);
                    (Ok(n), !conn.write_buf.is_empty())
                }
                Ok(Err(e)) => (Err(format!("read error: {}", e)), false),
                Err(_) => (Ok(0), !conn.write_buf.is_empty()),
            }
        };

        match read_result {
            Ok(n) if n > 0 => {
                self.drain_read_buffer(id).await;
            }
            Ok(_) => {}
            Err(reason) => {
                self.disconnect(id, &reason).await;
                return;
            }
        }

        if has_write {
            self.flush_write(id).await;
        }
    }

    async fn flush_write(&mut self, id: u64) {
        let conn = match self.connections.get_mut(&id) {
            Some(c) => c,
            None => return,
        };
        if conn.write_buf.is_empty() {
            return;
        }
        let data = std::mem::take(&mut conn.write_buf);
        if let Err(e) = conn.stream.write_all(&data).await {
            eprintln!("[p2p] write failed to {}: {}", conn.addr, e);
            self.disconnect(id, "write error").await;
        }
    }

    async fn drain_read_buffer(&mut self, id: u64) {
        loop {
            let parsed = {
                let conn = match self.connections.get_mut(&id) {
                    Some(c) => c,
                    None => return,
                };
                if conn.read_buf.is_empty() {
                    return;
                }
                let mut cursor = std::io::Cursor::new(conn.read_buf.clone());
                match read_message(&mut cursor) {
                    Ok((cmd, payload)) => {
                        let consumed = cursor.position() as usize;
                        conn.read_buf.drain(..consumed);
                        Some((cmd, payload))
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return,
                    Err(e) => {
                        eprintln!("[p2p] framing error from {}: {}", conn.addr, e);
                        self.disconnect(id, "framing error").await;
                        return;
                    }
                }
            };

            if let Some((cmd, payload)) = parsed {
                self.handle_message(id, &cmd, &payload).await;
            } else {
                return;
            }
        }
    }

    async fn handle_message(&mut self, id: u64, cmd: &str, payload: &[u8]) {
        let msg = match Message::deserialize(cmd, payload) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[p2p] invalid message from {}: {}", id, e);
                self.punish(id, 10, "invalid message").await;
                return;
            }
        };

        if let Some(peer) = self.peer_manager.get_mut(id) {
            peer.last_recv = Instant::now();
        }

        match msg {
            Message::Version(v) => {
                let result = {
                    let peer = match self.peer_manager.get_mut(id) {
                        Some(p) => p,
                        None => return,
                    };
                    handshake::handle_version(peer, &v)
                };
                match result {
                    Ok(_) => {
                        self.queue_message(id, &Message::Verack);
                    }
                    Err(e) => {
                        self.disconnect(id, &format!("version rejected: {}", e))
                            .await;
                    }
                }
            }
            Message::Verack => {
                let result = {
                    let peer = match self.peer_manager.get_mut(id) {
                        Some(p) => p,
                        None => return,
                    };
                    handshake::handle_verack(peer)
                };
                match result {
                    Ok(true) => {
                        let (is_feeler, peer_addr) = match self.peer_manager.get(id) {
                            Some(p) => (p.conn_type == ConnectionType::Feeler, p.addr),
                            None => (false, "0.0.0.0:0".parse().unwrap()),
                        };

                        if is_feeler {
                            println!(
                                "[p2p] feeler connection to {} successful, disconnecting",
                                peer_addr
                            );
                            self.disconnect(id, "feeler complete").await;
                            return;
                        }

                        self.peer_manager
                            .push_event(PeerEvent::HandshakeComplete(id));
                    }
                    Ok(false) => {}
                    Err(e) => {
                        self.disconnect(id, &format!("verack rejected: {}", e))
                            .await;
                    }
                }
            }
            Message::Ping(nonce) => {
                self.queue_message(id, &Message::Pong(nonce));
            }
            Message::Pong(nonce) => {
                if let Some(peer) = self.peer_manager.get_mut(id) {
                    if !peer.mark_pong_received(nonce) {
                        peer.add_misbehavior(1);
                    }
                }
            }
            Message::GetAddr => {
                let addrs = self.peer_manager.connected_addrs();
                let net_addrs: Vec<_> = addrs
                    .into_iter()
                    .map(crate::p2p::message::NetAddr::from_socket)
                    .collect();
                if !net_addrs.is_empty() {
                    self.queue_message(id, &Message::Addr(net_addrs));
                }
            }
            Message::Addr(addrs) => {
                let mut count = 0;
                for na in addrs {
                    let sa = na.to_socket();
                    if sa.port() != crate::blockchain::network::default_port() {
                        continue;
                    }
                    if self.addr_book.add(sa) {
                        count += 1;
                    }
                }
                if count > 0 {
                    println!("[p2p] {} new addresses from {}", count, id);
                }
            }
            Message::Inv(items) => {
                // Rate limit inv messages per connection.
                {
                    let conn = match self.connections.get_mut(&id) {
                        Some(c) => c,
                        None => return,
                    };
                    if conn.last_reset.elapsed().as_secs() >= RATE_LIMIT_WINDOW_SECS {
                        conn.inv_count = 0;
                        conn.tx_count = 0;
                        conn.last_reset = Instant::now();
                    }
                    conn.inv_count += 1;
                    if conn.inv_count > MAX_INV_PER_SECOND {
                        // The borrow of `conn` ends here, allowing `self.punish` to be called.
                        self.punish(id, 20, "inv rate limit exceeded").await;
                        return;
                    }
                }

                let new_items = self.inv.filter_new(&items);
                if new_items.is_empty() {
                    return;
                }
                let mut to_request = Vec::new();
                let chain = self.chain.read().await;
                for inv in &new_items {
                    if inv.inv_type == InvType::Block {
                        if chain.chain.iter().any(|b| b.hash == inv.hash) {
                            self.inv.add_block(&inv.hash);
                            continue;
                        }
                        // Only request blocks if the window is not full.
                        if self.sync_state.request_block(&inv.hash) {
                            to_request.push(inv.clone());
                        }
                    } else if inv.inv_type == InvType::Transaction && !self.inv.has_tx(&inv.hash) {
                        let is_block_relay = self
                            .peer_manager
                            .get(id)
                            .map(|p| p.conn_type == ConnectionType::OutboundBlockRelayOnly)
                            .unwrap_or(false);

                        if !is_block_relay {
                            if chain.mempool.iter().any(|t| t.txid == inv.hash) {
                                self.inv.add_tx(&inv.hash);
                            } else {
                                to_request.push(inv.clone());
                            }
                        }
                    }
                }
                drop(chain);
                if !to_request.is_empty() {
                    self.queue_message(id, &Message::GetData(to_request));
                }
            }
            Message::GetData(items) => {
                let chain = self.chain.read().await;
                let mut to_send: Vec<Message> = Vec::new();
                for inv in items {
                    match inv.inv_type {
                        InvType::Block => {
                            if let Some(b) =
                                chain.chain.iter().find(|b| b.hash == inv.hash).cloned()
                            {
                                to_send.push(Message::Block(b));
                            }
                        }
                        InvType::Transaction => {
                            if let Some(tx) =
                                chain.mempool.iter().find(|t| t.txid == inv.hash).cloned()
                            {
                                to_send.push(Message::Tx(tx));
                            }
                        }
                    }
                }
                drop(chain);
                for msg in to_send {
                    self.queue_message(id, &msg);
                }
            }
            Message::GetHeaders(loc) => {
                let chain = self.chain.read().await;
                let headers = build_headers_response(&chain, &loc);
                drop(chain);
                self.queue_message(id, &Message::Headers(headers));
            }
            Message::Headers(headers) => {
                self.handle_headers(id, headers).await;
            }
            Message::Block(b) => {
                self.handle_block(id, b).await;
            }
            Message::Tx(tx) => {
                // Rate limit transaction messages per connection.
                {
                    let conn = match self.connections.get_mut(&id) {
                        Some(c) => c,
                        None => return,
                    };
                    if conn.last_reset.elapsed().as_secs() >= RATE_LIMIT_WINDOW_SECS {
                        conn.tx_count = 0;
                        conn.inv_count = 0;
                        conn.last_reset = Instant::now();
                    }
                    conn.tx_count += 1;
                    if conn.tx_count > MAX_TX_PER_SECOND {
                        // The borrow of `conn` ends here, allowing `self.punish` to be called.
                        self.punish(id, 20, "tx rate limit exceeded").await;
                        return;
                    }
                }

                let is_block_relay = self
                    .peer_manager
                    .get(id)
                    .map(|p| p.conn_type == ConnectionType::OutboundBlockRelayOnly)
                    .unwrap_or(false);
                if is_block_relay {
                    return;
                }

                if validation::is_special_tx(&tx) {
                    self.punish(id, 50, "special transaction from peer").await;
                    return;
                }

                let mut chain = self.chain.write().await;
                if chain.add_transaction(tx.clone()) {
                    self.inv.add_tx(&tx.txid);

                    let now = now_millis();
                    println!(
                        "[mempool] {} Accepted peer transaction {} from {} to {}",
                        format_datetime(now),
                        tx.txid,
                        tx.from,
                        tx.to
                    );

                    drop(chain);
                    self.broadcast_inv_to_full_relay(&[Inventory {
                        inv_type: InvType::Transaction,
                        hash: tx.txid.clone(),
                    }]);
                }
            }
            Message::Reject {
                command,
                code,
                reason,
            } => {
                eprintln!(
                    "[p2p] reject from {}: {} code={} reason={}",
                    id, command, code, reason
                );
            }
        }
    }

    async fn handle_headers(&mut self, id: u64, headers: Vec<Block>) {
        if headers.is_empty() {
            return;
        }

        // Verify that the batch size does not exceed the configured limit.
        if headers.len() > peers::HEADERS_PER_REQUEST as usize {
            self.punish(id, 20, "too many headers in one message").await;
            return;
        }

        let chain = self.chain.read().await;
        let first = &headers[0];

        let connects = chain
            .chain
            .iter()
            .any(|b| b.index + 1 == first.index && b.hash == first.previous_hash);
        if !connects {
            drop(chain);
            self.punish(id, 20, "headers do not connect to any known block")
                .await;
            return;
        }

        for window in headers.windows(2) {
            if window[1].index != window[0].index + 1 || window[1].previous_hash != window[0].hash {
                drop(chain);
                self.punish(id, 20, "headers are not sequential").await;
                return;
            }
        }

        let tip_index = chain.chain.last().map(|b| b.index).unwrap_or(0);
        let mut to_request = Vec::new();
        for h in &headers {
            if validation::rule_block_pow(h).is_err() {
                drop(chain);
                self.punish(id, 50, "invalid header proof of work").await;
                return;
            }
            if h.index > tip_index && !self.inv.has_block(&h.hash) {
                // Respect the moving download window.
                if self.sync_state.request_block(&h.hash) {
                    to_request.push(Inventory {
                        inv_type: InvType::Block,
                        hash: h.hash.clone(),
                    });
                }
            }
        }
        drop(chain);
        if !to_request.is_empty() {
            self.queue_message(id, &Message::GetData(to_request));
        }
    }

    async fn handle_block(&mut self, id: u64, block: Block) {
        // Mark the block as received in the sync state.
        self.sync_state.received_block(&block.hash);

        let mut chain = self.chain.write().await;
        match crate::p2p::sync::SyncState::accept_block(&mut chain, block.clone()) {
            Ok(_) => {
                self.inv.add_block(&block.hash);
                let _ = storage::save_chain(&chain);
                drop(chain);
                self.broadcast_inv_to_full_relay(&[Inventory {
                    inv_type: InvType::Block,
                    hash: block.hash.clone(),
                }]);

                // Immediately request the next batch of headers if we
                // are still behind. This prevents sync stalls at batch
                // boundaries.
                self.send_getheaders(id).await;

                println!("[p2p] block {} accepted from {}", block.index, id);
            }
            Err(e) => {
                drop(chain);
                if e.contains("already exists") {
                    self.inv.add_block(&block.hash);
                    return;
                }
                if e.contains("does not follow tip") {
                    println!(
                        "[p2p] block from {} does not follow tip, requesting headers",
                        id
                    );
                    self.send_getheaders(id).await;
                } else {
                    self.punish(id, 50, &format!("block rejected: {}", e)).await;
                }
            }
        }
    }

    fn broadcast_inv_to_full_relay(&mut self, items: &[Inventory]) {
        let full_relay_peers: Vec<u64> = self
            .peer_manager
            .all_peers()
            .iter()
            .filter(|p| {
                p.is_connected()
                    && (p.conn_type == ConnectionType::OutboundFullRelay
                        || p.conn_type == ConnectionType::Inbound)
            })
            .map(|p| p.id)
            .collect();

        if full_relay_peers.is_empty() {
            return;
        }
        let msg = Message::Inv(items.to_vec());
        for id in full_relay_peers {
            self.queue_message(id, &msg);
        }
    }

    async fn punish(&mut self, id: u64, _score: u32, reason: &str) {
        let peer_ip = match self.peer_manager.get(id) {
            Some(p) => p.addr.ip(),
            None => return,
        };
        self.banman.discourage(peer_ip);
        self.disconnect(id, reason).await;
    }

    async fn disconnect(&mut self, id: u64, reason: &str) {
        if let Some(mut conn) = self.connections.remove(&id) {
            let _ = conn.stream.shutdown().await;
        }
        self.peer_manager.remove(id, reason);
        println!("[p2p] disconnect {}: {}", id, reason);
    }

    async fn process_events(&mut self) {
        let events = self.peer_manager.drain_events();
        for ev in events {
            match ev {
                PeerEvent::HandshakeComplete(id) => {
                    println!("[p2p] handshake complete with {}", id);
                    self.send_getheaders(id).await;
                    self.queue_message(id, &Message::GetAddr);
                }
                PeerEvent::PingDue(id, nonce) => {
                    self.queue_message(id, &Message::Ping(nonce));
                }
                PeerEvent::Disconnected(id, reason) => {
                    self.connections.remove(&id);
                    println!("[p2p] peer {} disconnected: {}", id, reason);
                }
                PeerEvent::Banned(id, reason) => {
                    println!("[p2p] peer {} banned: {}", id, reason);
                }
                PeerEvent::Connected(_) => {}
            }
        }
    }

    async fn send_getheaders(&mut self, id: u64) {
        let chain = self.chain.read().await;
        let locator = build_locator(&chain);
        drop(chain);
        self.queue_message(id, &Message::GetHeaders(locator));
    }

    fn ip_to_asn(&self, ip: IpAddr) -> u32 {
        let ip_str = ip.to_string();
        for (cidr, asn) in &self.asmap {
            if cidr_matches(&ip_str, cidr) {
                return *asn;
            }
        }
        0
    }
}

fn cidr_matches(ip: &str, cidr: &str) -> bool {
    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        return false;
    }
    let network = parts[0];
    let prefix_len: u32 = parts[1].parse().unwrap_or(0);

    if prefix_len == 8 {
        ip.starts_with(&format!("{}.", network.split('.').next().unwrap_or("")))
    } else if prefix_len == 16 {
        let net_parts: Vec<&str> = network.split('.').collect();
        let ip_parts: Vec<&str> = ip.split('.').collect();
        if net_parts.len() >= 2 && ip_parts.len() >= 2 {
            net_parts[0] == ip_parts[0] && net_parts[1] == ip_parts[1]
        } else {
            false
        }
    } else if prefix_len == 24 {
        let net_parts: Vec<&str> = network.split('.').collect();
        let ip_parts: Vec<&str> = ip.split('.').collect();
        if net_parts.len() >= 3 && ip_parts.len() >= 3 {
            net_parts[0] == ip_parts[0]
                && net_parts[1] == ip_parts[1]
                && net_parts[2] == ip_parts[2]
        } else {
            false
        }
    } else {
        ip == network
    }
}

fn build_locator(chain: &Blockchain) -> BlockLocator {
    let mut hashes = Vec::new();
    let height = chain.height();
    let mut step = 1u64;
    let mut h = height;
    loop {
        if let Some(b) = chain.chain.get(h as usize) {
            hashes.push(b.hash.clone());
        }
        if h == 0 {
            break;
        }
        h = h.saturating_sub(step);
        if hashes.len() >= 10 {
            step = step.saturating_mul(2);
        }
        if hashes.len() >= 32 {
            break;
        }
    }
    BlockLocator {
        version: crate::p2p::message::PROTOCOL_VERSION,
        locator_hashes: hashes,
        hash_stop: "0".repeat(64),
    }
}

fn build_headers_response(chain: &Blockchain, loc: &BlockLocator) -> Vec<Block> {
    let mut start = 0u64;
    for h in &loc.locator_hashes {
        if let Some(b) = chain.chain.iter().find(|b| &b.hash == h) {
            start = b.index + 1;
            break;
        }
    }
    let end = (start + peers::HEADERS_PER_REQUEST).min(chain.chain.len() as u64);
    chain.chain[start as usize..end as usize]
        .iter()
        .map(|b| {
            let mut hb = b.clone();
            hb.transactions.clear();
            hb
        })
        .collect()
}

pub fn spawn(
    config: NetworkConfig,
    chain: Arc<RwLock<Blockchain>>,
    peer_registry: PeerRegistry,
    command_rx: mpsc::Receiver<NetworkCommand>,
) -> tokio::task::JoinHandle<Result<(), String>> {
    tokio::spawn(async move {
        let mut net = Network::new(config, chain, peer_registry, command_rx);
        net.run().await
    })
}
