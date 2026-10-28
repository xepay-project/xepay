// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Storage layer for XEPAY.

pub mod account;
pub mod adapter;
pub mod state;

pub use account::Account;
pub use state::StateDB;

pub use crate::storage_legacy::*;
