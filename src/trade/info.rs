use std::vec;
use std::{str::FromStr, sync::Arc};

use base64::Engine;
use base64::engine::general_purpose;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_client::{
    nonblocking::rpc_client::RpcClient, rpc_client::GetConfirmedSignaturesForAddress2Config,
    rpc_config::RpcTransactionConfig, rpc_response::RpcConfirmedTransactionStatusWithSignature,
};
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::TransactionVersion;
use solana_sdk::{message::Message, pubkey::Pubkey};
use solana_transaction_status::option_serializer::OptionSerializer;
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, EncodedTransaction, UiMessage, UiParsedInstruction,
    UiTransactionEncoding, UiTransactionTokenBalance,
};

use crate::global::{
    METEORA_DAMM_V2_PROGRAM_ID, METEORA_DLMM_V2_PROGRAM_ID, METEORA_DYNAMIC_BOND_CURVE_PROGRAM_ID,
    METEORA_POOL_PROGRAM_ID, ORCA_WHIRLPOOLS_PROGRAM_ID, PUMP_AAM_PROGRAM_ID,
    PUMP_BOND_CURVE_PROGRAM_ID, RAYDIUM_CLMM_POOL_PROGRAM_ID, RAYDIUM_CPMM_POOL_PROGRAM_ID,
    RAYDIUM_LAUNCHPAD_PROGRAM_ID, RAYDIUM_V4_POOL_PROGRAM_ID, SOL, USDC, USDT,
};
use crate::trade::Trade;
use crate::trade::pump::PumpBondCurveTransactionInfo;
use crate::types::{DexProgramType, Direction, TransactionType, UnifiedError, UnifiedResult};

/// a more readable transaction information structure.
#[derive(Debug, Clone)]
pub struct TransactionInfo {
    // Basic Identification Fields
    pub transaction_hash: String,
    pub signature: String,
    // Account Related Fields
    pub from: String,
    pub to: String,
    pub signer: String,
    pub fee_payer: String,
    pub signers: Vec<String>,           // All signers
    pub involved_accounts: Vec<String>, // All involved accounts
    pub writable_accounts: Vec<String>, // Writable accounts
    pub readonly_accounts: Vec<String>, // Read-only accounts
    // Amount Related Fields
    pub value: String,       // Transfer amount in lamports
    pub value_sol: f64,      // Transfer amount in SOL
    pub fee: u64,            // Transaction fee in lamports
    pub fee_sol: f64,        // Transaction fee in SOL
    pub pre_balance: u64,    // Balance before transaction
    pub post_balance: u64,   // Balance after transaction
    pub balance_change: i64, // Balance change (signed)
    // Block Related Fields
    pub block_number: u64,
    pub block_hash: String, // Block hash
    pub block_time: Option<i64>,
    pub slot: u64,
    pub epoch: u64,               // Epoch number
    pub recent_blockhash: String, // Recent blockhash used
    // Status Related Fields
    pub status: String,              // "success", "failed"
    pub confirmation_status: String, // "processed", "confirmed", "finalized"
    pub error_message: Option<String>,
    pub err: Option<Value>, // Raw error information
    pub is_confirmed: bool,
    pub is_finalized: bool,
    // Transaction Type Related
    pub transaction_type: Option<TransactionType>,
    pub program_id: String,
    pub instructions_count: u64,
    pub inner_instructions_count: u64, // Number of inner instructions
    pub version: u8,                   // Transaction version
    // Resource Consumption
    pub compute_units_consumed: Option<u64>, // Compute units consumed
    pub compute_unit_price: Option<u64>,     // Compute unit price
    // Instructions and Logs
    pub log_index: u64,
    pub data: Option<String>,
    pub logs: Vec<String>,
    pub instructions: Vec<InstructionInfo>, // Instruction details
    pub inner_instructions: Vec<InnerInstructionInfo>, // Inner instructions
    // Token Related
    pub token_mint: Option<String>,
    pub token_amount: Option<String>,
    pub token_decimals: Option<u8>,
    pub token_symbol: Option<String>,           // Token symbol
    pub token_name: Option<String>,             // Token name
    pub pre_token_balances: Vec<TokenBalance>,  // Token balances before transaction
    pub post_token_balances: Vec<TokenBalance>, // Token balances after transaction
    // NFT Related
    pub is_nft_transfer: bool,
    pub nft_mint: Option<String>,
    pub nft_name: Option<String>,
    pub nft_symbol: Option<String>,
    // DEX/DeFi Related
    pub is_swap: bool,
    pub dex_program_id: Option<String>,           // DEX program id
    pub dex_program_type: Option<DexProgramType>, // DEX program name
    pub dex_pool_program_id: Option<String>,      // DEX program pool id
    pub dex_pool_program_name: Option<String>,    // DEX program pool name
    pub input_mint: Option<String>,               // Input token mint
    pub output_mint: Option<String>,              // Output token mint
    pub input_amount: Option<u64>,                // Input amount
    pub output_amount: Option<u64>,               // Output amount
    // Business Extension Fields
    pub memo: Option<String>,
    pub timestamp: Option<u64>,
    pub tags: Vec<String>,
    pub category: String,          // Business category
    pub risk_level: u8,            // Risk level 0-10
    pub is_internal: bool,         // Whether internal transaction
    pub gas_used: Option<u64>,     // Gas used
    pub gas_price: Option<u64>,    // Gas price
    pub max_fee: Option<u64>,      // Maximum fee
    pub priority_fee: Option<u64>, // Priority fee
    // Network Related
    pub cluster: String,  // Cluster information
    pub rpc_node: String, // RPC node information
    // Metadata
    pub created_at: u64, // Record creation timestamp
    pub updated_at: u64, // Record update timestamp
    pub source: String,  // Data source
    pub confidence: f64, // Data confidence level 0.0-1.0
}

impl TransactionInfo {
    pub fn get_received_token(&self) -> Option<(String, u64)> {
        if let Some(left_addr) = self.get_pool_left_address() {
            if let Some(left_amount) = self.get_pool_left_amount() {
                if self.is_token_received(&left_addr) {
                    return Some((left_addr, left_amount));
                }
            }
        }
        if let Some(right_addr) = self.get_pool_right_address() {
            if let Some(right_amount) = self.get_pool_right_amount() {
                if self.is_token_received(&right_addr) {
                    return Some((right_addr, right_amount));
                }
            }
        }
        if let Some(right_addr) = self.get_pool_right_address() {
            if let Some(right_amount) = self.get_pool_right_amount() {
                return Some((right_addr, right_amount));
            }
        }
        None
    }

    pub fn get_spent_token(&self) -> Option<(String, u64)> {
        if let Some((received_addr, _)) = self.get_received_token() {
            if let Some(left_addr) = self.get_pool_left_address() {
                if left_addr != received_addr {
                    if let Some(left_amount) = self.get_pool_left_amount() {
                        return Some((left_addr, left_amount));
                    }
                }
            }
            if let Some(right_addr) = self.get_pool_right_address() {
                if right_addr != received_addr {
                    if let Some(right_amount) = self.get_pool_right_amount() {
                        return Some((right_addr, right_amount));
                    }
                }
            }
        }
        if let Some(left_addr) = self.get_pool_left_address() {
            if let Some(left_amount) = self.get_pool_left_amount() {
                if self.is_token_spent(&left_addr) {
                    return Some((left_addr, left_amount));
                }
            }
        }
        if let Some(right_addr) = self.get_pool_right_address() {
            if let Some(right_amount) = self.get_pool_right_amount() {
                if self.is_token_spent(&right_addr) {
                    return Some((right_addr, right_amount));
                }
            }
        }
        None
    }

    fn is_token_received(&self, mint: &str) -> bool {
        for post_balance in &self.post_token_balances {
            if post_balance.mint == mint {
                if let Some(pre_balance) = self
                    .pre_token_balances
                    .iter()
                    .find(|b| b.mint == mint && b.owner == post_balance.owner)
                {
                    let pre_amount = pre_balance
                        .ui_token_amount
                        .amount
                        .parse::<u64>()
                        .unwrap_or(0);
                    let post_amount = post_balance
                        .ui_token_amount
                        .amount
                        .parse::<u64>()
                        .unwrap_or(0);
                    return post_amount > pre_amount;
                }
            }
        }
        false
    }

    fn is_token_spent(&self, mint: &str) -> bool {
        for pre_balance in &self.pre_token_balances {
            if pre_balance.mint == mint {
                if let Some(post_balance) = self
                    .post_token_balances
                    .iter()
                    .find(|b| b.mint == mint && b.owner == pre_balance.owner)
                {
                    let pre_amount = pre_balance
                        .ui_token_amount
                        .amount
                        .parse::<u64>()
                        .unwrap_or(0);
                    let post_amount = post_balance
                        .ui_token_amount
                        .amount
                        .parse::<u64>()
                        .unwrap_or(0);
                    return pre_amount > post_amount;
                }
            }
        }
        false
    }

    pub fn get_token_received_amount(&self) -> Option<(String, u64)> {
        let mut max_amount = 0u64;
        let mut max_token = None;
        for post_balance in &self.post_token_balances {
            let mint = &post_balance.mint;
            let pre_amount = self
                .pre_token_balances
                .iter()
                .find(|b| &b.mint == mint && b.owner == post_balance.owner)
                .and_then(|b| b.ui_token_amount.amount.parse::<u64>().ok())
                .unwrap_or(0);
            let post_amount = post_balance
                .ui_token_amount
                .amount
                .parse::<u64>()
                .unwrap_or(0);
            if post_amount > pre_amount {
                let increase = post_amount - pre_amount;
                if increase > max_amount {
                    max_amount = increase;
                    max_token = Some(mint.clone());
                }
            }
        }
        max_token.map(|token| (token, max_amount))
    }

