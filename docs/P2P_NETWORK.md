# XEPAY P2P Network

The XEPAY peer-to-peer network uses raw TCP with a custom framing protocol.

## Message Framing

Each message has a header containing:
- Network Magic (4 bytes)
- Command (12 bytes)
- Payload Length (4 bytes)
- Checksum (4 bytes)

## Handshake

When two nodes connect, they exchange `version` and `verack` messages to establish compatibility and share their current chain height.

## Anti-Eclipse Mechanisms

1. **Anchors:** Persists up to 2 block-relay-only connections across restarts.
2. **Banman:** Manual ban list for malicious peers.
3. **ASMap:** Maps IP addresses to Autonomous System Numbers to prevent Sybil attacks.

## Synchronization

Nodes use a `getheaders` / `headers` / `getdata` / `block` flow to synchronize the blockchain. A moving download window ensures efficient use of bandwidth.
