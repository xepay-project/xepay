// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Message framing for the XEPAY peer-to-peer protocol.

use crate::consensus::spec;
use std::io::{self, Read, Write};
use xepay_crypto::sha3_256_hex;

pub const COMMAND_LEN: usize = 12;
pub const HEADER_LEN: usize = 4 + COMMAND_LEN + 4 + 4;
pub const MAX_PAYLOAD_LEN: usize = 32 * 1024 * 1024;

/// Compute the four-byte checksum of the payload:
/// SHA3-256(SHA3-256(payload))[..4].
pub fn checksum(payload: &[u8]) -> [u8; 4] {
    let first = sha3_256_hex(payload);
    let first_bytes = hex::decode(&first).expect("sha3 hex is always valid");
    let second = sha3_256_hex(&first_bytes);
    let second_bytes = hex::decode(&second).expect("sha3 hex is always valid");
    let mut out = [0u8; 4];
    out.copy_from_slice(&second_bytes[..4]);
    out
}

/// Read a complete message from the stream.
/// Return `(command, payload)`.
pub fn read_message<R: Read>(reader: &mut R) -> io::Result<(String, Vec<u8>)> {
    let mut header = [0u8; HEADER_LEN];
    reader.read_exact(&mut header)?;

    let expected_magic = spec::current_network().magic();
    let magic = &header[0..4];
    if magic != expected_magic {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "magic mismatch: got {:02x?}, expected {:02x?}",
                magic, expected_magic
            ),
        ));
    }

    let command_bytes = &header[4..4 + COMMAND_LEN];
    let command_end = command_bytes
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(COMMAND_LEN);
    let command = String::from_utf8_lossy(&command_bytes[..command_end]).to_string();

    let length = u32::from_le_bytes([header[16], header[17], header[18], header[19]]) as usize;

    if length > MAX_PAYLOAD_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("payload too large: {} bytes", length),
        ));
    }

    let expected_checksum = [header[20], header[21], header[22], header[23]];

    let mut payload = vec![0u8; length];
    reader.read_exact(&mut payload)?;

    let actual_checksum = checksum(&payload);
    if actual_checksum != expected_checksum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "checksum mismatch",
        ));
    }

    Ok((command, payload))
}

/// Write a complete message to the stream.
pub fn write_message<W: Write>(writer: &mut W, command: &str, payload: &[u8]) -> io::Result<()> {
    if command.len() > COMMAND_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("command too long: {}", command),
        ));
    }

    let mut header = [0u8; HEADER_LEN];
    let magic = spec::current_network().magic();
    header[0..4].copy_from_slice(&magic);

    let cmd_bytes = command.as_bytes();
    header[4..4 + cmd_bytes.len()].copy_from_slice(cmd_bytes);

    let length = payload.len() as u32;
    header[16..20].copy_from_slice(&length.to_le_bytes());

    let cs = checksum(payload);
    header[20..24].copy_from_slice(&cs);

    writer.write_all(&header)?;
    writer.write_all(payload)?;
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_message() {
        let mut buf = Vec::new();
        write_message(&mut buf, "ping", b"hello").unwrap();

        let mut cursor = std::io::Cursor::new(buf);
        let (cmd, payload) = read_message(&mut cursor).unwrap();
        assert_eq!(cmd, "ping");
        assert_eq!(payload, b"hello");
    }

    #[test]
    fn checksum_is_consistent() {
        let a = checksum(b"xepay");
        let b = checksum(b"xepay");
        assert_eq!(a, b);
    }

    #[test]
    fn wrong_magic_is_rejected() {
        let mut buf = Vec::new();
        write_message(&mut buf, "ping", b"").unwrap();
        buf[0] = 0x00;
        let mut cursor = std::io::Cursor::new(buf);
        assert!(read_message(&mut cursor).is_err());
    }

    #[test]
    fn wrong_checksum_is_rejected() {
        let mut buf = Vec::new();
        write_message(&mut buf, "ping", b"data").unwrap();
        let last = buf.len() - 1;
        buf[last] ^= 0xff;
        let mut cursor = std::io::Cursor::new(buf);
        assert!(read_message(&mut cursor).is_err());
    }
}
