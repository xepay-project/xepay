// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! RPC server and client for XEPAY.

use crate::blockchain::{self, Blockchain};
use crate::config::Config;
use crate::consensus;
use crate::consensus::spec::Network;
use crate::datadir;
use crate::hdwallet::{AddressType, HdWallet, HdAddressEntry};
use crate::p2p::network::{NetworkCommand, PeerRegistry};
use crate::pow;
use crate::storage;
use crate::transaction::Transaction;
use crate::utils::{format_datetime, format_xepc, now_millis, parse_xepc};
use crate::validation;
use crate::wallet::{Wallet, WalletEntry};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

// Default RPC address for Mainnet. This is the port the node will listen on for RPC requests.
pub const DEFAULT_RPC_ADDR: &str = "127.0.0.1:7554";

// Default fee for transactions in XEPC (0.001 XEPC).
pub const DEFAULT_FEE_XEPC: u64 = 100_000;

// Number of ALD (smallest unit) in one XEPC.
const ALD_PER_XEPC: u64 = 100_000_000;

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcRequest {
    pub method: String,
    pub params: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcResponse {
    pub ok: bool,
    pub result: Value,
    pub error: String,
}

impl RpcResponse {
    // Create a successful RPC response.
    pub fn ok(result: Value) -> Self {
        RpcResponse { ok: true, result, error: String::new() }
    }

    // Create an error RPC response.
    pub fn err(msg: &str) -> Self {
        RpcResponse { ok: false, result: Value::Null, error: msg.to_string() }
    }
}

// Retrieve the existing cookie from disk or generate a new one if it doesn't exist.
fn get_or_create_cookie() -> std::io::Result<String> {
    let path = datadir::cookie_path();
    if let Ok(data) = std::fs::read_to_string(&path) {
        let trimmed = data.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }

    let random = crate::utils::random_hex(32);
    let cookie = format!("__cookie__:{}", random);

    datadir::ensure_dirs()?;
    std::fs::write(&path, &cookie)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&path, perms)?;
    }

    Ok(cookie)
}

// Read the cookie file for client authentication.
fn read_cookie_for_client() -> std::io::Result<String> {
    let path = datadir::cookie_path();
    let data = std::fs::read_to_string(&path)?;
    Ok(data.trim().to_string())
}

// Send an RPC request to the local node.
pub fn call(method: &str, params: Vec<Value>) -> std::io::Result<RpcResponse> {
    let cfg = Config::load_or_default();
    call_at(&cfg, method, params)
}

// Send an RPC request to a specific configuration.
pub fn call_at(cfg: &Config, method: &str, params: Vec<Value>) -> std::io::Result<RpcResponse> {
    let mut stream = TcpStream::connect(&cfg.rpc_addr)?;

    let auth = match read_cookie_for_client() {
        Ok(c) => c,
        Err(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Cookie file not found. Is the node running?",
            ))
        }
    };

    let req = RpcRequest { method: method.to_string(), params };
    let body = serde_json::to_string(&req).unwrap();

    let request = format!(
        "POST / HTTP/1.1\r\nHost: {}\r\nAuthorization: {}\r\nContent-Length: {}\r\n\r\n{}",
        cfg.rpc_addr, auth, body.len(), body
    );

    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();

    loop {
        line.clear();
        reader.read_line(&mut line)?;
        if line == "\r\n" || line.is_empty() {
            break;
        }
    }

    let mut buf = String::new();
    reader.read_line(&mut buf)?;

    let resp: RpcResponse = serde_json::from_str(buf.trim()).map_err(std::io::Error::other)?;
    Ok(resp)
}

// Convert an amount in ALD to a human-readable XEPC string.
fn to_coin(amount: u64) -> String {
    format_xepc(amount)
}

// Convert an amount in ALD to a raw string.
fn to_aldian(amount: u64) -> String {
    amount.to_string()
}

// Parse a human-readable XEPC string into ALD.
fn parse_amount(s: &str) -> Result<u64, String> {
    parse_xepc(s)
}

// Select the best source address from the wallet to cover the required amount.
fn select_source_address(
    wallet: &Wallet,
    chain: &Blockchain,
    to: &str,
    need: u64,
    explicit_from: Option<&str>,
) -> Result<String, String> {
    let current_height = chain.height();

    // If an explicit source is provided, verify it exists and has enough balance.
    if let Some(from) = explicit_from {
        if from == to {
            return Err("sender and recipient addresses are identical".into());
        }
        if wallet.get_entry(from).is_none() {
            return Err(format!("from address {} not found in wallet", from));
        }
        let bal = chain.mature_balance_at(from, current_height);
        if bal >= need {
            return Ok(from.to_string());
        }
        eprintln!(
            "[rpc] WARNING: explicit from {} has insufficient mature balance ({} ALD < {} ALD needed), falling back to auto-select",
            from, bal, need
        );
    }

    // Auto-select the address with the highest mature balance that can cover the amount.
    let mut best: Option<(String, u64)> = None;
    for entry in &wallet.entries {
        if entry.address == to {
            continue; // Skip the destination address.
        }
        let bal = chain.mature_balance_at(&entry.address, current_height);
        if bal < need {
            continue;
        }
        match &best {
            Some((_, b)) if *b >= bal => {}
            _ => best = Some((entry.address.clone(), bal)),
        }
    }

    if let Some((addr, _)) = best {
        return Ok(addr);
    }

    // Construct a detailed error message if no address can cover the amount.
    let mut details: Vec<String> = Vec::new();
    let mut total_available: u64 = 0;
    for entry in &wallet.entries {
        let bal = chain.mature_balance_at(&entry.address, current_height);
        total_available = total_available.saturating_add(bal);
        let marker = if entry.address == to { " (destination)" } else { "" };
        details.push(format!("{} = {} ALD{}", entry.address, bal, marker));
    }
    if details.is_empty() {
        return Err("wallet has no addresses".into());
    }
    Err(format!(
        "insufficient mature balance: required {} ALD, no single address can cover it. Wallet addresses: {}. Total mature: {} ALD. Deposit funds or omit the `from` argument to auto-select.",
        need,
        details.join(", "),
        total_available
    ))
}

struct RpcState {
    chain: Arc<RwLock<Blockchain>>,
    network: Network,
    shutdown: Arc<AtomicBool>,
    interrupt: Arc<AtomicBool>,
    cookie: String,
    wallet_password: Arc<RwLock<Option<String>>>,
    peer_registry: PeerRegistry,
    network_cmd_tx: mpsc::Sender<NetworkCommand>,
}

impl RpcState {
    // Return the consensus fingerprint for the active network.
    fn fingerprint(&self) -> String {
        consensus::consensus_fingerprint_for(self.network)
    }
}

pub struct RpcServer {
    state: Arc<RpcState>,
    rpc_addr: String,
}

impl RpcServer {
    // Create a new RPC server instance.
    pub fn new(
        chain: Arc<RwLock<Blockchain>>,
        cfg: &Config,
        peer_registry: PeerRegistry,
        network_cmd_tx: mpsc::Sender<NetworkCommand>,
        network: Network,
    ) -> Self {
        let is_local = cfg.rpc_addr.starts_with("127.0.0.1") || cfg.rpc_addr.starts_with("localhost");
        if !is_local {
            panic!(
                "SECURITY ERROR: RPC is bound to {} but cookie authentication is only safe for local connections. Use an SSH tunnel or a reverse proxy with TLS for remote access.",
                cfg.rpc_addr
            );
        }

        let cookie = match get_or_create_cookie() {
            Ok(c) => c,
            Err(e) => panic!("Failed to create RPC cookie: {}", e),
        };

        let state = Arc::new(RpcState {
            chain,
            network,
            shutdown: Arc::new(AtomicBool::new(false)),
            interrupt: Arc::new(AtomicBool::new(false)),
            cookie,
            wallet_password: Arc::new(RwLock::new(None)),
            peer_registry,
            network_cmd_tx,
        });
        RpcServer { state, rpc_addr: cfg.rpc_addr.clone() }
    }

    // Return the shutdown flag to allow external control.
    pub fn shutdown_flag(&self) -> Arc<AtomicBool> {
        self.state.shutdown.clone()
    }

