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

pub mod account {}
