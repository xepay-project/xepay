// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Bech32 encoding/decoding for XEPAY Native SegWit addresses.

use bech32::{segwit, Fe32, Hrp}; // Import Hrp for parsing.

/// Encode a 20-byte witness program into a Native SegWit Bech32 address.
pub fn encode(hrp: &str, witness_program: &[u8]) -> Result<String, String> {
    if witness_program.len() != 20 {
        // Native SegWit (P2WPKH) requires a 20-byte witness program.
        return Err(format!(
            "witness program must be 20 bytes, got {}",
            witness_program.len()
        ));
    }

    // Parse the HRP string into an Hrp type.
    let hrp_parsed = Hrp::parse(hrp).map_err(|e| format!("invalid HRP: {}", e))?;

    // Encode using SegWit v0 (Fe32::Q = 0).
    segwit::encode(hrp_parsed, Fe32::Q, witness_program)
        .map_err(|e| format!("bech32 encoding failed: {}", e))
}

/// Decode a Native SegWit Bech32 address into its HRP and witness program.
pub fn decode(address: &str) -> Option<(String, Vec<u8>)> {
    // Decode the SegWit address.
    let (hrp, variant, data) = segwit::decode(address).ok()?;

    // Ensure it is SegWit v0 (witness version 0).
    if variant != Fe32::Q {
        return None;
    }

    // Return the HRP as a string and the witness program.
    Some((hrp.to_string(), data))
}
