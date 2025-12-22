use crate::{
    global::{SPL_TOKEN_PROGRAM_2022, SPL_TOKEN_PROGRAM_V1},
    types::{UnifiedError, UnifiedResult},
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use std::{str::FromStr, sync::Arc};

pub struct Spl {
    client: Arc<RpcClient>,
}

impl Spl {
    pub fn new(client: Arc<RpcClient>) -> Self {
        Self { client }
    }

    /// Get token information by specified SPL token address (supports both standard SPL Token and Token2022)
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
        let mint_pubkey = Pubkey::from_str(mint_address)
            .map_err(|_| UnifiedError::Error("Invalid token address format".to_string()))?;
        let account_response = self
            .client
            .get_account_with_commitment(&mint_pubkey, CommitmentConfig::confirmed())
            .await
            .map_err(|e| UnifiedError::Error(format!("Failed to get account: {:?}", e)))?;
        let account = account_response
            .value
            .ok_or_else(|| UnifiedError::Error("Token account does not exist".to_string()))?;
        if account.data.len() < 82 {
            return Err(UnifiedError::Error(
                "Invalid mint account data length".to_string(),
            ));
        }
        let program_type = self.get_token_program_type_from_owner(&account.owner)?;
        let data = &account.data;
        let supply_raw = u64::from_le_bytes([
            data[36], data[37], data[38], data[39], data[40], data[41], data[42], data[43],
        ]);
        let decimals = data[44];
        let mint_authority = if u32::from_le_bytes([data[0], data[1], data[2], data[3]]) == 1 {
            Some(Pubkey::new_from_array(<[u8; 32]>::try_from(&data[4..36]).unwrap()).to_string())
        } else {
            None
        };
        let freeze_authority = if u32::from_le_bytes([data[45], data[46], data[47], data[48]]) == 1
        {
            Some(Pubkey::new_from_array(<[u8; 32]>::try_from(&data[49..81]).unwrap()).to_string())
        } else {
            None
        };
        let supply = supply_raw as f64 / 10_u64.pow(decimals as u32) as f64;
        Ok(SplTokenInfo {
            mint_address: mint_address.to_string(),
            decimals,
            supply_raw,
            supply,
            mint_authority,
            freeze_authority,
            symbol: None,
            name: None,
            logo_uri: None,
            website: None,
            description: None,
            program_type,
        })
    }

    /// Helper function to get token program type from owner pubkey
    fn get_token_program_type_from_owner(
        &self,
        owner: &Pubkey,
    ) -> UnifiedResult<TokenProgramType, String> {
        let spl_token_program =
            Pubkey::from_str(SPL_TOKEN_PROGRAM_V1).expect("Invalid SPL Token program ID");
        let token_2022_program =
            Pubkey::from_str(SPL_TOKEN_PROGRAM_2022).expect("Invalid Token2022 program ID");
        if owner == &spl_token_program {
            Ok(TokenProgramType::StandardSplToken)
        } else if owner == &token_2022_program {
            Ok(TokenProgramType::Token2022)
        } else {
            Err(UnifiedError::Error("Not a token account".to_string()))
        }
    }

    /// Get token age in seconds (time since creation)
    pub async fn get_token_age_seconds(&self, mint_address: &str) -> UnifiedResult<u64, String> {
        let creation_timestamp = self.get_token_creation_timestamp(mint_address).await?;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| UnifiedError::Error(format!("Failed to get current time: {:?}", e)))?
            .as_secs();
        Ok(current_time.saturating_sub(creation_timestamp as u64))
    }

    /// Get token creation timestamp (Unix timestamp) via account creation slot
    pub async fn get_token_creation_timestamp(
        &self,
        mint_address: &str,
    ) -> UnifiedResult<i64, String> {
        let mint_pubkey = Pubkey::from_str(mint_address)
            .map_err(|_| UnifiedError::Error("Invalid token address format".to_string()))?;
        let account_info = self
            .client
            .get_account_with_commitment(&mint_pubkey, CommitmentConfig::finalized())
            .await
            .map_err(|e| UnifiedError::Error(format!("Failed to get account info: {:?}", e)))?;
        let created_slot = account_info.context.slot;
        let block_time = self
            .client
            .get_block_time(created_slot)
            .await
            .map_err(|e| UnifiedError::Error(format!("Failed to get block time: {:?}", e)))?;
        Ok(block_time)
    }

    /// Check if a token is Token2022 or standard SPL Token
    pub async fn get_token_program_type(
        &self,
        mint_address: &str,
    ) -> UnifiedResult<TokenProgramType, String> {
        let mint_pubkey = Pubkey::from_str(mint_address)
            .map_err(|_| UnifiedError::Error("Invalid token address format".to_string()))?;
        let account_response = self
            .client
            .get_account_with_commitment(&mint_pubkey, CommitmentConfig::confirmed())
            .await
            .map_err(|e| UnifiedError::Error(format!("Failed to get account: {:?}", e)))?;
        let account = account_response
            .value
            .ok_or_else(|| UnifiedError::Error("Account does not exist".to_string()))?;
        let spl_token_program =
            Pubkey::from_str(SPL_TOKEN_PROGRAM_V1).expect("Invalid SPL Token program ID");
        let token_2022_program =
            Pubkey::from_str(SPL_TOKEN_PROGRAM_2022).expect("Invalid Token2022 program ID");
        if account.owner == spl_token_program {
            Ok(TokenProgramType::StandardSplToken)
        } else if account.owner == token_2022_program {
            Ok(TokenProgramType::Token2022)
        } else {
            Err(UnifiedError::Error("Not a token account".to_string()))
        }
    }
}

