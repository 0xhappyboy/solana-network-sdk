pub mod trade {
    use std::collections::HashMap;

    use crate::trade::TokenBalance;

    pub fn build_signer_token_delta(
        pre: &[TokenBalance],
        post: &[TokenBalance],
        signer: &str,
    ) -> HashMap<String, i128> {
        let mut pre_map = HashMap::<String, i128>::new();
        let mut post_map = HashMap::<String, i128>::new();
        for b in pre {
            if b.owner == signer {
                let amt = b.ui_token_amount.amount.parse::<i128>().unwrap();
                *pre_map.entry(b.mint.clone()).or_insert(0) += amt;
            }
        }
        for b in post {
            if b.owner == signer {
                let amt = b.ui_token_amount.amount.parse::<i128>().unwrap();
                *post_map.entry(b.mint.clone()).or_insert(0) += amt;
            }
        }
        let mut delta = HashMap::new();
        for mint in pre_map.keys().chain(post_map.keys()) {
            let d = post_map.get(mint).unwrap_or(&0) - pre_map.get(mint).unwrap_or(&0);
            if d != 0 {
                delta.insert(mint.clone(), d);
            }
        }
        delta
    }
}

pub mod wallet {
    /// convert the private key in base58 format to a byte array
    /// # params
    /// * private_key private key
    pub fn private_key_base58_to_bytes(private_key: &str) -> Result<Vec<u8>, String> {
        match bs58::decode(private_key).into_vec() {
            Ok(v) => return Ok(v),
            Err(_) => return Err("base58 decode error".to_string()),
        }
    }
}

pub mod address {
    use std::str::FromStr;

    use spl_token::solana_program::pubkey::Pubkey;

    /// Verify that the Solana address format is valid
    ///
    /// # Params
    /// address - address streing
    ///
    /// # Example
    /// ```rust
    /// use solana_network_sdk::tool::wallet;
    ///
    /// let valid_address = "address";
    /// let invalid_address = "invalid_address";
    ///
    /// assert!(wallet::is_valid_address(valid_address));
    /// assert!(!wallet::is_valid_address(invalid_address));
    /// ```
    pub fn is_valid_address(address: &str) -> bool {
        Pubkey::from_str(address).is_ok()
    }

    /// Verify the address and return a Pubkey object
    ///
    /// # Params
    /// address - address
    ///
    /// # Returns
    /// Result<Pubkey, String> - Returns Pubkey if successful, returns error message if failed
    ///
    /// # Example
    /// ```rust
    /// use solana_network_sdk::tool::wallet;
    ///
    /// match wallet::validate_address_to_pubkey("address") {
    ///     Ok(pubkey) => println!("valid address: {}", pubkey),
    ///     Err(e) => println!("Invalid address: {}", e),
    /// }
    /// ```
    pub fn validate_address_to_pubkey(address: &str) -> Result<Pubkey, String> {
        Pubkey::from_str(address)
            .map_err(|e| format!("Invalid Solana Address '{}': {}", address, e))
    }

    /// Check if two addresses are the same (case-insensitive)
    /// # Arguments
    /// * address1 - First address
    /// * address2 - Second address
    /// # Returns
    /// * bool - Whether the two addresses are the same
    /// # Example
    /// ```
    /// use solana_network_sdk::tool::wallet;
    ///
    /// let addr1 = "address 1";
    /// let addr2 = "address 2"; // lowercase
    ///
    /// assert!(wallet::is_same_address(addr1, addr2));
    /// ```
    pub fn is_same_address(address1: &str, address2: &str) -> bool {
        if let (Ok(pubkey1), Ok(pubkey2)) = (Pubkey::from_str(address1), Pubkey::from_str(address2))
        {
            pubkey1 == pubkey2
        } else {
            false
        }
    }

    /// Generate a random valid Solana address (for testing purposes)
    /// # Returns
    /// * String - Randomly generated Solana address
    /// # Example
    /// ```
    /// use solana_network_sdk::tool::wallet;
    ///
    /// let random_address = wallet::generate_random_address();
    /// assert!(wallet::is_valid_address(&random_address));
    /// ```
    pub fn generate_random_address() -> String {
        let pubkey = Pubkey::new_unique();
        pubkey.to_string()
    }

    /// Convert a byte array to a Solana address
    /// # Arguments
    /// * bytes - 32-byte array
    /// # Returns
    /// * Result<String, String> - Success returns address string, failure returns error message
    /// # Example
    /// ```
    /// use solana_network_sdk::tool::wallet;
    ///
    /// let bytes = [1; 32]; // example byte array
    /// match wallet::bytes_to_address(&bytes) {
    ///     Ok(address) => println!("Generated address: {}", address),
    ///     Err(e) => println!("Error: {}", e),
    /// }
    /// ```
    pub fn bytes_to_address(bytes: &[u8]) -> Result<String, String> {
        if bytes.len() != 32 {
            return Err(format!(
                "Byte array length must be 32, current length: {}",
                bytes.len()
            ));
        }
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(bytes);
        let pubkey = Pubkey::new_from_array(key_bytes);
        Ok(pubkey.to_string())
    }

