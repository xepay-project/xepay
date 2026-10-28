# Copyright (c) 2026 Aldianokto
# Distributed under the Apache License, see the accompanying
# file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

SHELL := /bin/bash
CARGO ?= cargo
TARGET ?=

.DEFAULT_GOAL := all
.PHONY: all build test fmt fmt-check clippy check audit clean help

ifneq ($(TARGET),)
CARGO_TARGET := --target $(TARGET)
BIN_DIR := target/$(TARGET)/release
else
CARGO_TARGET :=
BIN_DIR := target/release
endif

BINARIES := xepay xepayd xepay-miner xepay-genesis

all: build

check-cargo:
	@command -v $(CARGO) >/dev/null 2>&1 || { \
		echo "cargo not found. Install Rust from https://rustup.rs/"; \
		exit 1; \
	}

build: check-cargo
	$(CARGO) build --release --locked --bins $(CARGO_TARGET)
	@for bin in $(BINARIES); do \
		cp "$(BIN_DIR)/$$bin" "./$$bin"; \
	done
	@echo "Binaries ready: $(BINARIES)"

test: check-cargo
	$(CARGO) test --workspace --all-targets --locked
	$(CARGO) test --workspace --doc --locked

fmt: check-cargo
	$(CARGO) fmt --all

fmt-check: check-cargo
	$(CARGO) fmt --all -- --check

clippy: check-cargo
	$(CARGO) clippy --workspace --all-targets --locked -- -D warnings

check: check-cargo
	$(CARGO) check --workspace --all-targets --locked

audit: check-cargo
	$(CARGO) audit

ci: fmt-check clippy test

clean:
	$(CARGO) clean
	@rm -f $(BINARIES)

help:
	@echo "Available targets:"
	@echo "  all        - build all binaries (default)"
	@echo "  build      - build release binaries and copy them to the repository root"
	@echo "  test       - run all tests, including documentation tests"
	@echo "  fmt        - format the source code"
	@echo "  fmt-check  - check formatting without modifying files"
	@echo "  clippy     - run clippy with warnings treated as errors"
	@echo "  check      - run cargo check on all targets"
	@echo "  audit      - run cargo audit"
	@echo "  ci         - run fmt-check, clippy, and test"
	@echo "  clean      - remove the target directory and the copied binaries"
