<h1 align="center">
    🤵 Solana Network SDK
</h1>
<h4 align="center">
实现了 Solana 网络上大部分常用的实用交易相关功能.
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

# 🏗️ 依赖

```shell
cargo add solana-network-sdk
```

# 📦 案例

## 创建客户端

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

## 批量扫描指定地址中的所有签名交易.

```rust
#[cfg(test)]
mod tests {
    use crate::Solana;
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn test_get_all_signatures_by_address_and_batch_find_transaction() -> Result<(), ()> {
        let solana = Solana::new(crate::types::Mode::MAIN).unwrap();
        let scan = Arc::new(solana.create_scan());
        scan.fetch_all_transactions_by_address(
            "CzVqatmaK6GfyEWZUcWromDvpq3MFxqSrUweZgbjHngh",
            Some(100),
            Some(100),
            Some(10),
            async |trades| {
                for trade in trades {
                    trade.display().await;
                }
            },
        )
        .await;
        Ok(())
    }
}
```

## 批量查询交易信息.

```rust
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
        println!("Batch Query Results: {:?}", trade_infos);
        println!("Batch Query Results Count: {:?}", trade_infos.len());
        Ok(())
    }
}
```

## 获取指定交易中代币与报价代币之间($sol)的比率.

```rust
#[cfg(test)]
mod tests {
    use crate::Solana;

    #[tokio::test]
    async fn test_get_token_quote_ratio() -> Result<(), ()> {
        let solana = Solana::new(crate::types::Mode::MAIN).unwrap();
        let trade = solana.create_trade();
        let t_info = trade.get_transaction_display_details("5ChbVDpaKdmKDVTc4tAPa7NHDR3rS31cxTH6ZJWpjZbmRRAYPsxXNLGxXJkvMXNjbKhAvrUmYFUTCtxbRyerfxF1").await.unwrap();
        println!(
            "Quote Token Ratio: {}",
            t_info.get_token_quote_ratio().unwrap()
        );
        Ok(())
    }
}
```

## 监听最新区块中最新的所有交易.

```rust
#[cfg(test)]
mod tests {
    use crate::Solana;

    use super::*;

    #[tokio::test]
    async fn test_poll_latest_block() {
        let solana = Solana::new(crate::types::Mode::MAIN).unwrap();
        let service = BlockService::new(solana.client_arc());
        let trade = solana.create_trade();
        service
            .poll_latest_block(async |block_info| match block_info {
                Some(info) => {
                    for sig in info.transaction_signatures {
                        println!("Signature: {:?}", sig);
                        let t = trade
                            .get_transaction_display_details(&format!("{:?}", sig))
                            .await
                            .unwrap();
                        let pump_t = t.get_pump_bond_curve_transaction_info();
                        println!("Received : {:?}", t.get_received_token_sol());
                        println!("Spent : {:?}", t.get_spent_token_sol());
                        println!("Pump Received : {:?}", pump_t.get_pump_received_token_sol());
                        println!("Pump Spent : {:?}", pump_t.get_pump_spent_token_sol());
                    }
                }
                None => (),
            })
            .await;
    }
}
```

## 获取交易过程中实际增加和减少的代币地址和数量.

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
    let reduce = t2.as_ref().unwrap().get_received_token_sol();
    println!("reduce :{:?} ", reduce); // reduce.0 = EhzVcKKmGjLk6pD5gLT6ZrTg62bMgPgTSCXXmANnSyQA; reduce.1 = 6444.329826091
}
```

## 从签名中指定的流动性池中检索基础代币/报价代币.

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

## 获取 pump.fun 交易数据结构中的实际增加值与代币地址和减少值与代币地址

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

## 估算交易费用

```rust
let solana = Solana::new(Mode::DEV).unwrap();
let trade = solana.create_trade();

match trade.estimate_fee().await {
Ok(fee) => println!("估算费用: {} lamports", fee),
Err(e) => eprintln!("估算费用错误: {}", e),
}
```

## 分页获取交易历史

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
println!("获取到 {} 笔交易", transactions.len());

            for tx in &transactions {
                println!("签名: {}", tx.signature);
                println!("区块槽位: {}", tx.slot);
                println!("状态: {:?}", tx.err);
            }

            if next_cursor.is_none() {
                println!("没有更多交易可获取");
                break;
            }

            cursor = next_cursor;
            println!("继续使用游标: {:?}", cursor);
        }
        Err(e) => {
            eprintln!("获取交易错误: {}", e);
            break;
        }
    }

}
```

## 获取筛选后的交易历史

