use std::{str::FromStr, sync::Arc};

use solana_client::{
    nonblocking::rpc_client::RpcClient, rpc_client::GetConfirmedSignaturesForAddress2Config,
    rpc_config::RpcTransactionConfig, rpc_response::RpcConfirmedTransactionStatusWithSignature,
};
use solana_sdk::signature::Signature;
use solana_sdk::{message::Message, pubkey::Pubkey};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};

use crate::types::{UnifiedError, UnifiedResult};

pub struct Trade {
    client: Arc<RpcClient>,
}
impl Trade {
    pub fn new(client: Arc<RpcClient>) -> Self {
        Self { client: client }
    }
    /// estimate fee
    pub async fn estimate_fee(&self) -> Result<u64, String> {
        match self.client.get_latest_blockhash().await {
            Ok(blockhash) => {
                match self
                    .client
                    .get_fee_for_message(&Message::new_with_blockhash(&[], None, &blockhash))
                    .await
                {
                    Ok(fee) => Ok(fee),
                    Err(_e) => Err("setimate fee error".to_string()),
                }
            }
            Err(e) => Err(format!("get block hash error: {:?}", e)),
        }
    }

    /// get the transaction records of the specified address based on the cursor.
    ///
    /// # Example
    /// ```rust
    /// let mut cursor: Option<String> = None;
    /// loop {
    ///     match trade
    ///         .get_transactions_history_by_cursor(
    ///             "wallet address",
    ///             cursor.clone(),
    ///             page size,
    ///         )
    ///         .await
    ///     {
    ///         Ok(r) => {
    ///             // r.0 is transaction history list
    ///             if r.1.is_none() {
    ///                 break;
    ///             }
    ///             cursor = r.1;
    ///         }
    ///         Err(_) => {
    ///             break;
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn get_transactions_history_by_cursor(
        &self,
        address: &str,
        cursor: Option<String>,
        page_size: u32,
    ) -> UnifiedResult<(
        Vec<RpcConfirmedTransactionStatusWithSignature>,
        Option<String>,
    )> {
        match Pubkey::from_str(address) {
            Ok(address) => {
                let before = match cursor {
                    Some(c) => Some(Signature::from_str(&c).unwrap()),
                    None => None,
                };
                let config = GetConfirmedSignaturesForAddress2Config {
                    before: before,
                    until: None,
                    limit: Some(page_size as usize),
                    commitment: None,
                };
                let signatures: Vec<RpcConfirmedTransactionStatusWithSignature> = match self
                    .client
                    .get_signatures_for_address_with_config(&address, config)
                    .await
                {
                    Ok(signatures) => {
                        if signatures.is_empty() {
                            return Err(UnifiedError::Error("signatures is empty".to_string()));
                        }
                        signatures
                    }
                    _ => Vec::<RpcConfirmedTransactionStatusWithSignature>::new(),
                };
                let next_cursor = signatures.last().map(
                    |sig_info: &RpcConfirmedTransactionStatusWithSignature| {
                        sig_info.signature.clone()
                    },
                );
                Ok((signatures, next_cursor))
            }
            Err(_) => Err(UnifiedError::Error("address from string error".to_string())),
        }
    }

    /// Get transaction records of a specified address and support filtering conditions
    ///
    /// # Params
    /// client - client
    /// address - wallet
    /// filter - filter condition closure, returning true means retaining the transaction record
    ///
    /// # Example
    /// ```rust
    /// let solana = Solana::new(Mode::DEV).unwrap();
    /// let client = solana.client_arc();
    /// let history = TradeHistory::get_filtered_transactions(
    ///     &client,
    ///     "wallet address",
    ///     |sig_info| {
    ///         // return true to retain transaction information
    ///         true
    ///     },
    /// ).await;
    /// ```
    pub async fn get_transactions_history_filtered<F>(
        client: &Arc<RpcClient>,
        address: &str,
        filter: F,
    ) -> UnifiedResult<Vec<RpcConfirmedTransactionStatusWithSignature>>
    where
        F: Fn(&RpcConfirmedTransactionStatusWithSignature) -> bool,
    {
        match Pubkey::from_str(address) {
            Ok(pubkey) => {
                let config = GetConfirmedSignaturesForAddress2Config {
                    before: None,
                    until: None,
                    limit: None,
                    commitment: None,
                };
                match client
                    .get_signatures_for_address_with_config(&pubkey, config)
                    .await
                {
                    Ok(signatures) => {
                        let filtered: Vec<RpcConfirmedTransactionStatusWithSignature> = signatures
                            .into_iter()
                            .filter(|sig_info| filter(sig_info))
                            .collect();
                        Ok(filtered)
                    }
                    Err(e) => Err(UnifiedError::Error(format!(
                        "failed to obtain transaction records: {:?}",
                        e
                    ))),
                }
            }
            Err(_) => Err(UnifiedError::Error("address format error".to_string())),
        }
    }

