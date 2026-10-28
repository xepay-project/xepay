// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

use xepay_crypto::sha3_256_hex;

#[test]
fn sha3_256_empty() {
    let h = sha3_256_hex(b"");
    assert_eq!(
        h,
        "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"
    );
}

#[test]
fn sha3_256_abc() {
    let h = sha3_256_hex(b"abc");
    assert_eq!(
        h,
        "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"
    );
}

#[test]
fn sha3_256_output_length() {
    let h = sha3_256_hex(b"XEPAY");
    assert_eq!(h.len(), 64);
}

#[test]
fn sha3_256_is_deterministic() {
    let a = sha3_256_hex(b"xepay");
    let b = sha3_256_hex(b"xepay");
    assert_eq!(a, b);
}