    // Start the RPC server loop.
    pub fn listen(&mut self) -> std::io::Result<()> {
        let listener = TcpListener::bind(&self.rpc_addr)?;
        let _ = listener.set_nonblocking(true);
        println!("[rpc] listening on {}", self.rpc_addr);
        println!("[rpc] authentication: cookie file at {}", datadir::as_str(&datadir::cookie_path()));

        loop {
            if self.state.shutdown.load(Ordering::SeqCst) {
                break;
            }

            match listener.accept() {
                Ok((stream, _)) => {
                    let state = self.state.clone();
                    std::thread::spawn(move || {
                        let mut handler = RpcHandler { state };
                        if let Err(e) = handler.handle(stream) {
                            eprintln!("[rpc] connection error: {}", e);
                        }
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => eprintln!("[rpc] accept failed: {}", e),
            }
        }
        Ok(())
    }
}

struct RpcHandler {
    state: Arc<RpcState>,
}

impl RpcHandler {
    // Parse the HTTP request and dispatch it to the correct method.
    fn handle(&mut self, stream: TcpStream) -> std::io::Result<()> {
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut line = String::new();

        let mut auth_header = String::new();
        let mut content_length = 0;

        loop {
            line.clear();
            reader.read_line(&mut line)?;
            if line == "\r\n" || line.is_empty() {
                break;
            }
            if line.starts_with("Authorization:") {
                auth_header = line.trim_start_matches("Authorization:").trim().to_string();
            }
            if line.starts_with("Content-Length:") {
                content_length = line
                    .trim_start_matches("Content-Length:")
                    .trim()
                    .parse()
                    .unwrap_or(0);
            }
        }

        if auth_header != self.state.cookie {
            let resp = RpcResponse::err("RPC authentication failed");
            write_http_response(stream, 401, &resp)?;
            return Ok(());
        }

        let mut buf = vec![0; content_length];
        reader.read_exact(&mut buf)?;
        let body = String::from_utf8_lossy(&buf);

        let req: RpcRequest = match serde_json::from_str(&body) {
            Ok(r) => r,
            Err(_) => {
                let resp = RpcResponse::err("invalid request");
                write_http_response(stream, 400, &resp)?;
                return Ok(());
            }
        };

        let resp = self.dispatch(&req);
        write_http_response(stream, 200, &resp)
    }

    // Dispatch the method to the corresponding handler function.
    fn dispatch(&mut self, req: &RpcRequest) -> RpcResponse {
        match req.method.as_str() {
            "getinfo" => self.m_getinfo(),
            "getsupplyinfo" => self.m_getsupplyinfo(),
            "getblockchaininfo" => self.m_getblockchaininfo(),
            "getnetworkinfo" => self.m_getnetworkinfo(),
            "getpeerinfo" => self.m_getpeerinfo(),
            "getcheckpoints" => m_getcheckpoints(),
            "getblocktemplate" => self.m_getblocktemplate(),
            "submitblock" => self.m_submitblock(req),
            "getbalance" => self.m_getbalance(req),
            "getblockhash" => self.m_getblockhash(req),
            "getblock" => self.m_getblock(req),
            "getblockheader" => self.m_getblockheader(req),
            "getblockcount" => self.m_getblockcount(),
            "getbestblockhash" => self.m_getbestblockhash(),
            "getdifficulty" => self.m_getdifficulty(),
            "getchaintips" => self.m_getchaintips(),
            "verifychain" => self.m_verifychain(),
            "getgenesishash" => self.m_getgenesishash(),
            "getgenesis" => self.m_getgenesis(),
            "getlistwallet" => m_getlistwallet(self.state.chain.clone()),
            "dumpprivkey" => m_dumpprivkey(req),
            "dumpwallet" => m_dumpwallet(),
            "migratewallet" => self.m_migratewallet(),
            "importprivkey" => m_importprivkey(req),
            "getcreatewallet" => self.m_getcreatewallet(),
            "getcreatewalletbech32" => self.m_getcreatewalletbech32(),
            "encryptwallet" => m_encryptwallet(req),
            "walletpassphrase" => self.m_walletpassphrase(req),
            "getaddressinfo" => m_getaddressinfo(req),
            "setlabel" => m_setlabel(req),
            "backupwallet" => m_backupwallet(req),
            "getaddtowallet" => m_getaddtowallet(),
            "getaddtoaddress" => m_getaddtoaddress(req),
            "generatetoaddress" => self.m_generatetoaddress(req),
            "sendtoaddress" => self.m_sendtoaddress(req),
            "stop" => self.m_stop(),
            "getrawtransaction" => self.m_getrawtransaction(req),
            "decoderawtransaction" => self.m_decoderawtransaction(req),
            "testmempoolaccept" => self.m_testmempoolaccept(req),
            "getmempoolinfo" => self.m_getmempoolinfo(),
            "validatemempool" => self.m_validatemempool(),
            "getnonce" => self.m_getnonce(req),
            
            // HD Wallet methods
            "createhdwallet" => self.m_createhdwallet(req),
            "loadhdwallet" => self.m_loadhdwallet(),
            "restorehdwallet" => self.m_restorehdwallet(req),
            "dumphdmnemonic" => self.m_dumphdmnemonic(),
            "derivehdaddress" => self.m_derivehdaddress(req),
            "listhdaddresses" => self.m_listhdaddresses(),

            _ => RpcResponse::err("unknown method"),
        }
    }

    // Get network information.
    fn m_getnetworkinfo(&self) -> RpcResponse {
        let reg = self.state.peer_registry.lock().unwrap();
        let connected = reg.iter().filter(|p| p.state == "Connected").count();
        RpcResponse::ok(json!({
            "network": self.state.network.name(),
            "magic": self.state.network.magic_hex(),
            "default_port": self.state.network.default_port(),
            "prune_after_height": blockchain::network::PRUNE_AFTER_HEIGHT,
            "assumed_blockchain_size_mb": blockchain::network::ASSUMED_BLOCKCHAIN_SIZE,
            "assumed_chain_state_size_mb": blockchain::network::ASSUMED_CHAIN_STATE_SIZE,
            "p2p_enabled": true,
            "connection_count": connected,
            "min_relay_fee_per_byte": blockchain::MIN_RELAY_FEE_PER_BYTE,
        }))
    }

    // Get information about connected peers.
    fn m_getpeerinfo(&self) -> RpcResponse {
        let reg = self.state.peer_registry.lock().unwrap();
        let peers: Vec<Value> = reg.iter().map(|p| json!({
            "id": p.id,
            "addr": p.addr,
            "outbound": p.outbound,
            "state": p.state,
        })).collect();
        RpcResponse::ok(json!(peers))
    }

    // Stop the node and save the chain data.
    fn m_stop(&mut self) -> RpcResponse {
        println!();
        println!("[node] Stopping the running node. Please wait...");
        println!("[node] Saving blockchain data...");

        self.state.interrupt.store(true, Ordering::SeqCst);
        self.state.shutdown.store(true, Ordering::SeqCst);

        let mut chain = self.state.chain.blocking_write();
        
        if let Err(e) = chain.state.flush() {
            println!("[node] WARNING: Failed to flush state db: {}", e);
        }

        match storage::save_chain(&chain) {
            Ok(_) => {
                println!("[node]   blocks     : {}", chain.chain.len());
                println!(
                    "[node]   chainstate : height={} difficulty/nbits={:#010x}",
                    chain.height(),
                    chain.nbits
                );
                println!("[node]   mempool    : {} transactions", chain.mempool.len());
                println!("[node]   location   : {}", datadir::as_str(&datadir::datadir()));
                println!("[node] Blockchain data saved successfully.");
            }
            Err(e) => {
                println!("[node] FAILED to save blockchain: {}", e);
                return RpcResponse::err(&format!("failed to save chain: {}", e));
            }
        }
        drop(chain);

        println!("[node] Closing P2P and RPC connections...");
        println!("[node] Shutdown successful.");
        println!();

        RpcResponse::ok(json!({
            "stopping": true,
            "message": "Stopping the running node. Please wait...",
            "saved": true,
            "status": "Shutdown successful",
        }))
    }

    // Get general node information.
    fn m_getinfo(&self) -> RpcResponse {
        let c = self.state.chain.blocking_read();
        RpcResponse::ok(json!({
            "coin_name": consensus::coin_name(),
            "symbol": consensus::symbol(),
            "unit_name": consensus::UNIT_NAME,
            "unit_symbol": consensus::UNIT_SYMBOL,
            "ald_per_xepc": consensus::ALD_PER_XEPC,
            "version": consensus::version(),
            "protocol_version": consensus::protocol_version(),
            "network": self.state.network.name(),
            "hash_algo": consensus::hash_algo(),
            "signature_algo": consensus::signature_algo(),
            "height": c.height(),
            "chain_len": c.chain.len(),
            "nbits": format!("{:#010x}", c.nbits),
            "difficulty": pow::nbits_to_difficulty(c.nbits),
            "pow_limit": consensus::pow_limit(),
            "circulating_xepc": to_coin(c.circulating_supply()),
            "circulating_ald": to_aldian(c.circulating_supply()),
            "max_supply_xepc": to_coin(consensus::max_supply()),
            "max_supply_ald": to_aldian(consensus::max_supply()),
            "era": c.current_era(),
            "subsidy_halving_interval": consensus::subsidy_halving_interval(),
            "blocks_until_halving": c.blocks_until_halving(),
            "pow_target_spacing_secs": consensus::pow_target_spacing_secs(),
            "pow_target_timespan_secs": consensus::pow_target_timespan_secs(),
            "miner_confirmation_window": consensus::miner_confirmation_window(),
            "difficulty_adjust_interval": consensus::difficulty_adjust_interval(),
            "mempool_len": c.mempool.len(),
            "mempool_size_bytes": c.mempool_size_bytes(),
            "min_relay_fee_per_byte": blockchain::MIN_RELAY_FEE_PER_BYTE,
            "checkpoints_count": blockchain::checkpoint::CHECKPOINTS.len(),
            "checkpoints_highest": blockchain::checkpoint::highest(),
            "consensus_fingerprint": self.state.fingerprint(),
            "datadir": datadir::as_str(&datadir::datadir()),
            "total_chainwork": c.total_chainwork().to_string(),
        }))
    }

    // Get supply and inflation information.
    fn m_getsupplyinfo(&self) -> RpcResponse {
        let c = self.state.chain.blocking_read();
        let circulating = c.circulating_supply();
        let max_supply = consensus::max_supply();
        let remaining = max_supply.saturating_sub(circulating);
        
        let percent_mined = if max_supply > 0 {
            (circulating as f64 / max_supply as f64) * 100.0
        } else {
            0.0
        };

        let current_height = c.height();
        let halving_interval = consensus::subsidy_halving_interval();
        let current_era = current_height / halving_interval;
        let max_halvings = consensus::max_halvings() as u64;
        
        let mut remaining_blocks: u64 = 0;
        let mut era_reward = consensus::initial_reward();
        
        for era in current_era..max_halvings {
            let blocks_in_era = if era == current_era {
                halving_interval - (current_height % halving_interval)
            } else {
                halving_interval
            };
            remaining_blocks += blocks_in_era;
            era_reward >>= 1;
            if era_reward == 0 { break; }
        }

        RpcResponse::ok(json!({
            "max_supply_xepc": to_coin(max_supply),
            "max_supply_ald": to_aldian(max_supply),
            "circulating_supply_xepc": to_coin(circulating),
            "circulating_supply_ald": to_aldian(circulating),
            "remaining_supply_xepc": to_coin(remaining),
            "remaining_supply_ald": to_aldian(remaining),
            "percent_mined": format!("{:.4}%", percent_mined),
            "current_era": current_era,
            "remaining_blocks_estimate": remaining_blocks,
            "is_max_supply_reached": remaining == 0,
        }))
    }

    // Get detailed blockchain information.
    fn m_getblockchaininfo(&self) -> RpcResponse {
        let c = self.state.chain.blocking_read();
        let tip = c.chain.last().unwrap();
        RpcResponse::ok(json!({
            "chain": self.state.network.name(),
            "blocks": c.height(),
            "headers": c.height(),
            "bestblockhash": tip.hash,
            "nbits": format!("{:#010x}", c.nbits),
            "difficulty": pow::nbits_to_difficulty(c.nbits),
            "pow_limit": consensus::pow_limit(),
            "mediantime": tip.timestamp,
            "verificationprogress": 1.0,
            "initialblockdownload": false,
            "pruned": false,
            "checkpoints_count": blockchain::checkpoint::CHECKPOINTS.len(),
            "checkpoints_highest": blockchain::checkpoint::highest(),
            "mempool_len": c.mempool.len(),
            "mempool_size_bytes": c.mempool_size_bytes(),
            "min_relay_fee_per_byte": blockchain::MIN_RELAY_FEE_PER_BYTE,
            "circulating_xepc": to_coin(c.circulating_supply()),
            "circulating_ald": to_aldian(c.circulating_supply()),
            "max_supply_xepc": to_coin(consensus::max_supply()),
            "max_supply_ald": to_aldian(consensus::max_supply()),
            "era": c.current_era(),
            "subsidy_halving_interval": consensus::subsidy_halving_interval(),
            "blocks_until_halving": c.blocks_until_halving(),
            "difficulty_adjust_interval": consensus::difficulty_adjust_interval(),
            "consensus_fingerprint": self.state.fingerprint(),
            "datadir": datadir::as_str(&datadir::datadir()),
            "total_chainwork": c.total_chainwork().to_string(),
        }))
    }

    // Get a block template for miners.
    fn m_getblocktemplate(&self) -> RpcResponse {
        let c = self.state.chain.blocking_read();
        let next_height = c.height() + 1;
        let subsidy = Blockchain::calculate_reward(next_height);
        let prev = c.chain.last().unwrap();
        let prev_hash = prev.hash.clone();
        let previous_chainwork = prev.chainwork;
        let total_fees: u64 = c.mempool.iter().map(|tx| tx.fee).sum();
        let reward = subsidy.saturating_add(total_fees);
        RpcResponse::ok(json!({
            "height": next_height,
            "previous_hash": prev_hash,
            "previous_chainwork": previous_chainwork.to_string(),
            "nbits": c.nbits,
            "reward_xepc": to_coin(reward),
            "reward_ald": to_aldian(reward),
            "subsidy_xepc": to_coin(subsidy),
            "subsidy_ald": to_aldian(subsidy),
            "total_fees_xepc": to_coin(total_fees),
            "total_fees_ald": to_aldian(total_fees),
            "mempool": c.mempool,
            "min_relay_fee_per_byte": blockchain::MIN_RELAY_FEE_PER_BYTE,
        }))
    }

    // Submit a mined block to the node.
    fn m_submitblock(&mut self, req: &RpcRequest) -> RpcResponse {
        if req.params.is_empty() {
            return RpcResponse::err("submitblock requires a block parameter");
        }
        let block: crate::block::Block = match serde_json::from_value(req.params[0].clone()) {
            Ok(b) => b,
            Err(e) => return RpcResponse::err(&format!("invalid block: {}", e)),
        };

        let mut chain = self.state.chain.blocking_write();
        let prev = chain.chain.last().unwrap().clone();

        if block.previous_hash != prev.hash {
            if chain.chain.iter().any(|b| b.hash == block.previous_hash) {
                println!("[rpc] submitblock: potential reorg detected, calling handle_reorg");
                let result = crate::p2p::sync::SyncState::handle_reorg(&mut chain, block.clone());
                drop(chain);
                return match result {
                    Ok(_) => {
                        let tx = self.state.network_cmd_tx.clone();
                        let hash = block.hash.clone();
                        let _ = tx.blocking_send(NetworkCommand::BroadcastBlock(hash));
                        RpcResponse::ok(json!({
                            "accepted": true,
                            "height": block.index,
                            "hash": block.hash,
                            "reorg": true,
                        }))
                    }
                    Err(e) => RpcResponse::err(&format!("reorg failed: {}", e)),
                };
            } else {
                return RpcResponse::err("block rejected: parent not found (orphan)");
            }
        }

        if let Err(e) = validation::validate_block(&mut chain, &prev, &block) {
            return RpcResponse::err(&format!("block rejected: {}", e));
        }

        for tx in &block.transactions {
            if let Err(e) = chain.apply_transaction(tx) {
                return RpcResponse::err(&format!("failed to apply tx: {}", e));
            }
        }
        if let Err(e) = chain.credit_reward(&block.mined_for, block.reward, block.index) {
            return RpcResponse::err(&format!("failed to credit reward: {}", e));
        }

        chain.chain.push(block.clone());
        chain.adjust_difficulty();
        chain.mempool.clear();
        chain.mempool_timestamps.clear();

        block.log_add_to_wallet();

        if let Err(e) = storage::save_chain(&chain) {
            return RpcResponse::err(&format!("failed to save chain: {}", e));
        }

        drop(chain);

        let tx = self.state.network_cmd_tx.clone();
        let hash = block.hash.clone();
        let _ = tx.blocking_send(NetworkCommand::BroadcastBlock(hash));

        RpcResponse::ok(json!({
            "accepted": true,
            "height": block.index,
            "hash": block.hash,
            "txid": block.coinbase_txid,
            "merkle_root": block.merkle_root,
            "mined_at": block.mined_at,
            "mined_for": block.mined_for,
            "nbits": block.nbits,
        }))
    }

    // Get the balance of an address or the entire wallet.
    fn m_getbalance(&self, req: &RpcRequest) -> RpcResponse {
        if let Some(addr) = req.params.first().and_then(|v| v.as_str()) {
            let chain = self.state.chain.blocking_read();
            if !validation::is_valid_address(addr, &chain) {
                return RpcResponse::err("invalid address format");
            }
            let height = chain.height();
            let balance = chain.balance_of(addr);
            let mature = chain.mature_balance_at(addr, height);
            let immature = chain.immature_balance_of(addr);
            let nonce = chain.nonce_of(addr);
            drop(chain);
            return RpcResponse::ok(json!({
                "address": addr,
                "balance_xepc": to_coin(balance),
                "balance_ald": to_aldian(balance),
                "mature_balance_xepc": to_coin(mature),
                "mature_balance_ald": to_aldian(mature),
                "immature_balance_xepc": to_coin(immature),
                "immature_balance_ald": to_aldian(immature),
                "nonce": nonce,
            }));
        }

        let w = match Wallet::load_default() {
            Ok(w) => w,
            Err(_) => return RpcResponse::err("wallet not found"),
        };
        let chain = self.state.chain.blocking_read();
        let mut total: u64 = 0;
        for entry in &w.entries {
            total = total.saturating_add(chain.balance_of(&entry.address));
        }
        RpcResponse::ok(json!({
            "wallet_total_balance_xepc": to_coin(total),
            "wallet_total_balance_ald": to_aldian(total),
            "addresses": w.entries.len(),
            "blockchain_height": chain.height(),
        }))
    }

    // Get the next expected nonce for an address.
    fn m_getnonce(&self, req: &RpcRequest) -> RpcResponse {
        let addr = match req.params.first().and_then(|v| v.as_str()) {
            Some(a) => a.to_string(),
            None => match Wallet::load_default() {
                Ok(w) => match w.default_address() {
                    Some(e) => e.address.clone(),
                    None => return RpcResponse::err("wallet has no addresses"),
                },
                Err(_) => return RpcResponse::err("wallet not found"),
            },
        };

        let chain = self.state.chain.blocking_read();
        if !validation::is_valid_address(&addr, &chain) {
            return RpcResponse::err("invalid address format");
        }
        drop(chain);

        let chain = self.state.chain.blocking_read();
        RpcResponse::ok(json!({
            "address": addr,
            "nonce": chain.nonce_of(&addr),
            "next_nonce": chain.next_nonce_for(&addr),
        }))
    }

    // Get the hash of a block at a specific height.
    fn m_getblockhash(&self, req: &RpcRequest) -> RpcResponse {
        let height = match req.params.first().and_then(|v| v.as_u64()) {
            Some(h) => h,
            None => return RpcResponse::err("getblockhash requires a height"),
        };
        let chain = self.state.chain.blocking_read();
        if height as usize >= chain.chain.len() {
            return RpcResponse::err("height out of range");
        }
        RpcResponse::ok(json!(chain.chain[height as usize].hash))
    }

    // Get detailed information about a block by its hash.
    fn m_getblock(&self, req: &RpcRequest) -> RpcResponse {
        let hash = match req.params.first().and_then(|v| v.as_str()) {
            Some(h) => h,
            None => return RpcResponse::err("getblock requires a hash"),
        };
        let chain = self.state.chain.blocking_read();
        let block = match chain.chain.iter().find(|b| b.hash == hash) {
            Some(b) => b,
            None => return RpcResponse::err("block not found"),
        };
        RpcResponse::ok(json!({
            "nversion": block.nversion,
            "index": block.index,
            "hash": block.hash,
            "previous_hash": block.previous_hash,
            "merkle_root": block.merkle_root,
            "state_root": block.state_root,
            "timestamp": block.timestamp,
            "mined_at": block.mined_at,
            "mined_for": block.mined_for,
            "nonce": block.nonce,
            "nbits": block.nbits,
            "reward_xepc": to_coin(block.reward),
            "reward_ald": to_aldian(block.reward),
            "subsidy_xepc": to_coin(block.subsidy),
            "subsidy_ald": to_aldian(block.subsidy),
            "total_fees_xepc": to_coin(block.total_fees),
            "total_fees_ald": to_aldian(block.total_fees),
            "coinbase_txid": block.coinbase_txid,
            "blockchain_size": block.blockchain_size,
            "genesis_output_script": block.genesis_output_script,
            "miner_pubkey": block.miner_pubkey,
            "miner_signature": block.miner_signature,
            "chainwork": block.chainwork.to_string(),
            "transactions": block.transactions,
        }))
    }

    // Get the header of a block by its hash.
    fn m_getblockheader(&self, req: &RpcRequest) -> RpcResponse {
        let hash = match req.params.first().and_then(|v| v.as_str()) {
            Some(h) => h,
            None => return RpcResponse::err("getblockheader requires a hash"),
        };

        let chain = self.state.chain.blocking_read();
        let block = match chain.chain.iter().find(|b| b.hash == hash) {
            Some(b) => b,
            None => return RpcResponse::err("block not found"),
        };

        RpcResponse::ok(json!({
            "hash": block.hash,
            "confirmations": chain.height() - block.index + 1,
            "height": block.index,
            "version": block.nversion,
            "merkleroot": block.merkle_root,
            "state_root": block.state_root,
            "time": block.timestamp,
            "mediantime": block.timestamp,
            "nonce": block.nonce,
            "bits": block.nbits,
            "previousblockhash": block.previous_hash,
            "chainwork": block.chainwork.to_string(),
            "nextblockhash": if block.index + 1 < chain.chain.len() as u64 {
                Some(chain.chain[(block.index + 1) as usize].hash.clone())
            } else {
                None
            }
        }))
    }

    // Get the current block count (height).
    fn m_getblockcount(&self) -> RpcResponse {
        let chain = self.state.chain.blocking_read();
        RpcResponse::ok(json!(chain.height()))
    }

    // Get the hash of the best (latest) block.
    fn m_getbestblockhash(&self) -> RpcResponse {
        let chain = self.state.chain.blocking_read();
        RpcResponse::ok(json!(chain.chain.last().unwrap().hash))
    }

    // Get the current difficulty.
    fn m_getdifficulty(&self) -> RpcResponse {
        let chain = self.state.chain.blocking_read();
        let diff = pow::nbits_to_difficulty(chain.nbits);
        RpcResponse::ok(json!(diff))
    }

    // Get information about chain tips.
    fn m_getchaintips(&self) -> RpcResponse {
        let chain = self.state.chain.blocking_read();
        let tip = chain.chain.last().unwrap();

        RpcResponse::ok(json!([{
            "height": chain.height(),
            "hash": tip.hash,
            "branchlen": 0,
            "status": "active",
            "chainwork": tip.chainwork.to_string(),
        }]))
    }

    // Get the hash of the genesis block.
    fn m_getgenesishash(&self) -> RpcResponse {
        let chain = self.state.chain.blocking_read();
        match chain.chain.first() {
            Some(g) => RpcResponse::ok(json!(g.hash)),
            None => RpcResponse::err("chain is empty"),
        }
    }

    // Get detailed information about the genesis block.
    fn m_getgenesis(&self) -> RpcResponse {
        let chain = self.state.chain.blocking_read();
        match chain.chain.first() {
            Some(g) => RpcResponse::ok(json!({
                "nversion": g.nversion,
                "hash": g.hash,
                "index": g.index,
                "timestamp": g.timestamp,
                "mined_at": g.mined_at,
                "previous_hash": g.previous_hash,
                "merkle_root": g.merkle_root,
                "state_root": g.state_root,
                "nonce": g.nonce,
                "nbits": g.nbits,
                "reward_xepc": to_coin(g.reward),
                "reward_ald": to_aldian(g.reward),
                "subsidy_xepc": to_coin(g.subsidy),
                "subsidy_ald": to_aldian(g.subsidy),
                "total_fees_xepc": to_coin(g.total_fees),
                "total_fees_ald": to_aldian(g.total_fees),
                "coinbase_txid": g.coinbase_txid,
                "blockchain_size": g.blockchain_size,
                "genesis_output_script": g.genesis_output_script,
                "network": self.state.network.name(),
                "consensus_fingerprint": self.state.fingerprint(),
                "transactions": g.transactions,
                "chainwork": g.chainwork.to_string(),
            })),
            None => RpcResponse::err("chain is empty"),
        }
    }

    // Verify the integrity of the entire chain.
    fn m_verifychain(&self) -> RpcResponse {
        let mut chain = self.state.chain.blocking_write();
        match validation::validate_chain(&mut chain) {
            Ok(_) => RpcResponse::ok(json!({
                "valid": true,
                "blocks_checked": chain.chain.len(),
                "height": chain.height(),
                "nbits": format!("{:#010x}", chain.nbits),
                "pow_limit": consensus::pow_limit(),
                "signature_algo": consensus::signature_algo(),
                "checkpoints_count": blockchain::checkpoint::CHECKPOINTS.len(),
                "consensus_fingerprint": self.state.fingerprint(),
            })),
            Err(e) => RpcResponse::ok(json!({
                "valid": false,
                "code": e.code,
                "message": e.message,
                "blocks_checked": chain.chain.len(),
                "height": chain.height(),
            })),
        }
    }

    // Get details of a raw transaction by its txid.
    fn m_getrawtransaction(&self, req: &RpcRequest) -> RpcResponse {
        let txid = match req.params.first().and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => return RpcResponse::err("getrawtransaction requires a txid"),
        };

        let chain = self.state.chain.blocking_read();
        for block in &chain.chain {
            for tx in &block.transactions {
                if tx.txid == txid {
                    return RpcResponse::ok(json!({
                        "txid": tx.txid,
                        "from": tx.from,
                        "to": tx.to,
                        "amount_xepc": to_coin(tx.amount),
                        "amount_ald": to_aldian(tx.amount),
                        "fee_xepc": to_coin(tx.fee),
                        "fee_ald": to_aldian(tx.fee),
                        "fee_rate": blockchain::Blockchain::fee_rate(tx),
                        "nonce": tx.nonce,
                        "note": tx.note,
                        "public_key": tx.public_key,
                        "signature": tx.signature,
                        "output_script": tx.output_script,
                        "in_mempool": false,
                        "block_hash": block.hash,
                        "block_height": block.index,
                        "block_mined_at": block.mined_at,
                    }));
                }
            }
            if block.coinbase_txid == txid {
                return RpcResponse::ok(json!({
                    "txid": block.coinbase_txid,
                    "from": "COINBASE",
                    "to": block.mined_for,
                    "amount_xepc": to_coin(block.reward),
                    "amount_ald": to_aldian(block.reward),
                    "subsidy_xepc": to_coin(block.subsidy),
                    "subsidy_ald": to_aldian(block.subsidy),
                    "total_fees_xepc": to_coin(block.total_fees),
                    "total_fees_ald": to_aldian(block.total_fees),
                    "fee_xepc": "0.00000000",
                    "in_mempool": false,
                    "block_hash": block.hash,
                    "block_height": block.index,
                    "block_mined_at": block.mined_at,
                    "is_coinbase": true,
                }));
            }
        }

        for tx in &chain.mempool {
            if tx.txid == txid {
                return RpcResponse::ok(json!({
                    "txid": tx.txid,
                    "from": tx.from,
                    "to": tx.to,
                    "amount_xepc": to_coin(tx.amount),
                    "amount_ald": to_aldian(tx.amount),
                    "fee_xepc": to_coin(tx.fee),
                    "fee_ald": to_aldian(tx.fee),
                    "fee_rate": blockchain::Blockchain::fee_rate(tx),
                    "nonce": tx.nonce,
                    "note": tx.note,
                    "public_key": tx.public_key,
                    "signature": tx.signature,
                    "output_script": tx.output_script,
                    "in_mempool": true,
                }));
            }
        }

        RpcResponse::err("transaction not found")
    }

    // Decode a raw transaction into a readable format.
    fn m_decoderawtransaction(&self, req: &RpcRequest) -> RpcResponse {
        let txid = match req.params.first().and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => return RpcResponse::err("decoderawtransaction requires a txid"),
        };

        let chain = self.state.chain.blocking_read();
        for block in &chain.chain {
            for tx in &block.transactions {
                if tx.txid == txid {
                    return RpcResponse::ok(json!({
                        "txid": tx.txid,
                        "from": tx.from,
                        "to": tx.to,
                        "amount_xepc": to_coin(tx.amount),
                        "amount_ald": to_aldian(tx.amount),
                        "fee_xepc": to_coin(tx.fee),
                        "fee_ald": to_aldian(tx.fee),
                        "fee_rate": blockchain::Blockchain::fee_rate(tx),
                        "nonce": tx.nonce,
                        "note": tx.note,
                        "public_key": tx.public_key,
                        "signature": tx.signature,
                        "block_hash": block.hash,
                        "block_height": block.index,
                    }));
                }
            }
        }

        for tx in &chain.mempool {
            if tx.txid == txid {
                return RpcResponse::ok(json!({
                    "txid": tx.txid,
                    "from": tx.from,
                    "to": tx.to,
                    "amount_xepc": to_coin(tx.amount),
                    "amount_ald": to_aldian(tx.amount),
                    "fee_xepc": to_coin(tx.fee),
                    "fee_ald": to_aldian(tx.fee),
                    "fee_rate": blockchain::Blockchain::fee_rate(tx),
                    "nonce": tx.nonce,
                    "note": tx.note,
                    "public_key": tx.public_key,
                    "signature": tx.signature,
                    "in_mempool": true,
                }));
            }
        }

        RpcResponse::err("transaction not found")
    }

    // Test whether a transaction would be accepted into the mempool.
    fn m_testmempoolaccept(&self, req: &RpcRequest) -> RpcResponse {
        let txid = match req.params.first().and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => return RpcResponse::err("testmempoolaccept requires a txid"),
        };

        let chain = self.state.chain.blocking_read();

        let tx = chain.mempool.iter().find(|t| t.txid == txid).cloned();

        if tx.is_none() {
            return RpcResponse::ok(json!([{
                "txid": txid,
                "allowed": false,
                "reject-reason": "transaction not found in mempool"
            }]));
        }

        let tx = tx.unwrap();

        match validation::validate_transaction(&chain, &tx) {
            Ok(_) => {
                let fee_rate = blockchain::Blockchain::fee_rate(&tx);
                RpcResponse::ok(json!([{
                    "txid": txid,
                    "allowed": true,
                    "vsize": blockchain::Blockchain::tx_size_bytes(&tx),
                    "fees": {
                        "base_xepc": to_coin(tx.fee),
                        "base_ald": to_aldian(tx.fee),
                    },
                    "fee_rate": fee_rate,
                }]))
            }
            Err(e) => RpcResponse::ok(json!([{
                "txid": txid,
                "allowed": false,
                "reject-reason": e.message
            }])),
        }
    }

