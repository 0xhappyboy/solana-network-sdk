pub mod account;
pub mod block;
pub mod global;
pub mod message;
pub mod pyth;
pub mod scan;
pub mod spl;
pub mod tool;
pub mod trade;
pub mod types;
pub mod wallet;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_network_client::SolanaClient;
use solana_sdk::{epoch_info::EpochInfo, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};
use std::{str::FromStr, sync::Arc};

use crate::{
    account::Account,
    block::Block,
    scan::Scan,
    spl::Spl,
    trade::Trade,
    types::{Mode, UnifiedError, UnifiedResult},
};

/// solana client Abstraction
pub struct Solana {
    mode: Mode,
    pub solana_client: Option<Arc<SolanaClient>>,
}

impl Solana {
    /// create solana object
    pub fn new(mode: Mode) -> Result<Solana, String> {
        Ok(Self {
            mode,
            solana_client: Some(Arc::new(
                SolanaClient::new(solana_network_client::Mode::MAIN)
                    .map_err(|e| format!("create solana client error: {:?}", e))
                    .unwrap(),
            )),
        })
    }
    /// get client arc
    /// Example
    /// ```rust
    /// let s = Solana::new(Mode::TEST);
    /// let client = s.client_arc().await;
    /// ```
    pub fn client_arc(&self) -> Arc<RpcClient> {
        self.solana_client.as_ref().unwrap().client_arc()
    }

    /// get solana core version
    /// Example
    /// ```rust
    /// let s = Solana::new(Mode::TEST);
    /// let client = s.core_version().await;
    /// ```
    pub async fn core_version(&self) -> Result<String, String> {
        match self.client_arc().get_version().await {
            Ok(version) => {
                return Ok(version.solana_core);
            }
            Err(e) => {
                return Err(format!("get core version error: {:?}", e));
            }
        }
    }

    /// get feature set
    /// Example
    /// ```rust
    /// let s = Solana::new(Mode::TEST);
    /// let client = s.feature_set().await;
    /// ```
    pub async fn feature_set(&self) -> Result<String, String> {
        match self.client_arc().get_version().await {
            Ok(version) => {
                return Ok(version.feature_set.unwrap().to_string());
            }
            Err(e) => {
                return Err(format!("get core version error: {:?}", e));
            }
        }
    }

    /// get block height
    /// Example
    /// ```rust
    /// let s = Solana::new(Mode::TEST);
    /// let client = s.block_height().await;
    /// ```
    pub async fn block_height(&self) -> Result<u64, String> {
        match self.client_arc().get_block_height().await {
            Ok(h) => {
                return Ok(h);
            }
            Err(e) => {
                return Err(format!("get core version error: {:?}", e));
            }
        }
    }

    /// last block hash
    /// Example
    /// ```rust
    /// let s = Solana::new(Mode::TEST);
    /// let client = s.last_block_hash().await;
    /// ```
    pub async fn last_block_hash(&self) -> Result<String, String> {
        match self.client_arc().get_latest_blockhash().await {
            Ok(h) => {
                return Ok(h.to_string());
            }
            Err(e) => {
                return Err(format!("get core version error: {:?}", e));
            }
        }
    }

    /// get current slot
    /// Example
    /// ```rust
    /// let s = Solana::new(Mode::TEST);
    /// let client = s.slot().await;
    /// ```
    pub async fn slot(&self) -> Result<u64, String> {
        match self.client_arc().get_slot().await {
            Ok(slot) => {
                return Ok(slot);
            }
            Err(e) => {
                return Err(format!("get core version error: {:?}", e));
            }
        }
    }
    /// get current epoch info
    /// Example
    /// ```rust
    /// let s = Solana::new(Mode::TEST);
    /// let client = s.epoch().await;
    /// ```
    pub async fn epoch(&self) -> Result<EpochInfo, String> {
        match self.client_arc().get_epoch_info().await {
            Ok(epoch) => {
                return Ok(epoch);
            }
            Err(e) => {
                return Err(format!("get core version error: {:?}", e));
            }
        }
    }
    /// get account
    /// # Returns
    /// * 0 solana balance
    /// * 1 solana lamports balance
    pub async fn get_account_balance(&self, public_key: &str) -> UnifiedResult<(f64, u64), f64> {
        let pubkey = Pubkey::from_str(&public_key).map_err(|e| UnifiedError::Error(0.0))?;
        let balance = self
            .client_arc()
            .get_balance(&pubkey)
            .await
            .map_err(|e| UnifiedError::Error(0.0))?;
        Ok((balance as f64 / LAMPORTS_PER_SOL as f64, balance))
    }

