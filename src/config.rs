// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Configuration file parsing and persistence.

use crate::consensus::spec::{self, Network};
use crate::datadir;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub network: Network,
    pub rpc_addr: String,
    pub rpc_user: String,
    pub rpc_password: String,
    pub listen_addr: String,
    pub bootstrap: Vec<String>,
    pub mining_address: String,
    pub created_at: u128,
    pub protocol_version: u32,
}

impl Config {
    pub fn new() -> Self {
        let network = spec::current_network();
        Config {
            network,
            rpc_addr: format!("127.0.0.1:{}", network.default_rpc_port()),
            rpc_user: "xepay".to_string(),
            rpc_password: "rahasia".to_string(),
            listen_addr: format!("0.0.0.0:{}", network.default_port()),
            bootstrap: Vec::new(),
            mining_address: "XEPCMINERDEFAULT".to_string(),
            created_at: crate::utils::now_millis(),
            protocol_version: crate::blockchain::protocol_version(),
        }
    }

    pub fn default_path() -> String {
        datadir::as_str(&datadir::config_path())
    }

    pub fn load_or_default() -> Self {
        let path = Self::default_path();
        let data = match std::fs::read_to_string(&path) {
            Ok(d) => d,
            Err(_) => return Config::new(),
        };
        Config::parse(&data)
    }

    pub fn parse(data: &str) -> Self {
        let mut cfg = Config::new();

        for line in data.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let (key, value) = match line.split_once('=') {
                Some(kv) => kv,
                None => continue,
            };

            let key = key.trim();
            let value = value.trim().trim_matches('"');

            match key {
                "network" => {
                    if let Some(n) = Network::from_name(value) {
                        cfg.network = n;
                        // Recompute ports from the network unless
                        // overridden later in the file.
                        cfg.rpc_addr = format!("127.0.0.1:{}", n.default_rpc_port());
                        cfg.listen_addr = format!("0.0.0.0:{}", n.default_port());
                    }
                }
                "rpcport" => cfg.rpc_addr = format!("127.0.0.1:{}", value),
                "rpcbind" => cfg.rpc_addr = value.to_string(),
                "rpcuser" => cfg.rpc_user = value.to_string(),
                "rpcpassword" => cfg.rpc_password = value.to_string(),
                "port" => cfg.listen_addr = format!("0.0.0.0:{}", value),
                "listen" => cfg.listen_addr = normalize_listen_addr(value),
                "bootstrap" => cfg.bootstrap.push(value.to_string()),
                "miningaddress" => cfg.mining_address = value.to_string(),
                _ => {}
            }
        }
        cfg
    }

    pub fn save_default(&self) -> std::io::Result<()> {
        datadir::ensure_dirs()?;
        let path = Self::default_path();

        let rpcport = self.rpc_addr.rsplit(':').next().unwrap_or("7554");
        let port = self.listen_addr.rsplit(':').next().unwrap_or("7555");

        let mut out = String::new();
        out.push_str("# XEPAY configuration\n");
        out.push_str(&format!("network={}\n", self.network.name()));
        out.push_str(&format!("rpcport={}\n", rpcport));
        out.push_str(&format!("rpcuser={}\n", self.rpc_user));
        out.push_str(&format!("rpcpassword={}\n", self.rpc_password));
        out.push_str(&format!("port={}\n", port));
        out.push_str(&format!("listen={}\n", self.listen_addr));
        out.push_str(&format!("miningaddress={}\n", self.mining_address));

        for b in &self.bootstrap {
            out.push_str(&format!("bootstrap={}\n", b));
        }

        std::fs::write(path, out)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

fn normalize_listen_addr(value: &str) -> String {
    if value.starts_with('/') {
        let parts: Vec<&str> = value.trim_start_matches('/').split('/').collect();
        if parts.len() >= 4 {
            let proto = parts[0];
            let ip = parts[1];
            let port = parts[3];
            if (proto == "ip4" || proto == "ip6") && parts[2] == "tcp" {
                if proto == "ip6" {
                    return format!("[{}]:{}", ip, port);
                }
                return format!("{}:{}", ip, port);
            }
        }
        return format!("0.0.0.0:{}", spec::current_network().default_port());
    }
    value.to_string()
}
