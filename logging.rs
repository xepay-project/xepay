// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

// logging.rs
// Structured logger for XEPAY (ticker: XEPY).
//
// Output format is machine-parseable key=value:
//   [LEVEL] event_name key1=value1 key2=value2 ...
//
// Level is controlled by the XEPAY_LOG environment variable:
//   XEPAY_LOG=debug   — show debug and above (default: info)
//   XEPAY_LOG=warn    — show warnings and above
//   XEPAY_LOG=error   — show errors only
//   XEPAY_LOG=quiet   — suppress all output

use std::sync::atomic::{AtomicU8, Ordering};

pub const LEVEL_QUIET: u8 = 0;
pub const LEVEL_ERROR: u8 = 1;
pub const LEVEL_WARN: u8 = 2;
pub const LEVEL_INFO: u8 = 3;
pub const LEVEL_DEBUG: u8 = 4;

static LEVEL: AtomicU8 = AtomicU8::new(LEVEL_INFO);

/// Initialize the logger from the XEPAY_LOG environment variable.
pub fn init() {
    let level = match std::env::var("XEPAY_LOG").as_deref() {
        Ok("quiet") => LEVEL_QUIET,
        Ok("error") => LEVEL_ERROR,
        Ok("warn") => LEVEL_WARN,
        Ok("info") | Err(_) => LEVEL_INFO,
        Ok("debug") => LEVEL_DEBUG,
        _ => LEVEL_INFO,
    };
    LEVEL.store(level, Ordering::Relaxed);
}

/// Set the logger level programmatically.
pub fn set_level(level: u8) {
    LEVEL.store(level, Ordering::Relaxed);
}

fn enabled(level: u8) -> bool {
    LEVEL.load(Ordering::Relaxed) >= level && LEVEL.load(Ordering::Relaxed) != LEVEL_QUIET
}

/// Emit a structured log line at the given level.
pub fn log(level: u8, event: &str, fields: &[(&str, String)]) {
    if !enabled(level) {
        return;
    }
    let tag = match level {
        LEVEL_ERROR => "ERROR",
        LEVEL_WARN => "WARN",
        LEVEL_INFO => "INFO",
        LEVEL_DEBUG => "DEBUG",
        _ => "INFO",
    };
    let mut line = format!("[{}] {}", tag, event);
    for (k, v) in fields {
        line.push(' ');
        line.push_str(k);
        line.push('=');
        line.push_str(v);
    }
    if level == LEVEL_ERROR {
        eprintln!("{}", line);
    } else {
        println!("{}", line);
    }
}

/// Log at INFO level.
pub fn info(event: &str, fields: &[(&str, String)]) {
    log(LEVEL_INFO, event, fields);
}

/// Log at WARN level.
pub fn warn(event: &str, fields: &[(&str, String)]) {
    log(LEVEL_WARN, event, fields);
}

/// Log at ERROR level.
pub fn error(event: &str, fields: &[(&str, String)]) {
    log(LEVEL_ERROR, event, fields);
}

/// Log at DEBUG level.
pub fn debug(event: &str, fields: &[(&str, String)]) {
    log(LEVEL_DEBUG, event, fields);
}

/// Format an XEPYsat amount as "X.XXXXXXXX XEPY (N XEPYsat)".
pub fn fmt_amount(xepysat: u64) -> String {
    let whole = xepysat / 100_000_000;
    let frac = xepysat % 100_000_000;
    format!("{}.{:08} XEPY ({} XEPYsat)", whole, frac, xepysat)
}

/// Truncate a hex string for display (first 16 chars + "...").
pub fn short(hex: &str) -> String {
    if hex.len() <= 16 {
        hex.to_string()
    } else {
        format!("{}...", &hex[..16])
    }
}