    /// Get the short display format of an address (first 4 chars...last 4 chars)
    /// # Arguments
    /// * address - Full address
    /// # Returns
    /// * String - Short format address
    /// # Example
    /// ```
    /// use solana_network_sdk::tool::wallet;
    ///
    /// let address = "address";
    /// let short = wallet::get_short_address(address);
    /// println!("Short address: {}", short); // Output: HN7c...WrH
    /// ```
    pub fn get_short_address(address: &str) -> String {
        if address.len() <= 8 {
            return address.to_string();
        }
        let prefix = &address[0..4];
        let suffix = &address[address.len() - 4..];
        format!("{}...{}", prefix, suffix)
    }
}

pub mod token {

    pub const SOL: u8 = 9;
    pub const USDC: u8 = 6;
    pub const USDT: u8 = 6;
    pub const ETH: u8 = 9; // Wrapped ETH
    pub const BTC: u8 = 8; // Wrapped BTC
    pub const RAY: u8 = 6;
    pub const SRM: u8 = 6;
    pub const FTT: u8 = 6;
    pub const MSOL: u8 = 9;
    pub const JITOSOL: u8 = 9;

    /// SOL and Lamports conversion tools
    pub fn sol_to_lamports(sol_amount: f64) -> u64 {
        (sol_amount * 1_000_000_000.0).round() as u64
    }

    pub fn lamports_to_sol(lamports: u64) -> f64 {
        lamports as f64 / 1_000_000_000.0
    }

    /// SPL token precision conversion
    pub fn ui_amount_to_raw(ui_amount: f64, decimals: u8) -> u64 {
        (ui_amount * 10_f64.powi(decimals as i32)).round() as u64
    }

    pub fn raw_amount_to_ui(raw_amount: u64, decimals: u8) -> f64 {
        raw_amount as f64 / 10_f64.powi(decimals as i32)
    }

    /// Safely convert SOL to Lamports (overflow protection)
    pub fn safe_sol_to_lamports(sol_amount: f64) -> Option<u64> {
        let lamports = sol_amount * 1_000_000_000.0;
        if lamports > u64::MAX as f64 || lamports < 0.0 || lamports.is_nan() {
            None
        } else {
            Some(lamports.round() as u64)
        }
    }

    /// SOL → Lamports safe conversion (with detailed error)
    pub fn safe_sol_to_lamports_result(sol_amount: f64) -> Result<u64, String> {
        if sol_amount < 0.0 {
            return Err("SOL amount cannot be negative".to_string());
        }

        if sol_amount.is_nan() || sol_amount.is_infinite() {
            return Err("SOL amount is not a valid number".to_string());
        }

        let lamports = sol_amount * 1_000_000_000.0;
        if lamports > u64::MAX as f64 {
            return Err(format!(
                "SOL amount too large: {} SOL (max: {} SOL)",
                sol_amount,
                u64::MAX as f64 / 1_000_000_000.0
            ));
        }

        Ok(lamports.round() as u64)
    }

    /// Safely convert Lamports to SOL (underflow protection)
    pub fn safe_lamports_to_sol(lamports: u64) -> f64 {
        lamports as f64 / 1_000_000_000.0
        // This conversion is always safe because division cannot overflow
    }

    /// Lamports → SOL safe conversion (with error checking)
    pub fn safe_lamports_to_sol_result(lamports: u64) -> Result<f64, String> {
        // Lamports → SOL conversion is always safe, but business logic checks can be added
        if lamports == 0 {
            return Err("Lamports amount cannot be zero".to_string());
        }

        Ok(lamports as f64 / 1_000_000_000.0)
    }

    /// Safely convert UI amount to raw amount (overflow protection)
    pub fn safe_ui_to_raw(ui_amount: f64, decimals: u8) -> Option<u64> {
        if decimals > 9 {
            return None; // Solana maximum supported decimals is 9
        }

        let multiplier = 10_f64.powi(decimals as i32);
        let raw = ui_amount * multiplier;

        if raw > u64::MAX as f64 || raw < 0.0 || raw.is_nan() {
            None
        } else {
            Some(raw.round() as u64)
        }
    }

