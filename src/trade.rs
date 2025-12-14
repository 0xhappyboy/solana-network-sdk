use std::vec;
use std::{str::FromStr, sync::Arc};

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
    EncodedConfirmedTransactionWithStatusMeta, EncodedTransaction, UiInstruction, UiMessage,
    UiParsedInstruction, UiTransactionEncoding, UiTransactionTokenBalance,
};

use crate::global::{
    METEORA_DAMM_V2_PROGRAM_ID, METEORA_DLMM_V2_PROGRAM_ID, METEORA_POOL_PROGRAM_ID,
    ORCA_WHIRLPOOLS_PROGRAM_ID, PUMP_AAM_PROGRAM_ID, PUMP_BOND_CURVE_PROGRAM_ID,
    RAYDIUM_CLMM_POOL_PROGRAM_ID, RAYDIUM_CPMM_POOL_PROGRAM_ID, RAYDIUM_V4_POOL_PROGRAM_ID,
};
use crate::types::{Direction, UnifiedError, UnifiedResult};

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
    ) -> UnifiedResult<EncodedConfirmedTransactionWithStatusMeta, String> {
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
}

/// a more readable transaction information structure.
#[derive(Debug)]
pub struct TransactionInfo {
    // Basic Identification Fields
    pub transaction_hash: String,
    pub signature: String,
    // Account Related Fields
    pub from: String,
    pub to: String,
    pub fee_payer: String,
    pub signers: Vec<String>,           // All signers
    pub involved_accounts: Vec<String>, // All involved accounts
    pub writable_accounts: Vec<String>, // Writable accounts
    pub readonly_accounts: Vec<String>, // Read-only accounts
    // Amount Related Fields
    pub value: String,       // Transfer amount in lamports
    pub value_sol: f64,      // Transfer amount in SOL
    pub fee: u64,            // Transaction fee in lamports
    pub fee_sol: f64,        // Transaction fee in SOL
    pub pre_balance: u64,    // Balance before transaction
    pub post_balance: u64,   // Balance after transaction
    pub balance_change: i64, // Balance change (signed)
    // Block Related Fields
    pub block_number: u64,
    pub block_hash: String, // Block hash
    pub block_time: Option<i64>,
    pub slot: u64,
    pub epoch: u64,               // Epoch number
    pub recent_blockhash: String, // Recent blockhash used
    // Status Related Fields
    pub status: String,              // "success", "failed"
    pub confirmation_status: String, // "processed", "confirmed", "finalized"
    pub error_message: Option<String>,
    pub err: Option<Value>, // Raw error information
    pub is_confirmed: bool,
    pub is_finalized: bool,
    // Transaction Type Related
    pub transaction_type: String, // "transfer", "token_transfer", "program_interaction"
    pub program_id: String,
    pub instructions_count: u64,
    pub inner_instructions_count: u64, // Number of inner instructions
    pub version: u8,                   // Transaction version
    // Resource Consumption
    pub compute_units_consumed: Option<u64>, // Compute units consumed
    pub compute_unit_price: Option<u64>,     // Compute unit price
    // Instructions and Logs
    pub log_index: u64,
    pub data: Option<String>,
    pub logs: Vec<String>,
    pub instructions: Vec<InstructionInfo>, // Instruction details
    pub inner_instructions: Vec<InnerInstructionInfo>, // Inner instructions
    // Token Related
    pub token_mint: Option<String>,
    pub token_amount: Option<String>,
    pub token_decimals: Option<u8>,
    pub token_symbol: Option<String>,           // Token symbol
    pub token_name: Option<String>,             // Token name
    pub pre_token_balances: Vec<TokenBalance>,  // Token balances before transaction
    pub post_token_balances: Vec<TokenBalance>, // Token balances after transaction
    // NFT Related
    pub is_nft_transfer: bool,
    pub nft_mint: Option<String>,
    pub nft_name: Option<String>,
    pub nft_symbol: Option<String>,
    // DEX/DeFi Related
    pub is_swap: bool,
    pub dex_program_id: Option<String>,        // DEX program id
    pub dex_program_name: Option<String>,      // DEX program name
    pub dex_pool_program_id: Option<String>,   // DEX program pool id
    pub dex_pool_program_name: Option<String>, // DEX program pool name
    pub input_mint: Option<String>,            // Input token mint
    pub output_mint: Option<String>,           // Output token mint
    pub input_amount: Option<u64>,             // Input amount
    pub output_amount: Option<u64>,            // Output amount
    // Business Extension Fields
    pub memo: Option<String>,
    pub timestamp: Option<u64>,
    pub tags: Vec<String>,
    pub category: String,          // Business category
    pub risk_level: u8,            // Risk level 0-10
    pub is_internal: bool,         // Whether internal transaction
    pub gas_used: Option<u64>,     // Gas used
    pub gas_price: Option<u64>,    // Gas price
    pub max_fee: Option<u64>,      // Maximum fee
    pub priority_fee: Option<u64>, // Priority fee
    // Network Related
    pub cluster: String,  // Cluster information
    pub rpc_node: String, // RPC node information
    // Metadata
    pub created_at: u64, // Record creation timestamp
    pub updated_at: u64, // Record update timestamp
    pub source: String,  // Data source
    pub confidence: f64, // Data confidence level 0.0-1.0