    /// get the last transaction record of address A that contains address B.
    ///
    /// # Params
    /// address_a - main inquiry address
    /// address_b - the address to check for inclusion
    ///
    /// # Returns
    /// Ok(Some(RpcConfirmedTransactionStatusWithSignature)) - last transaction records containing address B
    /// Ok(None) - does not contain address B
    /// Err - error
    ///
    /// # Example
    /// ```rust
    /// let solana = Solana::new(Mode::DEV).unwrap();
    /// let trade = solana.create_trade();
    /// let contains = trade.contains_address_in_transactions(
    ///     "address a",
    ///     "address b"
    /// ).await;
    /// ```
    pub async fn get_last_transactions_contains_address(
        &self,
        address_a: &str,
        address_b: &str,
    ) -> UnifiedResult<Option<RpcConfirmedTransactionStatusWithSignature>> {
        let all_transactions =
            Self::get_transactions_history_filtered(&self.client, address_a, |_| true).await?;
        if all_transactions.is_empty() {
            return Err(UnifiedError::Error("transactions is empty".to_string()));
        }
        let address_b_pubkey = match Pubkey::from_str(address_b) {
            Ok(pubkey) => pubkey,
            Err(_) => return Err(UnifiedError::Error("address B format error".to_string())),
        };
        let address_b_str = address_b_pubkey.to_string();
        for transaction in all_transactions {
            if self
                .is_transaction_contains_address(&transaction.signature, &address_b_str)
                .await
            {
                return Ok(Some(transaction));
            }
        }
        Ok(None)
    }

    /// Get all transactions of address A that contain address B
    ///
    /// # Params
    /// address_a - main inquiry address
    /// address_b - address to include
    ///
    /// # Returns
    /// contains a list of all transaction records for address B
    ///
    /// # Example
    /// ```rust
    /// let solana = Solana::new(Mode::DEV).unwrap();
    /// let trade = solana.create_trade();
    /// let related_transactions = trade.get_transactions_vec_containing_address(
    ///     "address a",
    ///     "address b"
    /// ).await;
    /// ```
    pub async fn get_transactions_vec_containing_address(
        &self,
        address_a: &str,
        address_b: &str,
    ) -> UnifiedResult<Vec<RpcConfirmedTransactionStatusWithSignature>> {
        let all_transactions =
            Self::get_transactions_history_filtered(&self.client, address_a, |_| true).await?;
        if all_transactions.is_empty() {
            return Ok(Vec::new());
        }
        let address_b_str = address_b.to_string();
        let mut matching_transactions = Vec::new();
        for transaction in all_transactions {
            if self
                .is_transaction_contains_address(&transaction.signature, &address_b_str)
                .await
            {
                matching_transactions.push(transaction);
            }
        }
        Ok(matching_transactions)
    }

    /// get transaction details
    ///
    /// # params
    /// signature - transaction signature hash string
    ///
    /// # Example
    /// ```rust
    /// let solana = Solana::new(solana_trader::types::Mode::DEV).unwrap();
    /// let trade = solana.create_trade();
    /// let transaction_info = trade.get_transaction_details("transaction signature").await;
    /// ```
    pub async fn get_transaction_details(
        &self,
        signature: &str,
    ) -> UnifiedResult<EncodedConfirmedTransactionWithStatusMeta> {
        let signature = match Signature::from_str(&signature) {
            Ok(signature) => signature,
            Err(_) => todo!(),
        };
        let config = RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Json),
            commitment: None,
            max_supported_transaction_version: Some(0),
        };
        match self
            .client
            .get_transaction_with_config(&signature, config)
            .await
        {
            Ok(transaction) => Ok(transaction),
            Err(_e) => {
                // get tade info error
                Err(UnifiedError::Error("get tade info error".to_string()))
            }
        }
    }

    /// checks whether a single transaction contains a specified address
    async fn is_transaction_contains_address(&self, signature: &str, target_address: &str) -> bool {
        match self.get_transaction_details(signature).await {
            Ok(transaction) => {
                let transaction_str = format!("{:?}", transaction);
                transaction_str.contains(target_address)
            }
            Err(_) => false,
        }
    }
}