    /// UI Amount → Raw Amount safe conversion (with detailed error)
    pub fn safe_ui_to_raw_result(ui_amount: f64, decimals: u8) -> Result<u64, String> {
        if decimals > 9 {
            return Err(format!("Decimals cannot exceed 9, got: {}", decimals));
        }

        if ui_amount < 0.0 {
            return Err("Token amount cannot be negative".to_string());
        }

        if ui_amount.is_nan() || ui_amount.is_infinite() {
            return Err("Token amount is not a valid number".to_string());
        }

        let multiplier = 10_f64.powi(decimals as i32);
        let raw = ui_amount * multiplier;

        if raw > u64::MAX as f64 {
            return Err(format!(
                "Token amount too large: {} with {} decimals (max: {})",
                ui_amount,
                decimals,
                u64::MAX as f64 / multiplier
            ));
        }

        let rounded = raw.round();
        if (raw - rounded).abs() > 1e-12 {
            return Err(format!(
                "Token amount has too many decimal places: {} (max decimals: {})",
                ui_amount, decimals
            ));
        }

        Ok(rounded as u64)
    }

    /// Safely convert raw amount to UI amount (always safe)
    pub fn safe_raw_to_ui(raw_amount: u64, decimals: u8) -> Option<f64> {
        if decimals > 9 {
            return None;
        }

        let divisor = 10_f64.powi(decimals as i32);
        Some(raw_amount as f64 / divisor)
    }

    /// Raw Amount → UI Amount safe conversion (with error checking)
    pub fn safe_raw_to_ui_result(raw_amount: u64, decimals: u8) -> Result<f64, String> {
        if decimals > 9 {
            return Err(format!("Decimals cannot exceed 9, got: {}", decimals));
        }

        if raw_amount == 0 {
            return Err("Raw amount cannot be zero".to_string());
        }

        let divisor = 10_f64.powi(decimals as i32);
        Ok(raw_amount as f64 / divisor)
    }

    /// Batch safe conversion SOL → Lamports
    pub fn safe_batch_sol_to_lamports(sol_amounts: &[f64]) -> Vec<Option<u64>> {
        sol_amounts
            .iter()
            .map(|&amount| safe_sol_to_lamports(amount))
            .collect()
    }

    /// Batch safe conversion Lamports → SOL
    pub fn safe_batch_lamports_to_sol(lamports_amounts: &[u64]) -> Vec<f64> {
        lamports_amounts
            .iter()
            .map(|&amount| safe_lamports_to_sol(amount))
            .collect()
    }

    /// Batch safe conversion UI → Raw
    pub fn safe_batch_ui_to_raw(ui_amounts: &[f64], decimals: u8) -> Vec<Option<u64>> {
        ui_amounts
            .iter()
            .map(|&amount| safe_ui_to_raw(amount, decimals))
            .collect()
    }

    /// Batch safe conversion Raw → UI
    pub fn safe_batch_raw_to_ui(raw_amounts: &[u64], decimals: u8) -> Vec<Option<f64>> {
        raw_amounts
            .iter()
            .map(|&amount| safe_raw_to_ui(amount, decimals))
            .collect()
    }

    /// Batch conversion tools
    pub fn batch_sol_to_lamports(sol_amounts: &[f64]) -> Vec<u64> {
        sol_amounts
            .iter()
            .map(|&amount| sol_to_lamports(amount))
            .collect()
    }

    pub fn batch_ui_to_raw(ui_amounts: &[f64], decimals: u8) -> Vec<u64> {
        ui_amounts
            .iter()
            .map(|&amount| ui_amount_to_raw(amount, decimals))
            .collect()
    }

    /// Format display functions
    pub fn format_sol(lamports: u64) -> String {
        format!("{:.6} SOL", lamports_to_sol(lamports))
    }

    pub fn format_token(raw_amount: u64, decimals: u8, symbol: &str) -> String {
        format!("{:.6} {}", raw_amount_to_ui(raw_amount, decimals), symbol)
    }

    /// Smart formatting (adjust decimal places based on amount size)
    pub fn format_sol_smart(lamports: u64) -> String {
        let sol = lamports_to_sol(lamports);
        if sol >= 1.0 {
            format!("{:.4} SOL", sol)
        } else if sol >= 0.001 {
            format!("{:.6} SOL", sol)
        } else {
            format!("{} Lamports", lamports)
        }
    }

    /// Precision validation tools
    pub fn is_valid_decimals(decimals: u8) -> bool {
        decimals <= 9 // Solana maximum supported precision
    }

    pub fn validate_amount(amount: u64, decimals: u8) -> bool {
        // Check if amount is valid for given precision
        let max_amount = 10_u64.pow(15); // Reasonable upper limit
        amount <= max_amount
    }

    /// Math operations tools (avoid floating point precision issues)
    pub fn add_sol_amounts(lamports1: u64, lamports2: u64) -> Option<u64> {
        lamports1.checked_add(lamports2)
    }

    pub fn subtract_sol_amounts(lamports1: u64, lamports2: u64) -> Option<u64> {
        lamports1.checked_sub(lamports2)
    }

    /// Calculate percentage
    pub fn calculate_percentage(amount: u64, percentage: f64) -> Option<u64> {
        let result = (amount as f64 * percentage / 100.0).round() as u64;
        if result > amount { None } else { Some(result) }
    }
}
