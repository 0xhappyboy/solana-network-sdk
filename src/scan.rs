use solana_client::{
    nonblocking::rpc_client::RpcClient, rpc_client::GetConfirmedSignaturesForAddress2Config,
};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

/// Scanner for retrieving transaction signatures from Solana blockchain
/// Provides methods to fetch historical and recent transaction signatures for given addresses
pub struct Scan {
    /// Solana RPC client for making network requests
    client: Arc<RpcClient>,
}

impl Scan {
    /// Creates a new Scan instance with the provided RPC client
    ///
    /// # Params
    /// * `client` - Arc-wrapped RpcClient for making Solana RPC calls
    ///
    /// # Returns
    /// New Scan instance
    pub fn new(client: Arc<RpcClient>) -> Self {
        Self { client }
    }

    /// Fetches all historical transaction signatures for a given address
    /// Continues pagination until no more signatures are available
    ///
    /// # Params
    /// * `address` - Solana address (base58 encoded) to fetch signatures for
    /// * `interval_time` - Optional delay between requests in milliseconds (default: 200ms)
    /// * `batch_size` - Optional number of signatures to fetch per batch (default: 1000)
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - Vector of transaction signatures
    /// * `Err(String)` - Error message if address parsing or RPC call fails
    ///
    pub async fn get_all_signatures_by_address(
        &self,
        address: &str,
        interval_time: Option<u64>,
        batch_size: Option<u64>,
    ) -> Result<Vec<String>, String> {
        let pubkey = Pubkey::from_str(address).map_err(|e| format!("address error:{:?}", e))?;
        let mut all_signatures = Vec::new();
        let mut before: Option<Signature> = None;
        let mut iteration = 0;
        let sleep_duration = interval_time.unwrap_or(200);
        let batch_limit = batch_size.unwrap_or(1000);
        loop {
            iteration += 1;
            let config = GetConfirmedSignaturesForAddress2Config {
                before,
                until: None,
                limit: Some(batch_limit.try_into().unwrap()),
                commitment: None,
            };
            let signatures = match self
                .client
                .get_signatures_for_address_with_config(&pubkey, config)
                .await
            {
                Ok(sigs) => sigs,
                Err(e) => {
                    if e.to_string().contains("rate limit") || e.to_string().contains("429") {
                        tokio::time::sleep(Duration::from_millis(1000)).await;
                        continue;
                    }
                    break;
                }
            };
            if signatures.is_empty() {
                break;
            }
            let signature_strings: Vec<String> =
                signatures.iter().map(|sig| sig.signature.clone()).collect();
            let mut new_count = 0;
            for sig in &signature_strings {
                if !all_signatures.contains(sig) {
                    all_signatures.push(sig.clone());
                    new_count += 1;
                }
            }
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
            tokio::time::sleep(Duration::from_millis(sleep_duration)).await;
        }
        Ok(all_signatures)
    }

    /// Fetches a limited number of transaction signatures for a given address
    /// Stops when the specified limit is reached or no more signatures are available
    ///
    /// # Params
    /// * `address` - Solana address (base58 encoded) to fetch signatures for
    /// * `limit` - Maximum number of signatures to return
    /// * `interval_time` - Optional delay between requests in milliseconds (default: 200ms)
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - Vector of transaction signatures (up to the specified limit)
    /// * `Err(String)` - Error message if address parsing fails
    pub async fn get_signatures_with_limit(
        &self,
        address: &str,
        limit: usize,
        interval_time: Option<u64>,
    ) -> Result<Vec<String>, String> {
        let pubkey = Pubkey::from_str(address).map_err(|e| format!("address error:{:?}", e))?;
        let mut all_signatures = Vec::new();
        let mut before: Option<Signature> = None;
        let sleep_duration = interval_time.unwrap_or(200);
        while all_signatures.len() < limit {
            let remaining = limit - all_signatures.len();
            let batch_size = (remaining.min(1000)).min(u64::MAX.try_into().unwrap());
            let config = GetConfirmedSignaturesForAddress2Config {
                before,
                until: None,
                limit: Some(batch_size),
                commitment: None,
            };
            let signatures = match self
                .client
                .get_signatures_for_address_with_config(&pubkey, config)
                .await
            {
                Ok(sigs) => sigs,
                Err(e) => {
                    break;
                }
            };
            if signatures.is_empty() {
                break;
            }
            let signature_strings: Vec<String> =
                signatures.iter().map(|sig| sig.signature.clone()).collect();
            for sig in signature_strings {
                if !all_signatures.contains(&sig) && all_signatures.len() < limit {
                    all_signatures.push(sig);
                }
            }
            if all_signatures.len() >= limit {
                break;
            }
            if let Some(last_sig) = signatures.last() {
                before = match Signature::from_str(&last_sig.signature) {
                    Ok(sig) => Some(sig),
                    Err(_) => break,
                };
            } else {
                break;
            }
            tokio::time::sleep(Duration::from_millis(sleep_duration)).await;
        }
        Ok(all_signatures)
    }

    /// Quickly fetches the most recent transaction signatures for a given address
    /// Returns only the latest signatures without pagination
    ///
    /// # Params
    /// * `address` - Solana address (base58 encoded) to fetch signatures for
    /// * `count` - Number of recent signatures to return
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - Vector of the most recent transaction signatures
    /// * `Err(String)` - Error message if address parsing or RPC call fails
    pub async fn get_last_signatures(
        &self,
        address: &str,
        count: usize,
    ) -> Result<Vec<String>, String> {
        let pubkey = Pubkey::from_str(address).map_err(|e| format!("address error:{:?}", e))?;
        let config = GetConfirmedSignaturesForAddress2Config {
            before: None,
            until: None,
            limit: Some(count),
            commitment: None,
        };
        let signatures = self
            .client
            .get_signatures_for_address_with_config(&pubkey, config)
            .await
            .map_err(|e| format!("get signatures error:{:?}", e))?;
        let signature_strings: Vec<String> =
            signatures.iter().map(|sig| sig.signature.clone()).collect();
        Ok(signature_strings)
    }
}