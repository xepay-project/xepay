// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

use xepay::blockchain::{self, Blockchain};

#[test]
fn reward_era_0() {
    let r = Blockchain::calculate_reward(0);
    assert_eq!(r, blockchain::initial_reward());
}

#[test]
fn reward_era_1() {
    let r = Blockchain::calculate_reward(blockchain::subsidy_halving_interval());
    assert_eq!(r, blockchain::initial_reward() >> 1);
}

#[test]
fn reward_era_2() {
    let r = Blockchain::calculate_reward(blockchain::subsidy_halving_interval() * 2);
    assert_eq!(r, blockchain::initial_reward() >> 2);
}

#[test]
fn reward_at_max_halvings_is_zero() {
    let height = blockchain::subsidy_halving_interval() * (blockchain::max_halvings() as u64);
    let r = Blockchain::calculate_reward(height);
    assert_eq!(r, 0);
}

#[test]
fn reward_beyond_max_halvings_is_zero() {
    let height = blockchain::subsidy_halving_interval() * (blockchain::max_halvings() as u64 + 1);
    let r = Blockchain::calculate_reward(height);
    assert_eq!(r, 0);
}
