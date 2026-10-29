// Copyright (c) 2026 Aldianokto
// Distributed under the Apache License, see the accompanying
// file LICENSE or https://www.apache.org/licenses/LICENSE-2.0.

//! Consensus validation rules for XEPAY.

use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::consensus::rules;
use crate::consensus::spec::{self, Network};
use crate::pow;
use crate::secp256k1::{address, verify};
use crate::transaction::Transaction;
use std::collections::BTreeMap;
use std::fmt;

pub use rules::BLOCK_VERSION;
pub use rules::COINBASE_MATURITY;
pub use rules::MAX_INPUTS_PER_TX;
pub use rules::MAX_NOTE_LEN;
pub use rules::MAX_OUTPUTS_PER_TX;
pub use rules::MAX_TX_PER_BLOCK;
pub use rules::ZERO_HASH;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub code: &'static str,
    pub message: String,
}

impl ValidationError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        ValidationError {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

pub fn rule_address_format(addr: &str, chain: &Blockchain) -> Result<(), ValidationError> {
    let hrp = crate::blockchain::bech32_hrp();
    let is_bech32 = addr.starts_with(&format!("{}1", hrp));

    if crate::blockchain::is_softfork_active("disable_legacy_addresses", chain) && !is_bech32 {
        return Err(ValidationError::new(
            "E008",
            format!("legacy addresses are no longer accepted: {}", addr),
        ));
    }

    if is_bech32 {
        if !address::verify_bech32_address(addr) {
            return Err(ValidationError::new(
                "E001",
                format!("invalid bech32 address: {}", addr),
            ));
        }
        return Ok(());
    }

    if !address::verify_legacy_address(addr) {
        return Err(ValidationError::new(
            "E001",
            format!("invalid legacy address: {}", addr),
        ));
    }
    Ok(())
}

pub fn rule_self_send(tx: &Transaction) -> Result<(), ValidationError> {
    if tx.from == tx.to {
        return Err(ValidationError::new(
            "E004",
            "sender and recipient addresses are identical",
        ));
    }
    Ok(())
}

pub fn rule_amount_nonzero(tx: &Transaction) -> Result<(), ValidationError> {
    if tx.amount == 0 {
        return Err(ValidationError::new(
            "E005",
            "amount must be greater than zero",
        ));
    }
    Ok(())
}

pub fn rule_note_length(tx: &Transaction) -> Result<(), ValidationError> {
    if tx.note.len() > MAX_NOTE_LEN {
        return Err(ValidationError::new(
            "E006",
            format!("note exceeds {} bytes", MAX_NOTE_LEN),
        ));
    }
    Ok(())
}

pub fn rule_signature(tx: &Transaction) -> Result<(), ValidationError> {
    if is_special_tx(tx) {
        return Ok(());
    }
    if tx.public_key.is_empty() {
        return Err(ValidationError::new("E010", "public key is empty"));
    }
    if tx.signature.is_empty() {
        return Err(ValidationError::new("E010", "signature is empty"));
    }
    let pk = hex::decode(&tx.public_key)
        .map_err(|_| ValidationError::new("E010", "public key is not hexadecimal"))?;
    let sig = hex::decode(&tx.signature)
        .map_err(|_| ValidationError::new("E010", "signature is not hexadecimal"))?;
    let msg = hex::decode(tx.signing_hash())
        .map_err(|_| ValidationError::new("E010", "signing hash is not hexadecimal"))?;
    if !verify::verify(&pk, &msg, &sig) {
        return Err(ValidationError::new(
            "E010",
            format!("signature is invalid for txid {}", tx.txid),
        ));
    }
    Ok(())
}

pub fn rule_address_matches_pubkey(tx: &Transaction) -> Result<(), ValidationError> {
    if is_special_tx(tx) {
        return Ok(());
    }
    if tx.public_key.is_empty() {
        return Err(ValidationError::new("E011", "public key is empty"));
    }
    let pk = hex::decode(&tx.public_key)
        .map_err(|_| ValidationError::new("E011", "public key is not hexadecimal"))?;

    let hrp = crate::blockchain::bech32_hrp();
    let expected = if tx.from.starts_with(&format!("{}1", hrp)) {
        address::address_from_public_key_bech32(&pk).map_err(|_| {
            ValidationError::new("E011", "public key cannot be converted to a bech32 address")
        })?
    } else {
        address::address_from_public_key(&pk).map_err(|_| {
            ValidationError::new("E011", "public key cannot be converted to a legacy address")
        })?
    };

    if expected != tx.from {
        return Err(ValidationError::new(
            "E011",
            format!("address {} does not match the public key {}", tx.from, expected),
        ));
    }
    Ok(())
}

pub fn rule_txid_consistent(tx: &Transaction) -> Result<(), ValidationError> {
    if tx.txid != tx.compute_txid() {
        return Err(ValidationError::new(
            "E012",
            format!("txid {} does not match the computed value", tx.txid),
        ));
    }
    Ok(())
}

pub fn rule_nonce(chain: &Blockchain, tx: &Transaction) -> Result<(), ValidationError> {
    if is_special_tx(tx) {
        return Ok(());
    }
    let mut expected = chain.nonce_of(&tx.from);
    loop {
        let has_mempool_tx = chain
            .mempool
            .iter()
            .any(|t| t.from == tx.from && t.nonce == expected);
        if !has_mempool_tx {
            break;
        }
        expected += 1;
    }
    if tx.nonce != expected {
        return Err(ValidationError::new(
            "E022",
            format!("invalid nonce: got {}, expected {}", tx.nonce, expected),
        ));
    }
    Ok(())
}

pub fn rule_sufficient_balance(chain: &Blockchain, tx: &Transaction) -> Result<(), ValidationError> {
    if is_special_tx(tx) {
        return Ok(());
    }
    let balance = chain.mature_balance_at(&tx.from, chain.height());
    let need = tx.amount.saturating_add(tx.fee);
    if balance < need {
        return Err(ValidationError::new(
            "E021",
            format!(
                "insufficient mature balance: required {} sat, available {} sat",
                need, balance
            ),
        ));
    }
    Ok(())
}

pub fn rule_block_version(block: &Block) -> Result<(), ValidationError> {
    if block.nversion != BLOCK_VERSION {
        return Err(ValidationError::new(
            "E028",
            format!(
                "unsupported block version {} at index {}: expected {}",
                block.nversion, block.index, BLOCK_VERSION
            ),
        ));
    }
    Ok(())
}

pub fn rule_block_tx_count(block: &Block) -> Result<(), ValidationError> {
    if block.transactions.len() > MAX_TX_PER_BLOCK {
        return Err(ValidationError::new(
            "E030",
            format!(
                "block contains {} transactions, exceeding the limit of {}",
                block.transactions.len(),
                MAX_TX_PER_BLOCK
            ),
        ));
    }
    Ok(())
}

pub fn rule_block_subsidy(block: &Block) -> Result<(), ValidationError> {
    let expected = Blockchain::calculate_reward(block.index);
    if block.subsidy != expected {
        return Err(ValidationError::new(
            "E031",
            format!(
                "subsidy for block {} does not match the era: got {}, expected {}",
                block.index, block.subsidy, expected
            ),
        ));
    }
    Ok(())
}

pub fn rule_block_reward(block: &Block) -> Result<(), ValidationError> {
    let expected = block.subsidy.saturating_add(block.total_fees);
    if block.reward != expected {
        return Err(ValidationError::new(
            "E046",
            format!(
                "reward for block {} is invalid: got {}, expected {} (subsidy {} + fees {})",
                block.index, block.reward, expected, block.subsidy, block.total_fees
            ),
        ));
    }
    Ok(())
}

pub fn rule_block_total_fees(block: &Block) -> Result<(), ValidationError> {
    let computed: u64 = block.transactions.iter().map(|tx| tx.fee).sum();
    if block.total_fees != computed {
        return Err(ValidationError::new(
            "E047",
            format!(
                "total_fees for block {} is invalid: got {}, expected {}",
                block.index, block.total_fees, computed
            ),
        ));
    }
    Ok(())
}

pub fn rule_block_coinbase_consistent(block: &Block) -> Result<(), ValidationError> {
    if block.index == 0 {
        return Ok(());
    }
    let expected = xepay_crypto::sha3_256_hex(
        format!("{}|{}|{}", block.index, block.mined_for, block.timestamp).as_bytes(),
    );
    if block.coinbase_txid != expected {
        return Err(ValidationError::new(
            "E048",
            format!(
                "coinbase_txid for block {} is inconsistent with index, mined_for, and timestamp",
                block.index
            ),
        ));
    }
    Ok(())
}

pub fn rule_mined_for_valid(block: &Block, chain: &Blockchain) -> Result<(), ValidationError> {
    if block.index == 0 {
        return Ok(());
    }
    if block.mined_for.is_empty() {
        return Err(ValidationError::new(
            "E049",
            format!("block {} has an empty mined_for address", block.index),
        ));
    }
    rule_address_format(&block.mined_for, chain).map_err(|_| {
        ValidationError::new(
            "E049",
            format!(
                "mined_for address of block {} is invalid: {}",
                block.index, block.mined_for
            ),
        )
    })
}

pub fn rule_block_timestamp_monotonic(prev: &Block, cur: &Block) -> Result<(), ValidationError> {
    // Rule 1: Timestamp must not be earlier than the previous block.
    if cur.timestamp < prev.timestamp {
        return Err(ValidationError::new(
            "E032",
            format!(
                "timestamp of block {} is earlier than block {}",
                cur.index, prev.index
            ),
        ));
    }

    // Rule 2: Future Time Limit.
    // Reject blocks whose timestamp is more than 2 hours ahead of the current local time.
    // This prevents miners from manipulating the timestamp to abuse difficulty adjustments.
    let now = crate::utils::now_millis();
    let max_future_time = now.saturating_add(2 * 60 * 60 * 1000); // 2 hours in milliseconds

    if cur.timestamp > max_future_time {
        return Err(ValidationError::new(
            "E050",
            format!(
                "timestamp of block {} is too far in the future (max allowed: {}ms ahead)",
                cur.index,
                (cur.timestamp - now) / 1000
            ),
        ));
    }

    Ok(())
}

pub fn rule_block_nbits(chain: &Blockchain, block: &Block) -> Result<(), ValidationError> {
    if block.nbits != chain.nbits {
        return Err(ValidationError::new(
            "E033",
            format!(
                "nbits of block {:#010x} does not match chain nbits {:#010x}",
                block.nbits, chain.nbits
            ),
        ));
    }
    Ok(())
}

pub fn rule_block_hash_consistent(block: &Block) -> Result<(), ValidationError> {
    if block.hash != block.calculate_hash() {
        return Err(ValidationError::new(
            "E034",
            format!(
                "hash of block {} is not consistent with its contents",
                block.index
            ),
        ));
    }
    Ok(())
}

pub fn rule_block_previous_hash(prev: &Block, cur: &Block) -> Result<(), ValidationError> {
    if cur.previous_hash != prev.hash {
        return Err(ValidationError::new(
            "E035",
            format!(
                "previous_hash of block {} does not match the hash of block {}",
                cur.index, prev.index
            ),
        ));
    }
    Ok(())
}

pub fn rule_block_index_sequential(prev: &Block, cur: &Block) -> Result<(), ValidationError> {
    if cur.index != prev.index + 1 {
        return Err(ValidationError::new(
            "E036",
            format!(
                "index of block {} is not sequential after block {}",
                cur.index, prev.index
            ),
        ));
    }
    Ok(())
}

pub fn rule_block_pow(block: &Block) -> Result<(), ValidationError> {
    if !pow::hash_meets_nbits(&block.hash, block.nbits) {
        return Err(ValidationError::new(
            "E037",
            format!(
                "hash of block {} does not meet the target nbits {:#010x}",
                block.index, block.nbits
            ),
        ));
    }
    Ok(())
}

pub fn rule_block_checkpoint(block: &Block) -> Result<(), ValidationError> {
    if !crate::blockchain::checkpoint::verify(block.index, &block.hash) {
        return Err(ValidationError::new(
            "E038",
            format!("block {} violates a checkpoint", block.index),
        ));
    }
    Ok(())
}

pub fn rule_genesis(block: &Block) -> Result<(), ValidationError> {
    if block.index != 0 {
        return Err(ValidationError::new(
            "E039",
            "the genesis block must have index 0",
        ));
    }
    if block.previous_hash != ZERO_HASH {
        return Err(ValidationError::new(
            "E039",
            "the genesis block previous_hash must be zero",
        ));
    }
    Ok(())
}

pub fn rule_total_supply(chain: &Blockchain) -> Result<(), ValidationError> {
    let total = chain.circulating_supply();
    let max = crate::blockchain::max_supply();
    if total > max {
        return Err(ValidationError::new(
            "E041",
            format!(
                "total supply {} sat exceeds the maximum of {} sat",
                total, max
            ),
        ));
    }
    Ok(())
}

pub fn rule_genesis_fingerprint_for(block: &Block, network: Network) -> Result<(), ValidationError> {
    let expected_txid = crate::blockchain::genesis_coinbase_txid_for(network);
    if block.coinbase_txid != expected_txid {
        return Err(ValidationError::new(
            "E042",
            format!(
                "genesis coinbase_txid does not match the consensus fingerprint: got {}, expected {}",
                block.coinbase_txid, expected_txid
            ),
        ));
    }
    Ok(())
}

pub fn rule_genesis_fingerprint(block: &Block) -> Result<(), ValidationError> {
    rule_genesis_fingerprint_for(block, spec::current_network())
}

pub fn rule_block_merkle_root(block: &Block) -> Result<(), ValidationError> {
    let expected = Block::compute_merkle_root(&block.transactions);
    if block.merkle_root != expected {
        return Err(ValidationError::new(
            "E043",
            format!(
                "merkle root of block {} does not match: got {}, expected {}",
                block.index, block.merkle_root, expected
            ),
        ));
    }
    Ok(())
}

pub fn rule_genesis_output_script(block: &Block) -> Result<(), ValidationError> {
    if block.index != 0 {
        return Ok(());
    }
    let expected = crate::blockchain::genesis_output_script();
    if block.genesis_output_script != expected {
        return Err(ValidationError::new(
            "E044",
            format!(
                "genesis output script does not match: got {}, expected {}",
                block.genesis_output_script, expected
            ),
        ));
    }
    Ok(())
}

pub fn rule_genesis_pow(block: &Block) -> Result<(), ValidationError> {
    if !pow::hash_meets_nbits(&block.hash, block.nbits) {
        return Err(ValidationError::new(
            "E045",
            format!(
                "genesis hash does not meet the target nbits {:#010x}",
                block.nbits
            ),
        ));
    }
    Ok(())
}

pub fn is_special_tx(tx: &Transaction) -> bool {
    tx.from == "COINBASE" || tx.from == "GENESIS" || tx.from == "NETWORK" || tx.from == "MEMPOOL"
}

pub fn is_valid_address(addr: &str, chain: &Blockchain) -> bool {
    rule_address_format(addr, chain).is_ok()
}

pub fn validate_transaction(chain: &Blockchain, tx: &Transaction) -> Result<(), ValidationError> {
    if is_special_tx(tx) {
        return Ok(());
    }

    rule_txid_consistent(tx)?;
    rule_address_format(&tx.from, chain)?;
    rule_address_format(&tx.to, chain)?;
    rule_self_send(tx)?;
    rule_amount_nonzero(tx)?;
    rule_note_length(tx)?;
    rule_signature(tx)?;
    rule_address_matches_pubkey(tx)?;
    rule_nonce(chain, tx)?;
    rule_sufficient_balance(chain, tx)?;

    Ok(())
}

pub fn validate_block_transactions(
    chain: &mut Blockchain,
    transactions: &[Transaction],
    _height: u64,
) -> Result<(), ValidationError> {
    let mut overlay_balances: BTreeMap<String, u64> = BTreeMap::new();
    let mut overlay_nonces: BTreeMap<String, u64> = BTreeMap::new();

    for tx in transactions {
        if is_special_tx(tx) {
            continue;
        }

        let base_nonce = overlay_nonces
            .get(&tx.from)
            .copied()
            .unwrap_or_else(|| chain.nonce_of(&tx.from));
        if tx.nonce != base_nonce {
            return Err(ValidationError::new(
                "E022",
                format!("invalid nonce for {}: got {}, expected {}", tx.from, tx.nonce, base_nonce),
            ));
        }

        let base_balance = overlay_balances
            .get(&tx.from)
            .copied()
            .unwrap_or_else(|| chain.mature_balance_at(&tx.from, chain.height()));

        let need = match tx.amount.checked_add(tx.fee) {
            Some(v) => v,
            None => return Err(ValidationError::new("E023", "amount + fee overflows u64")),
        };
        if base_balance < need {
            return Err(ValidationError::new(
                "E021",
                format!("insufficient mature balance for {}: required {} sat, available {} sat", tx.from, need, base_balance),
            ));
        }

        let new_from = base_balance - need;
        overlay_balances.insert(tx.from.clone(), new_from);

        let to_balance = overlay_balances
            .get(&tx.to)
            .copied()
            .unwrap_or_else(|| chain.balance_of(&tx.to));
        let new_to = match to_balance.checked_add(tx.amount) {
            Some(v) => v,
            None => return Err(ValidationError::new("E024", "recipient balance overflows u64")),
        };
        overlay_balances.insert(tx.to.clone(), new_to);

        overlay_nonces.insert(tx.from.clone(), base_nonce + 1);
    }

    Ok(())
}

pub fn validate_block(
    chain: &mut Blockchain,
    prev: &Block,
    block: &Block,
) -> Result<(), ValidationError> {
    rule_block_version(block)?;
    rule_block_index_sequential(prev, block)?;
    rule_block_hash_consistent(block)?;
    rule_block_previous_hash(prev, block)?;
    rule_block_merkle_root(block)?;
    rule_block_subsidy(block)?;
    rule_block_total_fees(block)?;
    rule_block_reward(block)?;
    rule_block_coinbase_consistent(block)?;
    rule_mined_for_valid(block, chain)?;
    rule_block_pow(block)?;
    rule_block_tx_count(block)?;
    rule_block_timestamp_monotonic(prev, block)?;
    rule_block_checkpoint(block)?;
    rule_block_nbits(chain, block)?;
    rule_block_chainwork(prev, block)?;
    validate_block_transactions(chain, &block.transactions, block.index)?;
    Ok(())
}

pub fn rule_block_chainwork(prev: &Block, cur: &Block) -> Result<(), ValidationError> {
    let expected_block_work = Block::compute_work(cur.nbits);
    let expected_chainwork = prev.chainwork.saturating_add(expected_block_work);
    if cur.chainwork != expected_chainwork {
        return Err(ValidationError::new(
            "E051",
            format!(
                "chainwork of block {} is invalid: got {}, expected {}",
                cur.index, cur.chainwork, expected_chainwork
            ),
        ));
    }
    Ok(())
}

pub fn validate_chain(chain: &mut Blockchain) -> Result<(), ValidationError> {
    if chain.chain.is_empty() {
        return Err(ValidationError::new("E040", "chain is empty"));
    }

    let (genesis_txs, genesis_index) = {
        let genesis = &chain.chain[0];
        rule_genesis(genesis)?;
        rule_genesis_fingerprint(genesis)?;
        rule_genesis_output_script(genesis)?;
        rule_genesis_pow(genesis)?;
        rule_block_hash_consistent(genesis)?;
        rule_block_merkle_root(genesis)?;
        rule_block_version(genesis)?;
        (genesis.transactions.clone(), genesis.index)
    };
    rule_total_supply(chain)?;

    if !genesis_txs.is_empty() {
        validate_block_transactions(chain, &genesis_txs, genesis_index)?;
    }

    for i in 1..chain.chain.len() {
        let (prev_hash, prev_index, prev_timestamp, prev_chainwork) = {
            let prev = &chain.chain[i - 1];
            (prev.hash.clone(), prev.index, prev.timestamp, prev.chainwork)
        };
        let (
            _cur_hash,
            cur_index,
            cur_nversion,
            cur_previous_hash,
            _cur_merkle_root,
            cur_subsidy,
            cur_total_fees,
            cur_reward,
            cur_coinbase_txid,
            cur_mined_for,
            cur_nbits,
            cur_timestamp,
            cur_tx_count,
            cur_chainwork,
            cur_transactions,
        ) = {
            let cur = &chain.chain[i];
            (
                cur.hash.clone(),
                cur.index,
                cur.nversion,
                cur.previous_hash.clone(),
                cur.merkle_root.clone(),
                cur.subsidy,
                cur.total_fees,
                cur.reward,
                cur.coinbase_txid.clone(),
                cur.mined_for.clone(),
                cur.nbits,
                cur.timestamp,
                cur.transactions.len(),
                cur.chainwork,
                cur.transactions.clone(),
            )
        };

        if cur_nversion != BLOCK_VERSION {
            return Err(ValidationError::new("E028",
                format!("unsupported block version {} at index {}: expected {}", cur_nversion, cur_index, BLOCK_VERSION)));
        }
        if cur_index != prev_index + 1 {
            return Err(ValidationError::new("E036",
                format!("index of block {} is not sequential after block {}", cur_index, prev_index)));
        }
        if cur_previous_hash != prev_hash {
            return Err(ValidationError::new("E035",
                format!("previous_hash of block {} does not match the hash of block {}", cur_index, prev_index)));
        }
        {
            let cur = &chain.chain[i];
            if cur.hash != cur.calculate_hash() {
                return Err(ValidationError::new("E034",
                    format!("hash of block {} is not consistent with its contents", cur_index)));
            }
            let expected_merkle = Block::compute_merkle_root(&cur.transactions);
            if cur.merkle_root != expected_merkle {
                return Err(ValidationError::new("E043",
                    format!("merkle root of block {} does not match: got {}, expected {}", cur_index, cur.merkle_root, expected_merkle)));
            }
            if !pow::hash_meets_nbits(&cur.hash, cur.nbits) {
                return Err(ValidationError::new("E037",
                    format!("hash of block {} does not meet the target nbits {:#010x}", cur_index, cur.nbits)));
            }
            if !crate::blockchain::checkpoint::verify(cur.index, &cur.hash) {
                return Err(ValidationError::new("E038",
                    format!("block {} violates a checkpoint", cur_index)));
            }
        }
        let expected_subsidy = Blockchain::calculate_reward(cur_index);
        if cur_subsidy != expected_subsidy {
            return Err(ValidationError::new("E031",
                format!("subsidy for block {} does not match the era: got {}, expected {}", cur_index, cur_subsidy, expected_subsidy)));
        }
        let expected_reward = cur_subsidy.saturating_add(cur_total_fees);
        if cur_reward != expected_reward {
            return Err(ValidationError::new("E046",
                format!("reward for block {} is invalid: got {}, expected {} (subsidy {} + fees {})", cur_index, cur_reward, expected_reward, cur_subsidy, cur_total_fees)));
        }
        if cur_index != 0 {
            let expected_txid = xepay_crypto::sha3_256_hex(
                format!("{}|{}|{}", cur_index, cur_mined_for, cur_timestamp).as_bytes(),
            );
            if cur_coinbase_txid != expected_txid {
                return Err(ValidationError::new("E048",
                    format!("coinbase_txid for block {} is inconsistent with index, mined_for, and timestamp", cur_index)));
            }
            if cur_mined_for.is_empty() {
                return Err(ValidationError::new("E049",
                    format!("block {} has an empty mined_for address", cur_index)));
            }
            rule_address_format(&cur_mined_for, chain).map_err(|_| {
                ValidationError::new("E049",
                    format!("mined_for address of block {} is invalid: {}", cur_index, cur_mined_for))
            })?;
        }
        if cur_tx_count > MAX_TX_PER_BLOCK {
            return Err(ValidationError::new("E030",
                format!("block contains {} transactions, exceeding the limit of {}", cur_tx_count, MAX_TX_PER_BLOCK)));
        }
        if cur_timestamp < prev_timestamp {
            return Err(ValidationError::new("E032",
                format!("timestamp of block {} is earlier than block {}", cur_index, prev_index)));
        }

        let now = crate::utils::now_millis();
        let max_future_time = now.saturating_add(2 * 60 * 60 * 1000);
        if cur_timestamp > max_future_time {
             return Err(ValidationError::new("E050",
                format!("timestamp of block {} is too far in the future", cur_index)));
        }

        if cur_nbits != chain.nbits {
            return Err(ValidationError::new("E033",
                format!("nbits of block {:#010x} does not match chain nbits {:#010x}", cur_nbits, chain.nbits)));
        }

        let expected_block_work = Block::compute_work(cur_nbits);
        let expected_chainwork = prev_chainwork.saturating_add(expected_block_work);
        if cur_chainwork != expected_chainwork {
            return Err(ValidationError::new("E051",
                format!("chainwork of block {} is invalid: got {}, expected {}", cur_index, cur_chainwork, expected_chainwork)));
        }

        validate_block_transactions(chain, &cur_transactions, cur_index)?;
    }

    Ok(())
}

pub fn validate_mempool(chain: &Blockchain) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    for tx in &chain.mempool {
        if let Err(e) = validate_transaction(chain, tx) {
            errors.push(e);
        }
    }
    errors
}