    // Get a summary of the mempool.
    fn m_getmempoolinfo(&self) -> RpcResponse {
        let chain = self.state.chain.blocking_read();
        let txs = &chain.mempool;
        let count = txs.len();
        let mut total_amount: u64 = 0;
        let mut total_fee: u64 = 0;
        for tx in txs {
            total_amount = total_amount.saturating_add(tx.amount);
            total_fee = total_fee.saturating_add(tx.fee);
        }
        let avg_fee = if count > 0 { total_fee / count as u64 } else { 0 };
        RpcResponse::ok(json!({
            "size": count,
            "size_bytes": chain.mempool_size_bytes(),
            "max_size_bytes": blockchain::MAX_MEMPOOL_SIZE_BYTES,
            "min_relay_fee_per_byte": blockchain::MIN_RELAY_FEE_PER_BYTE,
            "total_amount_xepc": to_coin(total_amount),
            "total_amount_ald": to_aldian(total_amount),
            "total_fee_xepc": to_coin(total_fee),
            "total_fee_ald": to_aldian(total_fee),
            "avg_fee_xepc": to_coin(avg_fee),
            "avg_fee_ald": to_aldian(avg_fee),
            "default_fee_xepc": to_coin(DEFAULT_FEE_XEPC),
            "default_fee_ald": to_aldian(DEFAULT_FEE_XEPC),
        }))
    }

