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

The Trade module provides functionality for interacting with the Solana blockchain, including obtaining transaction history, analyzing transaction details, and checking address relationships.

### Get the actual token addresses and amount added and removed during the transaction.

```rust
#[tokio::test]
async fn a() {
    let solana = Solana::new(Mode::MAIN);
    let trade = solana.unwrap().create_trade();
    let t2: Result<TransactionInfo, UnifiedError<String>> = trade.get_transaction_display_details(
        "CLoekmTsTYyFgHLEj7YE1GMycHHLhxE6KB49tQgHF98pVCzEh7WaYXGaSUNjnZ12Zi2JQcB8kgP27mkx9PoKUQK",
    ).await;
    let increase = t2.as_ref().unwrap().get_received_token_sol();
    println!("increase :{:?} ", increase); // increase.0 = EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v; increase.1 = 48.655907
    let reduce = t2.as_ref().unwrap().get_spent_token_sol();
    println!("reduce :{:?} ", reduce); // reduce.0 = EhzVcKKmGjLk6pD5gLT6ZrTg62bMgPgTSCXXmANnSyQA; reduce.1 = 6444.329826091
}
```

### Retrieve base/quote tokens from the liquidity pool specified in the signature.

```rust
#[tokio::test]
async fn a() {
    let solana = Solana::new(Mode::MAIN);
    let trade = solana.unwrap().create_trade();
    let t2: Result<TransactionInfo, UnifiedError<String>> = trade.get_transaction_display_details(
        "CLoekmTsTYyFgHLEj7YE1GMycHHLhxE6KB49tQgHF98pVCzEh7WaYXGaSUNjnZ12Zi2JQcB8kgP27mkx9PoKUQK",
    ).await;
    println!("Liquidity Pool Base Token Address :{:?}", t2.as_ref().unwrap().get_pool_left_address()); // EhzVcKKmGjLk6pD5gLT6ZrTg62bMgPgTSCXXmANnSyQA
    println!("Liquidity Pool Base Token Amount :{:?}", t2.as_ref().unwrap().get_pool_left_amount_sol()); // 6444.329826091
    println!("Liquidity Pool Quote Token Address :{:?}", t2.as_ref().unwrap().get_pool_right_address()); // EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
    println!("Liquidity Pool Quote Token Amount :{:?}", t2.as_ref().unwrap().get_pool_right_amount_sol()); // 48.655907
}
```

### Get the actual token addresses and amount added and removed during the pump.fun bond curve transaction.

```rust
#[tokio::test]
async fn a() {
    let solana = Solana::new(Mode::MAIN);
    let trade = solana.unwrap().create_trade();
    let t3: Result<TransactionInfo, UnifiedError<String>> = trae.get_transaction_display_details(
        "5cCVC1KMfaC1QLYeuwuSL5eQQZxZMn8R9rwqAxBkf8tE7FrmkzcTF7qNpaJUGFU5uyud7kr5ESsx8Tn1rUHmrAqu",
    ).await;
    let pump_bond_curve = t3.as_ref().unwrap().get_pump_bond_curve_transaction_info();
    println!("increase :{:?} ", pump_bond_curve.get_pump_received_token_sol()); // increase.0 = 2og84mzRgrM4Q1sWZAkVAhZoszb7uo6oW9SjKLj5pump; increase.1 = 3908476.581809
    println!("reduce :{:?} ", pump_bond_curve.get_pump_spent_token_sol()); // reduce.0 = So11111111111111111111111111111111111111112; reduce.1 = 0.508921875
}
```

### Estimate Transaction Fee

```rust
let solana = Solana::new(Mode::DEV).unwrap();
let trade = solana.create_trade();

match trade.estimate_fee().await {
Ok(fee) => println!("Estimated fee: {} lamports", fee),
Err(e) => eprintln!("Error estimating fee: {}", e),
}
```

### Get Transaction History with Pagination

