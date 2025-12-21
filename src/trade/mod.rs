pub mod info;
pub mod pump;
use std::vec;
use std::{str::FromStr, sync::Arc};

use base64::Engine;
use base64::engine::general_purpose;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_client::{
    nonblocking::rpc_client::RpcClient, rpc_client::GetConfirmedSignaturesForAddress2Config,
    rpc_config::RpcTransactionConfig, rpc_response::RpcConfirmedTransactionStatusWithSignature,
};
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::TransactionVersion;
use solana_sdk::{message::Message, pubkey::Pubkey};
use solana_transaction_status::option_serializer::OptionSerializer;
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, EncodedTransaction, UiMessage, UiParsedInstruction,
    UiTransactionEncoding, UiTransactionTokenBalance,
};

use crate::global::{
    METEORA_DAMM_V2_PROGRAM_ID, METEORA_DLMM_V2_PROGRAM_ID, METEORA_POOL_PROGRAM_ID,
    ORCA_WHIRLPOOLS_PROGRAM_ID, PUMP_AAM_PROGRAM_ID, PUMP_BOND_CURVE_PROGRAM_ID,
    RAYDIUM_CLMM_POOL_PROGRAM_ID, RAYDIUM_CPMM_POOL_PROGRAM_ID, RAYDIUM_V4_POOL_PROGRAM_ID, SOL,
    USDC, USDT,
};
use crate::trade::info::TransactionInfo;
use crate::trade::pump::PumpBondCurveTransactionInfo;
use crate::types::{DexProgramType, Direction, TransactionType, UnifiedError, UnifiedResult};

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
    ) -> UnifiedResult<
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
    ) -> UnifiedResult<Vec<RpcConfirmedTransactionStatusWithSignature>, String>
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
    ) -> UnifiedResult<Option<RpcConfirmedTransactionStatusWithSignature>, String> {
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
    ) -> UnifiedResult<Vec<RpcConfirmedTransactionStatusWithSignature>, String> {
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

    /// get the transaction record with address A as the payer and address B included
    /// loose filtering: Address A is the payer and the transaction contains address B (not necessarily the payer)
    /// address B only needs to appear in the transaction (may be the payer, payer, signer)
    ///
    /// # Params
    /// address_a - Recipient address
    /// address_b - Payer address
    /// limit - Maximum number of transactions returned
    ///
    /// # Example
    /// ```rust
    /// let transactions = trade.get_transactions_by_recipient_and_payer(
    ///     "Recipient address",
    ///     "payer",
    ///     10
    /// ).await?;
    /// ```
    pub async fn get_transactions_by_recipient_and_payer(
        &self,
        address_a: &str,
        address_b: &str,
        limit: usize,
    ) -> UnifiedResult<Vec<RpcConfirmedTransactionStatusWithSignature>, String> {
        let all_transactions =
            Self::get_transactions_history_filtered(&self.client, address_a, |_| true).await?;
        let mut matching_transactions = Vec::new();
        let address_b_pubkey = Pubkey::from_str(address_b)
            .map_err(|_| UnifiedError::Error("address B format error".to_string()))?;
        let address_b_str = address_b_pubkey.to_string();
        for transaction in all_transactions.into_iter().take(limit) {
            // Check if the transaction contains address B
            if !self
                .is_transaction_contains_address(&transaction.signature, &address_b_str)
                .await
            {
                continue;
            }
            match self.get_transaction_details(&transaction.signature).await {
                Ok(tx_details) => {
                    let transaction_info = TransactionInfo::from_encoded_transaction(
                        &tx_details,
                        &transaction.signature,
                    );
                    if Self::is_address_recipient_in_transaction(&transaction_info, address_a) {
                        matching_transactions.push(transaction);
                    }
                }
                Err(_) => continue,
            }
        }
        Ok(matching_transactions)
    }

    /// transaction where address A is the payer and address B is the payer
    /// strict screening: Ensure that address A is the recipient and address B is the payer
    /// clear address B to address A capital flow
    ///
    /// # Params
    /// address_a - Recipient address
    /// address_b - Payer address
    /// limit - Maximum number of transactions returned
    ///
    pub async fn get_transactions_by_recipient_and_payer_strict(
        &self,
        address_a: &str,
        address_b: &str,
        limit: usize,
    ) -> UnifiedResult<Vec<RpcConfirmedTransactionStatusWithSignature>, String> {
        let candidate_transactions = self
            .get_transactions_by_recipient_and_payer(address_a, address_b, limit * 2)
            .await?;
        let mut confirmed_transactions = Vec::new();
        for transaction in candidate_transactions.into_iter().take(limit) {
            match self.get_transaction_details(&transaction.signature).await {
                Ok(tx_details) => {
                    let transaction_info = TransactionInfo::from_encoded_transaction(
                        &tx_details,
                        &transaction.signature,
                    );
                    // Address A is the payer and Address B is the payer
                    if Self::is_address_recipient_in_transaction(&transaction_info, address_a)
                        && Self::is_address_payer_in_transaction(&transaction_info, address_b)
                    {
                        confirmed_transactions.push(transaction);
                    }
                }
                Err(_) => continue,
            }
        }
        Ok(confirmed_transactions)
    }

    /// Determine whether address A is the recipient in the transaction
    ///
    /// # Params
    /// transaction_info - transaction information
    /// address - address to check
    ///
    /// # Returns
    /// true - address is the recipient
    /// false - address is not the recipient
    fn is_address_recipient_in_transaction(
        transaction_info: &TransactionInfo,
        address: &str,
    ) -> bool {
        if transaction_info.balance_change > 0 {
            if let Some(post_balance) = transaction_info
                .post_token_balances
                .iter()
                .find(|balance| balance.owner == address)
            {
                if let Some(pre_balance) = transaction_info
                    .pre_token_balances
                    .iter()
                    .find(|balance| balance.owner == address && balance.mint == post_balance.mint)
                {
                    let pre_amount = pre_balance
                        .ui_token_amount
                        .amount
                        .parse::<u64>()
                        .unwrap_or(0);
                    let post_amount = post_balance
                        .ui_token_amount
                        .amount
                        .parse::<u64>()
                        .unwrap_or(0);
                    return post_amount > pre_amount;
                }
            }
        }
        false
    }

    /// Determine whether address B is the payer in the transaction
    ///
    /// # Params
    /// transaction_info - transaction info
    /// address - address to check
    ///
    /// # Returns
    /// true - address B is the payer
    /// false - address B is not the payer
    fn is_address_payer_in_transaction(
        transaction_info: &TransactionInfo,
        address_b: &str,
    ) -> bool {
        if transaction_info.signers.contains(&address_b.to_string()) {
            return true;
        }
        false
    }

    /// Quickly determine whether there is a payment relationship between two addresses (address B pays address A)
    ///
    /// # Params
    /// address_a - Recipient address
    /// address_b - Payer address
    ///
    /// # Returns
    /// Some(signature) - If there is a payment relationship
    /// None - If there is no payment relationship
    pub async fn has_payment_relationship(
        &self,
        address_a: &str,
        address_b: &str,
    ) -> UnifiedResult<Option<String>, String> {
        let transactions = self
            .get_transactions_by_recipient_and_payer_strict(address_a, address_b, 1)
            .await?;
        if let Some(transaction) = transactions.first() {
            Ok(Some(transaction.signature.clone()))
        } else {
            Ok(None)
        }
    }

    /// Get the total amount paid by address B to address A
    ///
    /// # Params
    /// address_a - Recipient address
    /// address_b - Payer address
    /// time_range - Time range (seconds), None means all time
    ///
    /// # Returns
    /// Total payment amount (lamports)
    pub async fn get_total_payment_amount(
        &self,
        address_a: &str,
        address_b: &str,
        time_range: Option<u64>,
    ) -> UnifiedResult<u64, String> {
        let transactions = self
            .get_transactions_by_recipient_and_payer_strict(address_a, address_b, 100)
            .await?;
        let mut total_amount = 0u64;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        for transaction in transactions {
            match self.get_transaction_details(&transaction.signature).await {
                Ok(tx_details) => {
                    let transaction_info = TransactionInfo::from_encoded_transaction(
                        &tx_details,
                        &transaction.signature,
                    );
                    if let Some(range) = time_range {
                        if let Some(block_time) = transaction_info.block_time {
                            if (now - block_time as u64) > range {
                                continue;
                            }
                        }
                    }
                    if let Ok(amount) = transaction_info.value.parse::<u64>() {
                        total_amount += amount;
                    }
                }
                Err(_) => continue,
            }
        }
        Ok(total_amount)
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
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta, String> {
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
            Err(e) => {
                // get tade info error
                Err(format!("get tade info error: {:?}", e))
            }
        }
    }

    /// Get transaction details in batch
    ///
    /// # Parameters
    /// signatures - Array of transaction signature strings (slice)
    ///
    /// # Returns
    /// `Result<Vec<EncodedConfirmedTransactionWithStatusMeta>, String>`
    /// - `Ok(transactions)`: List of successfully retrieved transaction details
    /// - `Err(error)`: Error during batch query process (e.g., network error)
    ///
    /// # Features
    /// - Executes multiple transaction queries in parallel for improved efficiency
    /// - Automatically filters failed queries, returning only successful transactions
    /// - Individual query failures do not affect other queries
    ///
    /// # Example
    /// ```rust
    /// let solana = Solana::new(solana_trader::types::Mode::MAIN).unwrap();
    /// let trade = solana.create_trade();
    ///
    /// let signatures = vec![
    ///     "4x4b2F6...", // transaction signature 1
    ///     "5y5c3G7...", // transaction signature 2
    ///     "6z6d4H8...", // transaction signature 3
    /// ];
    ///
    /// // Batch query transactions
    /// let transactions = trade.get_transaction_details_batch(&signatures).await?;
    ///
    /// // Process each transaction
    /// for tx in transactions {
    ///     let tx_info = TransactionInfo::from_encoded_transaction(&tx, "signature");
    ///     println!("Transaction slot: {}", tx_info.slot);
    /// }
    /// ```
    ///
    /// # Performance Recommendations
    /// - Recommended to query no more than 50 transaction signatures at once to avoid RPC limits
    /// - For large numbers of queries, consider batching them
    /// - Failed queries output error messages to stderr without interrupting the entire batch operation
    pub async fn get_transaction_details_batch(
        &self,
        signatures: Vec<&str>,
    ) -> Result<Vec<EncodedConfirmedTransactionWithStatusMeta>, String> {
        let mut futures = Vec::new();
        for signature in signatures {
            let signature_str = signature.to_string();
            let client = self.client.clone();
            let future = async move {
                match Signature::from_str(&signature_str) {
                    Ok(sig) => {
                        let config = RpcTransactionConfig {
                            encoding: Some(UiTransactionEncoding::Json),
                            commitment: None,
                            max_supported_transaction_version: Some(0),
                        };
                        match client.get_transaction_with_config(&sig, config).await {
                            Ok(transaction) => Ok(transaction),
                            Err(e) => Err(format!(
                                "get transaction error for {}: {:?}",
                                signature_str, e
                            )),
                        }
                    }
                    Err(_) => Err(format!("invalid signature: {}", signature_str)),
                }
            };
            futures.push(future);
        }
        let results = join_all(futures).await;
        let mut successful_transactions = Vec::new();
        for result in results {
            match result {
                Ok(tx) => successful_transactions.push(tx),
                Err(e) => {
                    eprintln!("Transaction query error: {}", e);
                }
            }
        }
        Ok(successful_transactions)
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
    pub async fn get_transaction_display_details(
        &self,
        signature: &str,
    ) -> UnifiedResult<TransactionInfo, String> {
        Ok(TransactionInfo::from_encoded_transaction(
            &self
                .get_transaction_details(signature)
                .await
                .map_err(|e| format!("get transaction details error {:?}", e))
                .unwrap(),
            signature,
        ))
    }

    /// get transaction details in batch
    ///
    /// # params
    /// signatures - transaction signature hash string array
    ///
    /// # Example
    /// ```rust
    /// let solana = Solana::new(solana_trader::types::Mode::DEV).unwrap();
    /// let trade = solana.create_trade();
    /// let transaction_infos = trade.get_transaction_display_details_batch(&["signature1", "signature2"]).await;
    /// ```
    pub async fn get_transaction_display_details_batch(
        &self,
        signatures: Vec<&str>,
    ) -> UnifiedResult<Vec<TransactionInfo>, String> {
        let raw_transactions = self
            .get_transaction_details_batch(signatures.clone())
            .await
            .map_err(|e| format!("get batch transaction details error: {}", e))
            .unwrap();
        let transaction_infos = raw_transactions
            .iter()
            .enumerate()
            .filter_map(|(i, tx)| {
                if i < signatures.len() {
                    Some(TransactionInfo::from_encoded_transaction(tx, signatures[i]))
                } else {
                    None
                }
            })
            .collect();
        Ok(transaction_infos)
    }
}

#[cfg(test)]
mod tests {
    use crate::Solana;

    #[tokio::test]
    async fn test_get_transaction_display_details_batch() -> Result<(), ()> {
        let solana = Solana::new(crate::types::Mode::MAIN).unwrap();
        let trade = solana.create_trade();
        let signs = vec![
            "28sRV5e3NYhy9CR8r5Es8vYYouF95VZpkYjMr65fAziYMzFzHjCpbpb6YmFB5pusa6ZD3LbJo2kM8iH8mjT21QXq",
            "j8Vs7qDSU1qmGaN4mRfiVLbX1vxwEPhVgHEqQnzzbvG7Z5LWKnQfu9ZyfMWk5Lpw1QenZgGhaiRFu8D2CaYGXaq",
            "22zKdFE9Dd1x917h7f9yCYDmoYFTcVDrLJe58jwNgjrRnbzh4GXxney13b2AAPDbtD93HZC9kQa8G9tb9WLQDFae",
            "3Rfy3QwXcXTGGvdDnt2yVuX4FkUbonBNJUcN1SKGzNiWxK9SudSnw3MFXU8PsC17o1j5TNX7Jeemx51kn2brosbG",
            "vcrEnzsx3mdqoLccxramUD4A65KfG8ippcWACRLYPF5tq7MNWBSpyeJhEX51fKrYFV66xuEi3Htmgxrjtwm9K5L",
            "3mrYV3rzxWmekwyeSVP2KLhQsTUs3JSAAKTg4fobWkdrVi7jicX9U8okySKYcHGsqjpQKmbSvo1SSdjPVFokoUvQ",
            "2P43xMwMzVBjnnSxKgVtXs7jApF9cpBigXWCjsuhs52xxi7axwWrxjDX7Wvy4pbLLiYUgBTBhwNDNvjmrBMUFWok",
            "4Vfdy5hpgpi2yiVuuPP7e1tq83K31u1amXXuK1AjKFFEzH8tXZDbaAuqNFPTJ4MJCCvXhNkdMS3FSZUUyKH6tVBD",
            "34VPwTWQAXYEjAQRinhxAbEHaGULXt6uPcLjPfzcfX26ZBUQER8VebeFS9xsEYCdd3caMHRJvCES8LG2q6M9JNmx",
            "3uMea311NS4hEmPe9mJbmPxS6C5wKDK3Urj6oMnaE6MzXoB3Ydt1z5LyAZGTDguZbh6MiEMvV9sGYp8uZWVUxEYj",
            "2ghLVUfrxaXJCXFrs7V8Y4S45XkdVkgTkjsv16cGgHzRCm5nF54ySDx3jdfU6BwcmA58K1C46NgbmS3CgMbyCS3S",
            "5gKTTkuboEZoNys4cK3T3sM5x52y14tWLjRKbFeQQmXPfTst5GpPFDWNo9r1Dc9Ns4ivj6d5VcDwNSFT6WSaaJv8",
            "2SB72xuo8EMyCBZsFt1Hrt9eVXR3qeoq1p3naNaSkTnQkvqr73xTwCtuWqg3tjMsgCC98LVsEzHDUMhirwEcZzjr",
            "4JyQcwxqaXgC74kLV7Cxp8zCctjDRQY2ywRGsUJ5QpkRg9gTRWk6hQAhkJVeDQWTeDQEiDhh8iK621QybCTRBDwL",
            "3hdkaxLkG9XnHAhP2e16uqjsehi6HGNT2b5HKtYY79S83Gz9Dh2ApfGXdoMsBFUEfUjFR26mXAj3XgSone5SSvg9",
            "4Br83oFTZh3CsxA93Kq2m1dbzV8AEKZSCU5Jc4g8AhcQnegcGKii8G68tVA4JmKLiTSDEtY63SU3HiwiK8vCLZzU",
            "YomXdFYSfLCyfoHUQxAxFAs5jCeNXgQhkg5USuU1b7yJ4iwBGXUzPVLMw6HhL95EC7pnt77hXhVtUoAi5Nun4tX",
            "4RsEzDjVEkakioFnYZaNe2gWwfi2KiuZ35m1rPgr7gtaA4uthYSEeMXyJT61nYELVEiewiL4m1C2ScE3t45pSxy4",
            "3TMCCijBZaFgCsqBTj5jJu5XTwYEfcFfgDHMk4fLQ1Vc3sEf2qUGR9ffyZL3im9DncXsui8R3Lgy7gyXV1DrRrg6",
        ];
        let trade_infos = trade
            .get_transaction_display_details_batch(signs)
            .await
            .unwrap();
        println!("Batch Query Results Count: {:?}", trade_infos.len());
        for info in trade_infos {
            println!(
                "
                ==========================================
                hash: {:?}\n,
                is swap: {:?}\n,
                is pump transcation: {:?},
                ==========================================
                ",
                info.transaction_hash,
                if info.is_swap() { "Yes" } else { "No" },
                if info.is_pump() { "Yes" } else { "No" }
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_parse_trade_info() -> Result<(), ()> {
        let solana = Solana::new(crate::types::Mode::MAIN).unwrap();
        let trade = solana.create_trade();
        let trade_info = trade.get_transaction_display_details("2UpRfA6Z2qh6UZDmtRouCq5Wfe8F4E7f8tHrMawgtFtN6mcpf9k89AaMeqznr2FCRBJYWP9kwCZbi87B1aEKHTFq").await.unwrap();
        println!("=====================================================");
        println!("Signature: {:?}", trade_info.transaction_hash);
        println!(
            "Is Swap: {:?}",
            if trade_info.is_swap { "Yes" } else { "No" }
        );
        println!("Token: {:?}", trade_info.get_pool_left_address());
        println!("Quote Token: {:?}", trade_info.get_pool_right_address());
        println!("Received Token: {:?}", trade_info.get_received_token_sol());
        println!("Spent Token: {:?}", trade_info.get_spent_token_sol());
        println!("Quote Ratio: {:?}", trade_info.get_token_quote_ratio());
        println!("=====================================================");
        Ok(())
    }

    #[tokio::test]
    async fn test_get_token_quote_ratio() -> Result<(), ()> {
        let solana = Solana::new(crate::types::Mode::MAIN).unwrap();
        let trade = solana.create_trade();
        let t_info = trade.get_transaction_display_details("2tEx6Y92BtqJV73cBATabdA8TpvHqPrbGHAjMsEHcgzQEYdn8FzxefPWoYXJCVWeuGe4uz5jdH3Vbj7ySK9mfzwM").await.unwrap();
        println!(
            "Quote Token Ratio: {}",
            t_info.get_token_quote_ratio().unwrap()
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_is_dbc_trade() -> Result<(), ()> {
        let solana = Solana::new(crate::types::Mode::MAIN).unwrap();
        let trade = solana.create_trade();
        let t_info = trade.get_transaction_display_details("4q9gPA9zQCRm5UMmdTX6X4N7nTBFe5CEqH8voewStDou7atyBiu9JHbm2K6hSWp7eRVtbV9q5pKGmPxtpsaZyGt1").await.unwrap();
        t_info.display();
        Ok(())
    }

    #[tokio::test]
    async fn test_is_pump_trade() -> Result<(), ()> {
        let solana = Solana::new(crate::types::Mode::MAIN).unwrap();
        let trade = solana.create_trade();
        let t_info = trade.get_transaction_display_details("4Zwt4WYYTFehY8ZNdKD2z2tfQKDLh83dcT6NhAkXqvp9oayDabSX2qSZBi4RVjzSiHDvUSRoaoCsg6iTdg55bat5").await.unwrap();
        t_info.display();
        Ok(())
    }

    #[tokio::test]
    async fn test_is_raylaunchpad_trade() -> Result<(), ()> {
        let solana = Solana::new(crate::types::Mode::MAIN).unwrap();
        let trade = solana.create_trade();
        let t_info = trade.get_transaction_display_details("52ekT61LYVSgWxyQkC5TPYY3XniyLJja16aDN4oFhAFUWGWGiLvaYPxzHv2Krka2wwnu3nmsv55FPpwaTjRxyh4A").await.unwrap();
        t_info.display();
        Ok(())
    }
}
