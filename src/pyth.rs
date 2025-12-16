use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::{str::FromStr, sync::Arc};

// Pyth price feed addresses
const PYTH_SOL_USD: &str = "H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG";
const PYTH_ETH_USD: &str = "JBu1AL4obBcCMqKBBxhpWCNUt136ijcuMZLFvTP7iWdB";
const PYTH_BTC_USD: &str = "GVXRSBjFk6e6J3NbVPXohDJetcTjaeeuykUpbQF8UoMU";
const PYTH_USDC_USD: &str = "8GWTTbNiXdmyZREXbjsZBmCRuzdPrW55dnZGDkTRjWvb";
const PYTH_AVAX_USD: &str = "FVb5h1VmHPfVb1RfqZckchq18GxRv4iKt8T4eVTQAqdz";
const PYTH_BNB_USD: &str = "4CkQJBxhU8EZ2UjhigbtdaPbpTe6mqf811fipYBFbSYN";

/// Token enum for easier usage
#[derive(Debug, Clone, Copy)]
pub enum Token {
    Sol,
    Eth,
    Btc,
    Usdc,
    Avax,
    Bnb,
}

impl Token {
    fn feed_address(&self) -> &'static str {
        match self {
            Token::Sol => PYTH_SOL_USD,
            Token::Eth => PYTH_ETH_USD,
            Token::Btc => PYTH_BTC_USD,
            Token::Usdc => PYTH_USDC_USD,
            Token::Avax => PYTH_AVAX_USD,
            Token::Bnb => PYTH_BNB_USD,
        }
    }

    fn price_range(&self) -> (f64, f64) {
        match self {
            Token::Sol => (10.0, 500.0),       // SOL price range
            Token::Eth => (1000.0, 5000.0),    // ETH price range
            Token::Btc => (20000.0, 100000.0), // BTC price range
            Token::Usdc => (0.99, 1.01),       // USDC price range (stablecoin)
            Token::Avax => (10.0, 200.0),      // AVAX price range
            Token::Bnb => (200.0, 1000.0),     // BNB price range
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Token::Sol => "SOL",
            Token::Eth => "ETH",
            Token::Btc => "BTC",
            Token::Usdc => "USDC",
            Token::Avax => "AVAX",
            Token::Bnb => "BNB",
        }
    }
}

pub struct Pyth {
    client: Arc<RpcClient>,
}

impl Pyth {
    pub fn new(client: Arc<RpcClient>) -> Self {
        Self { client }
    }

    /// Fetch token price directly from chain
    pub async fn get_token_price(&self, token: Token) -> Result<f64, String> {
        let feed_address = token.feed_address();
        let (min_price, max_price) = token.price_range();
        // 1. Get account
        let pubkey = Pubkey::from_str(feed_address).map_err(|e| format!("Invalid address: {}", e))?;
        let account = self
            .client
            .get_account(&pubkey)
            .await
            .map_err(|e| format!("Failed to get account: {}", e))?;
        let data = &account.data;
        // 2. Search for reasonable price
        for offset in 0..data.len().saturating_sub(8) {
            // Read 8 bytes as i64
            let bytes = match data.get(offset..offset + 8) {
                Some(b) => b.try_into().unwrap(),
                None => continue,
            };
            let raw_value = i64::from_le_bytes(bytes);
            // Try common exponents
            for expo in [-6, -7, -8, -9] {
                let price = raw_value as f64 * 10_f64.powi(expo);

                // Check if price is within reasonable range for the token
                if price >= min_price && price <= max_price {
                    // Additional validation: confidence interval should be reasonable
                    if self.verify_confidence(data, offset, expo, price).await {
                        return Ok(price);
                    }
                }
            }
        }
        Err(format!("No reasonable {} price found", token.name()))
    }

    /// Get SOL price directly from chain (backward compatibility)
    pub async fn get_sol_price(&self) -> Result<f64, String> {
        self.get_token_price(Token::Sol).await
    }

    /// Get ETH price
    pub async fn get_eth_price(&self) -> Result<f64, String> {
        self.get_token_price(Token::Eth).await
    }

    /// Get BTC price
    pub async fn get_btc_price(&self) -> Result<f64, String> {
        self.get_token_price(Token::Btc).await
    }

    /// Get USDC price
    pub async fn get_usdc_price(&self) -> Result<f64, String> {
        self.get_token_price(Token::Usdc).await
    }

    /// Get AVAX price
    pub async fn get_avax_price(&self) -> Result<f64, String> {
        self.get_token_price(Token::Avax).await
    }

    /// Get BNB price
    pub async fn get_bnb_price(&self) -> Result<f64, String> {
        self.get_token_price(Token::Bnb).await
    }

    /// Get multiple token prices in batch
    pub async fn get_multi_prices(&self, tokens: &[Token]) -> Result<Vec<(String, f64)>, String> {
        let mut results = Vec::new();
        for token in tokens {
            match self.get_token_price(token.clone()).await {
                Ok(price) => results.push((token.name().to_string(), price)),
                Err(e) => return Err(e),
            }
        }
        Ok(results)
    }

    /// Verify confidence interval
    async fn verify_confidence(
        &self,
        data: &[u8],
        price_offset: usize,
        expo: i32,
        price: f64,
    ) -> bool {
        // Confidence interval is typically 8 bytes after the price
        if price_offset + 16 > data.len() {
            return false;
        }
        let conf_bytes: [u8; 8] = data[price_offset + 8..price_offset + 16]
            .try_into()
            .unwrap_or([0; 8]);
        let raw_conf = u64::from_le_bytes(conf_bytes);
        let confidence = raw_conf as f64 * 10_f64.powi(expo);
        // Confidence should be positive and less than 5% of price
        confidence > 0.0 && confidence < price * 0.05
    }
}