```rust
let mut cursor: Option<String> = None;
loop {
match trade
.get_transactions_history_by_cursor(
"8MwwTfMp86sJ3b9B9W6cB3k6yLx4F5Gt2jK7N8P9Q0R",
cursor.clone(),
50,
)
.await
{
Ok((transactions, next_cursor)) => {
println!("Retrieved {} transactions", transactions.len());

            for tx in &transactions {
                println!("Signature: {}", tx.signature);
                println!("Slot: {}", tx.slot);
                println!("Status: {:?}", tx.err);
            }

            if next_cursor.is_none() {
                println!("No more transactions to fetch");
                break;
            }

            cursor = next_cursor;
            println!("Continuing with cursor: {:?}", cursor);
        }
        Err(e) => {
            eprintln!("Error fetching transactions: {}", e);
            break;
        }
    }

}
```

### Get Filtered Transaction History

```rust
let client = solana.client_arc();
let address = "8MwwTfMp86sJ3b9B9W6cB3k6yLx4F5Gt2jK7N8P9Q0R";

// Example 1: Filter by block time
let recent_transactions = Trade::get_transactions_history_filtered(
&client,
address,
|sig_info| {
// Filter transactions from the last 24 hours
if let Some(block_time) = sig_info.block_time {
let now = std::time::SystemTime::now()
.duration_since(std::time::UNIX_EPOCH)
.unwrap()
.as_secs();
now - block_time as u64 < 24 _ 60 _ 60
} else {
false
}
},
).await?;

// Example 2: Filter successful transactions only
let successful_transactions = Trade::get_transactions_history_filtered(
&client,
address,
|sig_info| sig_info.err.is_none(),
).await?;
```

### Get Last Transaction Containing Another Address

```rust
let address_a = "8MwwTfMp86sJ3b9B9W6cB3k6yLx4F5Gt2jK7N8P9Q0R";
let address_b = "B5Zg7W7L7jH6K8L9M0N1O2P3Q4R5S6T7U8V9W0X1Y2Z";

match trade.get_last_transactions_contains_address(address_a, address_b).await {
Ok(Some(transaction)) => {
println!("Found transaction containing both addresses:");
println!("Signature: {}", transaction.signature);
println!("Slot: {}", transaction.slot);
println!("Block Time: {:?}", transaction.block_time);
}
Ok(None) => println!("No transaction found containing both addresses"),
Err(e) => eprintln!("Error: {}", e),
}
```

### Get All Transactions Containing Another Address

```rust
let address_a = "8MwwTfMp86sJ3b9B9W6cB3k6yLx4F5Gt2jK7N8P9Q0R";
let address_b = "B5Zg7W7L7jH6K8L9M0N1O2P3Q4R5S6T7U8V9W0X1Y2Z";

match trade.get_transactions_vec_containing_address(address_a, address_b).await {
Ok(transactions) => {
println!("Found {} transactions containing both addresses", transactions.len());

        for tx in transactions {
            println!("- Signature: {}", tx.signature);
            println!("  Slot: {}", tx.slot);
            println!("  Status: {}", if tx.err.is_none() { "Success" } else { "Failed" });
        }
    }
    Err(e) => eprintln!("Error: {}", e),

}
```

### Get Transaction Details

```rust
let signature = "5h6xBEauJ3PK6SWZrW5M4Q7GjS2eX2jGqKJ8H9i0K1L2M3N4O5P6Q7R8S9T0U1V2W3X4Y5Z6A7B8C9D0";

match trade.get_transaction_details(signature).await {
Ok(transaction) => {
let tx_info = TransactionInfo::from_encoded_transaction(
&transaction,
signature,
"mainnet"
);

        println!("Transaction Details:");
        println!("From: {}", tx_info.from);
        println!("To: {}", tx_info.to);
        println!("Amount: {} SOL", tx_info.value_sol);
        println!("Fee: {} lamports", tx_info.fee);
        println!("Status: {}", tx_info.status);
        println!("Type: {}", tx_info.transaction_type);
    }
    Err(e) => eprintln!("Error getting transaction details: {}", e),

}
```

### Get Transactions by Recipient and Payer (Loose)

