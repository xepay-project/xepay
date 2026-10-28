# Contrib

This directory contains scripts and descriptors for the Gitian build
process.

## Contents

- `gitian-build.sh` — script to run Gitian build, sign, and verify steps.
- `gitian-descriptors/` — build recipes for Linux, Windows, and macOS.
- `gitian-keys/` — directory for developer PGP keys. Private keys must never be committed.

## Gitian

Gitian is a deterministic build process used by Bitcoin and Dogecoin to
produce release binaries. Several independent developers build the
binaries from the same source, sign the results, and the binaries are
considered safe only if the hashes match.

Gitian cannot run on the standard GitHub Actions runner because it
requires a virtual machine (KVM, LXC, or Docker). The build is performed
on an isolated virtual machine on a local machine or a dedicated server.

### Setup

1. Clone the required repositories:

```bash
mkdir -p ~/gitian-build && cd ~/gitian-build
git clone https://github.com/devrandom/gitian-builder.git
git clone https://github.com/xepay-project/xepay.git xepay
git clone https://github.com/xepay-project/xepay.sigs.git xepay.sigs
