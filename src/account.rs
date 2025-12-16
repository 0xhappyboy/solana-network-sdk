use std::{str::FromStr, sync::Arc};

use solana_client::{
    nonblocking::rpc_client::RpcClient, 
    rpc_client::GetConfirmedSignaturesForAddress2Config,
};
use solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};

/// Account analysis structure for querying and analyzing Solana account information
pub struct Account {
    client: Arc<RpcClient>,
    address: String,
}

impl Account {
    /// Creates a new Account instance for analyzing a specific address
    /// 
    /// # Arguments
    /// * `client` - RPC client for Solana network queries
    /// * `address` - Solana account address to analyze
    /// 
    /// # Returns
    /// New Account instance
    pub fn new(client: Arc<RpcClient>, address: &str) -> Self {
        Self {
            client: client,
            address: address.to_string(),
        }
    }

    /// Updates the account address for this instance
    /// 
    /// # Arguments
    /// * `address` - New account address to analyze
    pub fn refresh_address(&mut self, address: &str) {
        self.address = address.to_string();
    }

    /// Parses and returns comprehensive account information as a formatted string
    /// 
    /// # Returns
    /// * `Ok(String)` - Formatted account information
    /// * `Err(String)` - Error message if parsing fails
    pub async fn parse_account_info(&self) -> Result<String, String> {
        let balance_info = self.get_balance_info().await?;
        let account_details = self.get_account_details().await?;
        let transaction_info = self.get_transaction_info().await?;
        let result = format!(
            "{}\n{}\n{}",
            balance_info, account_details, transaction_info
        );
        Ok(result)
    }

    /// Gets formatted balance information for the account
    /// 
    /// # Returns
    /// * `Ok(String)` - Formatted balance information (lamports and SOL)
    /// * `Err(String)` - Error message if balance query fails
    pub async fn get_balance_info(&self) -> Result<String, String> {
        let pubkey = Pubkey::from_str(&self.address)
            .map_err(|e| format!("Invalid address format: {:?}", e))?;
        let account = self.client.get_account(&pubkey).await
            .map_err(|e| format!("Failed to get account: {:?}", e))?;
        let balance_sol = account.lamports as f64 / LAMPORTS_PER_SOL as f64;
        Ok(format!(
            "Balance: {} lamports\nSOL: {}",
            account.lamports, balance_sol
        ))
    }

    /// Gets detailed account properties as a formatted string
    /// 
    /// # Returns
    /// * `Ok(String)` - Formatted account details
    /// * `Err(String)` - Error message if details query fails
    pub async fn get_account_details(&self) -> Result<String, String> {
        let executable = self.is_executable().await?;
        let owner = self.get_owner().await?;
        let data_size = self.get_data_size().await?;
        let rent_epoch = self.get_rent_epoch().await?;
        Ok(format!(
            "Executable: {}\nOwner Program: {}\nData Size: {} bytes\nRent Epoch: {}",
            executable, owner, data_size, rent_epoch
        ))
    }

    /// Checks if the account is an executable program
    /// 
    /// # Returns
    /// * `Ok(bool)` - True if account is executable, false otherwise
    /// * `Err(String)` - Error message if query fails
    pub async fn is_executable(&self) -> Result<bool, String> {
        let pubkey = Pubkey::from_str(&self.address)
            .map_err(|e| format!("Invalid address format: {:?}", e))?;
        self.client.get_account(&pubkey).await
            .map(|account| account.executable)
            .map_err(|e| format!("Failed to get account information: {:?}", e))
    }

    /// Gets the owner program of the account
    /// 
    /// # Returns
    /// * `Ok(String)` - Owner program address
    /// * `Err(String)` - Error message if query fails
    pub async fn get_owner(&self) -> Result<String, String> {
        let pubkey = Pubkey::from_str(&self.address)
            .map_err(|e| format!("Invalid address format: {:?}", e))?;
        self.client.get_account(&pubkey).await
            .map(|account| account.owner.to_string())
            .map_err(|e| format!("Failed to get owner: {:?}", e))
    }

