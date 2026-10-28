# XEPAY RPC API Reference

All RPC commands require cookie authentication. The cookie file is located at `~/.xepay/.cookie` (or the equivalent for your network).

## Blockchain Methods

- `getinfo`: General node information (height, difficulty, supply, etc).
- `getsupplyinfo`: Detailed supply and inflation information.
  - Returns: `max_supply`, `circulating_supply`, `remaining_supply`, `percent_mined`, `current_era`, and `remaining_blocks_estimate`.
  - Use this to check how many coins are left to be mined and when the next halving will occur.
- `getblockchaininfo`: Detailed blockchain state.
- `getblockcount`: Current height.
- `getbestblockhash`: Hash of the latest block.
- `getblockhash <height>`: Hash of a specific block.
- `getblock <hash>`: Full block details.
- `getblockheader <hash>`: Block header details.
- `getrawtransaction <txid>`: Transaction details.
- `decoderawtransaction <txid>`: Decode a transaction.
- `getmempoolinfo`: Mempool summary.
- `validatemempool`: Validate all transactions in the mempool.
- `verifychain`: Verify the integrity of the entire chain.

## Wallet Methods

- `getbalance [address]`: Balance of an address or the whole wallet.
- `sendtoaddress <amount> <to> [from] [fee] [nonce]`: Send funds. The wallet auto-selects a source address if `from` is omitted.
- `getlistwallet`: List all addresses in the legacy wallet.
- `getcreatewallet`: Create a new Legacy address.
- `getcreatewalletbech32`: Create a new Bech32 address.
- `importprivkey <secret>`: Import a private key.
- `dumpprivkey <address>`: Dump the private key of an address.
- `dumpwallet`: Dump all wallet addresses and private keys.
- `migratewallet`: Migrate legacy wallet to HD wallet (with automatic backup).
- `encryptwallet <password>`: Encrypt the wallet with AES-256-GCM.
- `walletpassphrase <password>`: Unlock the wallet.
- `getaddressinfo <address>`: Get detailed info about an address.
- `setlabel <address> <label>`: Set a label for an address.
- `backupwallet <destination>`: Backup the wallet file.

## HD Wallet Methods

- `createhdwallet [word_count]`: Create a new HD wallet (12 or 24 words).
- `restorehdwallet <mnemonic> [passphrase]`: Restore from mnemonic.
- `dumphdmnemonic`: Show the HD wallet mnemonic.
- `derivehdaddress <index> [legacy|bech32]`: Derive a new address.
- `listhdaddresses`: List all derived addresses.

## Mining Methods

- `generatetoaddress <n> <address>`: Mine n blocks to an address.
- `getblocktemplate`: Get a block template for external miners.
- `submitblock <block_json>`: Submit a mined block.

## Network Methods

- `getnetworkinfo`: Network parameters.
- `getpeerinfo`: List connected peers.
- `getchaintips`: Information about chain tips.
- `getcheckpoints`: List of active checkpoints.

## Control Methods

- `stop`: Stop the node and save the chain.
