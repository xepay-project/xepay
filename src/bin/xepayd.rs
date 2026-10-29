// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! XEPAY node daemon.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, RwLock};

use xepay::block::Block;
use xepay::blockchain::Blockchain;
use xepay::config::Config;
use xepay::consensus::spec::{self, Network};
use xepay::p2p::network::{spawn, NetworkCommand, NetworkConfig, PeerRegistry};
use xepay::rpc::{self, RpcServer};
use xepay::storage;

static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut selected_network = Network::Main;
    let mut is_daemon_child = false;
    let mut reindex = false;

    for arg in &args[1..] {
        match arg.as_str() {
            "-testnet" | "--testnet" => selected_network = Network::Test,
            "-regtest" | "--regtest" => selected_network = Network::Regtest,
            "-reindex" | "--reindex" => reindex = true,
            "--daemon-child" => is_daemon_child = true,
            "--help" | "-h" => {
                println!("Usage: xepayd [-testnet|--testnet] [-regtest|--regtest] [-reindex]");
                return;
            }
            "--version" => {
                println!("xepayd {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown option: {}", other);
                std::process::exit(1);
            }
            _ => {
                eprintln!("error: xepayd does not accept positional arguments");
                std::process::exit(1);
            }
        }
    }

    spec::set_current_network(selected_network);

    if reindex {
        std::env::set_var("XEPAY_REINDEX", "1");
    }

    if !is_daemon_child {
        install_detach_signal_handler();
    }

    println!(
        "XEPAY server starting (network: {})",
        selected_network.name()
    );
    if is_daemon_child {
        println!("[daemon] running in the background.");
    } else {
        println!(
            "Press Ctrl+C to detach the node into the background (the node will keep running)."
        );
        println!("Use `./xepay stop` to stop the node completely.");
    }

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main(selected_network, is_daemon_child));
}