    /// Gets the data size of the account
    /// 
    /// # Returns
    /// * `Ok(usize)` - Account data size in bytes
    /// * `Err(String)` - Error message if query fails
    pub async fn get_data_size(&self) -> Result<usize, String> {
        let pubkey = Pubkey::from_str(&self.address)
            .map_err(|e| format!("Invalid address format: {:?}", e))?;
        self.client.get_account(&pubkey).await
            .map(|account| account.data.len())
            .map_err(|e| format!("Failed to get data size: {:?}", e))
    }

    /// Gets the rent epoch of the account
    /// 
    /// # Returns
    /// * `Ok(u64)` - Rent epoch value
    /// * `Err(String)` - Error message if query fails
    pub async fn get_rent_epoch(&self) -> Result<u64, String> {
        let pubkey = Pubkey::from_str(&self.address)
            .map_err(|e| format!("Invalid address format: {:?}", e))?;
        self.client.get_account(&pubkey).await
            .map(|account| account.rent_epoch)
            .map_err(|e| format!("Failed to get rent epoch: {:?}", e))
    }

    /// Gets executable status (alias for is_executable)
    /// 
    /// # Returns
    /// * `Ok(bool)` - True if account is executable, false otherwise
    /// * `Err(String)` - Error message if query fails
    pub async fn get_executable(&self) -> Result<bool, String> {
        self.is_executable().await
    }

    /// Gets owner program address (alias for get_owner)
    /// 
    /// # Returns
    /// * `Ok(String)` - Owner program address
    /// * `Err(String)` - Error message if query fails
    pub async fn get_owner_address(&self) -> Result<String, String> {
        self.get_owner().await
    }

    /// Gets data size in bytes (alias for get_data_size)
    /// 
    /// # Returns
    /// * `Ok(usize)` - Account data size in bytes
    /// * `Err(String)` - Error message if query fails
    pub async fn get_size_bytes(&self) -> Result<usize, String> {
        self.get_data_size().await
    }

    /// Gets rent epoch (alias for get_rent_epoch)
    /// 
    /// # Returns
    /// * `Ok(u64)` - Rent epoch value
    /// * `Err(String)` - Error message if query fails
    pub async fn get_epoch(&self) -> Result<u64, String> {
        self.get_rent_epoch().await
    }

    /// Gets formatted transaction information
    /// 
    /// # Returns
    /// * `Ok(String)` - Formatted transaction information
    /// * `Err(String)` - Error message if query fails
    pub async fn get_transaction_info(&self) -> Result<String, String> {
        let tx_count = self.get_transaction_count(10).await?;
        let success_count = self.get_successful_transaction_count(10).await?;
        let last_tx_time = self.get_last_transaction_time().await?;
        Ok(format!(
            "Recent Transactions: {}\nSuccessful Transactions: {}\nLast Transaction Time: {:?}",
            tx_count, success_count, last_tx_time
        ))
    }

    /// Gets the number of transactions for the account
    /// 
    /// # Arguments
    /// * `limit` - Maximum number of transactions to query
    /// 
    /// # Returns
    /// * `Ok(usize)` - Number of transactions found (up to limit)
    /// * `Err(String)` - Error message if query fails
    pub async fn get_transaction_count(&self, limit: usize) -> Result<usize, String> {
        let pubkey = Pubkey::from_str(&self.address)
            .map_err(|e| format!("Invalid address format: {:?}", e))?;
        let config = GetConfirmedSignaturesForAddress2Config {
            before: None,
            until: None,
            limit: Some(limit),
            commitment: None,
        };
        self.client.get_signatures_for_address_with_config(&pubkey, config).await
            .map(|signatures| signatures.len())
            .map_err(|e| format!("Failed to get transaction count: {:?}", e))
    }

