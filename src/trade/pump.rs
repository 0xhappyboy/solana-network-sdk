use crate::global::{QUOTES, SOL, SPL_TOKEN_PROGRAM_V1, USDC, USDT};
use crate::{trade::TransactionInfo, types::Direction};
use base64::engine::general_purpose;
use base64::{self, Engine};
use solana_sdk::native_token::LAMPORTS_PER_SOL;

pub struct PumpBondCurveTransactionInfo<'a> {
    transaction_info: &'a TransactionInfo,
}

impl<'a> PumpBondCurveTransactionInfo<'a> {
    pub fn new(transaction_info: &'a TransactionInfo) -> Self {
        Self { transaction_info }
    }

    pub fn get_token_quote_ratio(&self) -> Option<f64> {
        let direction = self.get_pump_direction()?;
        match direction {
            Direction::Buy => {
                if let Some((spent_token, spent_amount)) = self.get_pump_spent_token() {
                    if QUOTES.contains(&spent_token.as_str()) {
                        if let Some((received_token, received_amount)) =
                            self.get_pump_received_token()
                        {
                            if !QUOTES.contains(&received_token.as_str()) && received_amount > 0 {
                                let spent_decimals = self.get_pump_token_decimals(&spent_token)?;
                                let received_decimals =
                                    self.get_pump_token_decimals(&received_token)?;
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
                if let Some((spent_token, spent_amount)) = self.get_pump_spent_token() {
                    if !QUOTES.contains(&spent_token.as_str()) && spent_amount > 0 {
                        if let Some((received_token, received_amount)) =
                            self.get_pump_received_token()
                        {
                            if QUOTES.contains(&received_token.as_str()) {
                                let spent_decimals = self.get_pump_token_decimals(&spent_token)?;
                                let received_decimals =
                                    self.get_pump_token_decimals(&received_token)?;
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
        if let Some(price_sol) = self.get_pump_token_price_sol() {
            return Some(price_sol);
        }
        None
    }

    pub fn get_pump_pool_left_address(&self) -> Option<String> {
        for log in &self.transaction_info.logs {
            if log.contains("TransferChecked") {
                let words: Vec<&str> = log.split_whitespace().collect();
                for word in &words {
                    let trimmed = word.trim_matches(|c: char| !c.is_alphanumeric());
                    if trimmed.len() == 44
                        && trimmed.chars().all(|c| c.is_alphanumeric())
                        && trimmed != SOL
                        && trimmed != USDC
                        && trimmed != USDT
                        && trimmed != SPL_TOKEN_PROGRAM_V1
                    {
                        return Some(trimmed.to_string());
                    }
                }
            }
        }
        for balance in &self.transaction_info.pre_token_balances {
            let mint = &balance.mint;
            if mint != SOL && mint != USDC && mint != USDT {
                return Some(mint.clone());
            }
        }
        None
    }

    pub fn get_pump_pool_left_amount(&self) -> Option<u64> {
        if let Some((_, amount)) = self.transaction_info.get_token_received_amount() {
            return Some(amount);
        }
        for log in &self.transaction_info.logs {
            if let Some((_, amount)) = self.decode_pump_base64_data(log) {
                return Some(amount);
            }
        }
        for log in &self.transaction_info.logs {
            if log.contains("for") && log.contains("DEGEN") {
                let parts: Vec<&str> = log.split_whitespace().collect();
                for (i, part) in parts.iter().enumerate() {
                    if part.contains("DEGEN") && i > 0 {
                        let amount_str = parts[i - 1].replace(',', "");
                        if let Ok(amount_f64) = amount_str.parse::<f64>() {
                            return Some((amount_f64 * 1_000_000.0) as u64);
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get_pump_pool_right_address(&self) -> Option<String> {
        Some(SOL.to_string())
    }

    pub fn get_pump_pool_right_amount(&self) -> Option<u64> {
        if let Some((spent_token, spent_amount)) = self.transaction_info.get_token_spent_amount() {
            if spent_token == SOL {
                return Some(spent_amount);
            }
        }
        if let Ok(lamports) = self.transaction_info.value.parse::<u64>() {
            if lamports > 0 {
                return Some(lamports);
            }
        }
        for log in &self.transaction_info.logs {
            if log.contains("SOL") && log.contains("for") {
                let parts: Vec<&str> = log.split_whitespace().collect();
                for (i, part) in parts.iter().enumerate() {
                    if part.contains("SOL") && i > 0 {
                        let amount_str = parts[i - 1];
                        if let Ok(amount_f64) = amount_str.parse::<f64>() {
                            return Some((amount_f64 * LAMPORTS_PER_SOL as f64) as u64);
                        }
                    }
                }
            }
        }
        for log in &self.transaction_info.logs {
            if let Some((token, amount)) = self.decode_pump_base64_data(log) {
                if token == SOL {
                    return Some(amount);
                }
            }
        }
        None
    }

    pub fn get_pump_received_token(&self) -> Option<(String, u64)> {
        let meme_token_decrease = self.get_meme_token_decrease();
        let sol_increase = self.get_sol_increase();
        if meme_token_decrease.is_some() && sol_increase.is_some() {
            return sol_increase;
        }
        let sol_decrease = self.get_sol_decrease();
        let meme_token_increase = self.get_meme_token_increase();
        if sol_decrease.is_some() && meme_token_increase.is_some() {
            return meme_token_increase;
        }
        self.get_received_token_fallback()
    }

    fn get_meme_token_decrease(&self) -> Option<(String, u64)> {
        for pre_balance in &self.transaction_info.pre_token_balances {
            let mint = &pre_balance.mint;
            if mint != SOL && mint != USDC && mint != USDT {
                let post_amount = self
                    .transaction_info
                    .post_token_balances
                    .iter()
                    .find(|b| b.mint == *mint && b.owner == pre_balance.owner)
                    .and_then(|b| b.ui_token_amount.amount.parse::<u64>().ok())
                    .unwrap_or(0);
                let pre_amount = pre_balance
                    .ui_token_amount
                    .amount
                    .parse::<u64>()
                    .unwrap_or(0);
                if pre_amount > post_amount {
                    return Some((mint.clone(), pre_amount - post_amount));
                }
            }
        }
        None
    }

    fn get_sol_increase(&self) -> Option<(String, u64)> {
        for post_balance in &self.transaction_info.post_token_balances {
            if post_balance.mint == SOL {
                let pre_amount = self
                    .transaction_info
                    .pre_token_balances
                    .iter()
                    .find(|b| b.mint == SOL && b.owner == post_balance.owner)
                    .and_then(|b| b.ui_token_amount.amount.parse::<u64>().ok())
                    .unwrap_or(0);
                let post_amount = post_balance
                    .ui_token_amount
                    .amount
                    .parse::<u64>()
                    .unwrap_or(0);

                if post_amount > pre_amount {
                    return Some((SOL.to_string(), post_amount - pre_amount));
                }
            }
        }
        if self.transaction_info.balance_change > 0 {
            return Some((SOL.to_string(), self.transaction_info.balance_change as u64));
        }
        None
    }

    fn get_sol_decrease(&self) -> Option<(String, u64)> {
        for pre_balance in &self.transaction_info.pre_token_balances {
            if pre_balance.mint == SOL {
                let post_amount = self
                    .transaction_info
                    .post_token_balances
                    .iter()
                    .find(|b| b.mint == SOL && b.owner == pre_balance.owner)
                    .and_then(|b| b.ui_token_amount.amount.parse::<u64>().ok())
                    .unwrap_or(0);
                let pre_amount = pre_balance
                    .ui_token_amount
                    .amount
                    .parse::<u64>()
                    .unwrap_or(0);

                if pre_amount > post_amount {
                    return Some((SOL.to_string(), pre_amount - post_amount));
                }
            }
        }
        if self.transaction_info.balance_change < 0 {
            let spent = (-self.transaction_info.balance_change) as u64;
            let net_spent = spent.saturating_sub(self.transaction_info.fee);
            if net_spent > 0 {
                return Some((SOL.to_string(), net_spent));
            }
        }
        None
    }

    fn get_meme_token_increase(&self) -> Option<(String, u64)> {
        for post_balance in &self.transaction_info.post_token_balances {
            let mint = &post_balance.mint;
            if mint != SOL && mint != USDC && mint != USDT {
                let pre_amount = self
                    .transaction_info
                    .pre_token_balances
                    .iter()
                    .find(|b| b.mint == *mint && b.owner == post_balance.owner)
                    .and_then(|b| b.ui_token_amount.amount.parse::<u64>().ok())
                    .unwrap_or(0);
                let post_amount = post_balance
                    .ui_token_amount
                    .amount
                    .parse::<u64>()
                    .unwrap_or(0);
                if post_amount > pre_amount {
                    return Some((mint.clone(), post_amount - pre_amount));
                }
            }
        }
        None
    }

    fn get_spent_token_fallback(&self) -> Option<(String, u64)> {
        self.get_meme_token_decrease()
            .or_else(|| self.get_sol_decrease())
    }

    fn get_received_token_fallback(&self) -> Option<(String, u64)> {
        self.get_meme_token_increase()
            .or_else(|| self.get_sol_increase())
    }

    fn decode_pump_base64_data(&self, log: &str) -> Option<(String, u64)> {
        if let Some(base64_start) = log.find("Program data:") {
            let base64_str = &log[base64_start + 13..].trim();
            if base64_str.starts_with("vdt/") {
                let clean_base64 = base64_str.replace("vdt/", "");
                if let Ok(decoded) = general_purpose::STANDARD.decode(clean_base64) {
                    for offset in &[24, 32, 40, 48] {
                        if *offset + 8 <= decoded.len() {
                            let amount = u64::from_le_bytes([
                                decoded[*offset],
                                decoded[*offset + 1],
                                decoded[*offset + 2],
                                decoded[*offset + 3],
                                decoded[*offset + 4],
                                decoded[*offset + 5],
                                decoded[*offset + 6],
                                decoded[*offset + 7],
                            ]);
                            if amount >= 1_000 && amount <= 10_000_000_000_000 {
                                return Some((SOL.to_string(), amount));
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get_pump_spent_token(&self) -> Option<(String, u64)> {
        let meme_token_decrease = self.get_meme_token_decrease();
        let sol_increase = self.get_sol_increase();
        if meme_token_decrease.is_some() && sol_increase.is_some() {
            return meme_token_decrease;
        }
        let sol_decrease = self.get_sol_decrease();
        let meme_token_increase = self.get_meme_token_increase();
        if sol_decrease.is_some() && meme_token_increase.is_some() {
            return sol_decrease;
        }
        self.get_spent_token_fallback()
    }

    pub fn get_pump_direction(&self) -> Option<Direction> {
        if let Some((spent_token, _)) = self.get_pump_spent_token() {
            if spent_token == SOL || spent_token == USDC || spent_token == USDT {
                return Some(Direction::Buy);
            } else {
                return Some(Direction::Sell);
            }
        }
        None
    }

    pub fn get_pump_pool_left_amount_sol(&self) -> Option<f64> {
        self.get_pump_pool_left_amount().and_then(|amount| {
            let decimals = self.get_pump_left_token_decimals()?;
            Some(amount as f64 / 10_u64.pow(decimals as u32) as f64)
        })
    }

    pub fn get_pump_pool_right_amount_sol(&self) -> Option<f64> {
        self.get_pump_pool_right_amount().and_then(|lamports| {
            if let Some(address) = self.get_pump_pool_right_address() {
                if address == SOL {
                    return Some(lamports as f64 / LAMPORTS_PER_SOL as f64);
                }
            }
            let decimals = self.get_pump_right_token_decimals()?;
            Some(lamports as f64 / 10_u64.pow(decimals as u32) as f64)
        })
    }

    pub fn get_pump_received_token_sol(&self) -> Option<(String, f64)> {
        self.get_pump_received_token()
            .and_then(|(address, amount)| {
                let decimals = self.get_pump_token_decimals(&address)?;
                Some((address, amount as f64 / 10_u64.pow(decimals as u32) as f64))
            })
    }

    pub fn get_pump_spent_token_sol(&self) -> Option<(String, f64)> {
        self.get_pump_spent_token().and_then(|(address, amount)| {
            let decimals = self.get_pump_token_decimals(&address)?;
            Some((address, amount as f64 / 10_u64.pow(decimals as u32) as f64))
        })
    }

    pub fn get_pump_token_price_sol(&self) -> Option<f64> {
        let direction = self.get_pump_direction()?;
        match direction {
            Direction::Buy => {
                if let Some((spent_token, spent_amount_sol)) = self.get_pump_spent_token_sol() {
                    if spent_token == SOL || spent_token == USDC || spent_token == USDT {
                        if let Some((received_token, received_amount)) =
                            self.get_pump_received_token_sol()
                        {
                            if received_token != SOL
                                && received_token != USDC
                                && received_token != USDT
                            {
                                return Some(spent_amount_sol / received_amount);
                            }
                        }
                    }
                }
            }
            Direction::Sell => {
                if let Some((spent_token, spent_amount)) = self.get_pump_spent_token_sol() {
                    if spent_token != SOL && spent_token != USDC && spent_token != USDT {
                        if let Some((received_token, received_amount_sol)) =
                            self.get_pump_received_token_sol()
                        {
                            if received_token == SOL
                                || received_token == USDC
                                || received_token == USDT
                            {
                                return Some(received_amount_sol / spent_amount);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get_pump_total_value_sol(&self) -> Option<f64> {
        let direction = self.get_pump_direction()?;
        match direction {
            Direction::Buy => {
                if let Some((spent_token, spent_amount_sol)) = self.get_pump_spent_token_sol() {
                    if spent_token == SOL || spent_token == USDC || spent_token == USDT {
                        return Some(spent_amount_sol);
                    }
                }
            }
            Direction::Sell => {
                if let Some((received_token, received_amount_sol)) =
                    self.get_pump_received_token_sol()
                {
                    if received_token == SOL || received_token == USDC || received_token == USDT {
                        return Some(received_amount_sol);
                    }
                }
            }
        }
        None
    }

    pub fn get_pump_meme_token_amount_sol(&self) -> Option<(String, f64)> {
        let direction = self.get_pump_direction()?;
        match direction {
            Direction::Buy => {
                if let Some((received_token, received_amount)) = self.get_pump_received_token_sol()
                {
                    if received_token != SOL && received_token != USDC && received_token != USDT {
                        return Some((received_token, received_amount));
                    }
                }
            }
            Direction::Sell => {
                if let Some((spent_token, spent_amount)) = self.get_pump_spent_token_sol() {
                    if spent_token != SOL && spent_token != USDC && spent_token != USDT {
                        return Some((spent_token, spent_amount));
                    }
                }
            }
        }
        None
    }

    pub fn get_pump_sol_amount_sol(&self) -> Option<f64> {
        let direction = self.get_pump_direction()?;
        match direction {
            Direction::Buy => {
                if let Some((spent_token, spent_amount_sol)) = self.get_pump_spent_token_sol() {
                    if spent_token == SOL {
                        return Some(spent_amount_sol);
                    } else if spent_token == USDC || spent_token == USDT {
                        return Some(spent_amount_sol);
                    }
                }
            }
            Direction::Sell => {
                if let Some((received_token, received_amount_sol)) =
                    self.get_pump_received_token_sol()
                {
                    if received_token == SOL {
                        return Some(received_amount_sol);
                    }
                }
            }
        }
        None
    }

    pub fn get_pump_volume_usd(&self) -> Option<f64> {
        let total_value_sol = self.get_pump_total_value_sol()?;
        let sol_price_usd = 150.0;
        Some(total_value_sol * sol_price_usd)
    }

    fn get_pump_left_token_decimals(&self) -> Option<u8> {
        if let Some(address) = self.get_pump_pool_left_address() {
            return self.get_pump_token_decimals(&address);
        }
        None
    }

    fn get_pump_right_token_decimals(&self) -> Option<u8> {
        if let Some(address) = self.get_pump_pool_right_address() {
            return self.get_pump_token_decimals(&address);
        }
        None
    }

    fn get_pump_token_decimals(&self, mint: &str) -> Option<u8> {
        if mint == SOL {
            return Some(9);
        }
        if mint == USDC || mint == USDT {
            return Some(6);
        }
        for balance in self
            .transaction_info
            .pre_token_balances
            .iter()
            .chain(&self.transaction_info.post_token_balances)
        {
            if balance.mint == mint {
                return Some(balance.ui_token_amount.decimals);
            }
        }
        Some(6)
    }
}
