<h1 align="center">
    🤵 Solana Network SDK
</h1>
<h4 align="center">
Implemented most of the commonly used practical transaction-related functions on the Solana network.
</h4>
<p align="center">
  <a href="https://github.com/0xhappyboy/solana-network-sdk/LICENSE"><img src="https://img.shields.io/badge/License-GPL3.0-d1d1f6.svg?style=flat&labelColor=1C2C2E&color=BEC5C9&logo=googledocs&label=license&logoColor=BEC5C9" alt="License"></a>
    <a href="https://crates.io/crates/solana-network-sdk">
<img src="https://img.shields.io/badge/crates-solana--network--sdk-20B2AA.svg?style=flat&labelColor=0F1F2D&color=FFD700&logo=rust&logoColor=FFD700">
</a>
</p>
<p align="center">
<a href="./README_zh-CN.md">简体中文</a> | <a href="./README.md">English</a>
</p>

# 🏗️ Depend

```shell
cargo add solana-network-sdk
```

# 📦 Example

## Create Client

```rust
use solana_network_sdk::Solana;
use solana_network_sdk::types::Mode;

#[tokio::main]
async fn main() -> Result<(), String> {
    let solana = solana_network_sdk::Solana::new(solana_network_sdk::types::Mode::MAIN)
                .map_err(|e| format!("create solane clietn error:{:?}", e))
                .unwrap()
}

```

## Trade Module

### Overview
The Trade module provides functionality to interact with the Solana blockchain, including transaction history retrieval, transaction analysis, payment relationship detection, and detailed transaction parsing.

### Trade
Main struct for trading operations.

### Methods

### new(client: Arc<RpcClient>) -> Self
Creates a new `Trade` instance with the given RPC client.

**Parameters:**
- `client`: Arc-wrapped RPC client for Solana

**Returns:**
- `Trade` instance

### estimate_fee() -> Result<u64, String>
Estimates the current transaction fee on the Solana network.

**Returns:**
- `Ok(u64)`: Estimated fee in lamports
- `Err(String)`: Error message if estimation fails

### get_transactions_history_by_cursor(address: &str, cursor: Option<String>, page_size: u32) -> UnifiedResult<(Vec<RpcConfirmedTransactionStatusWithSignature>, Option<String>), String>`
Retrieves transaction history for a specified address with pagination support.

**Parameters:**
- `address`: Wallet address as string
- `cursor`: Optional cursor for pagination (signature of last transaction)
- `page_size`: Number of transactions to retrieve per page

**Returns:**
- `Ok((Vec<RpcConfirmedTransactionStatusWithSignature>, Option<String>))`: Tuple containing transaction list and next cursor
- `Err(String)`: Error message if retrieval fails

**Example:**
```rust
let mut cursor: Option<String> = None;
loop {
    match trade
        .get_transactions_history_by_cursor(
            "wallet address",
            cursor.clone(),
            page_size,
        )
        .await
    {
        Ok(r) => {
            // r.0 is transaction history list
            if r.1.is_none() {
                break;
            }
            cursor = r.1;
        }
        Err(_) => {
            break;
        }
    }
}
```

### get_transactions_history_filtered(client: &Arc<RpcClient>, address: &str, filter: F) -> UnifiedResult<Vec<RpcConfirmedTransactionStatusWithSignature>, String>
Retrieves filtered transaction history for a specified address.

**Parameters:**
- `client`: RPC client reference
- `address`: Wallet address
- `filter`: Closure that returns `true` to keep the transaction record

**Returns:**
- `Ok(Vec<RpcConfirmedTransactionStatusWithSignature>)`: Filtered transaction list
- `Err(String)`: Error message if retrieval fails

**Example:**
```rust
let history = Trade::get_transactions_history_filtered(
    &client,
    "wallet address",
    |sig_info| {
        // return true to retain transaction information
        true
    },
).await;
```