    // Validate every transaction in the mempool.
    fn m_validatemempool(&self) -> RpcResponse {
        let chain = self.state.chain.blocking_read();
        let errors = validation::validate_mempool(&chain);
        let items: Vec<Value> = errors
            .iter()
            .map(|e| json!({"code": e.code, "message": e.message}))
            .collect();
        RpcResponse::ok(json!({
            "valid": items.is_empty(),
            "mempool_len": chain.mempool.len(),
            "errors": items,
        }))
    }

    // Mine blocks directly to an address.
    fn m_generatetoaddress(&mut self, req: &RpcRequest) -> RpcResponse {
        let n = match req.params.first().and_then(|v| v.as_u64()) {
            Some(n) => n,
            None => return RpcResponse::err("generatetoaddress requires a block count"),
        };
        let addr = match req.params.get(1).and_then(|v| v.as_str()) {
            Some(a) => a.to_string(),
            None => return RpcResponse::err("generatetoaddress requires an address"),
        };

        {
            let chain = self.state.chain.blocking_read();
            if !validation::is_valid_address(&addr, &chain) {
                return RpcResponse::err("invalid address format");
            }
            drop(chain);
        }

        self.state.interrupt.store(false, Ordering::SeqCst);

        let mut hashes: Vec<String> = Vec::new();

        for _ in 0..n {
            if self.state.interrupt.load(Ordering::SeqCst) {
                println!("[rpc] generatetoaddress interrupted by stop");
                break;
            }

            let (
                next_index,
                prev_hash,
                subsidy,
                nbits,
                valid_txs,
                previous_chainwork,
                total_fees,
                state_root,
            ) = {
                let mut chain = self.state.chain.blocking_write();
                let prev = chain.chain.last().unwrap().clone();
                let next_index = prev.index + 1;
                let subsidy = Blockchain::calculate_reward(next_index);

                if chain.circulating_supply().saturating_add(subsidy) > consensus::max_supply() {
                    return RpcResponse::err("max supply reached");
                }

                let all_txs = std::mem::take(&mut chain.mempool);
                chain.mempool_timestamps.clear();
                let mut valid_txs = Vec::new();
                
                for tx in all_txs {
                    match chain.apply_and_execute_transaction(&tx) {
                        Ok(_) => valid_txs.push(tx),
                        Err(e) => {
                            println!("[mempool] Dropped invalid tx {} during mining: {}", tx.txid, e);
                        }
                    }
                }

                let total_fees = valid_txs.iter().map(|tx| tx.fee).sum::<u64>();
                
                let reward = subsidy
                    .checked_add(total_fees)
                    .expect("reward overflow should be impossible");
                    
                if let Err(e) = chain.credit_reward(&addr, reward, next_index) {
                    return RpcResponse::err(&format!("failed to credit reward: {}", e));
                }

                let state_root = match chain.state.root_hash() {
                    Ok(root) => hex::encode(root),
                    Err(e) => {
                        return RpcResponse::err(&format!("failed to compute state root: {}", e));
                    }
                };

                (
                    next_index,
                    prev.hash,
                    subsidy,
                    chain.nbits,
                    valid_txs,
                    prev.chainwork,
                    total_fees,
                    state_root,
                )
            };

            let mut block = match crate::block::Block::new_for(
                next_index,
                prev_hash,
                subsidy,
                total_fees,
                nbits,
                valid_txs,
                addr.clone(),
                state_root,
                previous_chainwork,
            ) {
                Ok(b) => b,
                Err(e) => {
                    return RpcResponse::err(&format!("failed to create block: {}", e));
                }
            };

            if !pow::mine_interruptible(&mut block, nbits, Some(&self.state.interrupt)) {
                println!("[rpc] mining interrupted at height {}", next_index);
                break;
            }

            {
                let mut chain = self.state.chain.blocking_write();
                chain.chain.push(block.clone());
                chain.adjust_difficulty();
                block.log_add_to_wallet();
                if let Err(e) = storage::save_chain(&chain) {
                    return RpcResponse::err(&format!("failed to save chain: {}", e));
                }
            }

            let tx = self.state.network_cmd_tx.clone();
            let hash = block.hash.clone();
            let _ = tx.blocking_send(NetworkCommand::BroadcastBlock(hash));

            hashes.push(block.hash.clone());
        }

        RpcResponse::ok(json!(hashes))
    }