```rust
let client = solana.client_arc();
let address = "8MwwTfMp86sJ3b9B9W6cB3k6yLx4F5Gt2jK7N8P9Q0R";

// 示例 1: 按区块时间筛选
let recent_transactions = Trade::get_transactions_history_filtered(
&client,
address,
|sig_info| {
// 筛选最近 24 小时内的交易
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

// 示例 2: 仅筛选成功交易
let successful_transactions = Trade::get_transactions_history_filtered(
&client,
address,
|sig_info| sig_info.err.is_none(),
).await?;
```

## 获取包含另一地址的最后交易

```rust
let address_a = "8MwwTfMp86sJ3b9B9W6cB3k6yLx4F5Gt2jK7N8P9Q0R";
let address_b = "B5Zg7W7L7jH6K8L9M0N1O2P3Q4R5S6T7U8V9W0X1Y2Z";

match trade.get_last_transactions_contains_address(address_a, address_b).await {
Ok(Some(transaction)) => {
println!("找到包含两个地址的交易:");
println!("签名: {}", transaction.signature);
println!("区块槽位: {}", transaction.slot);
println!("区块时间: {:?}", transaction.block_time);
}
Ok(None) => println!("未找到包含两个地址的交易"),
Err(e) => eprintln!("错误: {}", e),
}
```

## 获取所有包含另一地址的交易

```rust
let address_a = "8MwwTfMp86sJ3b9B9W6cB3k6yLx4F5Gt2jK7N8P9Q0R";
let address_b = "B5Zg7W7L7jH6K8L9M0N1O2P3Q4R5S6T7U8V9W0X1Y2Z";

match trade.get_transactions_vec_containing_address(address_a, address_b).await {
Ok(transactions) => {
println!("找到 {} 笔包含两个地址的交易", transactions.len());

        for tx in transactions {
            println!("- 签名: {}", tx.signature);
            println!("  区块槽位: {}", tx.slot);
            println!("  状态: {}", if tx.err.is_none() { "成功" } else { "失败" });
        }
    }
    Err(e) => eprintln!("错误: {}", e),

}
```

## 获取交易详情

```rust
let signature = "5h6xBEauJ3PK6SWZrW5M4Q7GjS2eX2jGqKJ8H9i0K1L2M3N4O5P6Q7R8S9T0U1V2W3X4Y5Z6A7B8C9D0";

match trade.get_transaction_details(signature).await {
Ok(transaction) => {
let tx_info = TransactionInfo::from_encoded_transaction(
&transaction,
signature,
"主网"
);

        println!("交易详情:");
        println!("发送方: {}", tx_info.from);
        println!("接收方: {}", tx_info.to);
        println!("金额: {} SOL", tx_info.value_sol);
        println!("手续费: {} lamports", tx_info.fee);
        println!("状态: {}", tx_info.status);
        println!("类型: {}", tx_info.transaction_type);
    }
    Err(e) => eprintln!("获取交易详情错误: {}", e),

}
```

## 按收款方和付款方获取交易（宽松）

```rust
let recipient = "8MwwTfMp86sJ3b9B9W6cB3k6yLx4F5Gt2jK7N8P9Q0R";
let payer = "B5Zg7W7L7jH6K8L9M0N1O2P3Q4R5S6T7U8V9W0X1Y2Z";
let limit = 10;

match trade.get_transactions_by_recipient_and_payer(recipient, payer, limit).await {
Ok(transactions) => {
println!("找到 {} 笔交易，其中 {} 是收款方，{} 参与其中",
transactions.len(), recipient, payer);

        for tx in transactions {
            println!("签名: {}", tx.signature);
        }
    }
    Err(e) => eprintln!("错误: {}", e),

}
```

## 按收款方和付款方获取交易（严格）

```rust
let recipient = "8MwwTfMp86sJ3b9B9W6cB3k6yLx4F5Gt2jK7N8P9Q0R";
let payer = "B5Zg7W7L7jH6K8L9M0N1O2P3Q4R5S6T7U8V9W0X1Y2Z";
let limit = 10;

match trade.get_transactions_by_recipient_and_payer_strict(recipient, payer, limit).await {
Ok(transactions) => {
println!("找到 {} 笔交易，其中 {} 是收款方，{} 是付款方",
transactions.len(), recipient, payer);
        for tx in transactions {
            match trade.get_transaction_details(&tx.signature).await {
                Ok(details) => {
                    let tx_info = TransactionInfo::from_encoded_transaction(
                        &details,
                        &tx.signature,
                        "主网"
                    );
                    println!("- {}: {} SOL", tx.signature, tx_info.value_sol);
                }
                Err(_) => println!("- {}: 无法获取详情", tx.signature),
            }
        }
    }
    Err(e) => eprintln!("错误: {}", e),

}
```

## 检查支付关系

