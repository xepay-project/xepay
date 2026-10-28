#!/usr/bin/env bash
# Copyright (c) 2026 Aldianokto
# Distributed under the Apache License, see the accompanying
# file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo not found. Install Rust from https://rustup.rs/"
    exit 1
fi

TARGET="${1:-}"

if [ -n "$TARGET" ]; then
    cargo build --release --locked --target "$TARGET" --bins
    BIN_DIR="target/$TARGET/release"
else
    cargo build --release --locked --bins
    BIN_DIR="target/release"
fi

BINARIES=(xepay xepayd xepay-miner xepay-genesis)

for bin in "${BINARIES[@]}"; do
    cp "$BIN_DIR/$bin" "./$bin"
    chmod +x "./$bin"
done

echo "Binaries ready:"
for bin in "${BINARIES[@]}"; do
    echo "  ./$bin"
done