    // trade direction
    pub direction: Option<Direction>,
}

impl TransactionInfo {
    pub fn from_encoded_transaction(
        tx: &EncodedConfirmedTransactionWithStatusMeta,
        signature: &str,
    ) -> Self {
        let mut info = Self::default();
        info.transaction_hash = signature.to_string();
        info.signature = signature.to_string();
        info.block_number = tx.slot;
        info.slot = tx.slot;
        info.block_time = tx.block_time;
        if let Some(meta) = &tx.transaction.meta {
            info.status = if meta.err.is_none() {
                "success".to_string()
            } else {
                "failed".to_string()
            };
            info.error_message = meta.err.as_ref().map(|e| format!("{:?}", e));
            info.err = meta
                .err
                .clone()
                .map(serde_json::to_value)
                .transpose()
                .unwrap_or(None);
            info.fee = meta.fee;
            info.fee_sol = meta.fee as f64 / LAMPORTS_PER_SOL as f64;
            info.compute_units_consumed = match &meta.compute_units_consumed {
                OptionSerializer::Some(value) => Some(*value),
                OptionSerializer::None => None,
                OptionSerializer::Skip => None,
            };
        }
        Self::parse_transaction_content(&mut info, tx);
        // trading direction calculation
        if info.from != "unknown" && info.to != "unknown" {
            if info.balance_change > 0 {
                // In DEX trading, "in" means buying (you receive tokens and pay SOL).
                info.direction = Some(Direction::In);
            } else if info.balance_change < 0 {
                // In DEX trading, "out" means selling (you pay tokens and receive SOL).
                info.direction = Some(Direction::Out);
            }
        }
        info.created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        info.updated_at = info.created_at;
        info.source = "rpc".to_string();
        info.confidence = 1.0;
        info
    }

    fn parse_transaction_content(
        info: &mut TransactionInfo,
        tx: &EncodedConfirmedTransactionWithStatusMeta,
    ) {
        let transaction_with_meta = &tx.transaction;
        match &transaction_with_meta.transaction {
            EncodedTransaction::Json(json_tx) => {
                match &json_tx.message {
                    UiMessage::Parsed(parsed_msg) => {
                        Self::parse_parsed_message(info, parsed_msg);
                    }
                    UiMessage::Raw(raw_msg) => {
                        Self::parse_raw_message(info, raw_msg, tx);
                    }
                }
                info.signers = json_tx.signatures.clone();
            }
            EncodedTransaction::Binary(_, _) => {
                info.transaction_type = "binary".to_string();
            }
            _ => {
                info.transaction_type = "other_encoding".to_string();
            }
        }
        if let Some(meta) = &transaction_with_meta.meta {
            Self::parse_metadata(info, meta, tx);
        }
        if let Some(version) = &transaction_with_meta.version {
            match version {
                TransactionVersion::Legacy(legacy) => {}
                TransactionVersion::Number(num) => {
                    info.version = *num;
                }
            }
        }
    }

    ///  parse parsed message
    fn parse_parsed_message(
        info: &mut TransactionInfo,
        parsed_msg: &solana_transaction_status::UiParsedMessage,
    ) {
        info.involved_accounts = parsed_msg
            .account_keys
            .iter()
            .filter_map(|acc| Some(acc.pubkey.clone()))
            .collect();
        info.instructions_count = parsed_msg.instructions.len() as u64;
        info.instructions = parsed_msg
            .instructions
            .iter()
            .map(|inst| {
                let (stack_height, program) = match inst {
                    solana_transaction_status::UiInstruction::Compiled(compiled) => {
                        (compiled.stack_height, "compiled".to_string())
                    }
                    solana_transaction_status::UiInstruction::Parsed(parsed) => match parsed {
                        solana_transaction_status::UiParsedInstruction::Parsed(parsed_inst) => {
                            (parsed_inst.stack_height, parsed_inst.program.clone())
                        }
                        solana_transaction_status::UiParsedInstruction::PartiallyDecoded(
                            partial_inst,
                        ) => (None, "partially_decoded".to_string()),
                    },
                };
                InstructionInfo {
                    program_id: Self::extract_program_id_from_ui_instruction(inst),
                    accounts: Self::extract_accounts_from_ui_instruction(inst),
                    data: Self::extract_data_from_ui_instruction(inst),
                    stack_height: stack_height,
                    program: program,
                }
            })
            .collect();
        info.recent_blockhash = parsed_msg.recent_blockhash.clone();
        Self::extract_transfer_info(info, parsed_msg);
    }

    fn parse_raw_message(
        info: &mut TransactionInfo,
        raw_msg: &solana_transaction_status::UiRawMessage,
        tx: &EncodedConfirmedTransactionWithStatusMeta,
    ) {
        info.involved_accounts = raw_msg
            .account_keys
            .iter()
            .map(|pk| pk.to_string())
            .collect();
        info.instructions_count = raw_msg.instructions.len() as u64;
        info.recent_blockhash = raw_msg.recent_blockhash.clone();
        info.transaction_type = "raw".to_string();
        info.program_id = "unknown".to_string();
    }

