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
