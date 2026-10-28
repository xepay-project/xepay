#!/usr/bin/env bash
# Copyright (c) 2026 Aldianokto
# Distributed under the Apache License, see the accompanying
# file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILDER_DIR="$SCRIPT_DIR/gitian-builder"

build=false
sign=false
verify=false
clean=false
backend="kvm"
linux=true
windows=true
osx=false
signer=""
commit=""
proc=2
mem=2000

usage() {
    cat <<EOF
Usage: $0 [options] signer commit

Options:
  -b|--build          Build binaries
  -s|--sign           Sign the build output
  -v|--verify         Verify the build output
  -c|--clean          Remove the build directory before building
  -o|--os <os>        Operating systems: l (Linux), w (Windows), x (macOS). Default lw.
  -B|--backend <b>    Build backend: kvm, lxc, or docker. Default kvm.
  -j <n>              Number of processes. Default 2.
  -m <n>              Memory in MiB. Default 2000.
  -h|--help           Show this help message.

Example:
  $0 -b -o lw -j 4 -m 4000 mykey abc123...
EOF
}

while :; do
    case "${1:-}" in
        -b|--build) build=true; shift ;;
        -s|--sign) sign=true; shift ;;
        -v|--verify) verify=true; shift ;;
        -c|--clean) clean=true; shift ;;
        -o|--os)
            os="${2:-}"
            linux=false; windows=false; osx=false
            [[ "$os" == *l* ]] && linux=true
            [[ "$os" == *w* ]] && windows=true
            [[ "$os" == *x* ]] && osx=true
            shift 2
            ;;
        -B|--backend) backend="${2:-}"; shift 2 ;;
        -j) proc="${2:-}"; shift 2 ;;
        -m) mem="${2:-}"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) break ;;
    esac
done

signer="${1:-}"
commit="${2:-}"

if [ -z "$signer" ] || [ -z "$commit" ]; then
    usage
    exit 1
fi

if ! [[ "$proc" =~ ^[0-9]+$ ]] || [ "$proc" -lt 1 ]; then
    echo "Error: -j must be a positive integer"
    exit 1
fi

if ! [[ "$mem" =~ ^[0-9]+$ ]] || [ "$mem" -lt 1000 ]; then
    echo "Error: -m must be an integer greater than or equal to 1000"
    exit 1
fi

if [ "$linux" = false ] && [ "$windows" = false ] && [ "$osx" = false ]; then
    echo "Error: no operating system selected"
    exit 1
fi

if [ ! -d "$BUILDER_DIR" ]; then
    echo "Error: gitian-builder directory not found at $BUILDER_DIR"
    echo "Clone it first:"
    echo "  git clone https://github.com/devrandom/gitian-builder.git $BUILDER_DIR"
    exit 1
fi

for cmd in git; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "Error: required command not found: $cmd"
        exit 1
    fi
done

if [ "$clean" = true ]; then
    echo "Cleaning build directory..."
    rm -rf "$BUILDER_DIR/build"
fi

run_gbuild() {
    local descriptor="$1"
    local name="$2"
    echo "Building $name..."
    if [ ! -f "$REPO_DIR/contrib/gitian-descriptors/$descriptor" ]; then
        echo "Error: descriptor not found: $descriptor"
        exit 1
    fi
    pushd "$BUILDER_DIR" >/dev/null
    trap 'popd >/dev/null' EXIT
    ./bin/gbuild --backend "$backend" -j "$proc" -m "$mem" \
        --commit xepay="$commit" \
        "$REPO_DIR/contrib/gitian-descriptors/$descriptor"
    popd >/dev/null
    trap - EXIT
}

run_gsign() {
    local descriptor="$1"
    echo "Signing $descriptor..."
    pushd "$BUILDER_DIR" >/dev/null
    trap 'popd >/dev/null' EXIT
    ./bin/gsign -p "$signer" -r xepay --release "$commit" \
        "$REPO_DIR/contrib/gitian-descriptors/$descriptor"
    popd >/dev/null
    trap - EXIT
}

run_gverify() {
    local descriptor="$1"
    echo "Verifying $descriptor..."
    pushd "$BUILDER_DIR" >/dev/null
    trap 'popd >/dev/null' EXIT
    ./bin/gverify -r xepay --release "$commit" \
        "$REPO_DIR/contrib/gitian-descriptors/$descriptor"
    popd >/dev/null
    trap - EXIT
}

if [ "$build" = true ]; then
    [ "$linux" = true ] && run_gbuild "gitian-linux.yml" "Linux"
    [ "$windows" = true ] && run_gbuild "gitian-win.yml" "Windows"
    [ "$osx" = true ] && run_gbuild "gitian-osx.yml" "macOS"
fi

if [ "$sign" = true ]; then
    [ "$linux" = true ] && run_gsign "gitian-linux.yml"
    [ "$windows" = true ] && run_gsign "gitian-win.yml"
    [ "$osx" = true ] && run_gsign "gitian-osx.yml"
fi

if [ "$verify" = true ]; then
    [ "$linux" = true ] && run_gverify "gitian-linux.yml"
    [ "$windows" = true ] && run_gverify "gitian-win.yml"
    [ "$osx" = true ] && run_gverify "gitian-osx.yml"
fi

echo
echo "Completed. Check the output in $BUILDER_DIR/build/out/"