    pub async fn get_account_data(&self, address: &str) -> UnifiedResult<Vec<u8>, String> {
        Ok(self
            .solana_client
            .as_ref()
            .unwrap()
            .client_arc()
            .get_account_data(
                &Pubkey::from_str(address)
                    .map_err(|e| UnifiedError::Error(format!("{:?}", e)))
                    .unwrap(),
            )
            .await
            .map_err(|e| UnifiedError::Error(format!("{:?}", e)))
            .unwrap())
    }

    /// create account
    pub fn create_account(&self, address: &str) -> Account {
        Account::new(self.client_arc(), address)
    }
    /// create trade
    pub fn create_trade(&self) -> Trade {
        Trade::new(self.client_arc())
    }
    /// create block service
    pub fn create_block_service(&self) -> Block {
        Block::new(self.client_arc())
    }
    /// create scan
    pub fn create_scan(&self) -> Scan {
        Scan::new(self.client_arc())
    }
    /// create spl
    pub fn create_spl(&self) -> Spl {
        Spl::new(self.client_arc())
    }
}

#[cfg(test)]
mod tests {
    use raydium_sdk::Raydium;
    use solana_network_client::SolanaClient;

    use super::*;

    #[tokio::test]
    async fn test_clmm_data_parsing() -> Result<(), Box<dyn std::error::Error>> {
        let solana_client = SolanaClient::new(solana_network_client::Mode::MAIN).unwrap();
        let raydium = Raydium::new(Arc::new(solana_client));
        let pool_data = raydium
            .get_liquidity_pool_clmm("DYZopjL34W4XpxbZaEjsCsXsrt6HbgE8WMCmPF1oPCwM")
            .await;
        println!("Pool Info: {:?}", pool_data);
        Ok(())
    }

    #[tokio::test]
    async fn test_1() -> Result<(), Box<dyn std::error::Error>> {
        let solana_client = SolanaClient::new(solana_network_client::Mode::MAIN).unwrap();
        let raydium = Raydium::new(Arc::new(solana_client));
        let pool_data = raydium
            .get_liquidity_pool_cpmm("8Lq7gz2aEzkMQNfLpYmjv3V8JbD26LRbFd11SnRicCE6")
            .await;
        println!("Pool Info: {:?}", pool_data);
        Ok(())
    }

    #[tokio::test]
    async fn test_2() -> Result<(), Box<dyn std::error::Error>> {
        let solana_client = SolanaClient::new(solana_network_client::Mode::MAIN).unwrap();
        let raydium = Raydium::new(Arc::new(solana_client));
        // 58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2 SOL-USDC pool
        let pool_data = raydium
            .get_liquidity_pool_v4("58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2")
            .await;
        println!("Pool Info: {:?}", pool_data);
        Ok(())
    }

    #[tokio::test]
    async fn test_launchpad_data_parsing() -> Result<(), Box<dyn std::error::Error>> {
        let solana_client = SolanaClient::new(solana_network_client::Mode::MAIN).unwrap();
        let raydium = Raydium::new(Arc::new(solana_client));
        let pool_data = raydium
            .get_liquidity_pool_launchpad("GSxb28CtEf9vJHEoB9D2NoNwbbkj8SxQN3WN86qvMULZ")
            .await;
        println!("Pool Info: {:?}", pool_data);
        Ok(())
    }
}