    // Send funds to an address.
    fn m_sendtoaddress(&mut self, req: &RpcRequest) -> RpcResponse {
        if req.params.len() < 2 {
            return RpcResponse::err("sendtoaddress requires at least <amount> and <to_address>");
        }

        let is_sendall = req.params[0].as_str() == Some("sendall");

        let to = match req.params[1].as_str() {
            Some(a) => a.to_string(),
            None => return RpcResponse::err("invalid destination address"),
        };

        {
            let chain = self.state.chain.blocking_read();
            if !validation::is_valid_address(&to, &chain) {
                return RpcResponse::err("invalid destination address format");
            }
        }

        let mut wallet = match Wallet::load_default() {
            Ok(w) => w,
            Err(_) => return RpcResponse::err("wallet not found"),
        };

        if wallet.encrypted {
            let pass_opt = self.state.wallet_password.blocking_read().clone();
            match pass_opt {
                Some(pass) => match wallet.decrypt(&pass) {
                    Ok(decrypted) => wallet = decrypted,
                    Err(_) => {
                        return RpcResponse::err("wallet is locked. Run walletpassphrase first.")
                    }
                },
                None => return RpcResponse::err("wallet is locked. Run walletpassphrase first."),
            }
        }

        let estimated_tx_size: u64 = 250;
        let fee: u64 = if let Some(custom) = req.params.get(3).and_then(|v| v.as_u64()) {
            if custom == 0 {
                return RpcResponse::err("fee must be greater than zero");
            }
            custom
        } else {
            (blockchain::MIN_RELAY_FEE_PER_BYTE * estimated_tx_size).max(DEFAULT_FEE_XEPC)
        };

        let amount_ald: u64 = if is_sendall {
            0
        } else {
            match req.params[0].as_str() {
                Some(s) => match parse_amount(s) {
                    Ok(v) => v,
                    Err(e) => return RpcResponse::err(&format!("invalid amount: {}", e)),
                },
                None => match req.params[0].as_f64() {
                    Some(v) => {
                        if !(v > 0.0) {
                            return RpcResponse::err("amount must be greater than zero");
                        }
                        let s = format!("{:.8}", v);
                        match parse_amount(&s) {
                            Ok(v) => v,
                            Err(e) => {
                                return RpcResponse::err(&format!("invalid amount: {}", e))
                            }
                        }
                    }
                    None => return RpcResponse::err("invalid amount"),
                },
            }
        };

        if !is_sendall && amount_ald == 0 {
            return RpcResponse::err("amount must be greater than zero");
        }
        if !is_sendall && amount_ald > consensus::max_supply() {
            return RpcResponse::err(&format!(
                "amount exceeds the maximum supply of {} ALD",
                consensus::max_supply()
            ));
        }

        let need_for_selection: u64 = if is_sendall {
            fee
        } else {
            match amount_ald.checked_add(fee) {
                Some(v) => v,
                None => return RpcResponse::err("amount + fee overflows u64"),
            }
        };

        let explicit_from = if req.params.len() >= 3 {
            req.params[2].as_str().map(|s| s.to_string())
        } else {
            None
        };

        let from_address: String = {
            let chain = self.state.chain.blocking_read();
            match select_source_address(
                &wallet,
                &chain,
                &to,
                need_for_selection,
                explicit_from.as_deref(),
            ) {
                Ok(a) => a,
                Err(e) => return RpcResponse::err(&e),
            }
        };

        let amount_ald: u64 = if is_sendall {
            let chain = self.state.chain.blocking_read();
            let height = chain.height();
            let bal = chain.mature_balance_at(&from_address, height);
            if bal <= fee {
                return RpcResponse::err(&format!(
                    "mature balance {} ALD is too small to cover the fee of {} ALD",
                    bal, fee
                ));
            }
            bal - fee
        } else {
            amount_ald
        };

        if amount_ald == 0 {
            return RpcResponse::err("amount must be greater than zero");
        }

        let entry = match wallet.get_entry(&from_address) {
            Some(e) => e.clone(),
            None => {
                return RpcResponse::err(&format!(
                    "source address {} not found in wallet",
                    from_address
                ))
            }
        };

        if entry.address == to {
            return RpcResponse::err("sender and recipient addresses are identical");
        }

        // NOTE: Custom nonce input has been removed to prevent conflicts.
        // The system will always auto-generate the correct nonce.
        let nonce = {
            let chain = self.state.chain.blocking_read();
            chain.next_nonce_for(&entry.address)
        };

        // Create and sign the transaction.
        let mut tx = Transaction::new(&entry.address, &to, amount_ald, fee, nonce);
        if let Err(e) = tx.sign(&entry.secret) {
            return RpcResponse::err(&format!("failed to sign: {}", e));
        }
        let txid = tx.txid.clone();

        // Add the transaction to the mempool.
        let mut chain = self.state.chain.blocking_write();
        if !chain.add_transaction(tx.clone()) {
            return RpcResponse::err(
                "transaction rejected by mempool (RBF, eviction, or min relay fee)",
            );
        }

        let now = now_millis();
        println!(
            "[mempool] {} Accepted transaction {} from {} to {} amount={} XEPC fee={} XEPC nonce={}",
            format_datetime(now),
            txid,
            entry.address,
            to,
            to_coin(amount_ald),
            to_coin(fee),
            nonce
        );

        if let Err(e) = storage::save_chain(&chain) {
            return RpcResponse::err(&format!("failed to save chain: {}", e));
        }
        let mempool_len = chain.mempool.len();
        drop(chain);

        // Broadcast the transaction to the P2P network.
        let cmd_tx = self.state.network_cmd_tx.clone();
        let _ = cmd_tx.blocking_send(NetworkCommand::BroadcastTransaction(txid.clone()));

        RpcResponse::ok(json!({
            "txid": txid,
            "from": entry.address,
            "to": to,
            "amount_xepc": to_coin(amount_ald),
            "amount_ald": to_aldian(amount_ald),
            "fee_xepc": to_coin(fee),
            "fee_ald": to_aldian(fee),
            "fee_rate": fee / estimated_tx_size,
            "nonce": nonce,
            "sendall": is_sendall,
            "mempool_len": mempool_len,
        }))
    }

