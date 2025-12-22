use bs58;
use solana_sdk::{signature::Signer, signer::keypair::Keypair};

use crate::tool::wallet::private_key_base58_to_bytes;

#[derive(Debug)]
pub struct Wallet {
    pub public_key: String,
    pub private_key_32: String,
    pub private_key_64: String,
    pub keypair: Option<Keypair>,
}

impl Wallet {
    /// create new wallet
    /// # Returns
    /// wallet object
    /// # Example
    /// ```rust
    /// let w: Wallet = Wallet::create_new_wallet();
    /// ```
    pub fn create_new_wallet() -> Wallet {
        let k = Keypair::new();
        let public_key = k.pubkey();
        let secret_key_32_bytes = k.secret_bytes();
        let secret_key_64_bytes = k.to_bytes();
        let secret_key_32 = bs58::encode(&secret_key_32_bytes).into_string();
        let secret_key_64 = bs58::encode(&secret_key_64_bytes).into_string();
        Wallet {
            public_key: public_key.to_string(),
            private_key_32: secret_key_32,
            private_key_64: secret_key_64,
            keypair: Some(k),
        }
    }
    /// restore a wallet from a 64 bytes private key.
    /// # private_key
    /// * 64 bytes private key
    /// # Example
    /// ```rust
    /// let w = Wallet::from_private_key_64("64 bytes private");
    /// ```
    pub fn from_private_key_64(private_key: &str) -> Wallet {
        let k = Keypair::from_base58_string(private_key);
        let private_key_32 = bs58::encode(k.secret_bytes()).into_string();
        Wallet {
            public_key: k.pubkey().to_string(),
            private_key_32: private_key_32,
            private_key_64: private_key.to_string(),
            keypair: Some(k),
        }
    }
    /// restore a wallet from a 32 bytes private key.
    /// # private_key
    /// * 64 bytes private key
    /// # Example
    /// ```rust
    /// let w = Wallet::from_private_key_64("32 bytes private");
    /// ```
    pub fn from_private_key_32(private_key: &str) -> Result<Wallet, String> {
        match private_key_base58_to_bytes(private_key) {
            Ok(v) => {
                if v.len() == 32 {
                    let mut buf = [0u8; 32];
                    buf.copy_from_slice(&v[..]);
                    let k = Keypair::new_from_array(buf);
                    let w = Wallet {
                        public_key: k.pubkey().to_string(),
                        private_key_32: private_key.to_string(),
                        private_key_64: bs58::encode(k.to_bytes()).into_string(),
                        keypair: Some(k),
                    };
                    return Ok(w);
                } else {
                    return Err("exceeds 32 bytes.".to_string());
                }
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}
