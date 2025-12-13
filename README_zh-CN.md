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

## 交易模块

### 概述

`Trade` 模块提供与 Solana 区块链交互的功能，包括交易历史检索、交易分析、支付关系检测和详细交易解析。

### 结构体

### `Trade`

交易操作的主要结构体。

### 方法

### `new(client: Arc<RpcClient>) -> Self`

使用给定的 RPC 客户端创建新的 `Trade` 实例。

**参数:**

- `client`: 用于 Solana 的 Arc 包装的 RPC 客户端

**返回:**

- `Trade` 实例

### `estimate_fee() -> Result<u64, String>`

估算 Solana 网络上的当前交易费用。

**返回:**

- `Ok(u64)`: 以 lamports 为单位的估算费用
- `Err(String)`: 如果估算失败则返回错误信息

### get_transactions_history_by_cursor(address: &str, cursor: Option<String>, page_size: u32) -> UnifiedResult<(Vec<RpcConfirmedTransactionStatusWithSignature>, Option<String>), String>

检索指定地址的交易历史，支持分页。

**参数:**

- `address`: 钱包地址字符串
- `cursor`: 用于分页的可选游标（最后交易的签名）
- `page_size`: 每页检索的交易数量

**返回:**

- `Ok((Vec<RpcConfirmedTransactionStatusWithSignature>, Option<String>))`: 包含交易列表和下一个游标的元组
- `Err(String)`: 如果检索失败则返回错误信息

**示例:**

```rust
let mut cursor: Option<String> = None;
loop {
    match trade
        .get_transactions_history_by_cursor(
            "钱包地址",
            cursor.clone(),
            page_size,
        )
        .await
    {
        Ok(r) => {
            // r.0 是交易历史列表
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

检索指定地址的过滤后交易历史。

**参数:**

- `client`: RPC 客户端引用
- `address`: 钱包地址
- `filter`: 返回 `true` 以保留交易记录的闭包

**返回:**

- `Ok(Vec<RpcConfirmedTransactionStatusWithSignature>)`: 过滤后的交易列表
- `Err(String)`: 如果检索失败则返回错误信息

**示例:**

```rust
let history = Trade::get_transactions_history_filtered(
    &client,
    "钱包地址",
    |sig_info| {
        // 返回 true 以保留交易信息
        true
    },
).await;
```

### get_last_transactions_contains_address(address_a: &str, address_b: &str) -> UnifiedResult<Option<RpcConfirmedTransactionStatusWithSignature>, String>

获取地址 A 中包含地址 B 的最后一条交易记录。

**参数:**

- `address_a`: 主要查询地址
- `address_b`: 要检查是否包含的地址

**返回:**

- `Ok(Some(RpcConfirmedTransactionStatusWithSignature))`: 包含地址 B 的最后交易
- `Ok(None)`: 不包含地址 B
- `Err(String)`: 错误信息

### get_transactions_vec_containing_address(address_a: &str, address_b: &str) -> UnifiedResult<Vec<RpcConfirmedTransactionStatusWithSignature>, String>

获取地址 A 中包含地址 B 的所有交易。

**参数:**

- `address_a`: 主要查询地址
- `address_b`: 要包含的地址

**返回:**

- `Ok(Vec<RpcConfirmedTransactionStatusWithSignature>)`: 包含地址 B 的所有交易记录列表

### get_transaction_details(signature: &str) -> UnifiedResult<EncodedConfirmedTransactionWithStatusMeta, String>

通过签名获取详细的交易信息。

**参数:**

- `signature`: 交易签名哈希字符串

**返回:**

- `Ok(EncodedConfirmedTransactionWithStatusMeta)`: 详细的交易信息
- `Err(String)`: 错误信息

### get_transactions_by_recipient_and_payer(address_a: &str, address_b: &str, limit: usize) -> UnifiedResult<Vec<RpcConfirmedTransactionStatusWithSignature>, String>

获取地址 A 为收款人且交易包含地址 B 的交易（宽松过滤）。

**参数:**

- `address_a`: 收款人地址
- `address_b`: 付款人地址
- `limit`: 返回的最大交易数量

**返回:**

- `Ok(Vec<RpcConfirmedTransactionStatusWithSignature>)`: 匹配的交易

### get_transactions_by_recipient_and_payer_strict(address_a: &str, address_b: &str, limit: usize) -> UnifiedResult<Vec<RpcConfirmedTransactionStatusWithSignature>, String>

获取地址 A 为收款人且地址 B 为付款人的交易（严格过滤）。

**参数:**

- `address_a`: 收款人地址
- `address_b`: 付款人地址
- `limit`: 返回的最大交易数量

**返回:**

- `Ok(Vec<RpcConfirmedTransactionStatusWithSignature>)`: 确认的交易

### has_payment_relationship(address_a: &str, address_b: &str) -> UnifiedResult<Option<String>, String>

快速检查两个地址之间是否存在支付关系（地址 B 支付给地址 A）。

**参数:**

- `address_a`: 收款人地址
- `address_b`: 付款人地址

**返回:**

- `Ok(Some(String))`: 如果存在支付关系则返回交易签名
- `Ok(None)`: 没有支付关系
- `Err(String)`: 错误信息

### get_total_payment_amount(address_a: &str, address_b: &str, time_range: Option<u64>) -> UnifiedResult<u64, String>

获取地址 B 支付给地址 A 的总金额。

**参数:**

- `address_a`: 收款人地址
- `address_b`: 付款人地址
- `time_range`: 时间范围（秒），None 表示所有时间

**返回:**

- `Ok(u64)`: 总支付金额（lamports）
- `Err(String)`: 错误信息

### is_transaction_contains_address(signature: &str, target_address: &str) -> bool （私有方法）

检查单个交易是否包含指定地址。

### 辅助结构体

### TransactionInfo

更易读的交易信息结构体。

### 主要方法:

- `from_encoded_transaction()`: 从编码交易创建
- `is_recipient()`: 检查地址是否为收款人
- `is_payer()`: 检查地址是否为付款人
- `get_payment_amount()`: 获取支付金额（lamports）
- `get_payment_amount_sol()`: 获取支付金额（SOL）
- `is_successful()`: 检查交易是否成功
- `is_token_transfer()`: 检查是否为代币转账
- `get_net_amount()`: 获取扣除费用后的净金额
- `is_high_value()`: 检查是否为高价值交易
