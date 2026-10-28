# XEPAY Consensus Rules

XEPAY uses a strict set of rules to ensure all nodes agree on the state of the blockchain. Any change to these rules requires a hard fork or a new genesis block.

## Economic Parameters

| Parameter | Value |
|---|---|
| Max Supply | 485,000,000 XEPC |
| Initial Reward | 15 XEPC |
| Halving Interval | 16,166,666 blocks |
| Block Time | 15 seconds |
| Difficulty Adjust | 69,120 blocks |

## Hashing and Signatures

- **Hash Algorithm:** SHA3-256
- **Signature Algorithm:** secp256k1-ECDSA

## Block Structure

- **Block Version:** 1
- **Max Transactions per Block:** 10,000
- **Max Note Length:** 4,096 bytes
- **Coinbase Maturity:** 100 blocks
- **Target Spacing:** 15 seconds
- **Target Timespan:** 1,036,800 seconds (69,120 blocks * 15 seconds)

## Genesis Blocks

Each network (Mainnet, Testnet, Regtest) has a unique genesis block. The genesis block is locked forever. Any change to the consensus parameters requires mining a new genesis block.

## Validation Rules

All nodes must enforce the following rules:
- Block version must be 1.
- The previous hash must match the tip of the chain.
- The block hash must meet the target difficulty.
- The coinbase transaction must be valid (subsidy + fees).
- All transactions must have valid signatures and nonces.
- The state root must match the computed state after applying all transactions.

## Fee-Based Security

As the block subsidy decreases over time due to halving, miners rely on transaction fees to secure the network. XEPAY enforces a minimum relay fee (`MIN_RELAY_FEE_PER_BYTE`) to prevent spam and ensure miners are compensated.