    // Create a new Legacy address in the wallet.
    fn m_getcreatewallet(&mut self) -> RpcResponse {
        if let Err(e) = datadir::ensure_dirs() {
            return RpcResponse::err(&format!("failed to prepare datadir: {}", e));
        }

        let mut w = Wallet::load_default().unwrap_or_else(|_| Wallet {
            entries: Vec::new(),
            created_at: now_millis(),
            encrypted: false,
            salt: String::new(),
            nonce: String::new(),
        });

        if w.encrypted {
            let pass_opt = self.state.wallet_password.blocking_read().clone();
            match pass_opt {
                Some(pass) => match w.decrypt(&pass) {
                    Ok(decrypted) => w = decrypted,
                    Err(_) => {
                        return RpcResponse::err("wallet is locked. Run walletpassphrase first.")
                    }
                },
                None => return RpcResponse::err("wallet is locked. Run walletpassphrase first."),
            }
        }

        let addr = w.add_new_address("default".to_string());
        let entry = w.get_entry(&addr).unwrap().clone();

        match w.save_default() {
            Ok(_) => RpcResponse::ok(json!({
                "address": entry.address,
                "public_key": entry.public_key,
                "secret": entry.secret,
                "created_at": entry.created_at,
                "type": "legacy",
            })),
            Err(e) => RpcResponse::err(&format!("failed to save wallet: {}", e)),
        }
    }

    // Create a new Bech32 address in the wallet.
    fn m_getcreatewalletbech32(&mut self) -> RpcResponse {
        if let Err(e) = datadir::ensure_dirs() {
            return RpcResponse::err(&format!("failed to prepare datadir: {}", e));
        }

        let mut w = Wallet::load_default().unwrap_or_else(|_| Wallet {
            entries: Vec::new(),
            created_at: now_millis(),
            encrypted: false,
            salt: String::new(),
            nonce: String::new(),
        });

        if w.encrypted {
            let pass_opt = self.state.wallet_password.blocking_read().clone();
            match pass_opt {
                Some(pass) => match w.decrypt(&pass) {
                    Ok(decrypted) => w = decrypted,
                    Err(_) => {
                        return RpcResponse::err("wallet is locked. Run walletpassphrase first.")
                    }
                },
                None => return RpcResponse::err("wallet is locked. Run walletpassphrase first."),
            }
        }

        let kp = crate::secp256k1::keypair::KeyPair::generate().expect("keypair generation failed");
        let address = crate::secp256k1::address::address_from_public_key_bech32(&kp.public_key)
            .expect("bech32 address generation failed");

        let entry = WalletEntry {
            address: address.clone(),
            public_key: kp.public_key_hex(),
            secret: kp.secret_key_hex(),
            label: "bech32".to_string(),
            created_at: now_millis(),
            encryption_nonce: String::new(),
        };
        w.entries.push(entry);

        match w.save_default() {
            Ok(_) => RpcResponse::ok(json!({
                "address": address,
                "public_key": kp.public_key_hex(),
                "secret": kp.secret_key_hex(),
                "type": "bech32",
            })),
            Err(e) => RpcResponse::err(&format!("failed to save wallet: {}", e)),
        }
    }

    // Unlock the wallet using a password.
    fn m_walletpassphrase(&mut self, req: &RpcRequest) -> RpcResponse {
        let pass = match req.params.first().and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => return RpcResponse::err("walletpassphrase requires a password"),
        };

        let w = match Wallet::load_default() {
            Ok(w) => w,
            Err(_) => return RpcResponse::err("wallet not found"),
        };