    /// parse metadata
    fn parse_metadata(
        info: &mut TransactionInfo,
        meta: &solana_transaction_status::UiTransactionStatusMeta,
        tx: &EncodedConfirmedTransactionWithStatusMeta,
    ) {
        match &meta.log_messages {
            OptionSerializer::Some(logs) => info.logs = logs.clone(),
            _ => info.logs = vec![],
        }
        match &meta.inner_instructions {
            OptionSerializer::Some(inner_instructions) => {
                info.inner_instructions_count = inner_instructions.len() as u64;
                info.inner_instructions = inner_instructions
                    .iter()
                    .map(|inner| InnerInstructionInfo {
                        index: inner.index,
                        instructions: inner
                            .instructions
                            .iter()
                            .map(|inst| InstructionInfo {
                                program_id: Self::extract_program_id_from_ui_instruction(inst),
                                accounts: Self::extract_accounts_from_ui_instruction(inst),
                                data: Self::extract_data_from_ui_instruction(inst),
                                stack_height: None,
                                program: "inner".to_string(),
                            })
                            .collect(),
                    })
                    .collect();
            }
            _ => {
                info.inner_instructions_count = 0;
                info.inner_instructions = vec![];
            }
        }

        // parse balance changes
        Self::parse_balance_changes(info, meta, tx);

        // token balance
        match &meta.pre_token_balances {
            OptionSerializer::Some(pre_token_balances) => {
                info.pre_token_balances = pre_token_balances
                    .iter()
                    .map(|balance| TokenBalance {
                        account_index: balance.account_index,
                        mint: balance.mint.clone(),
                        owner: balance.owner.clone().unwrap_or("".to_string()),
                        ui_token_amount: UiTokenAmount {
                            ui_amount: balance.ui_token_amount.ui_amount,
                            decimals: balance.ui_token_amount.decimals,
                            amount: balance.ui_token_amount.amount.clone(),
                            ui_amount_string: Some(
                                balance.ui_token_amount.ui_amount_string.clone(),
                            ),
                        },
                    })
                    .collect();
            }
            _ => {
                info.pre_token_balances = vec![];
            }
        }

        match &meta.post_token_balances {
            OptionSerializer::Some(post_token_balances) => {
                info.post_token_balances = post_token_balances
                    .iter()
                    .map(|balance| TokenBalance {
                        account_index: balance.account_index,
                        mint: balance.mint.clone(),
                        owner: balance.owner.clone().unwrap_or("".to_string()),
                        ui_token_amount: UiTokenAmount {
                            ui_amount: balance.ui_token_amount.ui_amount,
                            decimals: balance.ui_token_amount.decimals,
                            amount: balance.ui_token_amount.amount.clone(),
                            ui_amount_string: Some(
                                balance.ui_token_amount.ui_amount_string.clone(),
                            ),
                        },
                    })
                    .collect();
            }
            _ => {
                info.post_token_balances = vec![];
            }
        }
        // parse token transactions
        Self::parse_token_transactions(info, meta);
    }

    /// parse balance changes
    fn parse_balance_changes(
        info: &mut TransactionInfo,
        meta: &solana_transaction_status::UiTransactionStatusMeta,
        tx: &EncodedConfirmedTransactionWithStatusMeta,
    ) {
        // get balance change information
        if let (pre_balances, post_balances) = (&meta.pre_balances, &meta.post_balances) {
            if pre_balances.len() == post_balances.len() && !pre_balances.is_empty() {
                info.pre_balance = pre_balances[0];
                info.post_balance = post_balances[0];
                info.balance_change = post_balances[0] as i64 - pre_balances[0] as i64;
                let mut from_index = None;
                let mut to_index = None;
                let mut transfer_amount = 0u64;
                for (i, (&pre, &post)) in pre_balances.iter().zip(post_balances.iter()).enumerate()
                {
                    if post < pre {
                        // the balance decreases, possibly due to the sender
                        from_index = Some(i);
                        transfer_amount = pre - post;
                    } else if post > pre {
                        // balance increases, possibly the recipient
                        to_index = Some(i);
                    }
                }
                if let (Some(from_idx), Some(to_idx)) = (from_index, to_index) {
                    if info.from == "unknown" || info.to == "unknown" {
                        // Try to get the account address from the transaction
                        let transaction_with_meta = &tx.transaction;
                        if let EncodedTransaction::Json(json_tx) =
                            &transaction_with_meta.transaction
                        {
                            match &json_tx.message {
                                UiMessage::Parsed(parsed_msg) => {
                                    if let account_keys = &parsed_msg.account_keys {
                                        if let (Some(from_account), Some(to_account)) =
                                            (account_keys.get(from_idx), account_keys.get(to_idx))
                                        {
                                            let (from_pubkey, to_pubkey) =
                                                (&from_account.pubkey, &to_account.pubkey);
                                            info.from = from_pubkey.clone();
                                            info.to = to_pubkey.clone();
                                            info.value = transfer_amount.to_string();
                                            info.value_sol =
                                                transfer_amount as f64 / LAMPORTS_PER_SOL as f64;
                                        }
                                    }
                                }
                                UiMessage::Raw(raw_msg) => {
                                    if let (Some(from_account), Some(to_account)) = (
                                        raw_msg.account_keys.get(from_idx),
                                        raw_msg.account_keys.get(to_idx),
                                    ) {
                                        info.from = from_account.to_string();
                                        info.to = to_account.to_string();
                                        info.value = transfer_amount.to_string();
                                        info.value_sol =
                                            transfer_amount as f64 / LAMPORTS_PER_SOL as f64;
                                    }
                                }
                            }
                        }
                    }
                }

                // set the payment source
                let transaction_with_meta = &tx.transaction;
                if let EncodedTransaction::Json(json_tx) = &transaction_with_meta.transaction {
                    match &json_tx.message {
                        UiMessage::Parsed(parsed_msg) => {
                            if let account_keys = &parsed_msg.account_keys {
                                if let Some(fee_payer) = account_keys.get(0) {
                                    if let pubkey = &fee_payer.pubkey {
                                        info.fee_payer = pubkey.clone();
                                    }
                                }
                            }
                        }
                        UiMessage::Raw(raw_msg) => {
                            if let Some(fee_payer) = raw_msg.account_keys.get(0) {
                                info.fee_payer = fee_payer.to_string();
                            }
                        }
                    }
                }
                match &meta.loaded_addresses {
                    OptionSerializer::Some(loaded_addresses) => {
                        info.writable_accounts = loaded_addresses
                            .writable
                            .iter()
                            .map(|acc| acc.to_string())
                            .collect();
                        info.readonly_accounts = loaded_addresses
                            .readonly
                            .iter()
                            .map(|acc| acc.to_string())
                            .collect();
                    }
                    _ => {
                        info.writable_accounts = Vec::new();
                        info.readonly_accounts = Vec::new();
                    }
                }
                // Collect all involved accounts
                let mut all_accounts = Vec::new();
                all_accounts.extend(info.writable_accounts.clone());
                all_accounts.extend(info.readonly_accounts.clone());
                all_accounts.dedup();
                info.involved_accounts = all_accounts;
            }
        }
    }