### get_last_transactions_contains_address(address_a: &str, address_b: &str) -> UnifiedResult<Option<RpcConfirmedTransactionStatusWithSignature>, String>
Gets the last transaction record of address A that contains address B.

**Parameters:**
- `address_a`: Main query address
- `address_b`: Address to check for inclusion

**Returns:**
- `Ok(Some(RpcConfirmedTransactionStatusWithSignature))`: Last transaction containing address B
- `Ok(None)`: No transaction contains address B
- `Err(String)`: Error message

### get_transactions_vec_containing_address(address_a: &str, address_b: &str) -> UnifiedResult<Vec<RpcConfirmedTransactionStatusWithSignature>, String>
Gets all transactions of address A that contain address B.

**Parameters:**
- `address_a`: Main query address
- `address_b`: Address to include

**Returns:**
- `Ok(Vec<RpcConfirmedTransactionStatusWithSignature>)`: List of all transaction records containing address B

### get_transaction_details(signature: &str) -> UnifiedResult<EncodedConfirmedTransactionWithStatusMeta, String>
Gets detailed transaction information by signature.

**Parameters:**
- `signature`: Transaction signature hash string

**Returns:**
- `Ok(EncodedConfirmedTransactionWithStatusMeta)`: Detailed transaction information
- `Err(String)`: Error message

### get_transactions_by_recipient_and_payer(address_a: &str, address_b: &str, limit: usize) -> UnifiedResult<Vec<RpcConfirmedTransactionStatusWithSignature>, String>
Gets transactions where address A is the recipient and the transaction contains address B (loose filtering).

**Parameters:**
- `address_a`: Recipient address
- `address_b`: Payer address
- `limit`: Maximum number of transactions to return

**Returns:**
- `Ok(Vec<RpcConfirmedTransactionStatusWithSignature>)`: Matching transactions

### get_transactions_by_recipient_and_payer_strict(address_a: &str, address_b: &str, limit: usize) -> UnifiedResult<Vec<RpcConfirmedTransactionStatusWithSignature>, String>
Gets transactions where address A is the recipient and address B is the payer (strict filtering).

**Parameters:**
- `address_a`: Recipient address
- `address_b`: Payer address
- `limit`: Maximum number of transactions to return

**Returns:**
- `Ok(Vec<RpcConfirmedTransactionStatusWithSignature>)`: Confirmed transactions

### has_payment_relationship(address_a: &str, address_b: &str) -> UnifiedResult<Option<String>, String>
Quickly checks if there is a payment relationship between two addresses (address B pays address A).

**Parameters:**
- `address_a`: Recipient address
- `address_b`: Payer address

**Returns:**
- `Ok(Some(String))`: Transaction signature if payment relationship exists
- `Ok(None)`: No payment relationship
- `Err(String)`: Error message

### get_total_payment_amount(address_a: &str, address_b: &str, time_range: Option<u64>) -> UnifiedResult<u64, String>
Gets the total amount paid by address B to address A.

**Parameters:**
- `address_a`: Recipient address
- `address_b`: Payer address
- `time_range`: Time range in seconds (None means all time)

**Returns:**
- `Ok(u64)`: Total payment amount in lamports
- `Err(String)`: Error message

### is_transaction_contains_address(signature: &str, target_address: &str) -> bool (private)
Checks whether a single transaction contains a specified address.

### TransactionInfo
A more readable transaction information structure.

### Key Methods:
- `from_encoded_transaction()`: Creates from encoded transaction
- `is_recipient()`: Checks if address is recipient
- `is_payer()`: Checks if address is payer
- `get_payment_amount()`: Gets payment amount in lamports
- `get_payment_amount_sol()`: Gets payment amount in SOL
- `is_successful()`: Checks if transaction was successful
- `is_token_transfer()`: Checks if token transfer
- `get_net_amount()`: Gets net amount after fees
- `is_high_value()`: Checks if high-value transaction