async fn async_main(selected_network: Network, is_daemon_child: bool) {
    let config = Config::load_or_default();
    println!("[config] network      : {}", config.network.name());
    println!("[config] rpc_addr     : {}", config.rpc_addr);
    println!("[config] listen_addr  : {}", config.listen_addr);
    println!("[config] bootstrap    : {:?}", config.bootstrap);
    println!("[config] mining_addr  : {}", config.mining_address);
    println!("[config] location     : {}", Config::default_path());

    if !std::path::Path::new(&Config::default_path()).exists() {
        let _ = config.save_default();
        println!(
            "[config] configuration file created at {}",
            Config::default_path()
        );
    }

    let _ = Block::genesis();

    let chain = match storage::load_chain() {
        Ok(c) => Arc::new(RwLock::new(c)),
        Err(e) => {
            eprintln!("[node] FATAL: failed to load chain: {}", e);
            eprintln!("[node] Refusing to start with an inconsistent state.");
            std::process::exit(1);
        }
    };

    let peer_registry: PeerRegistry = Arc::new(Mutex::new(Vec::new()));
    let (cmd_tx, cmd_rx) = mpsc::channel::<NetworkCommand>(100);
    let cmd_tx_for_shutdown = cmd_tx.clone();

    let listen_addr = match config.listen_addr.parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("[p2p] invalid listen address: {}", e);
            std::process::exit(1);
        }
    };

    let mut bootstrap = Vec::new();
    if selected_network != Network::Regtest {
        for b in &config.bootstrap {
            if let Ok(sa) = b.parse() {
                bootstrap.push(sa);
            } else {
                eprintln!("[p2p] invalid bootstrap address: {}", b);
            }
        }
    }

    let net_config = NetworkConfig {
        listen_addr,
        bootstrap,
        ..Default::default()
    };

    let chain_for_rpc = chain.clone();
    let cfg_for_rpc = config.clone();
    let rpc_registry = peer_registry.clone();
    let (rpc_shutdown, rpc_handle) = {
        let mut server = RpcServer::new(
            chain_for_rpc.clone(),
            &cfg_for_rpc,
            rpc_registry,
            cmd_tx,
            selected_network,
        );
        let flag = server.shutdown_flag();
        let handle = tokio::task::spawn_blocking(move || server.listen());
        (flag, handle)
    };

    let net_handle = spawn(net_config, chain.clone(), peer_registry, cmd_rx);

    tokio::spawn(async move {
        match net_handle.await {
            Ok(Ok(())) => println!("[p2p] network loop exited"),
            Ok(Err(e)) => {
                eprintln!("[p2p] fatal: {}", e);
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("[p2p] fatal: task panicked: {}", e);
                std::process::exit(1);
            }
        }
    });

    install_signal_handlers(rpc_shutdown.clone(), is_daemon_child);

    loop {
        if SHUTDOWN_REQUESTED.load(Ordering::SeqCst) || rpc_shutdown.load(Ordering::SeqCst) {
            println!("[node] full shutdown requested");
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    {
        let mut guard = chain.write().await;
        if let Err(e) = storage::save_chain(&mut guard) {
            eprintln!("[node] failed to save chain: {}", e);
        } else {
            println!("[node] chain saved");
        }
    }

    let _ = cmd_tx_for_shutdown.send(NetworkCommand::Shutdown).await;
    let _ = tokio::time::timeout(Duration::from_secs(5), rpc_handle).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    println!("[node] shutdown complete");
}

fn install_detach_signal_handler() {
    use nix::sys::signal::{self, SigHandler, Signal};

    extern "C" fn handle_sigint(_: i32) {
        detach_to_background();
    }

    unsafe {
        signal::signal(Signal::SIGINT, SigHandler::Handler(handle_sigint))
            .expect("failed to install SIGINT handler");
    }
}

fn install_signal_handlers(rpc_shutdown: Arc<AtomicBool>, is_daemon_child: bool) {
    use tokio::signal::unix::{signal, SignalKind};

    if is_daemon_child {
        match signal(SignalKind::hangup()) {
            Ok(mut sig) => {
                tokio::spawn(async move {
                    loop {
                        if sig.recv().await.is_some() {
                            println!("[signal] SIGHUP received (ignored in daemon mode).");
                        }
                    }
                });
            }
            Err(e) => eprintln!("[signal] failed to install SIGHUP handler: {}", e),
        }
    }

    match signal(SignalKind::terminate()) {
        Ok(mut sig) => {
            tokio::spawn(async move {
                if sig.recv().await.is_some() {
                    println!();
                    println!("[signal] SIGTERM (kill) received. Initiating full shutdown...");
                    SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
                }
            });
        }
        Err(e) => eprintln!("[signal] failed to install SIGTERM handler: {}", e),
    }

    let _ = rpc_shutdown;
}

// Spawns a new `xepayd --daemon-child` process in the background.
// Waits until the child's RPC server is ready before exiting the parent.
fn detach_to_background() {
    use std::process::Command;
    use std::thread::sleep;

    let exe = std::env::current_exe().expect("failed to get current exe");
    let args: Vec<String> = std::env::args().collect();

    let mut cmd = Command::new(exe);
    for arg in &args[1..] {
        if arg != "--daemon-child" {
            cmd.arg(arg);
        }
    }
    cmd.arg("--daemon-child");

    let log_path = xepay::datadir::debug_log_path();
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .expect("failed to open debug log");

    let log_file_clone = log_file.try_clone().expect("failed to clone log file");

    cmd.stdout(log_file);
    cmd.stderr(log_file_clone);

    match cmd.spawn() {
        Ok(child) => {
            println!("[daemon] spawned background process (PID: {})", child.id());
            println!("[daemon] waiting for the node to be ready...");

            let start = Instant::now();
            let timeout = Duration::from_secs(10);
            let mut ready = false;

            while start.elapsed() < timeout {
                if let Ok(resp) = rpc::call("getinfo", vec![]) {
                    if resp.ok {
                        ready = true;
                        break;
                    }
                }
                sleep(Duration::from_millis(200));
            }

            if ready {
                println!("[daemon] node is ready. Parent process exiting.");
            } else {
                println!("[daemon] WARNING: node did not become ready within 10 seconds.");
                println!("[daemon] Parent process exiting anyway. Check the log for details.");
            }

            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("[daemon] failed to spawn background process: {}", e);
            eprintln!("[daemon] continuing in the foreground.");
        }
    }
}