    pub fn get_token_spent_amount(&self) -> Option<(String, u64)> {
        let mut max_amount = 0u64;
        let mut max_token = None;
        for pre_balance in &self.pre_token_balances {
            let mint = &pre_balance.mint;
            let post_amount = self
                .post_token_balances
                .iter()
                .find(|b| &b.mint == mint && b.owner == pre_balance.owner)
                .and_then(|b| b.ui_token_amount.amount.parse::<u64>().ok())
                .unwrap_or(0);
            let pre_amount = pre_balance
                .ui_token_amount
                .amount
                .parse::<u64>()
                .unwrap_or(0);
            if pre_amount > post_amount {
                let decrease = pre_amount - post_amount;
                if decrease > max_amount {
                    max_amount = decrease;
                    max_token = Some(mint.clone());
                }
            }
        }
        max_token.map(|token| (token, max_amount))
    }

    pub fn get_pool_left_amount(&self) -> Option<u64> {
        if let Some(left_address) = self.get_pool_left_address() {
            use crate::global::{SOL, USD_1, USDC, USDT};
            let is_common_token = left_address == SOL
                || left_address == USDC
                || left_address == USDT
                || left_address == USD_1;
            if is_common_token {
                if let Some(max_amount) = self.get_max_amount_for_mint(&left_address) {
                    return Some(max_amount);
                }
            }
        }
        let address = self.get_pool_left_address()?;
        if let Some(amount) = self.input_amount {
            return Some(amount);
        }
        for log in &self.logs {
            if log.contains("Input amount:") || log.contains("amountIn:") {
                let cleaned = log.replace("Input amount:", "").replace("amountIn:", "");
                for word in cleaned.split_whitespace() {
                    if let Ok(amount) = word
                        .trim_matches(|c: char| !c.is_ascii_digit())
                        .parse::<u64>()
                    {
                        if amount > 0 {
                            return Some(amount);
                        }
                    }
                }
            }
            let dex_patterns = [
                ("amount_in:", ":"),
                ("input_amount:", ":"),
                ("fromAmount:", ":"),
                ("amountIn=", "="),
                ("in_amount:", ":"),
            ];
            for (pattern, separator) in dex_patterns.iter() {
                if log.contains(pattern) {
                    if let Some(start) = log.find(pattern) {
                        let rest = &log[start + pattern.len()..];
                        let end = rest
                            .find(|c: char| c == ' ' || c == ',' || c == '}')
                            .unwrap_or(rest.len());
                        let amount_str = &rest[..end].trim();
                        if let Ok(amount) = amount_str.parse::<u64>() {
                            if amount > 0 {
                                return Some(amount);
                            }
                        }
                    }
                }
            }
        }
        for instruction in &self.instructions {
            let data = &instruction.data;
            if data.contains("amountIn") || data.contains("input_amount") {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(amount) = json
                        .get("amountIn")
                        .or_else(|| json.get("input_amount"))
                        .or_else(|| json.get("fromAmount"))
                    {
                        if let Some(amount_num) = amount.as_u64() {
                            if amount_num > 0 {
                                return Some(amount_num);
                            }
                        } else if let Some(amount_str) = amount.as_str() {
                            if let Ok(amount_num) = amount_str.parse::<u64>() {
                                if amount_num > 0 {
                                    return Some(amount_num);
                                }
                            }
                        }
                    }
                }
            }
        }
        for inner_instruction in &self.inner_instructions {
            for inst in &inner_instruction.instructions {
                if inst.data.contains("amountIn") || inst.data.contains("input_amount") {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&inst.data) {
                        if let Some(amount) =
                            json.get("amountIn").or_else(|| json.get("input_amount"))
                        {
                            if let Some(amount_num) = amount.as_u64() {
                                if amount_num > 0 {
                                    return Some(amount_num);
                                }
                            }
                        }
                    }
                }
            }
        }
        let mut max_amount = 0u64;
        for pre_balance in &self.pre_token_balances {
            if pre_balance.mint == address {
                let post_amount = self
                    .post_token_balances
                    .iter()
                    .find(|b| b.mint == address && b.owner == pre_balance.owner)
                    .and_then(|b| b.ui_token_amount.amount.parse::<u64>().ok())
                    .unwrap_or(0);
                let pre_amount = pre_balance
                    .ui_token_amount
                    .amount
                    .parse::<u64>()
                    .unwrap_or(0);
                if pre_amount > post_amount {
                    let decrease = pre_amount - post_amount;
                    if decrease > max_amount {
                        max_amount = decrease;
                    }
                }
            }
        }
        if max_amount == 0 {
            let mut all_decreases = Vec::new();
            for pre_balance in &self.pre_token_balances {
                if pre_balance.mint == address {
                    let mut total_pre_amount = 0u64;
                    let mut total_post_amount = 0u64;
                    for post_balance in &self.post_token_balances {
                        if post_balance.mint == address && post_balance.owner == pre_balance.owner {
                            total_post_amount += post_balance
                                .ui_token_amount
                                .amount
                                .parse::<u64>()
                                .unwrap_or(0);
                        }
                    }
                    total_pre_amount += pre_balance
                        .ui_token_amount
                        .amount
                        .parse::<u64>()
                        .unwrap_or(0);
                    if total_pre_amount > total_post_amount {
                        all_decreases.push(total_pre_amount - total_post_amount);
                    }
                }
            }
            if let Some(&max_decrease) = all_decreases.iter().max() {
                if max_decrease > 0 {
                    max_amount = max_decrease;
                }
            }
        }
        if max_amount == 0 {
            for balance in &self.pre_token_balances {
                if balance.mint == address {
                    if let Some(ui_amount_str) = &balance.ui_token_amount.ui_amount_string {
                        let cleaned = ui_amount_str.replace(',', "");
                        if let Ok(ui_amount) = cleaned.parse::<f64>() {
                            let raw_amount = (ui_amount
                                * 10u64.pow(balance.ui_token_amount.decimals as u32) as f64)
                                as u64;
                            if raw_amount > max_amount {
                                max_amount = raw_amount;
                            }
                        }
                    }
                    if let Some(ui_amount) = balance.ui_token_amount.ui_amount {
                        let raw_amount = (ui_amount
                            * 10u64.pow(balance.ui_token_amount.decimals as u32) as f64)
                            as u64;
                        if raw_amount > max_amount {
                            max_amount = raw_amount;
                        }
                    }
                }
            }
        }
        if max_amount == 0 {
            if let Some((token, amount)) = self.get_token_spent_amount() {
                if token == address {
                    max_amount = amount;
                }
            }
        }
        if max_amount == 0 {
            let is_pump_fun_aggregator = self
                .logs
                .iter()
                .any(|log| log.contains("BuyExactInPumpFun") || log.contains("Pump.fun"));
            if is_pump_fun_aggregator {
                for log in &self.logs {
                    if log.contains("Program data:") {
                        if let Some(base64_start) = log.find("Program data:") {
                            let base64_str = &log[base64_start + 13..].trim();
                            if let Ok(decoded) = general_purpose::STANDARD.decode(base64_str) {
                                if decoded.len() >= 40 {
                                    for offset in (24..decoded.len() - 8).step_by(8) {
                                        if offset + 8 <= decoded.len() {
                                            let potential_amount = u64::from_le_bytes([
                                                decoded[offset],
                                                decoded[offset + 1],
                                                decoded[offset + 2],
                                                decoded[offset + 3],
                                                decoded[offset + 4],
                                                decoded[offset + 5],
                                                decoded[offset + 6],
                                                decoded[offset + 7],
                                            ]);
                                            if potential_amount > 1000
                                                && potential_amount < 1_000_000_000_000
                                            {
                                                max_amount = potential_amount;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if max_amount == 0 {
                    for log in &self.logs {
                        if log.contains("TransferChecked") && log.contains(&address) {
                            let parts: Vec<&str> = log.split_whitespace().collect();
                            for part in parts {
                                if part.contains('.') {
                                    let cleaned = part.replace(',', "").replace(')', "");
                                    if let Ok(amount_f64) = cleaned.parse::<f64>() {
                                        let raw_amount = (amount_f64 * 1_000_000.0) as u64;
                                        if raw_amount > max_amount {
                                            max_amount = raw_amount;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if max_amount == 0 {
                    for log in &self.logs {
                        if log.contains("Transfer") && log.contains("lamports") {
                            let parts: Vec<&str> = log.split_whitespace().collect();
                            for part in parts {
                                if part.chars().all(|c| c.is_ascii_digit()) {
                                    if let Ok(lamports) = part.parse::<u64>() {
                                        if lamports > 1000 && lamports < 1_000_000_000_000 {
                                            max_amount = lamports;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if max_amount == 0 {
            use crate::global::SOL;
            if address == SOL {
                if self.value_sol > 0.0 {
                    max_amount = (self.value_sol * LAMPORTS_PER_SOL as f64) as u64;
                }
            }
        }
        if max_amount > 0 {
            Some(max_amount)
        } else {
            None
        }
    }

    pub fn get_pool_right_amount(&self) -> Option<u64> {
        if let Some(right_address) = self.get_pool_right_address() {
            use crate::global::{SOL, USD_1, USDC, USDT};
            let is_common_token = right_address == SOL
                || right_address == USDC
                || right_address == USDT
                || right_address == USD_1;
            if is_common_token {
                if let Some(max_amount) = self.get_max_amount_for_mint(&right_address) {
                    return Some(max_amount);
                }
            }
        }
        let address = self.get_pool_right_address()?;
        if let Some(amount) = self.output_amount {
            return Some(amount);
        }
        for log in &self.logs {
            if log.contains("Output amount:") || log.contains("amountOut:") {
                let cleaned = log.replace("Output amount:", "").replace("amountOut:", "");
                for word in cleaned.split_whitespace() {
                    if let Ok(amount) = word
                        .trim_matches(|c: char| !c.is_ascii_digit())
                        .parse::<u64>()
                    {
                        if amount > 0 {
                            return Some(amount);
                        }
                    }
                }
            }
            let dex_patterns = [
                ("amount_out:", ":"),
                ("output_amount:", ":"),
                ("toAmount:", ":"),
                ("amountOut=", "="),
                ("out_amount:", ":"),
            ];
            for (pattern, separator) in dex_patterns.iter() {
                if log.contains(pattern) {
                    if let Some(start) = log.find(pattern) {
                        let rest = &log[start + pattern.len()..];
                        let end = rest
                            .find(|c: char| c == ' ' || c == ',' || c == '}')
                            .unwrap_or(rest.len());
                        let amount_str = &rest[..end].trim();
                        if let Ok(amount) = amount_str.parse::<u64>() {
                            if amount > 0 {
                                return Some(amount);
                            }
                        }
                    }
                }
            }
        }
        for instruction in &self.instructions {
            let data = &instruction.data;
            if data.contains("amountOut") || data.contains("output_amount") {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(amount) = json
                        .get("amountOut")
                        .or_else(|| json.get("output_amount"))
                        .or_else(|| json.get("toAmount"))
                    {
                        if let Some(amount_num) = amount.as_u64() {
                            if amount_num > 0 {
                                return Some(amount_num);
                            }
                        } else if let Some(amount_str) = amount.as_str() {
                            if let Ok(amount_num) = amount_str.parse::<u64>() {
                                if amount_num > 0 {
                                    return Some(amount_num);
                                }
                            }
                        }
                    }
                }
            }
        }
        for inner_instruction in &self.inner_instructions {
            for inst in &inner_instruction.instructions {
                if inst.data.contains("amountOut") || inst.data.contains("output_amount") {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&inst.data) {
                        if let Some(amount) =
                            json.get("amountOut").or_else(|| json.get("output_amount"))
                        {
                            if let Some(amount_num) = amount.as_u64() {
                                if amount_num > 0 {
                                    return Some(amount_num);
                                }
                            }
                        }
                    }
                }
            }
        }
        let mut max_amount = 0u64;
        for post_balance in &self.post_token_balances {
            if post_balance.mint == address {
                let pre_amount = self
                    .pre_token_balances
                    .iter()
                    .find(|b| b.mint == address && b.owner == post_balance.owner)
                    .and_then(|b| b.ui_token_amount.amount.parse::<u64>().ok())
                    .unwrap_or(0);
                let post_amount = post_balance
                    .ui_token_amount
                    .amount
                    .parse::<u64>()
                    .unwrap_or(0);
                if post_amount > pre_amount {
                    let increase = post_amount - pre_amount;
                    if increase > max_amount {
                        max_amount = increase;
                    }
                }
            }
        }
        if max_amount == 0 {
            let mut all_increases = Vec::new();
            for post_balance in &self.post_token_balances {
                if post_balance.mint == address {
                    let mut total_pre_amount = 0u64;
                    let mut total_post_amount = 0u64;
                    for pre_balance in &self.pre_token_balances {
                        if pre_balance.mint == address && pre_balance.owner == post_balance.owner {
                            total_pre_amount += pre_balance
                                .ui_token_amount
                                .amount
                                .parse::<u64>()
                                .unwrap_or(0);
                        }
                    }
                    total_post_amount += post_balance
                        .ui_token_amount
                        .amount
                        .parse::<u64>()
                        .unwrap_or(0);
                    if total_post_amount > total_pre_amount {
                        all_increases.push(total_post_amount - total_pre_amount);
                    }
                }
            }
            if let Some(&max_increase) = all_increases.iter().max() {
                if max_increase > 0 {
                    max_amount = max_increase;
                }
            }
        }
        if max_amount == 0 {
            for balance in &self.post_token_balances {
                if balance.mint == address {
                    if let Some(ui_amount_str) = &balance.ui_token_amount.ui_amount_string {
                        let cleaned = ui_amount_str.replace(',', "");
                        if let Ok(ui_amount) = cleaned.parse::<f64>() {
                            let raw_amount = (ui_amount
                                * 10u64.pow(balance.ui_token_amount.decimals as u32) as f64)
                                as u64;
                            if raw_amount > max_amount {
                                max_amount = raw_amount;
                            }
                        }
                    }
                    if let Some(ui_amount) = balance.ui_token_amount.ui_amount {
                        let raw_amount = (ui_amount
                            * 10u64.pow(balance.ui_token_amount.decimals as u32) as f64)
                            as u64;
                        if raw_amount > max_amount {
                            max_amount = raw_amount;
                        }
                    }
                }
            }
        }
        if max_amount == 0 {
            if let Some((token, amount)) = self.get_token_received_amount() {
                if token == address {
                    max_amount = amount;
                }
            }
        }
        if max_amount == 0 {
            let is_pump_fun_aggregator = self
                .logs
                .iter()
                .any(|log| log.contains("BuyExactInPumpFun") || log.contains("Pump.fun"));
            if is_pump_fun_aggregator {
                for log in &self.logs {
                    if log.contains("Program data:") {
                        if let Some(base64_start) = log.find("Program data:") {
                            let base64_str = &log[base64_start + 13..].trim();
                            if let Ok(decoded) = general_purpose::STANDARD.decode(base64_str) {
                                if decoded.len() >= 40 {
                                    for offset in (32..decoded.len() - 8).step_by(8) {
                                        if offset + 8 <= decoded.len() {
                                            let potential_amount = u64::from_le_bytes([
                                                decoded[offset],
                                                decoded[offset + 1],
                                                decoded[offset + 2],
                                                decoded[offset + 3],
                                                decoded[offset + 4],
                                                decoded[offset + 5],
                                                decoded[offset + 6],
                                                decoded[offset + 7],
                                            ]);
                                            if potential_amount > 1000
                                                && potential_amount < 1_000_000_000_000
                                            {
                                                max_amount = potential_amount;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if max_amount == 0 {
                    let mut found_pump_section = false;
                    for log in &self.logs {
                        if log.contains("Invoking Pump.fun") {
                            found_pump_section = true;
                            continue;
                        }
                        if found_pump_section {
                            if log.contains(',') && !log.contains("Program") {
                                let parts: Vec<&str> = log.split_whitespace().collect();
                                for part in parts {
                                    let cleaned = part.replace(',', "").replace('$', "");
                                    if cleaned.contains('.') {
                                        if let Ok(amount_f64) = cleaned.parse::<f64>() {
                                            let raw_amount = (amount_f64 * 1_000_000.0) as u64;
                                            if raw_amount > max_amount {
                                                max_amount = raw_amount;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            if log.contains("Program") && log.contains("consumed") {
                                break;
                            }
                        }
                    }
                }
                if max_amount == 0 {
                    for log in &self.logs {
                        if let Some(idx) = log.find("for ") {
                            let after_for = &log[idx + 4..];
                            let amount_str = after_for.split_whitespace().next().unwrap_or("");
                            let cleaned = amount_str.replace(',', "");
                            if let Ok(amount_f64) = cleaned.parse::<f64>() {
                                let raw_amount = (amount_f64 * 1_000_000.0) as u64;
                                if raw_amount > max_amount {
                                    max_amount = raw_amount;
                                }
                            }
                        }
                    }
                }
                if max_amount == 0 {
                    for log in &self.logs {
                        if log.contains("TransferChecked") && log.contains(&address) {
                            let parts: Vec<&str> = log.split_whitespace().collect();
                            for part in parts {
                                if part.contains('.') {
                                    let cleaned = part.replace(',', "").replace(')', "");
                                    if let Ok(amount_f64) = cleaned.parse::<f64>() {
                                        let raw_amount = (amount_f64 * 1_000_000.0) as u64;
                                        if raw_amount > max_amount {
                                            max_amount = raw_amount;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if max_amount == 0 {
            use crate::global::SOL;
            if address == SOL {
                if self.value_sol > 0.0 {
                    max_amount = (self.value_sol * LAMPORTS_PER_SOL as f64) as u64;
                }
            }
        }
        if max_amount > 0 {
            Some(max_amount)
        } else {
            None
        }
    }

    pub fn get_token_quote_ratio(&self) -> Option<f64> {
        use crate::global::QUOTES;
        if let Some(dex_type) = &self.dex_program_type {
            if *dex_type == crate::types::DexProgramType::PumpBondCurve {
                return self
                    .get_pump_bond_curve_transaction_info()
                    .get_token_quote_ratio();
            }
        }
        let direction = self.get_direction();
        match direction {
            Direction::Buy => {
                if let Some((spent_token, spent_amount)) = self.get_spent_token() {
                    if QUOTES.contains(&spent_token.as_str()) {
                        if let Some((received_token, received_amount)) = self.get_received_token() {
                            if !QUOTES.contains(&received_token.as_str()) && received_amount > 0 {
                                let spent_decimals =
                                    self.get_token_decimals_for_mint(&spent_token)?;
                                let received_decimals =
                                    self.get_token_decimals_for_mint(&received_token)?;
                                let spent_f64 =
                                    spent_amount as f64 / 10_u64.pow(spent_decimals as u32) as f64;
                                let received_f64 = received_amount as f64
                                    / 10_u64.pow(received_decimals as u32) as f64;
                                return Some(spent_f64 / received_f64);
                            }
                        }
                    }
                }
            }
            Direction::Sell => {
                if let Some((spent_token, spent_amount)) = self.get_spent_token() {
                    if !QUOTES.contains(&spent_token.as_str()) && spent_amount > 0 {
                        if let Some((received_token, received_amount)) = self.get_received_token() {
                            if QUOTES.contains(&received_token.as_str()) {
                                let spent_decimals =
                                    self.get_token_decimals_for_mint(&spent_token)?;
                                let received_decimals =
                                    self.get_token_decimals_for_mint(&received_token)?;
                                let spent_f64 =
                                    spent_amount as f64 / 10_u64.pow(spent_decimals as u32) as f64;
                                let received_f64 = received_amount as f64
                                    / 10_u64.pow(received_decimals as u32) as f64;
                                return Some(received_f64 / spent_f64);
                            }
                        }
                    }
                }
            }
        }
        if let Some(left_amount_sol) = self.get_pool_left_amount_sol() {
            if let Some(right_amount_sol) = self.get_pool_right_amount_sol() {
                if left_amount_sol > 0.0 {
                    return Some(right_amount_sol / left_amount_sol);
                }
            }
        }
        None
    }

    // Get the maximum amount of a specified token address
    fn get_max_amount_for_mint(&self, mint: &str) -> Option<u64> {
        use crate::global::SOL;
        // Extract the maximum amount from the log.
        let mut max_amount = 0u64;
        // Find the maximum amount corresponding to the token in the log.
        for log in &self.logs {
            if log.contains(mint) {
                // Find the number before the token name
                if let Some(mint_index) = log.find(mint) {
                    // Search for the most recent number
                    let before_mint = &log[..mint_index];
                    let parts: Vec<&str> = before_mint.split_whitespace().collect();
                    // Find the first number from back to front.
                    for part in parts.iter().rev() {
                        let cleaned = part.replace(',', "");
                        if let Ok(amount_f64) = cleaned.parse::<f64>() {
                            let amount = if mint == SOL {
                                (amount_f64 * LAMPORTS_PER_SOL as f64) as u64
                            } else {
                                (amount_f64 * 1_000_000.0) as u64
                            };

                            if amount > max_amount {
                                max_amount = amount;
                            }
                            break;
                        }
                    }
                }
            }
        }
        // Check received amount (for the right pool).
        // Find the largest amount from changes in token balance.
        for post_balance in &self.post_token_balances {
            if post_balance.mint == mint {
                let pre_amount = self
                    .pre_token_balances
                    .iter()
                    .find(|b| b.mint == mint && b.owner == post_balance.owner)
                    .and_then(|b| b.ui_token_amount.amount.parse::<u64>().ok())
                    .unwrap_or(0);
                let post_amount = post_balance
                    .ui_token_amount
                    .amount
                    .parse::<u64>()
                    .unwrap_or(0);

                if post_amount > pre_amount {
                    let increase = post_amount - pre_amount;
                    if increase > max_amount {
                        max_amount = increase;
                    }
                }
            }
        }
        // Check the cost (for the left pool).
        for pre_balance in &self.pre_token_balances {
            if pre_balance.mint == mint {
                let post_amount = self
                    .post_token_balances
                    .iter()
                    .find(|b| b.mint == mint && b.owner == pre_balance.owner)
                    .and_then(|b| b.ui_token_amount.amount.parse::<u64>().ok())
                    .unwrap_or(0);
                let pre_amount = pre_balance
                    .ui_token_amount
                    .amount
                    .parse::<u64>()
                    .unwrap_or(0);
                if pre_amount > post_amount {
                    let decrease = pre_amount - post_amount;
                    if decrease > max_amount {
                        max_amount = decrease;
                    }
                }
            }
        }
        if max_amount > 0 {
            Some(max_amount)
        } else {
            None
        }
    }

    pub fn get_pool_left_address(&self) -> Option<String> {
        use crate::global::{SOL, USDC, USDT};
        for log in &self.logs {
            if let Some(address) = Self::extract_address_from_log(log, "fromMint") {
                if address != SOL && address != USDC && address != USDT {
                    return Some(address);
                }
            }
            if let Some(address) = Self::extract_address_from_log(log, "inputMint") {
                if address != SOL && address != USDC && address != USDT {
                    return Some(address);
                }
            }
        }
        if let Some(input_mint) = &self.input_mint {
            if input_mint != SOL && input_mint != USDC && input_mint != USDT {
                return Some(input_mint.clone());
            }
        }
        if let Some((address, _)) = self.get_token_spent_amount() {
            if address != SOL && address != USDC && address != USDT {
                return Some(address);
            }
        }
        for balance in self
            .pre_token_balances
            .iter()
            .chain(&self.post_token_balances)
        {
            let mint = &balance.mint;
            if mint != SOL && mint != USDC && mint != USDT {
                return Some(mint.clone());
            }
        }
        None
    }

    pub fn get_pool_right_address(&self) -> Option<String> {
        use crate::global::{SOL, USDC, USDT};
        for log in &self.logs {
            if let Some(address) = Self::extract_address_from_log(log, "toMint") {
                if address == SOL || address == USDC || address == USDT {
                    return Some(address);
                }
            }
            if let Some(address) = Self::extract_address_from_log(log, "outputMint") {
                if address == SOL || address == USDC || address == USDT {
                    return Some(address);
                }
            }
        }
        if let Some(output_mint) = &self.output_mint {
            if output_mint == SOL || output_mint == USDC || output_mint == USDT {
                return Some(output_mint.clone());
            }
        }
        if let Some((address, _)) = self.get_token_received_amount() {
            if address == SOL || address == USDC || address == USDT {
                return Some(address);
            }
        }
        for balance in self
            .pre_token_balances
            .iter()
            .chain(&self.post_token_balances)
        {
            let mint = &balance.mint;
            if mint == USDC {
                return Some(USDC.to_string());
            }
            if mint == USDT {
                return Some(USDT.to_string());
            }
            if mint == SOL {
                return Some(SOL.to_string());
            }
        }
        Some(SOL.to_string())
    }

    fn extract_address_from_log(log: &str, key: &str) -> Option<String> {
        if let Some(start_idx) = log.find(key) {
            let after_key = &log[start_idx + key.len()..];
            for word in after_key.split_whitespace() {
                let trimmed = word.trim_matches(|c: char| !c.is_alphanumeric());
                if trimmed.len() == 44 && trimmed.chars().all(|c| c.is_alphanumeric()) {
                    return Some(trimmed.to_string());
                }
            }
        }
        None
    }

    pub fn get_pool_left_amount_sol(&self) -> Option<f64> {
        self.get_pool_left_amount().and_then(|lamports| {
            let decimals = self.get_token_decimals_for_left_pool()?;
            Some(lamports as f64 / 10_u64.pow(decimals as u32) as f64)
        })
    }

    pub fn get_pool_right_amount_sol(&self) -> Option<f64> {
        self.get_pool_right_amount().and_then(|lamports| {
            let decimals = self.get_token_decimals_for_right_pool()?;
            Some(lamports as f64 / 10_u64.pow(decimals as u32) as f64)
        })
    }

    pub fn get_received_token_sol(&self) -> Option<(String, f64)> {
        self.get_received_token().and_then(|(address, amount)| {
            let decimals = self.get_token_decimals_for_mint(&address)?;
            Some((address, amount as f64 / 10_u64.pow(decimals as u32) as f64))
        })
    }

    pub fn get_spent_token_sol(&self) -> Option<(String, f64)> {
        self.get_spent_token().and_then(|(address, amount)| {
            let decimals = self.get_token_decimals_for_mint(&address)?;
            Some((address, amount as f64 / 10_u64.pow(decimals as u32) as f64))
        })
    }

    fn get_token_decimals_for_left_pool(&self) -> Option<u8> {
        if let Some(address) = self.get_pool_left_address() {
            return self.get_token_decimals_for_mint(&address);
        }
        None
    }

    fn get_token_decimals_for_right_pool(&self) -> Option<u8> {
        if let Some(address) = self.get_pool_right_address() {
            return self.get_token_decimals_for_mint(&address);
        }
        None
    }

    fn get_token_decimals_for_mint(&self, mint: &str) -> Option<u8> {
        use crate::global::SOL;
        if mint == SOL {
            return Some(9);
        }
        use crate::global::{USDC, USDT};
        if mint == USDC {
            return Some(6);
        }
        if mint == USDT {
            return Some(6);
        }
        for balance in self
            .pre_token_balances
            .iter()
            .chain(&self.post_token_balances)
        {
            if balance.mint == mint {
                return Some(balance.ui_token_amount.decimals);
            }
        }
        None
    }

    pub fn from_encoded_transaction(
        tx: &EncodedConfirmedTransactionWithStatusMeta,
        signature: &str,
    ) -> Self {
        let mut info = Self::default();
        info.transaction_hash = signature.to_string();
        info.signature = signature.to_string();
        info.block_number = tx.slot;
        info.slot = tx.slot;
        info.block_time = tx.block_time;
        if let Some(meta) = &tx.transaction.meta {
            info.status = if meta.err.is_none() {
                "success".to_string()
            } else {
                "failed".to_string()
            };
            info.error_message = meta.err.as_ref().map(|e| format!("{:?}", e));
            info.err = meta
                .err
                .clone()
                .map(serde_json::to_value)
                .transpose()
                .unwrap_or(None);
            info.fee = meta.fee;
            info.fee_sol = meta.fee as f64 / LAMPORTS_PER_SOL as f64;
            info.compute_units_consumed = match &meta.compute_units_consumed {
                OptionSerializer::Some(value) => Some(*value),
                OptionSerializer::None => None,
                OptionSerializer::Skip => None,
            };
        }
        Self::parse_transaction_content(&mut info, tx);
        info.created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        info.updated_at = info.created_at;
        info.source = "rpc".to_string();
        info.confidence = 1.0;
        info
    }

    fn parse_transaction_content(
        info: &mut TransactionInfo,
        tx: &EncodedConfirmedTransactionWithStatusMeta,
    ) {
        let transaction_with_meta = &tx.transaction;
        match &transaction_with_meta.transaction {
            EncodedTransaction::Json(json_tx) => {
                match &json_tx.message {
                    UiMessage::Parsed(parsed_msg) => {
                        Self::parse_parsed_message(info, parsed_msg);
                    }
                    UiMessage::Raw(raw_msg) => {
                        Self::parse_raw_message(info, raw_msg, tx);
                    }
                }
                info.signers = json_tx.signatures.clone();
            }
            EncodedTransaction::Binary(_, _) => {
                info.transaction_type = Some(TransactionType::Binary);
            }
            _ => {
                info.transaction_type = Some(TransactionType::Other);
            }
        }
        if let Some(meta) = &transaction_with_meta.meta {
            Self::parse_metadata(info, meta, tx);
        }
        if let Some(version) = &transaction_with_meta.version {
            match version {
                TransactionVersion::Legacy(legacy) => {}
                TransactionVersion::Number(num) => {
                    info.version = *num;
                }
            }
        }
    }

    ///  parse parsed message
    fn parse_parsed_message(
        info: &mut TransactionInfo,
        parsed_msg: &solana_transaction_status::UiParsedMessage,
    ) {
        info.involved_accounts = parsed_msg
            .account_keys
            .iter()
            .filter_map(|acc| Some(acc.pubkey.clone()))
            .collect();
        info.instructions_count = parsed_msg.instructions.len() as u64;
        info.instructions = parsed_msg
            .instructions
            .iter()
            .map(|inst| {
                let (stack_height, program) = match inst {
                    solana_transaction_status::UiInstruction::Compiled(compiled) => {
                        (compiled.stack_height, "compiled".to_string())
                    }
                    solana_transaction_status::UiInstruction::Parsed(parsed) => match parsed {
                        solana_transaction_status::UiParsedInstruction::Parsed(parsed_inst) => {
                            (parsed_inst.stack_height, parsed_inst.program.clone())
                        }
                        solana_transaction_status::UiParsedInstruction::PartiallyDecoded(
                            partial_inst,
                        ) => (None, "partially_decoded".to_string()),
                    },
                };
                InstructionInfo {
                    program_id: Self::extract_program_id_from_ui_instruction(inst),
                    accounts: Self::extract_accounts_from_ui_instruction(inst),
                    data: Self::extract_data_from_ui_instruction(inst),
                    stack_height: stack_height,
                    program: program,
                }
            })
            .collect();
        info.recent_blockhash = parsed_msg.recent_blockhash.clone();
        Self::extract_transfer_info(info, parsed_msg);
    }

    fn parse_raw_message(
        info: &mut TransactionInfo,
        raw_msg: &solana_transaction_status::UiRawMessage,
        tx: &EncodedConfirmedTransactionWithStatusMeta,
    ) {
        info.involved_accounts = raw_msg
            .account_keys
            .iter()
            .map(|pk| pk.to_string())
            .collect();
        info.instructions_count = raw_msg.instructions.len() as u64;
        info.recent_blockhash = raw_msg.recent_blockhash.clone();
        info.transaction_type = Some(TransactionType::Raw);
        info.program_id = "unknown".to_string();
    }

    /// parse metadata
    fn parse_metadata(
        info: &mut TransactionInfo,
        meta: &solana_transaction_status::UiTransactionStatusMeta,
        tx: &EncodedConfirmedTransactionWithStatusMeta,
    ) {
        match &meta.log_messages {
            OptionSerializer::Some(logs) => info.logs = logs.clone(),
            _ => info.logs = vec![],
        }
        match &meta.inner_instructions {
            OptionSerializer::Some(inner_instructions) => {
                info.inner_instructions_count = inner_instructions.len() as u64;
                info.inner_instructions = inner_instructions
                    .iter()
                    .map(|inner| InnerInstructionInfo {
                        index: inner.index,
                        instructions: inner
                            .instructions
                            .iter()
                            .map(|inst| InstructionInfo {
                                program_id: Self::extract_program_id_from_ui_instruction(inst),
                                accounts: Self::extract_accounts_from_ui_instruction(inst),
                                data: Self::extract_data_from_ui_instruction(inst),
                                stack_height: None,
                                program: "inner".to_string(),
                            })
                            .collect(),
                    })
                    .collect();
            }
            _ => {
                info.inner_instructions_count = 0;
                info.inner_instructions = vec![];
            }
        }
        // parse balance changes
        Self::parse_balance_changes(info, meta, tx);
        // token balance
        match &meta.pre_token_balances {
            OptionSerializer::Some(pre_token_balances) => {
                info.pre_token_balances = pre_token_balances
                    .iter()
                    .map(|balance| TokenBalance {
                        account_index: balance.account_index,
                        mint: balance.mint.clone(),
                        owner: balance.owner.clone().unwrap_or("".to_string()),
                        ui_token_amount: UiTokenAmount {
                            ui_amount: balance.ui_token_amount.ui_amount,
                            decimals: balance.ui_token_amount.decimals,
                            amount: balance.ui_token_amount.amount.clone(),
                            ui_amount_string: Some(
                                balance.ui_token_amount.ui_amount_string.clone(),
                            ),
                        },
                    })
                    .collect();
            }
            _ => {
                info.pre_token_balances = vec![];
            }
        }
        match &meta.post_token_balances {
            OptionSerializer::Some(post_token_balances) => {
                info.post_token_balances = post_token_balances
                    .iter()
                    .map(|balance| TokenBalance {
                        account_index: balance.account_index,
                        mint: balance.mint.clone(),
                        owner: balance.owner.clone().unwrap_or("".to_string()),
                        ui_token_amount: UiTokenAmount {
                            ui_amount: balance.ui_token_amount.ui_amount,
                            decimals: balance.ui_token_amount.decimals,
                            amount: balance.ui_token_amount.amount.clone(),
                            ui_amount_string: Some(
                                balance.ui_token_amount.ui_amount_string.clone(),
                            ),
                        },
                    })
                    .collect();
            }
            _ => {
                info.post_token_balances = vec![];
            }
        }
        // parse token transactions
        Self::parse_token_transactions(info, meta);
    }

    /// parse balance changes
    fn parse_balance_changes(
        info: &mut TransactionInfo,
        meta: &solana_transaction_status::UiTransactionStatusMeta,
        tx: &EncodedConfirmedTransactionWithStatusMeta,
    ) {
        // get balance change information
        if let (pre_balances, post_balances) = (&meta.pre_balances, &meta.post_balances) {
            if pre_balances.len() == post_balances.len() && !pre_balances.is_empty() {
                info.pre_balance = pre_balances[0];
                info.post_balance = post_balances[0];
                info.balance_change = post_balances[0] as i64 - pre_balances[0] as i64;
                let mut from_index = None;
                let mut to_index = None;
                let mut transfer_amount = 0u64;
                for (i, (&pre, &post)) in pre_balances.iter().zip(post_balances.iter()).enumerate()
                {
                    if post < pre {
                        // the balance decreases, possibly due to the sender
                        from_index = Some(i);
                        transfer_amount = pre - post;
                    } else if post > pre {
                        // balance increases, possibly the recipient
                        to_index = Some(i);
                    }
                }
                if let (Some(from_idx), Some(to_idx)) = (from_index, to_index) {
                    if info.from == "unknown" || info.to == "unknown" {
                        // Try to get the account address from the transaction
                        let transaction_with_meta = &tx.transaction;
                        if let EncodedTransaction::Json(json_tx) =
                            &transaction_with_meta.transaction
                        {
                            match &json_tx.message {
                                UiMessage::Parsed(parsed_msg) => {
                                    if let account_keys = &parsed_msg.account_keys {
                                        if let (Some(from_account), Some(to_account)) =
                                            (account_keys.get(from_idx), account_keys.get(to_idx))
                                        {
                                            let (from_pubkey, to_pubkey) =
                                                (&from_account.pubkey, &to_account.pubkey);
                                            info.from = from_pubkey.clone();
                                            info.to = to_pubkey.clone();
                                            info.value = transfer_amount.to_string();
                                            info.value_sol =
                                                transfer_amount as f64 / LAMPORTS_PER_SOL as f64;
                                        }
                                    }
                                }
                                UiMessage::Raw(raw_msg) => {
                                    if let (Some(from_account), Some(to_account)) = (
                                        raw_msg.account_keys.get(from_idx),
                                        raw_msg.account_keys.get(to_idx),
                                    ) {
                                        info.from = from_account.to_string();
                                        info.to = to_account.to_string();
                                        info.value = transfer_amount.to_string();
                                        info.value_sol =
                                            transfer_amount as f64 / LAMPORTS_PER_SOL as f64;
                                    }
                                }
                            }
                        }
                    }
                }

                // set the payment source
                let transaction_with_meta = &tx.transaction;
                if let EncodedTransaction::Json(json_tx) = &transaction_with_meta.transaction {
                    match &json_tx.message {
                        UiMessage::Parsed(parsed_msg) => {
                            if let account_keys = &parsed_msg.account_keys {
                                if let Some(fee_payer) = account_keys.get(0) {
                                    if let pubkey = &fee_payer.pubkey {
                                        info.fee_payer = pubkey.clone();
                                        info.signer = pubkey.clone();
                                    }
                                }
                            }
                        }
                        UiMessage::Raw(raw_msg) => {
                            if let Some(fee_payer) = raw_msg.account_keys.get(0) {
                                info.fee_payer = fee_payer.to_string();
                                info.signer = fee_payer.clone();
                            }
                        }
                    }
                }
                match &meta.loaded_addresses {
                    OptionSerializer::Some(loaded_addresses) => {
                        info.writable_accounts = loaded_addresses
                            .writable
                            .iter()
                            .map(|acc| acc.to_string())
                            .collect();
                        info.readonly_accounts = loaded_addresses
                            .readonly
                            .iter()
                            .map(|acc| acc.to_string())
                            .collect();
                    }
                    _ => {
                        info.writable_accounts = Vec::new();
                        info.readonly_accounts = Vec::new();
                    }
                }
                // Collect all involved accounts
                let mut all_accounts = Vec::new();
                all_accounts.extend(info.writable_accounts.clone());
                all_accounts.extend(info.readonly_accounts.clone());
                all_accounts.dedup();
                info.involved_accounts = all_accounts;
            }
        }
    }

    /// parse token transactions
    fn parse_token_transactions(
        info: &mut TransactionInfo,
        meta: &solana_transaction_status::UiTransactionStatusMeta,
    ) {
        // Check token balance changes
        if let (pre_token_balances, post_token_balances) =
            (&meta.pre_token_balances, &meta.post_token_balances)
        {
            match (pre_token_balances, post_token_balances) {
                (OptionSerializer::Some(pre_balances), OptionSerializer::Some(post_balances)) => {
                    if !pre_balances.is_empty() || !post_balances.is_empty() {
                        info.transaction_type = Some(TransactionType::TokenTransfer);
                        // check token transfer details
                        Self::check_token_transfers(info, pre_balances, post_balances);
                        // check nft transfer
                        Self::check_nft_transfer(info, pre_balances, post_balances);
                        // check dex transaction
                        Self::check_dex_transaction(info, meta);
                    }
                }
                _ => {}
            }
        }
    }

    // check token transfers
    fn check_token_transfers(
        info: &mut TransactionInfo,
        pre_balances: &[UiTransactionTokenBalance],
        post_balances: &[UiTransactionTokenBalance],
    ) {
        for pre_balance in pre_balances {
            if let Some(post_balance) = post_balances
                .iter()
                .find(|pb| pb.mint == pre_balance.mint && pb.owner == pre_balance.owner)
            {
                let pre_amount = pre_balance
                    .ui_token_amount
                    .amount
                    .parse::<u64>()
                    .unwrap_or(0);
                let post_amount = post_balance
                    .ui_token_amount
                    .amount
                    .parse::<u64>()
                    .unwrap_or(0);

                if pre_amount != post_amount {
                    info.pre_token_balances.push(TokenBalance {
                        account_index: pre_balance.account_index,
                        mint: pre_balance.mint.clone(),
                        owner: pre_balance.owner.clone().unwrap_or("".to_string()),
                        ui_token_amount: UiTokenAmount {
                            ui_amount: pre_balance.ui_token_amount.ui_amount,
                            decimals: pre_balance.ui_token_amount.decimals,
                            amount: pre_balance.ui_token_amount.amount.clone(),
                            ui_amount_string: Some(
                                pre_balance.ui_token_amount.ui_amount_string.clone(),
                            ),
                        },
                    });
                    info.post_token_balances.push(TokenBalance {
                        account_index: post_balance.account_index,
                        mint: post_balance.mint.clone(),
                        owner: post_balance.owner.clone().unwrap_or("".to_string()),
                        ui_token_amount: UiTokenAmount {
                            ui_amount: post_balance.ui_token_amount.ui_amount,
                            decimals: post_balance.ui_token_amount.decimals,
                            amount: post_balance.ui_token_amount.amount.clone(),
                            ui_amount_string: Some(
                                post_balance.ui_token_amount.ui_amount_string.clone(),
                            ),
                        },
                    });
                }
            }
        }
    }

    // check nft transfer
    fn check_nft_transfer(
        info: &mut TransactionInfo,
        pre_balances: &[UiTransactionTokenBalance],
        post_balances: &[UiTransactionTokenBalance],
    ) {
        for balance in pre_balances.iter().chain(post_balances) {
            if balance.ui_token_amount.decimals == 0 {
                let amount = balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);
                if amount == 1 {
                    info.is_nft_transfer = true;
                    info.nft_mint = Some(balance.mint.clone());
                    info.transaction_type = Some(TransactionType::NFTTransfer);
                    break;
                }
            }
        }
    }

    /// check dex transaction
    fn check_dex_transaction(
        info: &mut TransactionInfo,
        meta: &solana_transaction_status::UiTransactionStatusMeta,
    ) {
        if let logs = &meta.log_messages {
            let dex_keywords = [
                "Buy",
                "buy",
                "Sell",
                "sell",
                "swap",
                "Swap",
                "liquidity",
                "Liquidity",
                "pool",
                "Pool",
                "raydium",
                "Raydium",
                "orca",
                "Orca",
                "serum",
                "Serum",
                "market",
                "Market",
                "trade",
                "Trade",
                "Pump",
                "pump",
                "Pumpswap",
                "pumpswap",
                "pump.fun",
                "Pump.fun",
                "meteora",
                "Meteora",
            ];
            // dex
            for log in logs.clone().unwrap_or(vec![]) {
                if dex_keywords.iter().any(|&keyword| log.contains(keyword)) {
                    if (!info.is_swap) {
                        info.is_swap = true;
                    }
                }
            }
            for log in logs.clone().unwrap_or(vec![]) {
                // raydium
                if log.contains(RAYDIUM_V4_POOL_PROGRAM_ID)
                    || log.contains(RAYDIUM_CPMM_POOL_PROGRAM_ID)
                    || log.contains(RAYDIUM_CLMM_POOL_PROGRAM_ID)
                {
                    info.dex_program_type = Some(DexProgramType::Raydium);
                    // pool
                    for log in logs.clone().unwrap_or(vec![]) {
                        if log.contains(RAYDIUM_V4_POOL_PROGRAM_ID) {
                            info.dex_program_id = Some(RAYDIUM_V4_POOL_PROGRAM_ID.to_string());
                            info.dex_pool_program_id = Some(RAYDIUM_V4_POOL_PROGRAM_ID.to_string());
                            info.dex_pool_program_name = Some("raydium-v4-pool".to_string());
                            info.transaction_type = Some(TransactionType::Swap);
                            for log in logs.clone().unwrap_or(vec![]) {
                                if (log.contains("MintTo")) {
                                    info.transaction_type = Some(TransactionType::AddLiquidity);
                                }
                                if (log.contains("Burn")) {
                                    info.transaction_type = Some(TransactionType::RemoveLiquidity);
                                }
                            }
                        }
                        if log.contains(RAYDIUM_CPMM_POOL_PROGRAM_ID) {
                            info.dex_program_id = Some(RAYDIUM_CPMM_POOL_PROGRAM_ID.to_string());
                            info.dex_pool_program_id =
                                Some(RAYDIUM_CPMM_POOL_PROGRAM_ID.to_string());
                            info.dex_pool_program_name = Some("raydium-cpmm-pool".to_string());
                            info.transaction_type = Some(TransactionType::Swap);
                            for log in logs.clone().unwrap_or(vec![]) {
                                if (log.contains("MintTo")) {
                                    info.transaction_type = Some(TransactionType::AddLiquidity);
                                }
                                if (log.contains("Burn")) {
                                    info.transaction_type = Some(TransactionType::RemoveLiquidity);
                                }
                            }
                        }
                        if log.contains(RAYDIUM_CLMM_POOL_PROGRAM_ID) {
                            info.dex_program_id = Some(RAYDIUM_CLMM_POOL_PROGRAM_ID.to_string());
                            info.dex_pool_program_id =
                                Some(RAYDIUM_CLMM_POOL_PROGRAM_ID.to_string());
                            info.dex_pool_program_name = Some("raydium-clmm-pool".to_string());
                            info.transaction_type = Some(TransactionType::Swap);
                            for log in logs.clone().unwrap_or(vec![]) {
                                if (log.contains("IncreaseLiquidityV2")) {
                                    info.transaction_type = Some(TransactionType::AddLiquidity);
                                }
                                if (log.contains("Burn")) {
                                    info.transaction_type = Some(TransactionType::RemoveLiquidity);
                                }
                            }
                        }
                    }
                    return;
                }
                if log.contains(METEORA_DAMM_V2_PROGRAM_ID)
                    || log.contains(METEORA_DLMM_V2_PROGRAM_ID)
                    || log.contains(METEORA_POOL_PROGRAM_ID)
                {
                    info.dex_program_type = Some(DexProgramType::Meteora);
                    info.transaction_type = Some(TransactionType::Swap);
                    // pool
                    for log in logs.clone().unwrap_or(vec![]) {
                        if log.contains(METEORA_DAMM_V2_PROGRAM_ID) {
                            info.dex_program_id = Some(METEORA_DAMM_V2_PROGRAM_ID.to_string());
                            info.dex_pool_program_id = Some(METEORA_DAMM_V2_PROGRAM_ID.to_string());
                            info.dex_pool_program_name = Some("meteora-damm-v2-pool".to_string());
                            info.transaction_type = Some(TransactionType::Swap);
                            for log in logs.clone().unwrap_or(vec![]) {
                                if (log.contains("AddLiquidity")) {
                                    info.transaction_type = Some(TransactionType::AddLiquidity);
                                }
                                if (log.contains("RemoveLiquidity")) {
                                    info.transaction_type = Some(TransactionType::RemoveLiquidity);
                                }
                            }
                        }
                    }
                    for log in logs.clone().unwrap_or(vec![]) {
                        if log.contains(METEORA_DLMM_V2_PROGRAM_ID) {
                            info.dex_program_id = Some(METEORA_DLMM_V2_PROGRAM_ID.to_string());
                            info.dex_pool_program_id = Some(METEORA_DLMM_V2_PROGRAM_ID.to_string());
                            info.dex_pool_program_name = Some("meteora-dlmm-v2-pool".to_string());
                            info.transaction_type = Some(TransactionType::Swap);
                        }
                    }
                    for log in logs.clone().unwrap_or(vec![]) {
                        if log.contains(METEORA_POOL_PROGRAM_ID) {
                            info.dex_program_id = Some(METEORA_POOL_PROGRAM_ID.to_string());
                            info.dex_pool_program_id = Some(METEORA_POOL_PROGRAM_ID.to_string());
                            info.dex_pool_program_name = Some("meteora-pool".to_string());
                            info.transaction_type = Some(TransactionType::Swap);
                            for log in logs.clone().unwrap_or(vec![]) {
                                if (log.contains("AddBalanceLiquidity")) {
                                    info.transaction_type = Some(TransactionType::AddLiquidity);
                                }
                                if (log.contains("RemoveBalanceLiquidity")) {
                                    info.transaction_type = Some(TransactionType::RemoveLiquidity);
                                }
                            }
                        }
                    }
                    return;
                }
                if log.contains(ORCA_WHIRLPOOLS_PROGRAM_ID) {
                    info.dex_program_type = Some(DexProgramType::Orca);
                    for log in logs.clone().unwrap_or(vec![]) {
                        if log.contains(ORCA_WHIRLPOOLS_PROGRAM_ID) {
                            info.dex_program_id = Some(ORCA_WHIRLPOOLS_PROGRAM_ID.to_string());
                            info.dex_pool_program_id = Some(ORCA_WHIRLPOOLS_PROGRAM_ID.to_string());
                            info.dex_pool_program_name = Some("orca-whirl-pools".to_string());
                            info.transaction_type = Some(TransactionType::Swap);
                            for log in logs.clone().unwrap_or(vec![]) {
                                if (log.contains("IncreaseLiquidity")) {
                                    info.transaction_type = Some(TransactionType::AddLiquidity);
                                }
                                if (log.contains("DecreaseLiquidity")) {
                                    info.transaction_type = Some(TransactionType::RemoveLiquidity);
                                }
                            }
                        }
                    }
                    return;
                }
            }
            // pump
            let pump_keywords = [
                "Buy",
                "buy",
                "Sell",
                "sell",
                "swap",
                "Swap",
                "liquidity",
                "Liquidity",
                "pool",
                "Pool",
                "Pump",
                "pump",
                "Pumpswap",
                "pumpswap",
                "pump.fun",
                "Pump.fun",
            ];
            for log in logs.clone().unwrap_or(vec![]) {
                if pump_keywords.iter().any(|&keyword| log.contains(keyword)) {
                    if (!info.is_swap) {
                        info.is_swap = true;
                    }
                }
            }
            for log in logs.clone().unwrap_or(vec![]) {
                if log.contains(PUMP_AAM_PROGRAM_ID) {
                    info.dex_program_id = Some(PUMP_AAM_PROGRAM_ID.to_string());
                    info.dex_program_type = Some(DexProgramType::PumpAAM);
                    info.transaction_type = Some(TransactionType::Swap);
                    let mut deposit: bool = false;
                    let mut mintTo: bool = false;
                    let mut burn: bool = false;
                    let mut withdraw: bool = false;
                    for log in logs.clone().unwrap_or(vec![]) {
                        if (log.contains("Instruction: Deposit")) {
                            deposit = true;
                        }
                        if (log.contains("Instruction: MintTo")) {
                            mintTo = true
                        }
                        if (log.contains("Instruction: Burn")) {
                            burn = true;
                        }
                        if (log.contains("Instruction: Withdraw")) {
                            withdraw = true
                        }
                    }
                    if (deposit && mintTo) {
                        info.transaction_type = Some(TransactionType::AddLiquidity);
                    }
                    if (burn && withdraw) {
                        info.transaction_type = Some(TransactionType::RemoveLiquidity);
                    }
                    return;
                }
                if log.contains(PUMP_BOND_CURVE_PROGRAM_ID) {
                    info.dex_program_id = Some(PUMP_BOND_CURVE_PROGRAM_ID.to_string());
                    info.dex_program_type = Some(DexProgramType::PumpBondCurve);
                    info.transaction_type = Some(TransactionType::Swap);
                    return;
                }
            }
        }
    }

    fn extract_transfer_info(
        info: &mut TransactionInfo,
        parsed_msg: &solana_transaction_status::UiParsedMessage,
    ) {
        // Find system transfer instructions
        if let instructions = &parsed_msg.instructions {
            for instruction in instructions {
                match instruction {
                    solana_transaction_status::UiInstruction::Parsed(parsed_inst) => {
                        match parsed_inst {
                            solana_transaction_status::UiParsedInstruction::Parsed(
                                parsed_instruction,
                            ) => {
                                if parsed_instruction.program_id == "system" {
                                    if let serde_json::Value::Object(parsed_obj) =
                                        &parsed_instruction.parsed
                                    {
                                        if let Some(serde_json::Value::String(type_str)) =
                                            parsed_obj.get("type")
                                        {
                                            if type_str == "transfer" {
                                                Self::extract_parsed_transfer_info(
                                                    info, parsed_obj,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    solana_transaction_status::UiInstruction::Compiled(compiled_inst) => {
                        if compiled_inst.program_id_index == 0 {
                            if let Some(transfer_info) =
                                Self::extract_compiled_transfer_info(compiled_inst, parsed_msg)
                            {
                                info.transaction_type = Some(TransactionType::Transfer);
                                info.program_id = "system".to_string();
                                info.from = transfer_info.from;
                                info.to = transfer_info.to;
                                info.value = transfer_info.amount.to_string();
                                info.value_sol =
                                    transfer_info.amount as f64 / LAMPORTS_PER_SOL as f64;
                            }
                        }
                    }
                }
            }
        }
    }

    // extract transfer information from parsed instructions
    fn extract_parsed_transfer_info(
        info: &mut TransactionInfo,
        parsed_obj: &serde_json::Map<String, serde_json::Value>,
    ) {
        info.transaction_type = Some(TransactionType::Transfer);
        info.program_id = "system".to_string();
        if let Some(serde_json::Value::Object(info_obj)) = parsed_obj.get("info") {
            if let (Some(from), Some(to), Some(lamports)) = (
                info_obj.get("source").and_then(|v| v.as_str()),
                info_obj.get("destination").and_then(|v| v.as_str()),
                info_obj.get("lamports").and_then(|v| v.as_u64()),
            ) {
                info.from = from.to_string();
                info.to = to.to_string();
                info.value = lamports.to_string();
                info.value_sol = lamports as f64 / LAMPORTS_PER_SOL as f64;
            }
        }
    }

    // extract transfer information from compilation instructions
    fn extract_compiled_transfer_info(
        compiled_inst: &solana_transaction_status::UiCompiledInstruction,
        parsed_msg: &solana_transaction_status::UiParsedMessage,
    ) -> Option<CompiledTransferInfo> {
        // System transfer command data format:
        // First 4 bytes: Command identifier (2 indicates transfer)
        // Last 8 bytes: Lamports count
        let data = &compiled_inst.data;
        if data.len() >= 12 {
            let instruction_id = u32::from_le_bytes([
                data.as_bytes()[0],
                data.as_bytes()[1],
                data.as_bytes()[2],
                data.as_bytes()[3],
            ]);
            if instruction_id == 2 {
                let lamports = u64::from_le_bytes([
                    data.as_bytes()[4],
                    data.as_bytes()[5],
                    data.as_bytes()[6],
                    data.as_bytes()[7],
                    data.as_bytes()[8],
                    data.as_bytes()[9],
                    data.as_bytes()[10],
                    data.as_bytes()[11],
                ]);
                if let (Some(from_index), Some(to_index)) =
                    (compiled_inst.accounts.get(0), compiled_inst.accounts.get(1))
                {
                    if let account_keys = &parsed_msg.account_keys {
                        if let (Some(from_acc), Some(to_acc)) = (
                            account_keys.get(*from_index as usize),
                            account_keys.get(*to_index as usize),
                        ) {
                            if let (from_pubkey, to_pubkey) = (&from_acc.pubkey, &to_acc.pubkey) {
                                return Some(CompiledTransferInfo {
                                    from: from_pubkey.clone(),
                                    to: to_pubkey.clone(),
                                    amount: lamports,
                                });
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn extract_program_id_from_ui_instruction(
        inst: &solana_transaction_status::UiInstruction,
    ) -> String {
        match inst {
            solana_transaction_status::UiInstruction::Compiled(compiled) => {
                compiled.program_id_index.to_string()
            }
            solana_transaction_status::UiInstruction::Parsed(parsed) => match parsed {
                UiParsedInstruction::Parsed(parsed_instruction) => {
                    parsed_instruction.program_id.clone()
                }
                UiParsedInstruction::PartiallyDecoded(ui_partially_decoded_instruction) => {
                    ui_partially_decoded_instruction.program_id.clone()
                }
            },
        }
    }

    fn extract_accounts_from_ui_instruction(
        inst: &solana_transaction_status::UiInstruction,
    ) -> Vec<String> {
        match inst {
            solana_transaction_status::UiInstruction::Compiled(compiled) => compiled
                .accounts
                .iter()
                .map(|idx| idx.to_string())
                .collect(),
            solana_transaction_status::UiInstruction::Parsed(parsed) => match parsed {
                solana_transaction_status::UiParsedInstruction::Parsed(parsed_instruction) => {
                    Self::extract_accounts_from_parsed_value(&parsed_instruction.parsed)
                }
                solana_transaction_status::UiParsedInstruction::PartiallyDecoded(
                    partially_decoded,
                ) => partially_decoded
                    .accounts
                    .iter()
                    .map(|acc| acc.to_string())
                    .collect(),
            },
        }
    }

    fn extract_accounts_from_parsed_value(parsed: &serde_json::Value) -> Vec<String> {
        let mut accounts = Vec::new();
        if let Some(obj) = parsed.as_object() {
            if let Some(info) = obj.get("info") {
                if let Some(info_obj) = info.as_object() {
                    let account_fields = [
                        "source",
                        "destination",
                        "account",
                        "from",
                        "to",
                        "authority",
                    ];
                    for field in account_fields {
                        if let Some(account) = info_obj.get(field).and_then(|v| v.as_str()) {
                            accounts.push(account.to_string());
                        }
                    }
                }
            }
        }
        accounts
    }

    fn extract_data_from_ui_instruction(inst: &solana_transaction_status::UiInstruction) -> String {
        match inst {
            solana_transaction_status::UiInstruction::Compiled(compiled) => compiled.data.clone(),
            solana_transaction_status::UiInstruction::Parsed(parsed) => match parsed {
                solana_transaction_status::UiParsedInstruction::Parsed(parsed_instruction) => {
                    serde_json::to_string(&parsed_instruction.parsed).unwrap_or_default()
                }
                solana_transaction_status::UiParsedInstruction::PartiallyDecoded(
                    partially_decoded,
                ) => partially_decoded.data.clone(),
            },
        }
    }

    /// Determine whether the specified address in the current transaction is the recipient
    pub fn is_recipient(&self, address: &str) -> bool {
        Trade::is_address_recipient_in_transaction(self, address)
    }

    /// Determine whether the specified address in the current transaction is the payer
    pub fn is_payer(&self, address: &str) -> bool {
        Trade::is_address_payer_in_transaction(self, address)
    }

    /// Get the payment amount of the transaction (lamports)
    pub fn get_payment_amount(&self) -> u64 {
        self.value.parse::<u64>().unwrap_or(0)
    }

    /// Get the payment amount of the transaction (SOL)
    pub fn get_payment_amount_sol(&self) -> f64 {
        self.value_sol
    }

    /// get trade direction
    pub fn get_direction(&self) -> Direction {
        if (self.get_spent_token_sol().unwrap().0 == USDC
            || self.get_spent_token_sol().unwrap().0 == USDT
            || self.get_spent_token_sol().unwrap().0 == SOL)
        {
            Direction::Buy
        } else {
            Direction::Sell
        }
    }

    pub fn is_swap(&self) -> bool {
        self.is_swap
    }
}

impl Default for TransactionInfo {
    fn default() -> Self {
        Self {
            transaction_hash: String::new(),
            signature: String::new(),
            from: String::new(),
            to: String::new(),
            signer: String::new(),
            fee_payer: String::new(),
            signers: Vec::new(),
            involved_accounts: Vec::new(),
            writable_accounts: Vec::new(),
            readonly_accounts: Vec::new(),
            value: "0".to_string(),
            value_sol: 0.0,
            fee: 0,
            fee_sol: 0.0,
            pre_balance: 0,
            post_balance: 0,
            balance_change: 0,
            block_number: 0,
            block_hash: String::new(),
            block_time: None,
            slot: 0,
            epoch: 0,
            recent_blockhash: String::new(),
            status: "unknown".to_string(),
            confirmation_status: "processed".to_string(),
            error_message: None,
            err: None,
            is_confirmed: false,
            is_finalized: false,
            transaction_type: None,
            program_id: String::new(),
            instructions_count: 0,
            inner_instructions_count: 0,
            version: 0,
            compute_units_consumed: None,
            compute_unit_price: None,
            log_index: 0,
            data: None,
            logs: Vec::new(),
            instructions: Vec::new(),
            inner_instructions: Vec::new(),
            token_mint: None,
            token_amount: None,
            token_decimals: None,
            token_symbol: None,
            token_name: None,
            pre_token_balances: Vec::new(),
            post_token_balances: Vec::new(),
            is_nft_transfer: false,
            nft_mint: None,
            nft_name: None,
            nft_symbol: None,
            is_swap: false,

            dex_program_id: None,        // DEX program id
            dex_program_type: None,      // DEX program name
            dex_pool_program_id: None,   // DEX program pool id
            dex_pool_program_name: None, // DEX program pool name

            input_mint: None,
            output_mint: None,
            input_amount: None,
            output_amount: None,
            memo: None,
            timestamp: None,
            tags: Vec::new(),
            category: "general".to_string(),
            risk_level: 0,
            is_internal: false,
            gas_used: None,
            gas_price: None,
            max_fee: None,
            priority_fee: None,
            cluster: String::new(),
            rpc_node: String::new(),
            created_at: 0,
            updated_at: 0,
            source: "rpc".to_string(),
            confidence: 1.0,
        }
    }
}

impl TransactionInfo {
    pub fn is_pump(&self) -> bool {
        if let Some(dex_type) = &self.dex_program_type {
            if *dex_type == DexProgramType::PumpBondCurve || *dex_type == DexProgramType::PumpAAM {
                return true;
            }
        }
        return false;
    }
    pub fn get_pump_bond_curve_transaction_info(&self) -> PumpBondCurveTransactionInfo {
        PumpBondCurveTransactionInfo::new(self)
    }
}

impl TransactionInfo {
    pub fn is_successful(&self) -> bool {
        self.status == "success"
    }

    pub fn is_token_transfer(&self) -> bool {
        self.token_mint.is_some()
    }

    pub fn get_net_amount(&self) -> i64 {
        self.balance_change - self.fee as i64
    }

    pub fn is_high_value(&self) -> bool {
        self.value_sol > 1000.0
    }
}

impl TransactionInfo {
    /// is pump bond curve trade
    pub fn is_pump_bond_curve_trade(&self) -> bool {
        for log in &self.logs {
            if log.contains(PUMP_BOND_CURVE_PROGRAM_ID) {
                return true;
            }
        }
        if let Some(dex_program_id) = &self.dex_program_id {
            if dex_program_id == PUMP_BOND_CURVE_PROGRAM_ID {
                return true;
            }
        }
        for instruction in &self.instructions {
            if instruction.program_id == PUMP_BOND_CURVE_PROGRAM_ID {
                return true;
            }
        }
        false
    }

    /// is meteora dbc trade
    pub fn is_meteora_dbc_trade(&self) -> bool {
        for log in &self.logs {
            if log.contains(METEORA_DYNAMIC_BOND_CURVE_PROGRAM_ID) {
                return true;
            }
        }
        if let Some(dex_program_id) = &self.dex_program_id {
            if dex_program_id == METEORA_DYNAMIC_BOND_CURVE_PROGRAM_ID {
                return true;
            }
        }
        for instruction in &self.instructions {
            if instruction.program_id == METEORA_DYNAMIC_BOND_CURVE_PROGRAM_ID {
                return true;
            }
        }
        for log in &self.logs {
            if log.contains(METEORA_DLMM_V2_PROGRAM_ID) {
                return true;
            }
        }
        false
    }

    /// is raydium launchpad trade
    pub fn is_raydium_launchpad_trade(&self) -> bool {
        for log in &self.logs {
            if log.contains(RAYDIUM_LAUNCHPAD_PROGRAM_ID) {
                return true;
            }
        }
        if let Some(dex_program_id) = &self.dex_program_id {
            if dex_program_id == RAYDIUM_LAUNCHPAD_PROGRAM_ID {
                return true;
            }
        }
        for instruction in &self.instructions {
            if instruction.program_id == RAYDIUM_LAUNCHPAD_PROGRAM_ID {
                return true;
            }
        }
        if let Some(dex_type) = &self.dex_program_type {
            if *dex_type == DexProgramType::Raydium {
                for log in &self.logs {
                    if log.contains("launchpad")
                        || log.contains("Launchpad")
                        || log.contains("IDO")
                        || log.contains("ido")
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    // is vote program
    pub fn is_vote_program(&self) -> bool {
        use crate::global::VOTE_PROGRAM_ID;
        if self.program_id == *VOTE_PROGRAM_ID {
            return true;
        }
        for instruction in &self.instructions {
            if instruction.program_id == *VOTE_PROGRAM_ID {
                return true;
            }
        }
        for inner_instruction in &self.inner_instructions {
            for instruction in &inner_instruction.instructions {
                if instruction.program_id == *VOTE_PROGRAM_ID {
                    return true;
                }
            }
        }
        for log in &self.logs {
            if log.contains(VOTE_PROGRAM_ID) {
                return true;
            }
            if log.contains("vote") && (log.contains("program") || log.contains("Program")) {
                return true;
            }
        }
        false
    }
}

impl TransactionInfo {
    pub fn display(&self) {
        println!(
            "
==================================
Bond Curve Type
==================================
Is Raydium Launchpad: {:?}
Is Pump: {:?}
Is Meteora Dynamic Bond Curve: {:?}
==================================
Transaction Info
==================================
Trading Direction: {:?}
Base Token Info: {:?} - {:?}
Quote Token Info: {:?} - {:?}
Received Token: {:?} - {:?}
Spent Token: {:?} - {:?}
            ",
            self.is_raydium_launchpad_trade(),
            self.is_pump_bond_curve_trade(),
            self.is_meteora_dbc_trade(),
            self.get_direction(),
            self.get_pool_left_address(),
            self.get_pool_left_amount_sol(),
            self.get_pool_right_address(),
            self.get_pool_right_amount_sol(),
            self.get_received_token(),
            self.get_received_token_sol(),
            self.get_spent_token(),
            self.get_spent_token_sol(),
        );
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    Success,
    Failed(String),
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionInfo {
    pub program_id: String,
    pub accounts: Vec<String>,
    pub data: String,
    pub stack_height: Option<u32>,
    pub program: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnerInstructionInfo {
    pub index: u8,
    pub instructions: Vec<InstructionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub account_index: u8,
    pub mint: String,
    pub owner: String,
    pub ui_token_amount: UiTokenAmount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiTokenAmount {
    pub ui_amount: Option<f64>,
    pub decimals: u8,
    pub amount: String,
    pub ui_amount_string: Option<String>,
}

struct CompiledTransferInfo {
    from: String,
    to: String,
    amount: u64,
}
