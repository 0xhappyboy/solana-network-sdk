use anyhow::Result;
use serde::{Deserialize, Serialize};
use solana_client::{
    nonblocking::rpc_client::RpcClient, rpc_client::GetConfirmedSignaturesForAddress2Config,
    rpc_config::RpcTransactionConfig, rpc_response::RpcConfirmedTransactionStatusWithSignature,
};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, UiInstruction, UiParsedInstruction,
    UiTransactionEncoding, option_serializer::OptionSerializer,
};
use std::collections::HashMap;
use std::io::Write;
use std::str::FromStr;
use std::sync::Arc;

use crate::Solana;
use crate::global::{SOL, USDC, USDT};
use crate::types::Mode;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenTradeRecord {
    
    pub signature: String,
    
    pub block_time: i64,
    
    pub slot: u64,
    
    pub trade_type: String,
    
    pub token_mint: String,
    
    pub from: Option<String>,
    
    pub to: Option<String>,
    
    pub base_amount: f64,
    pub base_address: Option<String>,
    
    pub quote_amount: f64,
    pub quote_address: Option<String>,
    
    pub base_decimals: u8,
    
    pub quote_decimals: Option<u8>,
    
    pub quote_mint: Option<String>,
    
    pub is_dex: bool,
    
    pub dex_program: Option<String>,
    
    pub input_mint: Option<String>,
    
    pub output_mint: Option<String>,
    
    pub fee: u64,
    
    pub status: String,
    
    pub side: Option<String>,
}

pub struct Scan {
    client: Arc<RpcClient>,
}

impl Scan {
    
    pub fn new(client: Arc<RpcClient>) -> Self {
        Self { client }
    }