```rust
let recipient = "8MwwTfMp86sJ3b9B9W6cB3k6yLx4F5Gt2jK7N8P9Q0R";
let payer = "B5Zg7W7L7jH6K8L9M0N1O2P3Q4R5S6T7U8V9W0X1Y2Z";
let limit = 10;

match trade.get_transactions_by_recipient_and_payer(recipient, payer, limit).await {
Ok(transactions) => {
println!("Found {} transactions where {} is recipient and {} is involved",
transactions.len(), recipient, payer);

        for tx in transactions {
            println!("Signature: {}", tx.signature);
        }
    }
    Err(e) => eprintln!("Error: {}", e),

}
```

### Get Transactions by Recipient and Payer (Strict)

```rust
let recipient = "8MwwTfMp86sJ3b9B9W6cB3k6yLx4F5Gt2jK7N8P9Q0R";
let payer = "B5Zg7W7L7jH6K8L9M0N1O2P3Q4R5S6T7U8V9W0X1Y2Z";
let limit = 10;

match trade.get_transactions_by_recipient_and_payer_strict(recipient, payer, limit).await {
Ok(transactions) => {
println!("Found {} transactions where {} is recipient and {} is payer",
transactions.len(), recipient, payer);

        for tx in transactions {
            match trade.get_transaction_details(&tx.signature).await {
                Ok(details) => {
                    let tx_info = TransactionInfo::from_encoded_transaction(
                        &details,
                        &tx.signature,
                        "mainnet"
                    );
                    println!("- {}: {} SOL", tx.signature, tx_info.value_sol);
                }
                Err(_) => println!("- {}: Could not fetch details", tx.signature),
            }
        }
    }
    Err(e) => eprintln!("Error: {}", e),

}
```

### Check Payment Relationship

```rust
let recipient = "8MwwTfMp86sJ3b9B9W6cB3k6yLx4F5Gt2jK7N8P9Q0R";
let payer = "B5Zg7W7L7jH6K8L9M0N1O2P3Q4R5S6T7U8V9W0X1Y2Z";

match trade.has_payment_relationship(recipient, payer).await {
Ok(Some(signature)) => {
println!("Payment relationship found! Transaction signature: {}", signature);
}
Ok(None) => {
println!("No payment relationship found between {} and {}", recipient, payer);
}
Err(e) => eprintln!("Error checking payment relationship: {}", e),
}
```

### Get Total Payment Amount

```rust
let recipient = "8MwwTfMp86sJ3b9B9W6cB3k6yLx4F5Gt2jK7N8P9Q0R";
let payer = "B5Zg7W7L7jH6K8L9M0N1O2P3Q4R5S6T7U8V9W0X1Y2Z";

// Get total amount for all time
match trade.get_total_payment_amount(recipient, payer, None).await {
Ok(total_amount) => {
println!("Total amount paid: {} lamports ({:.4} SOL)",
total_amount, total_amount as f64 / LAMPORTS_PER_SOL as f64);
}
Err(e) => eprintln!("Error: {}", e),
}

// Get total amount for last 7 days
let seven_days = Some(7 _ 24 _ 60 * 60);
match trade.get_total_payment_amount(recipient, payer, seven_days).await {
Ok(total_amount) => {
println!("Amount paid in last 7 days: {} lamports", total_amount);
}
Err(e) => eprintln!("Error: {}", e),
}
```

### TransactionInfo Helper Methods

