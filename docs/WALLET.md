# XEPAY Wallet Guide

XEPAY supports two types of wallets: Legacy and HD (Hierarchical Deterministic).

## Legacy Wallet (`wallet.dat`)

- Create Legacy: `./xepay getcreatewallet`
- Create Bech32: `./xepay getcreatewalletbech32`
- List Addresses: `./xepay getlistwallet`
- Dump Private Key: `./xepay dumpprivkey "<address>"`
- Import Private Key: `./xepay importprivkey "<secret>"`
- Backup: `./xepay backupwallet "<destination>"`

## HD Wallet (`hdwallet.dat`)

- Create: `./xepay createhdwallet 12`
- Restore: `./xepay restorehdwallet "<mnemonic>"`
- Derive Address: `./xepay derivehdaddress <index> [legacy|bech32]`
- List Addresses: `./xepay listhdaddresses`
- Dump Mnemonic: `./xepay dumphdmnemonic`

## Migration

You can migrate your legacy wallet to an HD wallet using the `migratewallet` command.

    ./xepay migratewallet

## Encryption

- Encrypt: `./xepay encryptwallet "<password>"`
- Unlock: `./xepay walletpassphrase "<password>"`

**Important:** Always back up your `wallet.dat` and `hdwallet.dat` files. If you lose your mnemonic phrase and your wallet file, your funds are lost forever.