    /// Gets the number of successful transactions
    /// 
    /// # Arguments
    /// * `limit` - Maximum number of transactions to query
    /// 
    /// # Returns
    /// * `Ok(usize)` - Number of successful transactions
    /// * `Err(String)` - Error message if query fails
    pub async fn get_successful_transaction_count(&self, limit: usize) -> Result<usize, String> {
        let pubkey = Pubkey::from_str(&self.address)
            .map_err(|e| format!("Invalid address format: {:?}", e))?;
        let config = GetConfirmedSignaturesForAddress2Config {
            before: None,
            until: None,
            limit: Some(limit),
            commitment: None,
        };
        self.client.get_signatures_for_address_with_config(&pubkey, config).await
            .map(|signatures| signatures.iter().filter(|sig| sig.err.is_none()).count())
            .map_err(|e| format!("Failed to get transaction count: {:?}", e))
    }

    /// Gets the timestamp of the last transaction
    /// 
    /// # Returns
    /// * `Ok(Option<i64>)` - Unix timestamp of last transaction, or None if no transactions
    /// * `Err(String)` - Error message if query fails
    pub async fn get_last_transaction_time(&self) -> Result<Option<i64>, String> {
        let pubkey = Pubkey::from_str(&self.address)
            .map_err(|e| format!("Invalid address format: {:?}", e))?;
        let config = GetConfirmedSignaturesForAddress2Config {
            before: None,
            until: None,
            limit: Some(1),
            commitment: None,
        };
        self.client.get_signatures_for_address_with_config(&pubkey, config).await
            .map(|signatures| signatures.first().and_then(|sig| sig.block_time))
            .map_err(|e| format!("Failed to get transaction time: {:?}", e))
    }

    /// Gets the number of failed transactions
    /// 
    /// # Arguments
    /// * `limit` - Maximum number of transactions to query
    /// 
    /// # Returns
    /// * `Ok(usize)` - Number of failed transactions
    /// * `Err(String)` - Error message if query fails
    pub async fn get_failed_transaction_count(&self, limit: usize) -> Result<usize, String> {
        let pubkey = Pubkey::from_str(&self.address)
            .map_err(|e| format!("Invalid address format: {:?}", e))?;
        let config = GetConfirmedSignaturesForAddress2Config {
            before: None,
            until: None,
            limit: Some(limit),
            commitment: None,
        };
        self.client.get_signatures_for_address_with_config(&pubkey, config).await
            .map(|signatures| signatures.iter().filter(|sig| sig.err.is_some()).count())
            .map_err(|e| format!("Failed to get transaction count: {:?}", e))
    }

    /// Calculates transaction success rate
    /// 
    /// # Arguments
    /// * `limit` - Maximum number of transactions to analyze
    /// 
    /// # Returns
    /// * `Ok(f64)` - Success rate as percentage (0.0-100.0)
    /// * `Err(String)` - Error message if query fails
    pub async fn get_transaction_success_rate(&self, limit: usize) -> Result<f64, String> {
        let tx_count = self.get_transaction_count(limit).await? as f64;
        let success_count = self.get_successful_transaction_count(limit).await? as f64;
        if tx_count > 0.0 {
            Ok((success_count / tx_count) * 100.0)
        } else {
            Ok(0.0)
        }
    }

    /// Gets signatures of recent transactions
    /// 
    /// # Arguments
    /// * `limit` - Maximum number of signatures to return
    /// 
    /// # Returns
    /// * `Ok(Vec<String>)` - List of transaction signatures
    /// * `Err(String)` - Error message if query fails
    pub async fn get_recent_transaction_signatures(&self, limit: usize) -> Result<Vec<String>, String> {
        let pubkey = Pubkey::from_str(&self.address)
            .map_err(|e| format!("Invalid address format: {:?}", e))?;
        let config = GetConfirmedSignaturesForAddress2Config {
            before: None,
            until: None,
            limit: Some(limit),
            commitment: None,
        };
        self.client.get_signatures_for_address_with_config(&pubkey, config).await
            .map(|signatures| signatures.into_iter().map(|sig| sig.signature).collect())
            .map_err(|e| format!("Failed to get transaction signatures: {:?}", e))
    }

    /// Checks if the account has any transaction history
    /// 
    /// # Arguments
    /// * `limit` - Maximum number of transactions to check
    /// 
    /// # Returns
    /// * `Ok(bool)` - True if account has transactions, false otherwise
    /// * `Err(String)` - Error message if query fails
    pub async fn has_transactions(&self, limit: usize) -> Result<bool, String> {
        let tx_count = self.get_transaction_count(limit).await?;
        Ok(tx_count > 0)
    }

