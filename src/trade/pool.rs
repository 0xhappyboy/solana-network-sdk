use crate::trade::info::TransactionInfo;
use crate::global::{SOL, USD_1, USDC, USDT, WSOL};
use crate::types::Direction;

impl TransactionInfo {
    /// Get the final settlement quote token address (considering aggregator swaps)
    /// For aggregator trades, this returns the token that the user actually receives/spends
    pub fn get_final_settlement_quote_token(&self) -> String {
        if let Some((received_token, _)) = self.get_final_received_token() {
            if self.is_quote_token(&received_token) {
                return received_token;
            }
        }
        if let Some((spent_token, _)) = self.get_final_spent_token() {
            if self.is_quote_token(&spent_token) {
                return spent_token;
            }
        }
        if self.has_sol_or_wsol_activity() {
            if self.has_token(WSOL) {
                return WSOL.to_string(); // WSOL
            }
            return SOL.to_string(); // SOL
        }
        if self.has_token(USD_1) {
            return USD_1.to_string();
        }
        if self.has_token(USDC) {
            return USDC.to_string();
        }
        if self.has_token(USDT) {
            return USDT.to_string();
        }
        SOL.to_string()
    }
    
    /// Get the token that the signer finally received (considering aggregator trades)
    fn get_final_received_token(&self) -> Option<(String, u64)> {
        let signer_address = if !self.signer.is_empty() {
            &self.signer
        } else if !self.fee_payer.is_empty() {
            &self.fee_payer
        } else {
            return None;
        };
        let mut max_received_token: Option<String> = None;
        let mut max_received_amount = 0u64;
        for post_balance in &self.raw_post_token_balances {
            if post_balance.owner.as_ref() != Some(signer_address) {
                continue;
            }
            let pre_amount = self.raw_pre_token_balances
                .iter()
                .find(|pre| pre.mint == post_balance.mint && pre.owner.as_ref() == Some(signer_address))
                .and_then(|pre| pre.ui_token_amount.amount.parse::<u64>().ok())
                .unwrap_or(0);
            let post_amount = post_balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);
            if post_amount > pre_amount {
                let increase = post_amount - pre_amount;
                if increase > max_received_amount {
                    max_received_amount = increase;
                    max_received_token = Some(post_balance.mint.clone());
                }
            }
        }
        max_received_token.map(|token| (token, max_received_amount))
    }
    
    /// Get the token that the signer finally spent (considering aggregator trades)
    fn get_final_spent_token(&self) -> Option<(String, u64)> {
        let signer_address = if !self.signer.is_empty() {
            &self.signer
        } else if !self.fee_payer.is_empty() {
            &self.fee_payer
        } else {
            return None;
        };
        let mut max_spent_token: Option<String> = None;
        let mut max_spent_amount = 0u64;
        for pre_balance in &self.raw_pre_token_balances {
            if pre_balance.owner.as_ref() != Some(signer_address) {
                continue;
            }
            let post_amount = self.raw_post_token_balances
                .iter()
                .find(|post| post.mint == pre_balance.mint && post.owner.as_ref() == Some(signer_address))
                .and_then(|post| post.ui_token_amount.amount.parse::<u64>().ok())
                .unwrap_or(0);
            let pre_amount = pre_balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);
            if pre_amount > post_amount {
                let decrease = pre_amount - post_amount;
                if decrease > max_spent_amount {
                    max_spent_amount = decrease;
                    max_spent_token = Some(pre_balance.mint.clone());
                }
            }
        }
        max_spent_token.map(|token| (token, max_spent_amount))
    }
    
    /// Check if there's SOL or WSOL activity in the transaction
    fn has_sol_or_wsol_activity(&self) -> bool {
        if self.has_sol_activity() {
            return true;
        }
        self.has_token(WSOL)
    }
    
    /// Check if there's SOL activity
    fn has_sol_activity(&self) -> bool {
        if self.raw_pre_balances.is_empty() || self.raw_post_balances.is_empty() {
            return false;
        }
        for i in 0..self.raw_pre_balances.len().min(self.raw_post_balances.len()) {
            if self.raw_pre_balances[i] != self.raw_post_balances[i] {
                return true;
            }
        }
        false
    }
    
    /// Check if a specific token exists in the transaction
    fn has_token(&self, mint: &str) -> bool {
        self.raw_pre_token_balances.iter()
            .chain(&self.raw_post_token_balances)
            .any(|b| b.mint == mint)
    }
    
    /// Check if a token is a quote token (including WSOL)
    fn is_quote_token(&self, mint: &str) -> bool {
        mint == SOL || 
        mint == WSOL || // WSOL
        mint == USDC || 
        mint == USDT || 
        mint == USD_1
    }
    
    /// Get all tokens involved in the transaction
    fn get_all_involved_tokens(&self) -> Vec<String> {
        let mut tokens = Vec::new();
        for balance in self.raw_pre_token_balances.iter().chain(&self.raw_post_token_balances) {
            if !tokens.contains(&balance.mint) {
                tokens.push(balance.mint.clone());
            }
        }
        tokens
    }
    
    /// Get the real base token address from the transaction
    /// Base token is the non-quote token (not SOL, WSOL, USDC, USDT, USD1)
    pub fn get_pool_base_token_address(&self) -> Option<String> {
        let final_quote_token = self.get_final_settlement_quote_token();
        let mut max_base_token: Option<String> = None;
        let mut max_abs_change = 0.0f64;
        for token in self.get_all_involved_tokens() {
            if token == final_quote_token || self.is_quote_token(&token) {
                continue; // Skip quote tokens
            }
            if let Some(change) = self.get_signer_token_balance_change_decimal(&token) {
                if change.abs() > max_abs_change.abs() {
                    max_abs_change = change;
                    max_base_token = Some(token);
                }
            }
        }
        max_base_token
    }
    
    /// Get the real quote token address from the transaction
    /// Quote token is one of: SOL, WSOL, USDC, USDT, USD1
    /// For aggregator trades, this returns the final settlement token
    pub fn get_pool_quote_token_address(&self) -> String {
        self.get_final_settlement_quote_token()
    }
    
    /// Calculate signer's base token balance change (in token units with decimals)
    /// Positive means received base tokens, negative means spent base tokens
    pub fn get_signer_base_token_change_decimal(&self) -> Option<f64> {
        if let Some(base_token) = self.get_pool_base_token_address() {
            self.get_signer_token_balance_change_decimal(&base_token)
        } else {
            None
        }
    }
    
    /// Calculate signer's quote token balance change (in token units with decimals)
    /// Positive means received quote tokens, negative means spent quote tokens
    pub fn get_signer_quote_token_change_decimal(&self) -> Option<f64> {
        let quote_token = self.get_pool_quote_token_address();
        match quote_token.as_str() {
            SOL | WSOL => {
                Some(self.get_signer_net_sol_income_sol())
            }
            _ => {
                self.get_signer_token_balance_change_decimal(&quote_token)
            }
        }
    }
    
    /// Calculate signer's base token balance change (in lamports/raw units)
    pub fn get_signer_base_token_change_lamports(&self) -> i64 {
        if let Some(base_token) = self.get_pool_base_token_address() {
            self.get_signer_token_balance_change_lamports(&base_token)
        } else {
            0
        }
    }
    
    /// Calculate signer's quote token balance change (in lamports/raw units)
    pub fn get_signer_quote_token_change_lamports(&self) -> i64 {
        let quote_token = self.get_pool_quote_token_address();
        match quote_token.as_str() {
            SOL => self.get_signer_net_sol_income_lamports(),
            _ => self.get_signer_token_balance_change_lamports(&quote_token),
        }
    }
    
    /// Get signer's token balance change for a specific mint address (in lamports/raw units)
    fn get_signer_token_balance_change_lamports(&self, mint: &str) -> i64 {
        let signer_address = if !self.signer.is_empty() {
            &self.signer
        } else if !self.fee_payer.is_empty() {
            &self.fee_payer
        } else {
            return 0;
        };
        let pre_amount = self.raw_pre_token_balances
            .iter()
            .find(|b| b.mint == mint && b.owner.as_ref() == Some(signer_address))
            .and_then(|b| b.ui_token_amount.amount.parse::<u64>().ok())
            .unwrap_or(0);
        let post_amount = self.raw_post_token_balances
            .iter()
            .find(|b| b.mint == mint && b.owner.as_ref() == Some(signer_address))
            .and_then(|b| b.ui_token_amount.amount.parse::<u64>().ok())
            .unwrap_or(0);
        post_amount as i64 - pre_amount as i64
    }
    
    /// Get signer's token balance change for a specific mint address (in token units with decimals)
   fn get_signer_token_balance_change_decimal(&self, mint: &str) -> Option<f64> {
        let signer_address = if !self.signer.is_empty() {
            &self.signer
        } else if !self.fee_payer.is_empty() {
            &self.fee_payer
        } else {
            return None;
        };
        let decimals = self.get_token_decimals(mint)?;
        let pre_amount = self.raw_pre_token_balances
            .iter()
            .find(|b| b.mint == mint && b.owner.as_ref() == Some(signer_address))
            .and_then(|b| b.ui_token_amount.amount.parse::<u64>().ok())
            .unwrap_or(0);
        let post_amount = self.raw_post_token_balances
            .iter()
            .find(|b| b.mint == mint && b.owner.as_ref() == Some(signer_address))
            .and_then(|b| b.ui_token_amount.amount.parse::<u64>().ok())
            .unwrap_or(0);
        let change = post_amount as f64 - pre_amount as f64;
        Some(change / 10_u64.pow(decimals as u32) as f64)
    }

    /// Get token decimals for a specific mint
    fn get_token_decimals(&self, mint: &str) -> Option<u8> {
        // First look in token balances
        for balance in self.raw_pre_token_balances.iter().chain(&self.raw_post_token_balances) {
            if balance.mint == mint {
                return Some(balance.ui_token_amount.decimals);
            }
        }
        match mint {
            SOL => Some(9), // SOL has 9 decimals
            WSOL => Some(9), // WSOL also has 9 decimals
            USDC | USDT | USD_1 => Some(6), // Stablecoins usually have 6 decimals
            _ => None,
        }
    }
    
    /// Determine if signer is buying or selling base token
    pub fn get_direction(&self) -> Direction {
        if let (Some(base_change), Some(quote_change)) = (
            self.get_signer_base_token_change_decimal(),
            self.get_signer_quote_token_change_decimal()
        ) {
            match (base_change, quote_change) {
                (base, quote) if base > 0.0 && quote < 0.0 => Direction::Buy,  // Received base, spent quote
                (base, quote) if base < 0.0 && quote > 0.0 => Direction::Sell, // Spent base, received quote
                _ => Direction::Unknown,
            }
        } else {
            Direction::Unknown
        }
    }
    
    /// Get aggregator swap path information
    pub fn get_aggregator_path_info(&self) -> Vec<SwapStep> {
        let mut steps = Vec::new();
        for log in &self.raw_log_messages {
            if log.contains("Swap") && log.contains("for") && log.contains("on") {
                // Parse swap log, e.g.: "Swap 50,132.799581 fih for 175.275832 USD1 on Raydium CPMM"
                if let Some(step) = self.parse_swap_log(log) {
                    steps.push(step);
                }
            }
        }
        steps
    }
    
    /// Parse swap log
    fn parse_swap_log(&self, log: &str) -> Option<SwapStep> {
        let before_on = log.split("on").next()?.trim();
        let parts: Vec<&str> = before_on.split("for").collect();
        if parts.len() != 2 {
            return None;
        }
        let input_part = parts[0].trim();
        let output_part = parts[1].trim();
        let input_parts: Vec<&str> = input_part.split_whitespace().collect();
        if input_parts.len() < 2 {
            return None;
        }
        let input_amount = input_parts[0].replace(',', "").parse::<f64>().ok()?;
        let input_token = input_parts[1].to_string();
        let output_parts: Vec<&str> = output_part.split_whitespace().collect();
        if output_parts.is_empty() {
            return None;
        }
        let output_amount = output_parts[0].replace(',', "").parse::<f64>().ok()?;
        let output_token = if output_parts.len() > 1 {
            output_parts[1].to_string()
        } else {
            "unknown".to_string()
        };
        Some(SwapStep {
            input_token,
            input_amount,
            output_token,
            output_amount,
        })
    }
    
    /// Get detailed token information
    pub fn get_token_info(&self) -> TokenInfo {
        TokenInfo {
            base_token: self.get_pool_base_token_address(),
            quote_token: self.get_pool_quote_token_address(),
            base_change_lamports: self.get_signer_base_token_change_lamports(),
            quote_change_lamports: self.get_signer_quote_token_change_lamports(),
            base_change_decimal: self.get_signer_base_token_change_decimal(),
            quote_change_decimal: self.get_signer_quote_token_change_decimal(),
            direction: self.get_direction(),
            price: self.get_token_quote_ratio(),
            aggregator_path: self.get_aggregator_path_info(),
        }
    }

    /// Get liquidity pool address from the transaction
    /// The pool address is typically the owner of the token accounts involved in the swap
    pub fn get_pool_address(&self) -> Option<String> {
        let signer_address = if !self.signer.is_empty() { 
            &self.signer 
        } else if !self.fee_payer.is_empty() { 
            &self.fee_payer 
        } else { 
            return None; 
        };
        let mut owners = std::collections::HashSet::new();
        for balance in self.raw_pre_token_balances.iter().chain(&self.raw_post_token_balances) {
            if let Some(owner) = &balance.owner {
                if owner != signer_address {
                    owners.insert(owner.clone());
                }
            }
        }
        if owners.len() == 1 {
            return owners.into_iter().next();
        }
        for owner in owners {
            let has_pre_balance = self.raw_pre_token_balances.iter().any(|b| b.owner.as_ref() == Some(&owner));
            let has_post_balance = self.raw_post_token_balances.iter().any(|b| b.owner.as_ref() == Some(&owner));
            if has_pre_balance || has_post_balance {
                    return Some(owner);
            }
        }
        None
    }

    /// Calculate the token quote ratio (price): quote token amount per base token unit
    /// Returns the price in quote tokens per 1 base token
    /// Formula: price = abs(quote_change) / abs(base_change)
    pub fn get_token_quote_ratio(&self) -> Option<f64> {
        let base_change = self.get_signer_base_token_change_decimal()?;
        let quote_change = self.get_signer_quote_token_change_decimal()?;
        let base_abs = base_change.abs();
        let quote_abs = quote_change.abs();
        if base_abs <= 0.0 || quote_abs <= 0.0 {
            return None;
        }
        let price = quote_abs / base_abs;
        Some(price)
    }
    
    /// Get formatted price as string (for display purposes)
    pub fn get_token_quote_ratio_string(&self) -> Option<String> {
        self.get_token_quote_ratio()
            .map(|price| format!("{:.12}", price).trim_end_matches('0').trim_end_matches('.').to_string())
    }

    /// Get the token that the signer actually received (with amount in lamports)
    /// Returns a tuple of (token_address, amount_in_lamports)
    pub fn get_received_token_sol(&self) -> Option<(String, u64)> {
        self.get_final_received_token()
    }

    /// Get the token that the signer actually spent (with amount in lamports)
    /// Returns a tuple of (token_address, amount_in_lamports)
    pub fn get_spent_token_sol(&self) -> Option<(String, u64)> {
        self.get_final_spent_token()
    }

    /// Get the address of the token that the signer actually received
    /// Returns just the token address without amount
    pub fn get_received_token_address(&self) -> Option<String> {
        self.get_received_token_sol()
            .map(|(token_address, _)| token_address)
    }

    /// Get the address of the token that the signer actually spent
    /// Returns just the token address without amount
    pub fn get_spent_token_address(&self) -> Option<String> {
        self.get_spent_token_sol()
            .map(|(token_address, _)| token_address)
    }
}

/// Swap step information
#[derive(Debug, Clone)]
pub struct SwapStep {
    pub input_token: String,
    pub input_amount: f64,
    pub output_token: String,
    pub output_amount: f64,
}

/// Token information struct
#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub base_token: Option<String>,
    pub quote_token: String,
    pub base_change_lamports: i64,
    pub quote_change_lamports: i64,
    pub base_change_decimal: Option<f64>,
    pub quote_change_decimal: Option<f64>,
    pub direction: Direction,
    pub price: Option<f64>,
    pub aggregator_path: Vec<SwapStep>,
}