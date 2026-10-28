# XEPAY Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in XEPAY, please do not open a public issue. Contact the maintainers directly.

## Best Practices

1. Never share your private keys or mnemonic phrase.
2. Always verify the checksum of downloaded binaries.
3. Use a hardware wallet for large amounts.
4. Keep your node software up to date.
5. Run your node behind a firewall if possible.
6. Back up your wallet files regularly.

## Threat Model

XEPAY is designed to be resistant to:
- Eclipse attacks (via anchors and asmap).
- Sybil attacks (via peer diversity).
- Double-spend attempts (via consensus rules and PoW).
- State corruption (via strict state root verification and manual reindex).
- Integer overflow (via `checked_add` and `checked_mul` in all critical paths).

## Cryptography

- SHA3-256 for hashing.
- secp256k1 for ECDSA signatures.
- AES-256-GCM for wallet encryption.
- PBKDF2-HMAC-SHA256 for key derivation.