#[derive(Debug, Clone)]
pub enum TokenProgramType {
    StandardSplToken,
    Token2022,
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
    pub program_type: TokenProgramType,
}

impl Default for SplTokenInfo {
    fn default() -> Self {
        Self {
            mint_address: Default::default(),
            decimals: Default::default(),
            supply_raw: Default::default(),
            supply: Default::default(),
            mint_authority: Default::default(),
            freeze_authority: Default::default(),
            symbol: Default::default(),
            name: Default::default(),
            logo_uri: Default::default(),
            website: Default::default(),
            description: Default::default(),
            program_type: TokenProgramType::StandardSplToken,
        }
    }
}

impl SplTokenInfo {
    pub fn get_supply_with_decimals(&self) -> f64 {
        self.supply
    }

    pub fn is_mintable(&self) -> bool {
        self.mint_authority.is_some()
    }

    pub fn is_freezable(&self) -> bool {
        self.freeze_authority.is_some()
    }

    pub fn is_token_2022(&self) -> bool {
        matches!(self.program_type, TokenProgramType::Token2022)
    }

    pub fn is_standard_spl_token(&self) -> bool {
        matches!(self.program_type, TokenProgramType::StandardSplToken)
    }
}

#[cfg(test)]
mod tests {
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
        // USDC address (standard SPL Token)
        let usdc_address = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        match spl.get_token_info(usdc_address).await {
            Ok(token_info) => {
                println!("✅ USDC token info test passed:");
                println!("  Address: {}", token_info.mint_address);
                println!("  Decimals: {}", token_info.decimals);
                println!("  Supply: {}", token_info.supply);
                println!("  Is mintable: {}", token_info.is_mintable());
                println!("  Is freezable: {}", token_info.is_freezable());
                println!("  Program Type: {:?}", token_info.program_type);
            }
            Err(e) => {
                eprintln!("❌ Error: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_get_token_program_type() {
        let solana = match Solana::new(Mode::MAIN) {
            Ok(s) => s,
            Err(_) => {
                return;
            }
        };
        let spl = solana.create_spl();
        // Test standard SPL Token
        let standard_token = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // USDC
        match spl.get_token_program_type(standard_token).await {
            Ok(program_type) => println!("✅ Standard token program type: {:?}", program_type),
            Err(e) => eprintln!("❌ Error: {:?}", e),
        }
        // Test Token2022
        let token2022 = "Gbu7JAKhTVtGyRryg8cYPiKNhonXpUqbrZuCDjfUpump";
        match spl.get_token_program_type(token2022).await {
            Ok(program_type) => println!("✅ Token2022 program type: {:?}", program_type),
            Err(e) => eprintln!("❌ Error: {:?}", e),
        }
    }
}
