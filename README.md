<h1 align="center">
    🤵 Solana Network SDK
</h1>
<h4 align="center">
Implemented most of the commonly used practical transaction-related functions on the Solana network.
</h4>
<p align="center">
  <a href="https://github.com/0xhappyboy/solana-trader/LICENSE"><img src="https://img.shields.io/badge/License-GPL3.0-d1d1f6.svg?style=flat&labelColor=1C2C2E&color=BEC5C9&logo=googledocs&label=license&logoColor=BEC5C9" alt="License"></a>
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