```rust
// After obtaining a TransactionInfo object
let signature = "5h6xBEauJ3PK6SWZrW5M4Q7GjS2eX2jGqKJ8H9i0K1L2M3N4O5P6Q7R8S9T0U1V2W3X4Y5Z6A7B8C9D0";
match trade.get_transaction_details(signature).await {
Ok(transaction) => {
let tx_info = TransactionInfo::from_encoded_transaction(
&transaction,
signature,
"mainnet"
);

        // Check if transaction was successful
        if tx_info.is_successful() {
            println!("Transaction was successful");
        }

        // Check if it's a token transfer
        if tx_info.is_token_transfer() {
            println!("This is a token transfer");
            if let Some(mint) = &tx_info.token_mint {
                println!("Token mint: {}", mint);
            }
        }

        // Check if it's high value
        if tx_info.is_high_value() {
            println!("High value transaction detected!");
        }

        // Check if specific address is recipient
        let address_to_check = "8MwwTfMp86sJ3b9B9W6cB3k6yLx4F5Gt2jK7N8P9Q0R";
        if tx_info.is_recipient(address_to_check) {
            println!("{} is the recipient in this transaction", address_to_check);
        }

        // Check if specific address is payer
        if tx_info.is_payer(address_to_check) {
            println!("{} is the payer in this transaction", address_to_check);
        }

        // Get payment amount
        println!("Payment amount: {} lamports", tx_info.get_payment_amount());
        println!("Payment amount: {} SOL", tx_info.get_payment_amount_sol());

        // Get net amount (balance change minus fees)
        println!("Net amount change: {}", tx_info.get_net_amount());
    }
    Err(e) => eprintln!("Error: {}", e),

}
```

### Analyze Address Relationships

```rust
async fn analyze_address_relationships(
trade: &Trade,
address1: &str,
address2: &str,
) -> Result<(), String> {
println!("Analyzing relationship between {} and {}", address1, address2);

    // 1. Check if there's any payment relationship
    match trade.has_payment_relationship(address1, address2).await {
        Ok(Some(signature)) => {
            println!("Found payment from {} to {}", address2, address1);
            println!("Transaction: {}", signature);
        }
        Ok(None) => println!("No direct payment found from {} to {}", address2, address1),
        Err(e) => eprintln!("Error: {}", e),
    }

    // 2. Check reverse relationship
    match trade.has_payment_relationship(address2, address1).await {
        Ok(Some(signature)) => {
            println!("Found payment from {} to {}", address1, address2);
            println!("Transaction: {}", signature);
        }
        Ok(None) => println!("No direct payment found from {} to {}", address1, address2),
        Err(e) => eprintln!("Error: {}", e),
    }

    // 3. Get all transactions between them
    let transactions_a = trade.get_transactions_vec_containing_address(address1, address2).await?;
    let transactions_b = trade.get_transactions_vec_containing_address(address2, address1).await?;

    println!("Total transactions involving both addresses: {}",
             transactions_a.len() + transactions_b.len());

    // 4. Calculate total amounts
    let total_from_2_to_1 = trade.get_total_payment_amount(address1, address2, None).await?;
    let total_from_1_to_2 = trade.get_total_payment_amount(address2, address1, None).await?;

    println!("Total from {} to {}: {:.4} SOL",
             address2, address1,
             total_from_2_to_1 as f64 / LAMPORTS_PER_SOL as f64);
    println!("Total from {} to {}: {:.4} SOL",
             address1, address2,
             total_from_1_to_2 as f64 / LAMPORTS_PER_SOL as f64);

    Ok(())

}
```

## Scan Module

### Get All Historical Signatures

Fetches ALL historical transaction signatures for a given address using pagination.

```rust
use solana_network_sdk::Solana;
use solana_network_sdk::types::Mode;

#[tokio::main]
async fn main() -> Result<(), String> {
    let solana = Solana::new(Mode::MAIN).unwrap();
    let client = solana.client_arc();
    let scan = solana_network_sdk::scan::Scan::new(client.clone());
    // Fetch all historical signatures for USDC address
    let signatures = scan.get_all_signatures_by_address(
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC token address
        Some(300),  // 300ms delay between requests
        Some(500),  // 500 signatures per batch
    ).await?;
    println!("Total signatures: {}", signatures.len());
    // Print first 5 signatures
    for (i, sig) in signatures.iter().take(5).enumerate() {
        println!("{}. {}", i + 1, sig);
    }
    Ok(())
}
```

**Parameters:**