        match w.decrypt(&pass) {
            Ok(_) => {
                let mut lock = self.state.wallet_password.blocking_write();
                *lock = Some(pass);
                RpcResponse::ok(json!({
                    "message": "wallet decrypted successfully",
                    "addresses": w.entries.len(),
                }))
            }
            Err(e) => RpcResponse::err(&format!("failed to decrypt wallet: {}", e)),
        }
    }

    // Migrate a legacy wallet to an HD wallet.
    fn m_migratewallet(&mut self) -> RpcResponse {
        let legacy_wallet = match Wallet::load_default() {
            Ok(w) => w,
            Err(e) => return RpcResponse::err(&format!("failed to load legacy wallet: {}", e)),
        };

        if legacy_wallet.entries.is_empty() {
            return RpcResponse::err("legacy wallet has no addresses to migrate");
        }

        let wallet_path = datadir::wallet_path();
        let timestamp = now_millis();
        let backup_name = format!("wallet.dat.bak.{}", timestamp);
        let backup_path = datadir::datadir().join(&backup_name);

        if wallet_path.exists() {
            if let Err(e) = std::fs::copy(&wallet_path, &backup_path) {
                return RpcResponse::err(&format!("failed to backup wallet.dat: {}", e));
            }
        } else {
            return RpcResponse::err("wallet.dat not found on disk");
        }

        let mut hd_wallet = match HdWallet::load_default() {
            Ok(w) => w,
            Err(_) => match HdWallet::generate(12) {
                Ok(w) => w,
                Err(e) => return RpcResponse::err(&format!("failed to create HD wallet: {}", e)),
            },
        };

        let mut imported = 0;
        let mut skipped = 0;
        for entry in &legacy_wallet.entries {
            let hd_entry = HdAddressEntry {
                address: entry.address.clone(),
                public_key: entry.public_key.clone(),
                secret: entry.secret.clone(),
                path: "migrated".to_string(),
                index: 0,
                address_type: if entry.address.starts_with(&format!("{}1", crate::blockchain::bech32_hrp())) {
                    AddressType::Bech32
                } else {
                    AddressType::Legacy
                },
                created_at: now_millis(),
            };

            match hd_wallet.import_entry(hd_entry) {
                Ok(_) => imported += 1,
                Err(_) => skipped += 1,
            }
        }

        if let Err(e) = hd_wallet.save_default() {
            return RpcResponse::err(&format!("failed to save HD wallet: {}", e));
        }

        RpcResponse::ok(json!({
            "migrated": true,
            "backup_file": backup_name,
            "imported_count": imported,
            "skipped_count": skipped,
            "hd_wallet_total_entries": hd_wallet.entries.len(),
            "message": "Legacy wallet migrated to HD wallet. Backup created.",
        }))
    }

    // Create a new HD wallet.
    fn m_createhdwallet(&mut self, req: &RpcRequest) -> RpcResponse {
        let word_count = req.params.first().and_then(|v| v.as_u64()).unwrap_or(12) as usize;
        match HdWallet::generate(word_count) {
            Ok(wallet) => {
                if let Err(e) = wallet.save_default() {
                    return RpcResponse::err(&format!("failed to save HD wallet: {}", e));
                }
                RpcResponse::ok(json!({
                    "mnemonic": wallet.mnemonic,
                    "word_count": word_count,
                    "saved_to": HdWallet::default_path().to_string_lossy().to_string(),
                }))
            }
            Err(e) => RpcResponse::err(&e),
        }
    }

    // Load the HD wallet from disk.
    fn m_loadhdwallet(&self) -> RpcResponse {
        match HdWallet::load_default() {
            Ok(wallet) => RpcResponse::ok(json!({
                "mnemonic": wallet.mnemonic,
                "entries": wallet.entries,
                "created_at": wallet.created_at,
            })),
            Err(e) => RpcResponse::err(&format!("failed to load HD wallet: {}", e)),
        }
    }

    // Restore an HD wallet from a mnemonic phrase.
    fn m_restorehdwallet(&mut self, req: &RpcRequest) -> RpcResponse {
        let phrase = match req.params.first().and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => return RpcResponse::err("restorehdwallet requires a mnemonic phrase"),
        };
        let passphrase = req.params.get(1).and_then(|v| v.as_str());
        match HdWallet::from_mnemonic(&phrase, passphrase) {
            Ok(wallet) => {
                if let Err(e) = wallet.save_default() {
                    return RpcResponse::err(&format!("failed to save HD wallet: {}", e));
                }
                RpcResponse::ok(json!({
                    "restored": true,
                    "word_count": wallet.mnemonic.split_whitespace().count(),
                }))
            }
            Err(e) => RpcResponse::err(&e),
        }
    }

    // Dump the mnemonic phrase of the HD wallet.
    fn m_dumphdmnemonic(&self) -> RpcResponse {
        match HdWallet::load_default() {
            Ok(wallet) => RpcResponse::ok(json!({
                "mnemonic": wallet.mnemonic,
                "passphrase": wallet.passphrase,
            })),
            Err(e) => RpcResponse::err(&format!("failed to load HD wallet: {}", e)),
        }
    }

    // Derive a new address from the HD wallet.
    fn m_derivehdaddress(&mut self, req: &RpcRequest) -> RpcResponse {
        let index = match req.params.first().and_then(|v| v.as_u64()) {
            Some(i) => i as u32,
            None => return RpcResponse::err("derivehdaddress requires an index"),
        };
        let addr_type_str = req.params.get(1).and_then(|v| v.as_str()).unwrap_or("legacy");
        let addr_type = match AddressType::from_str(addr_type_str) {
            Some(t) => t,
            None => return RpcResponse::err("invalid address type, must be 'legacy' or 'bech32'"),
        };

        let mut wallet = match HdWallet::load_default() {
            Ok(w) => w,
            Err(e) => return RpcResponse::err(&format!("failed to load HD wallet: {}", e)),
        };

        match wallet.derive_address(index, addr_type) {
            Ok(entry) => {
                if let Err(e) = wallet.save_default() {
                    return RpcResponse::err(&format!("failed to save HD wallet: {}", e));
                }
                RpcResponse::ok(json!({
                    "address": entry.address,
                    "public_key": entry.public_key,
                    "path": entry.path,
                    "index": entry.index,
                    "address_type": match entry.address_type {
                        AddressType::Legacy => "legacy",
                        AddressType::Bech32 => "bech32",
                    },
                }))
            }
            Err(e) => RpcResponse::err(&e),
        }
    }

    // List all addresses derived in the HD wallet.
    fn m_listhdaddresses(&self) -> RpcResponse {
        match HdWallet::load_default() {
            Ok(wallet) => RpcResponse::ok(json!({
                "mnemonic": wallet.mnemonic,
                "entries": wallet.entries,
                "count": wallet.entries.len(),
            })),
            Err(e) => RpcResponse::err(&format!("failed to load HD wallet: {}", e)),
        }
    }
}

// Get the list of active checkpoints.
fn m_getcheckpoints() -> RpcResponse {
    let items: Vec<Value> = blockchain::checkpoint::CHECKPOINTS
        .iter()
        .map(|c| json!({"height": c.height, "hash": c.hash}))
        .collect();
    RpcResponse::ok(json!(items))
}

// List all addresses in the legacy wallet.
fn m_getlistwallet(chain_arc: Arc<RwLock<Blockchain>>) -> RpcResponse {
    let w = match Wallet::load_default() {
        Ok(w) => w,
        Err(_) => return RpcResponse::err("wallet not found"),
    };

    let chain = chain_arc.blocking_read();
    let current_height = chain.height();

    let mut addresses = Vec::new();
    let mut total_balance: u64 = 0;

    for entry in &w.entries {
        let balance = chain.balance_of(&entry.address);
        let mature = chain.mature_balance_at(&entry.address, current_height);
        let immature = chain.immature_balance_of(&entry.address);
        total_balance = total_balance.saturating_add(balance);

        addresses.push(json!({
            "address": entry.address,
            "public_key": entry.public_key,
            "label": entry.label,
            "created_at": entry.created_at,
            "balance_xepc": to_coin(balance),
            "balance_ald": to_aldian(balance),
            "mature_balance_xepc": to_coin(mature),
            "mature_balance_ald": to_aldian(mature),
            "immature_balance_xepc": to_coin(immature),
            "immature_balance_ald": to_aldian(immature),
            "nonce": chain.nonce_of(&entry.address),
        }));
    }

    RpcResponse::ok(json!({
        "addresses": addresses,
        "count": w.entries.len(),
        "wallet_total_balance_xepc": to_coin(total_balance),
        "wallet_total_balance_ald": to_aldian(total_balance),
        "blockchain_height": current_height,
    }))
}

// Dump the private key of a specific address.
fn m_dumpprivkey(req: &RpcRequest) -> RpcResponse {
    let target = match req.params.first().and_then(|v| v.as_str()) {
        Some(a) => a.to_string(),
        None => return RpcResponse::err("dumpprivkey requires an address"),
    };

    let w = match Wallet::load_default() {
        Ok(w) => w,
        Err(_) => return RpcResponse::err("wallet not found"),
    };

    let entry = match w.get_entry(&target) {
        Some(e) => e,
        None => return RpcResponse::err("address not found in wallet"),
    };

    RpcResponse::ok(json!({
        "address": entry.address,
        "privkey": entry.secret,
    }))
}