```rust
let recipient = "8MwwTfMp86sJ3b9B9W6cB3k6yLx4F5Gt2jK7N8P9Q0R";
let payer = "B5Zg7W7L7jH6K8L9M0N1O2P3Q4R5S6T7U8V9W0X1Y2Z";

match trade.has_payment_relationship(recipient, payer).await {
Ok(Some(signature)) => {
println!("发现支付关系！交易签名: {}", signature);
}
Ok(None) => {
println!("未发现 {} 和 {} 之间的支付关系", recipient, payer);
}
Err(e) => eprintln!("检查支付关系错误: {}", e),
}
```

## 获取总支付金额

```rust
let recipient = "8MwwTfMp86sJ3b9B9W6cB3k6yLx4F5Gt2jK7N8P9Q0R";
let payer = "B5Zg7W7L7jH6K8L9M0N1O2P3Q4R5S6T7U8V9W0X1Y2Z";

// 获取所有时间的总金额
match trade.get_total_payment_amount(recipient, payer, None).await {
Ok(total_amount) => {
println!("总支付金额: {} lamports ({:.4} SOL)",
total_amount, total_amount as f64 / LAMPORTS_PER_SOL as f64);
}
Err(e) => eprintln!("错误: {}", e),
}

// 获取最近 7 天的总金额
let seven_days = Some(7 _ 24 _ 60 * 60);
match trade.get_total_payment_amount(recipient, payer, seven_days).await {
Ok(total_amount) => {
println!("最近 7 天支付金额: {} lamports", total_amount);
}
Err(e) => eprintln!("错误: {}", e),
}
```

## TransactionInfo 辅助方法

```rust
// 获取 TransactionInfo 对象后
let signature = "5h6xBEauJ3PK6SWZrW5M4Q7GjS2eX2jGqKJ8H9i0K1L2M3N4O5P6Q7R8S9T0U1V2W3X4Y5Z6A7B8C9D0";
match trade.get_transaction_details(signature).await {
Ok(transaction) => {
let tx_info = TransactionInfo::from_encoded_transaction(&transaction, signature, "主网");
        // 检查交易是否成功
        if tx_info.is_successful() {
            println!("交易成功");
        }
        // 检查是否为代币转账
        if tx_info.is_token_transfer() {
            println!("这是代币转账");
            if let Some(mint) = &tx_info.token_mint {
                println!("代币铸造地址: {}", mint);
            }
        }
        // 检查是否为大额交易
        if tx_info.is_high_value() {
            println!("检测到大额交易！");
        }
        // 检查特定地址是否为收款方
        let address_to_check = "8MwwTfMp86sJ3b9B9W6cB3k6yLx4F5Gt2jK7N8P9Q0R";
        if tx_info.is_recipient(address_to_check) {
            println!("{} 是此交易的收款方", address_to_check);
        }
        // 检查特定地址是否为付款方
        if tx_info.is_payer(address_to_check) {
            println!("{} 是此交易的付款方", address_to_check);
        }
        // 获取支付金额
        println!("支付金额: {} lamports", tx_info.get_payment_amount());
        println!("支付金额: {} SOL", tx_info.get_payment_amount_sol());
        // 获取净金额（余额变化减去手续费）
        println!("净金额变化: {}", tx_info.get_net_amount());
    }
    Err(e) => eprintln!("错误: {}", e),
}
```

## 分析地址关系

```rust
async fn analyze_address_relationships(
trade: &Trade,
address1: &str,
address2: &str,
) -> Result<(), String> {
    println!("分析 {} 和 {} 之间的关系", address1, address2);
    // 1. 检查是否有支付关系
    match trade.has_payment_relationship(address1, address2).await {
        Ok(Some(signature)) => {
            println!("发现从 {} 到 {} 的支付", address2, address1);
            println!("交易: {}", signature);
        }
        Ok(None) => println!("未发现从 {} 到 {} 的直接支付", address2, address1),
        Err(e) => eprintln!("错误: {}", e),
    }
    // 2. 检查反向关系
    match trade.has_payment_relationship(address2, address1).await {
        Ok(Some(signature)) => {
            println!("发现从 {} 到 {} 的支付", address1, address2);
            println!("交易: {}", signature);
        }
        Ok(None) => println!("未发现从 {} 到 {} 的直接支付", address1, address2),
        Err(e) => eprintln!("错误: {}", e),
    }
    // 3. 获取所有涉及两者的交易
    let transactions_a = trade.get_transactions_vec_containing_address(address1, address2).await?;
    let transactions_b = trade.get_transactions_vec_containing_address(address2, address1).await?;
    println!("涉及两个地址的总交易数: {}",
             transactions_a.len() + transactions_b.len());
    // 4. 计算总金额
    let total_from_2_to_1 = trade.get_total_payment_amount(address1, address2, None).await?;
    let total_from_1_to_2 = trade.get_total_payment_amount(address2, address1, None).await?;
    println!("从 {} 到 {} 的总金额: {:.4} SOL",
             address2, address1,
             total_from_2_to_1 as f64 / LAMPORTS_PER_SOL as f64);
    println!("从 {} 到 {} 的总金额: {:.4} SOL",
             address1, address2,
             total_from_1_to_2 as f64 / LAMPORTS_PER_SOL as f64);
    Ok(())
}
```