- `address: &str` - Base58-encoded Solana address
- `interval_time: Option<u64>` - Delay in milliseconds between RPC calls (default: 200)
- `batch_size: Option<u64>` - Number of signatures per RPC call (default: 1000)

**Returns:** `Result<Vec<String>, String>`

### Get Limited Number of Signatures

Fetches a specific number of transaction signatures with safety mechanisms.

```rust
use solana_network_sdk::Solana;
use solana_network_sdk::types::Mode;

#[tokio::main]
async fn main() -> Result<(), String> {
    let solana = Solana::new(Mode::MAIN).unwrap();
    let client = solana.client_arc();
    let scan = solana_network_sdk::scan::Scan::new(client.clone());
    // Fetch up to 50 signatures with rate limiting
    let signatures = scan.get_signatures_with_limit(
        "Vote111111111111111111111111111111111111111", // Solana vote program
        50,         // Maximum 50 signatures
        Some(100),  // 100ms delay between requests
    ).await?;
    println!("Retrieved {} signatures:", signatures.len());
    for sig in &signatures {
        println!("  - {}", sig);
    }
    Ok(())
}
```

**Parameters:**

- `address: &str` - Base58-encoded Solana address
- `limit: usize` - Maximum number of signatures to return
- `interval_time: Option<u64>` - Delay in milliseconds between RPC calls (default: 200)

**Returns:** `Result<Vec<String>, String>`

### Get Latest Signatures

Quickly fetches the most recent transaction signatures without pagination.

```rust
use solana_network_sdk::Solana;
use solana_network_sdk::types::Mode;

#[tokio::main]
async fn main() -> Result<(), String> {
    let solana = Solana::new(Mode::MAIN).unwrap();
    let client = solana.client_arc();
    let scan = solana_network_sdk::scan::Scan::new(client.clone());
    // Fetch the 10 most recent signatures
    let signatures = scan.get_last_signatures(
        "So11111111111111111111111111111111111111112", // SOL token address
        10,  // Number of recent signatures
    ).await?;
    println!("Latest 10 signatures:");
    for (i, sig) in signatures.iter().enumerate() {
        println!("{}. {}", i + 1, sig);
    }
    Ok(())
}
```

**Parameters:**

- `address: &str` - Base58-encoded Solana address
- `count: usize` - Number of recent signatures to return

**Returns:** `Result<Vec<String>, String>`

### Process Token Address Signatures

```rust
use solana_network_sdk::Solana;
use solana_network_sdk::types::Mode;

#[tokio::main]
async fn main() -> Result<(), String> {
    let solana = Solana::new(Mode::MAIN).unwrap();
    let client = solana.client_arc();
    let scan = solana_network_sdk::scan::Scan::new(client.clone());
    let trade = solana_network_sdk::trade::Trade::new(client.clone());
    let token_address = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // USDC
    // Step 1: Get latest 5 signatures for quick analysis
    println!("Fetching latest signatures for {}...", token_address);
    let latest_signatures = scan.get_last_signatures(token_address, 5).await?;
    // Step 2: Get detailed transaction information for each signature
    for (i, signature) in latest_signatures.iter().enumerate() {
        println!("\n{}. Processing signature: {}", i + 1, signature);
        match trade.get_transaction_details(signature).await {
            Ok(transaction) => {
                let tx_info = solana_network_sdk::trade::TransactionInfo::from_encoded_transaction(
                    &transaction,
                    signature,
                    "mainnet"
                );
                println!("   Slot: {}", tx_info.slot);
                println!("   Status: {}", tx_info.status);
                println!("   Fee: {} lamports", tx_info.fee);
            }
            Err(e) => println!("   Error fetching details: {}", e),
        }
    }
    // Step 3: Get all historical signatures (paginated, for offline processing)
    println!("\nFetching all historical signatures (this may take a while)...");
    let all_signatures = scan.get_all_signatures_by_address(
        token_address,
        Some(200),  // 200ms delay
        Some(1000), // 1000 per batch
    ).await?;
    println!("Total historical signatures: {}", all_signatures.len());
    Ok(())
}
```
