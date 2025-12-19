use crate::types::{UnifiedError, UnifiedResult};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use spl_token::{solana_program::program_pack::Pack, state::Mint};
use std::{str::FromStr, sync::Arc};

pub struct Spl {
    client: Arc<RpcClient>,
}

impl Spl {
    pub fn new(client: Arc<RpcClient>) -> Self {
        Self { client }
    }

    /// Get token information by specified SPL token address
    ///
    /// # Parameters
    /// * `mint_address` - SPL token mint address
    ///
    /// # Returns
    /// * `Ok(SplTokenInfo)` - Successfully obtained and parsed token information
    /// * `Err(String)` - Failed to get or parse token information
    ///
    /// # Example
    /// ```rust
    /// let solana = Solana::new(Mode::DEV).unwrap();
    /// let spl = solana.create_spl();
    /// let token_info = spl.get_token_info("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").await?;
    /// ```
    pub async fn get_token_info(&self, mint_address: &str) -> UnifiedResult<SplTokenInfo, String> {
        let mint_pubkey = match Pubkey::from_str(mint_address) {
            Ok(pubkey) => pubkey,
            Err(_) => {
                return Err(UnifiedError::Error(
                    "Invalid token address format".to_string(),
                ));
            }
        };
        let account_data = match self
            .client
            .get_account_with_commitment(&mint_pubkey, CommitmentConfig::confirmed())
            .await
        {
            Ok(account) => match account.value {
                Some(acc) => acc.data,
                None => {
                    return Err(UnifiedError::Error(
                        "Token account does not exist".to_string(),
                    ));
                }
            },
            Err(e) => {
                return Err(UnifiedError::Error(format!(
                    "Failed to get account data: {:?}",
                    e
                )));
            }
        };
        let mint_info = match Mint::unpack(&account_data) {
            Ok(mint) => mint,
            Err(e) => {
                return Err(UnifiedError::Error(format!(
                    "Failed to parse token data: {:?}",
                    e
                )));
            }
        };
        let token_info = SplTokenInfo::from_mint(mint_info, mint_address);
        Ok(token_info)
    }
}

#[derive(Debug, Clone)]
pub struct SplTokenInfo {
    pub mint_address: String,
    pub decimals: u8,
    pub supply_raw: u64,
    pub supply: f64,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub logo_uri: Option<String>,
    pub website: Option<String>,
    pub description: Option<String>,
}

impl SplTokenInfo {
    fn from_mint(mint: Mint, mint_address: &str) -> Self {
        let supply_raw = mint.supply;
        let supply = supply_raw as f64 / 10_u64.pow(mint.decimals as u32) as f64;

        Self {
            mint_address: mint_address.to_string(),
            decimals: mint.decimals,
            supply_raw,
            supply,
            mint_authority: match mint.mint_authority {
                spl_token::solana_program::program_option::COption::Some(pk) => {
                    Some(pk.to_string())
                }
                spl_token::solana_program::program_option::COption::None => None,
            },
            freeze_authority: match mint.freeze_authority {
                spl_token::solana_program::program_option::COption::Some(pk) => {
                    Some(pk.to_string())
                }
                spl_token::solana_program::program_option::COption::None => None,
            },
            symbol: None,
            name: None,
            logo_uri: None,
            website: None,
            description: None,
        }
    }

    pub fn get_supply_with_decimals(&self) -> f64 {
        self.supply
    }

    pub fn is_mintable(&self) -> bool {
        self.mint_authority.is_some()
    }

    pub fn is_freezable(&self) -> bool {
        self.freeze_authority.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Solana;
    use crate::types::Mode;

    #[tokio::test]
    async fn test_get_token_info_usdc() {
        let solana = match Solana::new(Mode::MAIN) {
            Ok(s) => s,
            Err(_) => {
                return;
            }
        };
        let spl = solana.create_spl();
        // USDC address
        let usdc_address = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        match spl.get_token_info(usdc_address).await {
            Ok(token_info) => {
                println!("✅ USDC token info test passed:");
                println!("  Address: {}", token_info.mint_address);
                println!("  Decimals: {}", token_info.decimals);
                println!("  Supply: {}", token_info.supply);
                println!("  Is mintable: {}", token_info.is_mintable());
                println!("  Is freezable: {}", token_info.is_freezable());
            }
            Err(e) => {
                eprintln!("❌ Error: {:?}", e);
            }
        }
    }
}