# 扫描模块

## 获取所有历史签名

使用分页获取给定地址的所有历史交易签名。

```rust
use solana_network_sdk::Solana;
use solana_network_sdk::types::Mode;

#[tokio::main]
async fn main() -> Result<(), String> {
let solana = Solana::new(Mode::MAIN).unwrap();
let client = solana.client_arc();
let scan = solana_network_sdk::scan::Scan::new(client.clone());
    // 获取USDC地址的所有历史签名
    let signatures = scan.get_all_signatures_by_address(
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC代币地址
        Some(300),  // 请求间延迟300毫秒
        Some(500),  // 每批500个签名
    ).await?;
    println!("总签名数: {}", signatures.len());
    // 打印前5个签名
    for (i, sig) in signatures.iter().take(5).enumerate() {
        println!("{}. {}", i + 1, sig);
    }
    Ok(())
}
```

**参数:**

- `address: &str` - Base58 编码的 Solana 地址
- `interval_time: Option<u64>` - RPC 调用之间的延迟（毫秒，默认: 200）
- `batch_size: Option<u64>` - 每次 RPC 调用的签名数量（默认: 1000）

**返回:** `Result<Vec<String>, String>`

## 获取有限数量的签名

使用安全机制获取特定数量的交易签名。

```rust
use solana_network_sdk::Solana;
use solana_network_sdk::types::Mode;

#[tokio::main]
async fn main() -> Result<(), String> {
let solana = Solana::new(Mode::MAIN).unwrap();
let client = solana.client_arc();
let scan = solana_network_sdk::scan::Scan::new(client.clone());
    // 使用速率限制获取最多50个签名
    let signatures = scan.get_signatures_with_limit(
        "Vote111111111111111111111111111111111111111", // Solana投票程序
        50,         // 最多50个签名
        Some(100),  // 请求间延迟100毫秒
    ).await?;
    println!("已检索 {} 个签名:", signatures.len());
    for sig in &signatures {
        println!("  - {}", sig);
    }
    Ok(())
}
```

**参数:**

- `address: &str` - Base58 编码的 Solana 地址
- `limit: usize` - 要返回的最大签名数
- `interval_time: Option<u64>` - RPC 调用之间的延迟（毫秒，默认: 200）

**返回:** `Result<Vec<String>, String>`

## 获取最新签名

无需分页快速获取最新的交易签名。

```rust
use solana_network_sdk::Solana;
use solana_network_sdk::types::Mode;

#[tokio::main]
async fn main() -> Result<(), String> {
let solana = Solana::new(Mode::MAIN).unwrap();
let client = solana.client_arc();
let scan = solana_network_sdk::scan::Scan::new(client.clone());
    // 获取10个最新签名
    let signatures = scan.get_last_signatures(
        "So11111111111111111111111111111111111111112", // SOL代币地址
        10,  // 最近签名数量
    ).await?;
    println!("最新10个签名:");
    for (i, sig) in signatures.iter().enumerate() {
        println!("{}. {}", i + 1, sig);
    }
    Ok(())
}
```

**参数:**

- `address: &str` - Base58 编码的 Solana 地址
- `count: usize` - 要返回的最近签名数量

**返回:** `Result<Vec<String>, String>`

## 处理代币地址签名

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
    // 步骤1: 获取最新5个签名进行快速分析
    println!("正在获取 {} 的最新签名...", token_address);
    let latest_signatures = scan.get_last_signatures(token_address, 5).await?;
    // 步骤2: 为每个签名获取详细的交易信息
    for (i, signature) in latest_signatures.iter().enumerate() {
        println!("\n{}. 处理签名: {}", i + 1, signature);
        match trade.get_transaction_details(signature).await {
            Ok(transaction) => {
                let tx_info = solana_network_sdk::trade::TransactionInfo::from_encoded_transaction(
                    &transaction,
                    signature,
                    "mainnet"
                );
                println!("   槽位: {}", tx_info.slot);
                println!("   状态: {}", tx_info.status);
                println!("   手续费: {} lamports", tx_info.fee);
            }
            Err(e) => println!("   获取详情错误: {}", e),
        }
    }
    // 步骤3: 获取所有历史签名（分页，用于离线处理）
    println!("\n正在获取所有历史签名（这可能需要一段时间）...");
    let all_signatures = scan.get_all_signatures_by_address(
        token_address,
        Some(200),  // 200毫秒延迟
        Some(1000), // 每批1000个
    ).await?;
    println!("总历史签名数: {}", all_signatures.len());
    Ok(())
}
```