    pub async fn get_token_trade_history(
        &self,
        token_address: &str,
    ) -> Result<Vec<TokenTradeRecord>> {
        let mint_pubkey = Pubkey::from_str(token_address)?;
        let mut all_records = Vec::new();
        let mut before: Option<Signature> = None;
        loop {
            let signatures = match self.fetch_token_signatures(&mint_pubkey, before).await {
                Ok(sigs) => sigs,
                Err(e) => {
                    break;
                }
            };
            if signatures.is_empty() {
                break;
            }
            let records = match self.parse_transactions(&signatures, token_address).await {
                Ok(recs) => recs,
                Err(e) => Vec::new(),
            };
            all_records.extend(records);
            if let Some(last_sig) = signatures.last() {
                before = match Signature::from_str(&last_sig.signature) {
                    Ok(sig) => Some(sig),
                    Err(e) => {
                        break;
                    }
                };
            } else {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            
            if all_records.len() >= 10 {
                break;
            }
        }
        Ok(all_records)
    }

    
    async fn fetch_token_signatures(
        &self,
        mint_pubkey: &Pubkey,
        before: Option<Signature>,
    ) -> Result<Vec<RpcConfirmedTransactionStatusWithSignature>> {
        let config = GetConfirmedSignaturesForAddress2Config {
            before,
            until: None,
            limit: Some(100), 
            commitment: None,
        };
        match self
            .client
            .get_signatures_for_address_with_config(mint_pubkey, config)
            .await
        {
            Ok(signatures) => Ok(signatures),
            Err(e) => Err(e.into()),
        }
    }
    
    async fn parse_transactions(
        &self,
        signatures: &[RpcConfirmedTransactionStatusWithSignature],
        token_mint: &str,
    ) -> Result<Vec<TokenTradeRecord>> {
        let mut records = Vec::new();
        for (i, sig_info) in signatures.iter().enumerate() {
            let signature = sig_info.signature.clone();
            match self.fetch_transaction_detail(&signature).await {
                Ok(tx) => {
                    match self
                        .parse_transaction_to_record(&tx, token_mint, &signature)
                        .await
                    {
                        Some(record) => {
                            records.push(record);
                        }
                        None => {}
                    }
                }
                Err(e) => {}
            }
            if i >= 2 {
                break;
            }
        }
        Ok(records)
    }
    
    async fn fetch_transaction_detail(
        &self,
        signature: &str,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta> {
        let sig = Signature::from_str(signature)?;
        let config = RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Json),
            commitment: None,
            max_supported_transaction_version: Some(0),
        };
        let tx = self
            .client
            .get_transaction_with_config(&sig, config)
            .await?;
        Ok(tx)
    }

    
    async fn parse_transaction_to_record(
        &self,
        tx: &EncodedConfirmedTransactionWithStatusMeta,
        token_mint: &str,
        signature_str: &str,
    ) -> Option<TokenTradeRecord> {
        let meta = tx.transaction.meta.as_ref()?;
        let mut has_target_token = false;
        if let OptionSerializer::Some(pre_balances) = &meta.pre_token_balances {
            for balance in pre_balances {
                if balance.mint == token_mint {
                    has_target_token = true;
                    break;
                }
            }
        }
        if !has_target_token {
            if let OptionSerializer::Some(post_balances) = &meta.post_token_balances {
                for balance in post_balances {
                    if balance.mint == token_mint {
                        has_target_token = true;
                        break;
                    }
                }
            }
        }
        if !has_target_token {
            return None;
        }
        let mut record = TokenTradeRecord {
            signature: signature_str.to_string(),
            block_time: tx.block_time.unwrap_or(0),
            slot: tx.slot,
            trade_type: "unknown".to_string(),
            token_mint: token_mint.to_string(),
            from: None,
            to: None,
            base_amount: 0.0,
            base_address: Some("".to_string()),
            quote_amount: 0.0,
            quote_address: Some("".to_string()),
            base_decimals: 0,
            quote_decimals: None,
            quote_mint: None,
            is_dex: false,
            dex_program: None,
            input_mint: None,
            output_mint: None,
            fee: meta.fee,
            status: if meta.err.is_none() {
                "success".to_string()
            } else {
                "failed".to_string()
            },
            side: None,
        };
        self.parse_dex_info(&mut record, meta);
        if record.is_dex {
            self.parse_dex_swap_info(&mut record, meta, token_mint);
        } else {
            self.parse_token_transfer_info(&mut record, meta, token_mint);
        }
        Some(record)
    }

    
    fn parse_token_transfer_info(
        &self,
        record: &mut TokenTradeRecord,
        meta: &solana_transaction_status::UiTransactionStatusMeta,
        token_mint: &str,
    ) {
        if let (OptionSerializer::Some(pre_balances), OptionSerializer::Some(post_balances)) =
            (&meta.pre_token_balances, &meta.post_token_balances)
        {
            for pre_balance in pre_balances {
                if pre_balance.mint != token_mint {
                    continue;
                }
                for post_balance in post_balances {
                    if post_balance.mint != token_mint || pre_balance.owner != post_balance.owner {
                        continue;
                    }
                    let pre_amount: u64 = pre_balance.ui_token_amount.amount.parse().unwrap_or(0);
                    let post_amount: u64 = post_balance.ui_token_amount.amount.parse().unwrap_or(0);
                    record.base_decimals = pre_balance.ui_token_amount.decimals;
                    if pre_amount > post_amount {
                        record.trade_type = "transfer".to_string();
                        record.from = pre_balance.owner.clone().into();
                        record.base_amount = (pre_amount - post_amount) as f64
                            / (10u64.pow(record.base_decimals as u32)) as f64;
                    } else if post_amount > pre_amount {
                        record.trade_type = "transfer".to_string();
                        record.to = post_balance.owner.clone().into();
                        record.base_amount = (post_amount - pre_amount) as f64
                            / (10u64.pow(record.base_decimals as u32)) as f64;
                    }
                    break;
                }
            }
        }
        if record.base_amount == 0.0 {
            if let OptionSerializer::Some(post_balances) = &meta.post_token_balances {
                for balance in post_balances {
                    if balance.mint == token_mint {
                        record.base_decimals = balance.ui_token_amount.decimals;
                        let amount: u64 = balance.ui_token_amount.amount.parse().unwrap_or(0);
                        if let OptionSerializer::Some(logs) = &meta.log_messages {
                            for log in logs {
                                if log.contains("MintTo") {
                                    record.trade_type = "mint".to_string();
                                    record.to = balance.owner.clone().into();
                                    record.base_amount = amount as f64
                                        / (10u64.pow(record.base_decimals as u32)) as f64;
                                    return;
                                } else if log.contains("Burn") {
                                    record.trade_type = "burn".to_string();
                                    record.from = balance.owner.clone().into();
                                    record.base_amount = amount as f64
                                        / (10u64.pow(record.base_decimals as u32)) as f64;
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn parse_dex_info(
        &self,
        record: &mut TokenTradeRecord,
        meta: &solana_transaction_status::UiTransactionStatusMeta,
    ) {
        if let OptionSerializer::Some(logs) = &meta.log_messages {
            for log in logs {
                if log.contains("Swap") {
                    record.trade_type = "swap".to_string();
                    record.is_dex = true;
                }

                if log.contains("Raydium") {
                    record.dex_program = Some("Raydium".to_string());
                } else if log.contains("Orca") {
                    record.dex_program = Some("Orca".to_string());
                } else if log.contains("Jupiter") {
                    record.dex_program = Some("Jupiter".to_string());
                } else if log.contains("pump") || log.contains("Pump") {
                    record.dex_program = Some("Pump".to_string());
                }
            }
        }
        if let (OptionSerializer::Some(pre), OptionSerializer::Some(post)) =
            (&meta.pre_token_balances, &meta.post_token_balances)
        {
            let mut best = 0.0;
            for (a, b) in pre.iter().zip(post.iter()) {
                if a.mint == USDC || a.mint == USDT {
                    let pre_amt = a.ui_token_amount.ui_amount.unwrap_or(0.0);
                    let post_amt = b.ui_token_amount.ui_amount.unwrap_or(0.0);
                    let abs = (post_amt - pre_amt).abs();
                    if abs > best {
                        best = abs;
                        record.quote_amount = abs;
                        record.quote_address = Some(a.mint.clone());
                        record.quote_decimals = Some(a.ui_token_amount.decimals);
                    }
                }
            }
        }
        if record.quote_amount == 0.0 {
            if !meta.pre_balances.is_empty() && !meta.post_balances.is_empty() {
                let pre = meta.pre_balances[0] as f64 / 1e9;
                let post = meta.post_balances[0] as f64 / 1e9;
                let abs = (post - pre).abs();
                if abs > 0.0 {
                    record.quote_amount = abs;
                    record.quote_address = Some(SOL.to_string());
                    record.quote_decimals = Some(9);
                }
            }
        }
    }

    
    
    fn parse_dex_swap_info(
        &self,
        record: &mut TokenTradeRecord,
        meta: &solana_transaction_status::UiTransactionStatusMeta,
        token_mint: &str,
    ) {
        
        let (pre_balances, post_balances) =
            match (&meta.pre_token_balances, &meta.post_token_balances) {
                (OptionSerializer::Some(pre), OptionSerializer::Some(post)) => (pre, post),
                _ => {
                    return;
                }
            };

        
        let mut base_change = 0i64;
        let mut base_decimals = 0u8;

        for balance in pre_balances {
            if balance.mint == token_mint {
                let pre_amount: i64 = balance.ui_token_amount.amount.parse().unwrap_or(0);
                base_decimals = balance.ui_token_amount.decimals;

                for post_balance in post_balances {
                    if post_balance.mint == token_mint && balance.owner == post_balance.owner {
                        let post_amount: i64 =
                            post_balance.ui_token_amount.amount.parse().unwrap_or(0);
                        base_change = post_amount - pre_amount;
                        break;
                    }
                }
                if base_change != 0 {
                    break;
                }
            }
        }

        if base_change == 0 {
            for post_balance in post_balances {
                if post_balance.mint == token_mint {
                    base_decimals = post_balance.ui_token_amount.decimals;
                    let post_amount: i64 = post_balance.ui_token_amount.amount.parse().unwrap_or(0);

                    let mut has_pre = false;
                    for pre_balance in pre_balances {
                        if pre_balance.mint == token_mint && pre_balance.owner == post_balance.owner
                        {
                            has_pre = true;
                            let pre_amount: i64 =
                                pre_balance.ui_token_amount.amount.parse().unwrap_or(0);
                            base_change = post_amount - pre_amount;
                            break;
                        }
                    }
                    if !has_pre && post_amount > 0 {
                        base_change = post_amount;
                    }
                    break;
                }
            }
        }

        if base_change != 0 {
            record.base_decimals = base_decimals;
            record.base_amount =
                (base_change.abs() as f64) / (10u64.pow(base_decimals as u32)) as f64;

            
            if base_change > 0 {
                record.from = Some("dex".to_string());
                record.to = Some("user".to_string());
                record.trade_type = "swap_buy".to_string();
                record.side = Some("buy".to_string());
            } else if base_change < 0 {
                record.from = Some("user".to_string());
                record.to = Some("dex".to_string());
                record.trade_type = "swap_sell".to_string();
                record.side = Some("sell".to_string());
            }
        }

        
        let mut pre_map = HashMap::new();
        let mut post_map = HashMap::new();

        for balance in pre_balances {
            let owner = match &balance.owner {
                OptionSerializer::Some(owner) => owner.clone(),
                _ => continue,
            };
            let key = (balance.mint.clone(), owner);
            let amount = balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);
            pre_map.insert(key, amount);
        }

        for balance in post_balances {
            let owner = match &balance.owner {
                OptionSerializer::Some(owner) => owner.clone(),
                _ => continue,
            };
            let key = (balance.mint.clone(), owner);
            let amount = balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);
            post_map.insert(key, amount);
        }

        
        let mut best_quote_change = 0i64;
        let mut best_quote_mint = None;
        let mut best_quote_decimals = None;

        
        let common_quote_mints = [
            (USDC.to_string(), 6), 
            (USDT.to_string(), 6), 
            (SOL.to_string(), 9),  
        ];

        for (mint_addr, decimals) in &common_quote_mints {
            if *mint_addr == token_mint {
                continue; 
            }

            
            let mut total_change: i64 = 0;

            
            let mut all_keys = Vec::new();
            for key in pre_map.keys().chain(post_map.keys()) {
                if key.0 == *mint_addr && !all_keys.contains(key) {
                    all_keys.push(key.clone());
                }
            }

            for key in all_keys {
                let pre_amount = pre_map.get(&key).unwrap_or(&0);
                let post_amount = post_map.get(&key).unwrap_or(&0);
                let change = *post_amount as i64 - *pre_amount as i64;
                total_change += change;
            }

            
            if total_change.abs() > best_quote_change.abs() && total_change != 0 {
                best_quote_change = total_change;
                best_quote_mint = Some(mint_addr.clone());
                best_quote_decimals = Some(*decimals);
            }
        }

        
        if best_quote_change == 0 {
            if let OptionSerializer::Some(logs) = &meta.log_messages {
                for log in logs {
                    if log.contains("Swap")
                        && (log.contains("$")
                            || log.contains("SOL")
                            || log.contains("USDC")
                            || log.contains("USDT"))
                    {
                        if let Some(dollar_idx) = log.find('$') {
                            let after_dollar = &log[dollar_idx + 1..];
                            if let Some(end_idx) =
                                after_dollar.find(|c: char| !c.is_numeric() && c != '.')
                            {
                                let dollar_str = &after_dollar[..end_idx];
                                if let Ok(dollar_value) = dollar_str.parse::<f64>() {
                                    record.quote_amount = dollar_value;
                                    record.quote_mint = Some("USD".to_string());
                                    record.quote_decimals = Some(2);
                                    record.quote_address = Some("USD".to_string());
                                    return;
                                }
                            }
                        }
                        let parts: Vec<&str> = log.split_whitespace().collect();
                        for i in 0..parts.len().saturating_sub(1) {
                            if let Ok(amount) = parts[i].parse::<f64>() {
                                let next_part = parts[i + 1].to_lowercase();
                                if next_part.contains("sol") {
                                    record.quote_amount = amount;
                                    record.quote_mint = Some(SOL.to_string());
                                    record.quote_decimals = Some(9);
                                    record.quote_address = Some(SOL.to_string());
                                    if base_change > 0 {
                                        record.input_mint = Some(SOL.to_string());
                                        record.output_mint = Some(token_mint.to_string());
                                    } else {
                                        record.input_mint = Some(token_mint.to_string());
                                        record.output_mint = Some(SOL.to_string());
                                    }
                                    return;
                                } else if next_part.contains("usdc") {
                                    record.quote_amount = amount;
                                    record.quote_mint = Some(USDC.to_string());
                                    record.quote_decimals = Some(6);
                                    record.quote_address = Some(USDC.to_string());
                                    if base_change > 0 {
                                        record.input_mint = Some(USDC.to_string());
                                        record.output_mint = Some(token_mint.to_string());
                                    } else {
                                        record.input_mint = Some(token_mint.to_string());
                                        record.output_mint = Some(USDC.to_string());
                                    }
                                    return;
                                } else if next_part.contains("usdt") {
                                    record.quote_amount = amount;
                                    record.quote_mint = Some(USDT.to_string());
                                    record.quote_decimals = Some(6);
                                    record.quote_address = Some(USDT.to_string());
                                    if base_change > 0 {
                                        record.input_mint = Some(USDT.to_string());
                                        record.output_mint = Some(token_mint.to_string());
                                    } else {
                                        record.input_mint = Some(token_mint.to_string());
                                        record.output_mint = Some(USDT.to_string());
                                    }
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
        if let (Some(qm), Some(qd)) = (best_quote_mint, best_quote_decimals) {
            record.quote_mint = Some(qm.clone());
            record.quote_decimals = Some(qd);
            record.quote_amount = (best_quote_change.abs() as f64) / (10u64.pow(qd as u32)) as f64;
            record.quote_address = Some(qm.clone());
            if base_change > 0 {
                record.input_mint = Some(qm.clone());
                record.output_mint = Some(token_mint.to_string());
            } else if base_change < 0 {
                record.input_mint = Some(token_mint.to_string());
                record.output_mint = Some(qm);
            }
        }
    }
}
