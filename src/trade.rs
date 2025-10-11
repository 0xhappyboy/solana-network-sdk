use std::{str::FromStr, sync::Arc};

use solana_client::{
    nonblocking::rpc_client::RpcClient, rpc_client::GetConfirmedSignaturesForAddress2Config,
    rpc_config::RpcTransactionConfig, rpc_response::RpcConfirmedTransactionStatusWithSignature,
};
use solana_sdk::signature::Signature;
use solana_sdk::{message::Message, pubkey::Pubkey};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};

/// Transaction information processing function type
type FnTradeInfoHandle = fn(EncodedConfirmedTransactionWithStatusMeta);

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
    /// # Example
    /// ```rust
    /// let mut cursor: Option<String> = None;
    /// loop {
    ///     match trade
    ///         .get_paginated_history_with_cursor(
    ///             "public key",
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
    pub async fn get_paginated_history_with_cursor(
        &self,
        address: &str,
        cursor: Option<String>,
        page_size: u32,
    ) -> Result<
        (
            Vec<RpcConfirmedTransactionStatusWithSignature>,
            Option<String>,
        ),
        String,
    > {
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
                            return Err("signatures is empty".to_string());
                        }
                        signatures
                    }
                    _ => Vec::<RpcConfirmedTransactionStatusWithSignature>::new(),
                };
                let next_cursor = signatures.last().map(|sig_info| sig_info.signature.clone());
                Ok((signatures, next_cursor))
            }
            Err(_) => Err("address from string error".to_string()),
        }
    }

    /// get transaction details
    /// # params
    /// * signature transaction signature string
    /// # Example
    /// ```rust
    /// let solana = Solana::new(solana_trader::types::Mode::DEV).unwrap();
    /// let trade = solana.create_trade();
    ///     trade.get_trade_details(
    ///     "transaction signature",|info|{
    ///     // handle transaction information.
    ///    }).await;
    /// ```
    pub async fn get_trade_details(&self, signature: &str, handle: FnTradeInfoHandle) {
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
            Ok(transaction) => {
                handle(transaction);
            }
            Err(_e) => {
                // get tade info error
            }
        }
    }
}

pub struct TradeHistory;
impl TradeHistory {}