// Dump all addresses and private keys in the wallet.
fn m_dumpwallet() -> RpcResponse {
    let w = match Wallet::load_default() {
        Ok(w) => w,
        Err(_) => return RpcResponse::err("wallet not found"),
    };

    if w.encrypted {
        return RpcResponse::err("wallet is encrypted. Use walletpassphrase to unlock it first.");
    }

    let mut addresses = Vec::new();
    for entry in &w.entries {
        addresses.push(json!({
            "address": entry.address,
            "public_key": entry.public_key,
            "secret": entry.secret,
            "label": entry.label,
        }));
    }

    RpcResponse::ok(json!({
        "count": addresses.len(),
        "addresses": addresses,
        "message": "wallet dumped successfully. Keep this data secure.",
    }))
}

// Import a private key into the wallet.
fn m_importprivkey(req: &RpcRequest) -> RpcResponse {
    let secret = match req.params.first().and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return RpcResponse::err("importprivkey requires a private key"),
    };

    if secret.len() != 64 || !secret.chars().all(|c| c.is_ascii_hexdigit()) {
        return RpcResponse::err("private key must be 64 hexadecimal characters");
    }

    let mut w = Wallet::load_default().unwrap_or_else(|_| Wallet {
        entries: Vec::new(),
        created_at: now_millis(),
        encrypted: false,
        salt: String::new(),
        nonce: String::new(),
    });

    if w.encrypted {
        return RpcResponse::err("wallet is encrypted. Use walletpassphrase to unlock it first.");
    }

    let bytes = match hex::decode(&secret) {
        Ok(b) => b,
        Err(e) => return RpcResponse::err(&format!("invalid hex: {}", e)),
    };

    let kp = match crate::secp256k1::keypair::KeyPair::from_secret(&bytes) {
        Ok(k) => k,
        Err(e) => return RpcResponse::err(&format!("invalid private key: {}", e)),
    };

    let legacy_addr = match crate::secp256k1::address::address_from_public_key(&kp.public_key) {
        Ok(a) => a,
        Err(e) => return RpcResponse::err(&format!("failed to generate legacy address: {}", e)),
    };

    let bech32_addr = match crate::secp256k1::address::address_from_public_key_bech32(&kp.public_key) {
        Ok(a) => a,
        Err(e) => return RpcResponse::err(&format!("failed to generate bech32 address: {}", e)),
    };

    let public_key_hex = kp.public_key_hex();
    let secret_key_hex = kp.secret_key_hex();

    if w.get_entry(&legacy_addr).is_none() {
        let entry = WalletEntry {
            address: legacy_addr.clone(),
            public_key: public_key_hex.clone(),
            secret: secret_key_hex.clone(),
            label: "imported-legacy".to_string(),
            created_at: now_millis(),
            encryption_nonce: String::new(),
        };
        w.entries.push(entry);
    }

    if w.get_entry(&bech32_addr).is_none() {
        let entry = WalletEntry {
            address: bech32_addr.clone(),
            public_key: public_key_hex.clone(),
            secret: secret_key_hex.clone(),
            label: "imported-bech32".to_string(),
            created_at: now_millis(),
            encryption_nonce: String::new(),
        };
        w.entries.push(entry);
    }

    match w.save_default() {
        Ok(_) => RpcResponse::ok(json!({
            "legacy_address": legacy_addr,
            "bech32_address": bech32_addr,
            "public_key": public_key_hex,
            "message": "private key imported successfully for both Legacy and Bech32 formats",
        })),
        Err(e) => RpcResponse::err(&format!("failed to save wallet: {}", e)),
    }
}

// Encrypt the wallet with a password.
fn m_encryptwallet(req: &RpcRequest) -> RpcResponse {
    let pass = match req.params.first().and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => return RpcResponse::err("encryptwallet requires a password"),
    };

    let mut w = match Wallet::load_default() {
        Ok(w) => w,
        Err(_) => return RpcResponse::err("wallet not found"),
    };

    if let Err(e) = w.encrypt(&pass) {
        return RpcResponse::err(&format!("failed to encrypt wallet: {}", e));
    }

    match w.save_default() {
        Ok(_) => RpcResponse::ok(json!({"message": "wallet encrypted successfully"})),
        Err(e) => RpcResponse::err(&format!("failed to save wallet: {}", e)),
    }
}

// Get detailed information about an address.
fn m_getaddressinfo(req: &RpcRequest) -> RpcResponse {
    let target = match req.params.first().and_then(|v| v.as_str()) {
        Some(a) => a.to_string(),
        None => return RpcResponse::err("getaddressinfo requires an address"),
    };

    let w = match Wallet::load_default() {
        Ok(w) => w,
        Err(_) => return RpcResponse::err("wallet not found"),
    };

    let entry = w.get_entry(&target);

    RpcResponse::ok(json!({
        "address": target,
        "ismine": entry.is_some(),
        "label": entry.map(|e| e.label.clone()).unwrap_or_default(),
        "public_key": entry.map(|e| e.public_key.clone()).unwrap_or_default(),
        "created_at": entry.map(|e| e.created_at).unwrap_or(0),
    }))
}

// Set a label for an address in the wallet.
fn m_setlabel(req: &RpcRequest) -> RpcResponse {
    if req.params.len() < 2 {
        return RpcResponse::err("setlabel requires an address and a label");
    }
    let target = match req.params[0].as_str() {
        Some(a) => a.to_string(),
        None => return RpcResponse::err("invalid address"),
    };
    let label = match req.params[1].as_str() {
        Some(l) => l.to_string(),
        None => return RpcResponse::err("invalid label"),
    };

    let mut w = match Wallet::load_default() {
        Ok(w) => w,
        Err(_) => return RpcResponse::err("wallet not found"),
    };

    let mut found = false;
    for e in w.entries.iter_mut() {
        if e.address == target {
            e.label = label.clone();
            found = true;
            break;
        }
    }

    if !found {
        return RpcResponse::err("address not found in wallet");
    }

    match w.save_default() {
        Ok(_) => RpcResponse::ok(json!({
            "address": target,
            "label": label,
            "message": "label updated successfully",
        })),
        Err(e) => RpcResponse::err(&format!("failed to save wallet: {}", e)),
    }
}

// Backup the wallet file to a destination path.
fn m_backupwallet(req: &RpcRequest) -> RpcResponse {
    let dest = match req.params.first().and_then(|v| v.as_str()) {
        Some(d) => d.to_string(),
        None => return RpcResponse::err("backupwallet requires a destination path"),
    };

    let src = datadir::as_str(&datadir::wallet_path());

    if !std::path::Path::new(&src).exists() {
        return RpcResponse::err("wallet file not found");
    }

    match std::fs::copy(&src, &dest) {
        Ok(_) => RpcResponse::ok(json!({
            "source": src,
            "destination": dest,
            "message": "wallet backed up successfully",
        })),
        Err(e) => RpcResponse::err(&format!("failed to backup wallet: {}", e)),
    }
}

// Use the default wallet address as the mining target.
fn m_getaddtowallet() -> RpcResponse {
    let w = match Wallet::load_default() {
        Ok(w) => w,
        Err(_) => return RpcResponse::err("wallet not found"),
    };
    let entry = match w.default_address() {
        Some(e) => e,
        None => return RpcResponse::err("wallet has no addresses"),
    };
    let mut cfg = Config::load_or_default();
    cfg.mining_address = entry.address.clone();
    match cfg.save_default() {
        Ok(_) => RpcResponse::ok(json!({
            "mining_address": entry.address,
            "config_file": Config::default_path(),
        })),
        Err(e) => RpcResponse::err(&format!("failed to save config: {}", e)),
    }
}

// Set a manual mining address.
fn m_getaddtoaddress(req: &RpcRequest) -> RpcResponse {
    let addr = match req.params.first().and_then(|v| v.as_str()) {
        Some(a) => a.to_string(),
        None => return RpcResponse::err("getaddtoaddress requires an address"),
    };
    let chain = Blockchain::new();
    if !validation::is_valid_address(&addr, &chain) {
        return RpcResponse::err("invalid address format");
    }
    let mut cfg = Config::load_or_default();
    cfg.mining_address = addr.clone();
    match cfg.save_default() {
        Ok(_) => RpcResponse::ok(json!({
            "mining_address": addr,
            "config_file": Config::default_path(),
        })),
        Err(e) => RpcResponse::err(&format!("failed to save config: {}", e)),
    }
}

// Write a standard HTTP response to the stream.
fn write_http_response(
    mut stream: TcpStream,
    status: u16,
    resp: &RpcResponse,
) -> std::io::Result<()> {
    let body = serde_json::to_string(resp).unwrap();
    let status_text = if status == 200 { "OK" } else { "Error" };
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        status,
        status_text,
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()
}

// Call an RPC method and print the result to the console.
pub fn call_and_print(method: &str, params: Vec<Value>) {
    match call(method, params) {
        Ok(resp) => {
            if resp.ok {
                println!("{}", serde_json::to_string_pretty(&resp.result).unwrap());
            } else {
                println!("error: {}", resp.error);
            }
        }
        Err(e) => {
            let cfg = Config::load_or_default();
            println!("[rpc] cannot connect to node at {}: {}", cfg.rpc_addr, e);
            println!("[rpc] start xepayd first.");
        }
    }
}