    /// parse token transactions
    fn parse_token_transactions(
        info: &mut TransactionInfo,
        meta: &solana_transaction_status::UiTransactionStatusMeta,
    ) {
        // Check token balance changes
        if let (pre_token_balances, post_token_balances) =
            (&meta.pre_token_balances, &meta.post_token_balances)
        {
            match (pre_token_balances, post_token_balances) {
                (OptionSerializer::Some(pre_balances), OptionSerializer::Some(post_balances)) => {
                    if !pre_balances.is_empty() || !post_balances.is_empty() {
                        info.transaction_type = "token_transfer".to_string();
                        // check token transfer details
                        Self::check_token_transfers(info, pre_balances, post_balances);
                        // check nft transfer
                        Self::check_nft_transfer(info, pre_balances, post_balances);
                        // check dex transaction
                        Self::check_dex_transaction(info, meta);
                    }
                }
                _ => {}
            }
        }
    }

    // check token transfers
    fn check_token_transfers(
        info: &mut TransactionInfo,
        pre_balances: &[UiTransactionTokenBalance],
        post_balances: &[UiTransactionTokenBalance],
    ) {
        for pre_balance in pre_balances {
            if let Some(post_balance) = post_balances
                .iter()
                .find(|pb| pb.mint == pre_balance.mint && pb.owner == pre_balance.owner)
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

                if pre_amount != post_amount {
                    info.pre_token_balances.push(TokenBalance {
                        account_index: pre_balance.account_index,
                        mint: pre_balance.mint.clone(),
                        owner: pre_balance.owner.clone().unwrap_or("".to_string()),
                        ui_token_amount: UiTokenAmount {
                            ui_amount: pre_balance.ui_token_amount.ui_amount,
                            decimals: pre_balance.ui_token_amount.decimals,
                            amount: pre_balance.ui_token_amount.amount.clone(),
                            ui_amount_string: Some(
                                pre_balance.ui_token_amount.ui_amount_string.clone(),
                            ),
                        },
                    });
                    info.post_token_balances.push(TokenBalance {
                        account_index: post_balance.account_index,
                        mint: post_balance.mint.clone(),
                        owner: post_balance.owner.clone().unwrap_or("".to_string()),
                        ui_token_amount: UiTokenAmount {
                            ui_amount: post_balance.ui_token_amount.ui_amount,
                            decimals: post_balance.ui_token_amount.decimals,
                            amount: post_balance.ui_token_amount.amount.clone(),
                            ui_amount_string: Some(
                                post_balance.ui_token_amount.ui_amount_string.clone(),
                            ),
                        },
                    });
                }
            }
        }
    }

    // check nft transfer
    fn check_nft_transfer(
        info: &mut TransactionInfo,
        pre_balances: &[UiTransactionTokenBalance],
        post_balances: &[UiTransactionTokenBalance],
    ) {
        for balance in pre_balances.iter().chain(post_balances) {
            if balance.ui_token_amount.decimals == 0 {
                let amount = balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);
                if amount == 1 {
                    info.is_nft_transfer = true;
                    info.nft_mint = Some(balance.mint.clone());
                    info.transaction_type = "nft_transfer".to_string();
                    break;
                }
            }
        }
    }

    /// check dex transaction
    fn check_dex_transaction(
        info: &mut TransactionInfo,
        meta: &solana_transaction_status::UiTransactionStatusMeta,
    ) {
        if let logs = &meta.log_messages {
            let dex_keywords = [
                "Buy",
                "buy",
                "Sell",
                "sell",
                "swap",
                "Swap",
                "liquidity",
                "Liquidity",
                "pool",
                "Pool",
                "raydium",
                "Raydium",
                "orca",
                "Orca",
                "serum",
                "Serum",
                "market",
                "Market",
                "trade",
                "Trade",
                "Pump",
                "pump",
                "Pumpswap",
                "pumpswap",
                "pump.fun",
                "Pump.fun",
                "meteora",
                "Meteora",
            ];
            // dex
            for log in logs.clone().unwrap_or(vec![]) {
                if dex_keywords.iter().any(|&keyword| log.contains(keyword)) {
                    if (!info.is_swap) {
                        info.is_swap = true;
                    }
                }
            }
            for log in logs.clone().unwrap_or(vec![]) {
                // raydium
                if log.contains(RAYDIUM_V4_POOL_PROGRAM_ID)
                    || log.contains(RAYDIUM_CPMM_POOL_PROGRAM_ID)
                    || log.contains(RAYDIUM_CLMM_POOL_PROGRAM_ID)
                {
                    info.dex_program_name = Some("raydium".to_string());
                    // pool
                    for log in logs.clone().unwrap_or(vec![]) {
                        if log.contains(RAYDIUM_V4_POOL_PROGRAM_ID) {
                            info.dex_program_id = Some(RAYDIUM_V4_POOL_PROGRAM_ID.to_string());
                            info.dex_pool_program_id = Some(RAYDIUM_V4_POOL_PROGRAM_ID.to_string());
                            info.dex_pool_program_name = Some("raydium-v4-pool".to_string());
                            info.transaction_type = "swap".to_string();
                            for log in logs.clone().unwrap_or(vec![]) {
                                if (log.contains("MintTo")) {
                                    info.transaction_type = "addLiquidity".to_string();
                                }
                                if (log.contains("Burn")) {
                                    info.transaction_type = "removeLiquidity".to_string();
                                }
                            }
                        }
                        if log.contains(RAYDIUM_CPMM_POOL_PROGRAM_ID) {
                            info.dex_program_id = Some(RAYDIUM_CPMM_POOL_PROGRAM_ID.to_string());
                            info.dex_pool_program_id =
                                Some(RAYDIUM_CPMM_POOL_PROGRAM_ID.to_string());
                            info.dex_pool_program_name = Some("raydium-cpmm-pool".to_string());
                            info.transaction_type = "swap".to_string();
                            for log in logs.clone().unwrap_or(vec![]) {
                                if (log.contains("MintTo")) {
                                    info.transaction_type = "addLiquidity".to_string();
                                }
                                if (log.contains("Burn")) {
                                    info.transaction_type = "removeLiquidity".to_string();
                                }
                            }
                        }
                        if log.contains(RAYDIUM_CLMM_POOL_PROGRAM_ID) {
                            info.dex_program_id = Some(RAYDIUM_CLMM_POOL_PROGRAM_ID.to_string());
                            info.dex_pool_program_id =
                                Some(RAYDIUM_CLMM_POOL_PROGRAM_ID.to_string());
                            info.dex_pool_program_name = Some("raydium-clmm-pool".to_string());
                            info.transaction_type = "swap".to_string();
                            for log in logs.clone().unwrap_or(vec![]) {
                                if (log.contains("IncreaseLiquidityV2")) {
                                    info.transaction_type = "addLiquidity".to_string();
                                }
                                if (log.contains("Burn")) {
                                    info.transaction_type = "removeLiquidity".to_string();
                                }
                            }
                        }
                    }
                    return;
                }
                if log.contains(METEORA_DAMM_V2_PROGRAM_ID)
                    || log.contains(METEORA_DLMM_V2_PROGRAM_ID)
                    || log.contains(METEORA_POOL_PROGRAM_ID)
                {
                    info.dex_program_name = Some("meteora".to_string());
                    info.transaction_type = "swap".to_string();
                    // pool
                    for log in logs.clone().unwrap_or(vec![]) {
                        if log.contains(METEORA_DAMM_V2_PROGRAM_ID) {
                            info.dex_program_id = Some(METEORA_DAMM_V2_PROGRAM_ID.to_string());
                            info.dex_pool_program_id = Some(METEORA_DAMM_V2_PROGRAM_ID.to_string());
                            info.dex_pool_program_name = Some("meteora-damm-v2-pool".to_string());
                            info.transaction_type = "swap".to_string();
                            for log in logs.clone().unwrap_or(vec![]) {
                                if (log.contains("AddLiquidity")) {
                                    info.transaction_type = "addLiquidity".to_string();
                                }
                                if (log.contains("RemoveLiquidity")) {
                                    info.transaction_type = "removeLiquidity".to_string();
                                }
                            }
                        }
                    }
                    for log in logs.clone().unwrap_or(vec![]) {
                        if log.contains(METEORA_DLMM_V2_PROGRAM_ID) {
                            info.dex_program_id = Some(METEORA_DLMM_V2_PROGRAM_ID.to_string());
                            info.dex_pool_program_id = Some(METEORA_DLMM_V2_PROGRAM_ID.to_string());
                            info.dex_pool_program_name = Some("meteora-dlmm-v2-pool".to_string());
                            info.transaction_type = "swap".to_string();
                        }
                    }
                    for log in logs.clone().unwrap_or(vec![]) {
                        if log.contains(METEORA_POOL_PROGRAM_ID) {
                            info.dex_program_id = Some(METEORA_POOL_PROGRAM_ID.to_string());
                            info.dex_pool_program_id = Some(METEORA_POOL_PROGRAM_ID.to_string());
                            info.dex_pool_program_name = Some("meteora-pool".to_string());
                            info.transaction_type = "swap".to_string();
                            for log in logs.clone().unwrap_or(vec![]) {
                                if (log.contains("AddBalanceLiquidity")) {
                                    info.transaction_type = "addLiquidity".to_string();
                                }
                                if (log.contains("RemoveBalanceLiquidity")) {
                                    info.transaction_type = "removeLiquidity".to_string();
                                }
                            }
                        }
                    }
                    return;
                }
                if log.contains(ORCA_WHIRLPOOLS_PROGRAM_ID) {
                    info.dex_program_name = Some("orca".to_string());
                    for log in logs.clone().unwrap_or(vec![]) {
                        if log.contains(ORCA_WHIRLPOOLS_PROGRAM_ID) {
                            info.dex_program_id = Some(ORCA_WHIRLPOOLS_PROGRAM_ID.to_string());
                            info.dex_pool_program_id = Some(ORCA_WHIRLPOOLS_PROGRAM_ID.to_string());
                            info.dex_pool_program_name = Some("orca-whirl-pools".to_string());
                            info.transaction_type = "swap".to_string();
                            for log in logs.clone().unwrap_or(vec![]) {
                                if (log.contains("IncreaseLiquidity")) {
                                    info.transaction_type = "addLiquidity".to_string();
                                }
                                if (log.contains("DecreaseLiquidity")) {
                                    info.transaction_type = "removeLiquidity".to_string();
                                }
                            }
                        }
                    }
                    return;
                }
            }
            // pump
            let pump_keywords = [
                "Buy",
                "buy",
                "Sell",
                "sell",
                "swap",
                "Swap",
                "liquidity",
                "Liquidity",
                "pool",
                "Pool",
                "Pump",
                "pump",
                "Pumpswap",
                "pumpswap",
                "pump.fun",
                "Pump.fun",
            ];
            for log in logs.clone().unwrap_or(vec![]) {
                if pump_keywords.iter().any(|&keyword| log.contains(keyword)) {
                    if (!info.is_swap) {
                        info.is_swap = true;
                    }
                }
            }
            for log in logs.clone().unwrap_or(vec![]) {
                if log.contains(PUMP_AAM_PROGRAM_ID) {
                    info.dex_program_id = Some(PUMP_AAM_PROGRAM_ID.to_string());
                    info.dex_program_name = Some("pump-aam".to_string());
                    info.transaction_type = "swap".to_string();
                    let mut deposit: bool = false;
                    let mut mintTo: bool = false;
                    let mut burn: bool = false;
                    let mut withdraw: bool = false;
                    for log in logs.clone().unwrap_or(vec![]) {
                        if (log.contains("Instruction: Deposit")) {
                            deposit = true;
                        }
                        if (log.contains("Instruction: MintTo")) {
                            mintTo = true
                        }
                        if (log.contains("Instruction: Burn")) {
                            burn = true;
                        }
                        if (log.contains("Instruction: Withdraw")) {
                            withdraw = true
                        }
                    }
                    if (deposit && mintTo) {
                        info.transaction_type = "addLiquidity".to_string();
                    }
                    if (burn && withdraw) {
                        info.transaction_type = "removeLiquidity".to_string();
                    }
                    return;
                }
                if log.contains(PUMP_BOND_CURVE_PROGRAM_ID) {
                    info.dex_program_id = Some(PUMP_BOND_CURVE_PROGRAM_ID.to_string());
                    info.dex_program_name = Some("pump-bond-curve".to_string());
                    info.transaction_type = "swap".to_string();
                    return;
                }
            }
        }
    }

    fn extract_transfer_info(
        info: &mut TransactionInfo,
        parsed_msg: &solana_transaction_status::UiParsedMessage,
    ) {
        // Find system transfer instructions
        if let instructions = &parsed_msg.instructions {
            for instruction in instructions {
                match instruction {
                    solana_transaction_status::UiInstruction::Parsed(parsed_inst) => {
                        match parsed_inst {
                            solana_transaction_status::UiParsedInstruction::Parsed(
                                parsed_instruction,
                            ) => {
                                if parsed_instruction.program_id == "system" {
                                    if let serde_json::Value::Object(parsed_obj) =
                                        &parsed_instruction.parsed
                                    {
                                        if let Some(serde_json::Value::String(type_str)) =
                                            parsed_obj.get("type")
                                        {
                                            if type_str == "transfer" {
                                                Self::extract_parsed_transfer_info(
                                                    info, parsed_obj,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    solana_transaction_status::UiInstruction::Compiled(compiled_inst) => {
                        if compiled_inst.program_id_index == 0 {
                            if let Some(transfer_info) =
                                Self::extract_compiled_transfer_info(compiled_inst, parsed_msg)
                            {
                                info.transaction_type = "transfer".to_string();
                                info.program_id = "system".to_string();
                                info.from = transfer_info.from;
                                info.to = transfer_info.to;
                                info.value = transfer_info.amount.to_string();
                                info.value_sol =
                                    transfer_info.amount as f64 / LAMPORTS_PER_SOL as f64;
                            }
                        }
                    }
                }
            }
        }
    }

    // extract transfer information from parsed instructions
    fn extract_parsed_transfer_info(
        info: &mut TransactionInfo,
        parsed_obj: &serde_json::Map<String, serde_json::Value>,
    ) {
        info.transaction_type = "transfer".to_string();
        info.program_id = "system".to_string();
        if let Some(serde_json::Value::Object(info_obj)) = parsed_obj.get("info") {
            if let (Some(from), Some(to), Some(lamports)) = (
                info_obj.get("source").and_then(|v| v.as_str()),
                info_obj.get("destination").and_then(|v| v.as_str()),
                info_obj.get("lamports").and_then(|v| v.as_u64()),
            ) {
                info.from = from.to_string();
                info.to = to.to_string();
                info.value = lamports.to_string();
                info.value_sol = lamports as f64 / LAMPORTS_PER_SOL as f64;
            }
        }
    }

    // extract transfer information from compilation instructions
    fn extract_compiled_transfer_info(
        compiled_inst: &solana_transaction_status::UiCompiledInstruction,
        parsed_msg: &solana_transaction_status::UiParsedMessage,
    ) -> Option<CompiledTransferInfo> {
        // System transfer command data format:
        // First 4 bytes: Command identifier (2 indicates transfer)
        // Last 8 bytes: Lamports count
        let data = &compiled_inst.data;
        if data.len() >= 12 {
            let instruction_id = u32::from_le_bytes([
                data.as_bytes()[0],
                data.as_bytes()[1],
                data.as_bytes()[2],
                data.as_bytes()[3],
            ]);
            if instruction_id == 2 {
                let lamports = u64::from_le_bytes([
                    data.as_bytes()[4],
                    data.as_bytes()[5],
                    data.as_bytes()[6],
                    data.as_bytes()[7],
                    data.as_bytes()[8],
                    data.as_bytes()[9],
                    data.as_bytes()[10],
                    data.as_bytes()[11],
                ]);
                if let (Some(from_index), Some(to_index)) =
                    (compiled_inst.accounts.get(0), compiled_inst.accounts.get(1))
                {
                    if let account_keys = &parsed_msg.account_keys {
                        if let (Some(from_acc), Some(to_acc)) = (
                            account_keys.get(*from_index as usize),
                            account_keys.get(*to_index as usize),
                        ) {
                            if let (from_pubkey, to_pubkey) = (&from_acc.pubkey, &to_acc.pubkey) {
                                return Some(CompiledTransferInfo {
                                    from: from_pubkey.clone(),
                                    to: to_pubkey.clone(),
                                    amount: lamports,
                                });
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn extract_program_id_from_ui_instruction(
        inst: &solana_transaction_status::UiInstruction,
    ) -> String {
        match inst {
            solana_transaction_status::UiInstruction::Compiled(compiled) => {
                compiled.program_id_index.to_string()
            }
            solana_transaction_status::UiInstruction::Parsed(parsed) => match parsed {
                UiParsedInstruction::Parsed(parsed_instruction) => {
                    parsed_instruction.program_id.clone()
                }
                UiParsedInstruction::PartiallyDecoded(ui_partially_decoded_instruction) => {
                    ui_partially_decoded_instruction.program_id.clone()
                }
            },
        }
    }

    fn extract_accounts_from_ui_instruction(
        inst: &solana_transaction_status::UiInstruction,
    ) -> Vec<String> {
        match inst {
            solana_transaction_status::UiInstruction::Compiled(compiled) => compiled
                .accounts
                .iter()
                .map(|idx| idx.to_string())
                .collect(),
            solana_transaction_status::UiInstruction::Parsed(parsed) => match parsed {
                solana_transaction_status::UiParsedInstruction::Parsed(parsed_instruction) => {
                    Self::extract_accounts_from_parsed_value(&parsed_instruction.parsed)
                }
                solana_transaction_status::UiParsedInstruction::PartiallyDecoded(
                    partially_decoded,
                ) => partially_decoded
                    .accounts
                    .iter()
                    .map(|acc| acc.to_string())
                    .collect(),
            },
        }
    }

    fn extract_accounts_from_parsed_value(parsed: &serde_json::Value) -> Vec<String> {
        let mut accounts = Vec::new();
        if let Some(obj) = parsed.as_object() {
            if let Some(info) = obj.get("info") {
                if let Some(info_obj) = info.as_object() {
                    let account_fields = [
                        "source",
                        "destination",
                        "account",
                        "from",
                        "to",
                        "authority",
                    ];
                    for field in account_fields {
                        if let Some(account) = info_obj.get(field).and_then(|v| v.as_str()) {
                            accounts.push(account.to_string());
                        }
                    }
                }
            }
        }
        accounts
    }

    fn extract_data_from_ui_instruction(inst: &solana_transaction_status::UiInstruction) -> String {
        match inst {
            solana_transaction_status::UiInstruction::Compiled(compiled) => compiled.data.clone(),
            solana_transaction_status::UiInstruction::Parsed(parsed) => match parsed {
                solana_transaction_status::UiParsedInstruction::Parsed(parsed_instruction) => {
                    serde_json::to_string(&parsed_instruction.parsed).unwrap_or_default()
                }
                solana_transaction_status::UiParsedInstruction::PartiallyDecoded(
                    partially_decoded,
                ) => partially_decoded.data.clone(),
            },
        }
    }

    /// Determine whether the specified address in the current transaction is the recipient
    pub fn is_recipient(&self, address: &str) -> bool {
        Trade::is_address_recipient_in_transaction(self, address)
    }

    /// Determine whether the specified address in the current transaction is the payer
    pub fn is_payer(&self, address: &str) -> bool {
        Trade::is_address_payer_in_transaction(self, address)
    }

    /// Get the payment amount of the transaction (lamports)
    pub fn get_payment_amount(&self) -> u64 {
        self.value.parse::<u64>().unwrap_or(0)
    }

    /// Get the payment amount of the transaction (SOL)
    pub fn get_payment_amount_sol(&self) -> f64 {
        self.value_sol
    }
}

impl Default for TransactionInfo {
    fn default() -> Self {
        Self {
            transaction_hash: String::new(),
            signature: String::new(),
            from: String::new(),
            to: String::new(),
            fee_payer: String::new(),
            signers: Vec::new(),
            involved_accounts: Vec::new(),
            writable_accounts: Vec::new(),
            readonly_accounts: Vec::new(),
            value: "0".to_string(),
            value_sol: 0.0,
            fee: 0,
            fee_sol: 0.0,
            pre_balance: 0,
            post_balance: 0,
            balance_change: 0,
            block_number: 0,
            block_hash: String::new(),
            block_time: None,
            slot: 0,
            epoch: 0,
            recent_blockhash: String::new(),
            status: "unknown".to_string(),
            confirmation_status: "processed".to_string(),
            error_message: None,
            err: None,
            is_confirmed: false,
            is_finalized: false,
            transaction_type: "unknown".to_string(),
            program_id: String::new(),
            instructions_count: 0,
            inner_instructions_count: 0,
            version: 0,
            compute_units_consumed: None,
            compute_unit_price: None,
            log_index: 0,
            data: None,
            logs: Vec::new(),
            instructions: Vec::new(),
            inner_instructions: Vec::new(),
            token_mint: None,
            token_amount: None,
            token_decimals: None,
            token_symbol: None,
            token_name: None,
            pre_token_balances: Vec::new(),
            post_token_balances: Vec::new(),
            is_nft_transfer: false,
            nft_mint: None,
            nft_name: None,
            nft_symbol: None,
            is_swap: false,

            dex_program_id: None,        // DEX program id
            dex_program_name: None,      // DEX program name
            dex_pool_program_id: None,   // DEX program pool id
            dex_pool_program_name: None, // DEX program pool name

            input_mint: None,
            output_mint: None,
            input_amount: None,
            output_amount: None,
            memo: None,
            timestamp: None,
            tags: Vec::new(),
            category: "general".to_string(),
            risk_level: 0,
            is_internal: false,
            gas_used: None,
            gas_price: None,
            max_fee: None,
            priority_fee: None,
            cluster: String::new(),
            rpc_node: String::new(),
            created_at: 0,
            updated_at: 0,
            source: "rpc".to_string(),
            confidence: 1.0,
            direction: None,
        }
    }
}

impl TransactionInfo {
    pub fn is_successful(&self) -> bool {
        self.status == "success"
    }

    pub fn is_token_transfer(&self) -> bool {
        self.token_mint.is_some()
    }

    pub fn get_net_amount(&self) -> i64 {
        self.balance_change - self.fee as i64
    }

    pub fn is_high_value(&self) -> bool {
        self.value_sol > 1000.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    Success,
    Failed(String),
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    Transfer,
    TokenTransfer(String),
    NFTTransfer(String),
    Swap(String, String),
    ProgramInteraction(String),
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionInfo {
    pub program_id: String,
    pub accounts: Vec<String>,
    pub data: String,
    pub stack_height: Option<u32>,
    pub program: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnerInstructionInfo {
    pub index: u8,
    pub instructions: Vec<InstructionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub account_index: u8,
    pub mint: String,
    pub owner: String,
    pub ui_token_amount: UiTokenAmount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiTokenAmount {
    pub ui_amount: Option<f64>,
    pub decimals: u8,
    pub amount: String,
    pub ui_amount_string: Option<String>,
}

struct CompiledTransferInfo {
    from: String,
    to: String,
    amount: u64,
}
