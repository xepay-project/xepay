// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Utility functions shared across the crate.

use rand::RngCore;
use serde::{Deserialize, Deserializer, Serializer};

/// Return the current Unix timestamp in milliseconds.
pub fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

/// Format a `u64` amount (in Aldian, the smallest unit) as a human-readable
/// XEPC string with 8 decimal places. Used only for display and RPC output;
/// the consensus layer always works with `u64` Aldian.
pub fn format_xepc(amount: u64) -> String {
    let whole = amount / 100_000_000;
    let frac = amount % 100_000_000;
    format!("{}.{:08}", whole, frac)
}

/// Format a `u64` amount as a raw Aldian integer (no decimals).
pub fn format_aldian(amount: u64) -> String {
    amount.to_string()
}

/// Parse a human-readable XEPC string (e.g. "15.50000000") into a `u64`
/// amount in Aldian. Used only for RPC input; the consensus layer always
/// works with `u64`.
pub fn parse_xepc(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty amount".into());
    }
    if s.starts_with('-') {
        return Err("amount must not be negative".into());
    }

    let (whole_str, frac_str) = match s.split_once('.') {
        Some((w, f)) => (w, f),
        None => (s, ""),
    };

    if whole_str.is_empty() && frac_str.is_empty() {
        return Err("empty amount".into());
    }

    let whole: u64 = if whole_str.is_empty() {
        0
    } else {
        whole_str
            .parse()
            .map_err(|_| format!("invalid whole part: {}", whole_str))?
    };

    if frac_str.len() > 8 {
        return Err(format!(
            "too many fractional digits: {} (maximum is 8)",
            frac_str.len()
        ));
    }

    let mut frac_padded = frac_str.to_string();
    while frac_padded.len() < 8 {
        frac_padded.push('0');
    }
    let frac: u64 = if frac_padded.is_empty() {
        0
    } else {
        frac_padded
            .parse()
            .map_err(|_| format!("invalid fractional part: {}", frac_str))?
    };

    let whole_ald = whole
        .checked_mul(100_000_000)
        .ok_or_else(|| "amount overflows u64".to_string())?;
    let total = whole_ald
        .checked_add(frac)
        .ok_or_else(|| "amount overflows u64".to_string())?;

    Ok(total)
}

/// Format a Unix timestamp in milliseconds as "YYYY-MM-DD HH:MM:SS" in UTC.
pub fn format_datetime(millis: u128) -> String {
    let secs = (millis / 1000) as i64;
    let (year, month, day, hour, minute, second) = civil_from_unix(secs);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hour, minute, second
    )
}

fn civil_from_unix(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let secs_of_day = secs.rem_euclid(86_400);

    let hour = (secs_of_day / 3600) as u32;
    let minute = ((secs_of_day % 3600) / 60) as u32;
    let second = (secs_of_day % 60) as u32;

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if m <= 2 { y + 1 } else { y };

    (year, m, d, hour, minute, second)
}

/// Generate a random hex string of `n` bytes.
pub fn random_hex(n: usize) -> String {
    let mut buf = vec![0u8; n];
    rand::thread_rng().fill_bytes(&mut buf);
    hex::encode(buf)
}

/// Convert a hex string to bytes.
pub fn hex_to_bytes(hex_str: &str) -> Result<Vec<u8>, String> {
    hex::decode(hex_str).map_err(|e| e.to_string())
}

/// Serialize a `u128` as a decimal string in JSON.
pub mod u128_string {
    use super::*;

    pub fn serialize<S>(value: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<u128>().map_err(serde::de::Error::custom)
    }
}