    /// Gets the number of transactions within a specified time range
    /// 
    /// # Arguments
    /// * `start_time` - Start timestamp (Unix seconds)
    /// * `end_time` - End timestamp (Unix seconds)
    /// * `limit` - Maximum number of transactions to query
    /// 
    /// # Returns
    /// * `Ok(usize)` - Number of transactions within time range
    /// * `Err(String)` - Error message if query fails
    pub async fn get_transactions_in_time_range(
        &self,
        start_time: i64,
        end_time: i64,
        limit: usize,
    ) -> Result<usize, String> {
        let pubkey = Pubkey::from_str(&self.address)
            .map_err(|e| format!("Invalid address format: {:?}", e))?;
        let config = GetConfirmedSignaturesForAddress2Config {
            before: None,
            until: None,
            limit: Some(limit),
            commitment: None,
        };
        self.client.get_signatures_for_address_with_config(&pubkey, config).await
            .map(|signatures| {
                signatures.iter()
                    .filter(|sig| {
                        if let Some(block_time) = sig.block_time {
                            block_time >= start_time && block_time <= end_time
                        } else {
                            false
                        }
                    })
                    .count()
            })
            .map_err(|e| format!("Failed to get transaction count: {:?}", e))
    }

    /// Gets account balance in lamports
    /// 
    /// # Returns
    /// * `Ok(u64)` - Balance in lamports
    /// * `Err(String)` - Error message if query fails
    pub async fn get_balance(&self) -> Result<u64, String> {
        let pubkey = Pubkey::from_str(&self.address)
            .map_err(|e| format!("Invalid address format: {:?}", e))?;
        self.client.get_account(&pubkey).await
            .map(|account| account.lamports)
            .map_err(|e| format!("Failed to get balance: {:?}", e))
    }

    /// Gets account balance in SOL
    /// 
    /// # Returns
    /// * `Ok(f64)` - Balance in SOL
    /// * `Err(String)` - Error message if query fails
    pub async fn get_balance_sol(&self) -> Result<f64, String> {
        self.get_balance()
            .await
            .map(|balance| balance as f64 / LAMPORTS_PER_SOL as f64)
    }

    /// Gets transaction history (signatures only)
    /// 
    /// # Arguments
    /// * `limit` - Maximum number of transactions to return
    /// 
    /// # Returns
    /// * `Ok(Vec<String>)` - List of transaction signatures
    /// * `Err(String)` - Error message if query fails
    pub async fn get_transaction_history(&self, limit: usize) -> Result<Vec<String>, String> {
        let pubkey = Pubkey::from_str(&self.address)
            .map_err(|e| format!("Invalid address format: {:?}", e))?;
        let config = GetConfirmedSignaturesForAddress2Config {
            before: None,
            until: None,
            limit: Some(limit),
            commitment: None,
        };
        match self.client.get_signatures_for_address_with_config(&pubkey, config).await {
            Ok(signatures) => {
                let tx_hashes: Vec<String> = signatures.into_iter()
                    .map(|sig| sig.signature)
                    .collect();
                Ok(tx_hashes)
            }
            Err(e) => Err(format!("Failed to get transaction history: {:?}", e)),
        }
    }

    /// Checks if account is active (has recent transactions)
    /// 
    /// # Arguments
    /// * `days_threshold` - Maximum number of days since last transaction to be considered active
    /// 
    /// # Returns
    /// * `Ok(bool)` - True if account is active, false otherwise
    /// * `Err(String)` - Error message if query fails
    pub async fn is_account_active(&self, days_threshold: i64) -> Result<bool, String> {
        let last_tx_time = self.get_last_transaction_time().await?;
        match last_tx_time {
            Some(block_time) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                let days_diff = (now - block_time) / (24 * 3600);
                Ok(days_diff <= days_threshold)
            }
            None => Ok(false),
        }
    }
}