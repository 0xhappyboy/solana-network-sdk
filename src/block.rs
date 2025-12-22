use serde::{Deserialize, Serialize};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcBlockConfig};
use solana_sdk::{clock::Slot, hash::Hash, signature::Signature};
use solana_transaction_status::{UiConfirmedBlock, UiTransactionEncoding};
use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub slot: Slot,
    pub blockhash: Hash,
    pub previous_blockhash: Hash,
    pub parent_slot: Slot,
    pub block_time: Option<i64>,
    pub block_height: Option<u64>,
    pub rewards: Vec<Reward>,
    pub transaction_count: usize,
    pub transaction_signatures: Vec<Signature>,
}

impl BlockInfo {
    pub fn parse(ui_block: UiConfirmedBlock) -> Self {
        let blockhash = Hash::from_str(&ui_block.blockhash).unwrap_or_else(|_| Hash::default());
        let previous_blockhash =
            Hash::from_str(&ui_block.previous_blockhash).unwrap_or_else(|_| Hash::default());
        let transaction_signatures: Vec<Signature> = ui_block
            .signatures
            .unwrap_or_default()
            .into_iter()
            .map(|sig_str| Signature::from_str(&sig_str).unwrap_or_default())
            .collect();
        let rewards = ui_block
            .rewards
            .unwrap_or_default()
            .into_iter()
            .map(|reward| Reward {
                pubkey: reward.pubkey,
                lamports: reward.lamports,
                post_balance: reward.post_balance,
                reward_type: reward.reward_type.map(|rt| rt.to_string()),
            })
            .collect();
        let transaction_count = transaction_signatures.len();
        BlockInfo {
            slot: ui_block.parent_slot + 1,
            blockhash,
            previous_blockhash,
            parent_slot: ui_block.parent_slot,
            block_time: ui_block.block_time,
            block_height: ui_block.block_height,
            rewards,
            transaction_count,
            transaction_signatures,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reward {
    pub pubkey: String,
    pub lamports: i64,
    pub post_balance: u64,
    pub reward_type: Option<String>,
}

pub struct Block {
    client: Arc<RpcClient>,
}

impl Block {
    pub fn new(client: Arc<RpcClient>) -> Self {
        Self { client }
    }

    async fn get_latest_block(&self) -> Result<Option<BlockInfo>, String> {
        let slot = self.client.get_slot().await.map_err(|e| e.to_string())?;
        self.get_block_by_slot(slot).await
    }

    pub async fn poll_latest_block<F>(&self, mut callback: F)
    where
        F: AsyncFnMut(Option<BlockInfo>),
    {
        let mut last_slot: Option<Slot> = None;

        loop {
            match self.get_latest_block().await {
                Ok(Some(block)) => {
                    if last_slot.map(|last| block.slot > last).unwrap_or(true) {
                        last_slot = Some(block.slot);
                        callback(Some(block)).await;
                    } else {
                        callback(None).await;
                    }
                }
                _ => callback(None).await,
            }
            // tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    pub async fn get_block_by_slot(&self, slot: Slot) -> Result<Option<BlockInfo>, String> {
        let config = RpcBlockConfig {
            encoding: Some(UiTransactionEncoding::Base64),
            transaction_details: Some(solana_transaction_status::TransactionDetails::Signatures),
            rewards: Some(true),
            commitment: None,
            max_supported_transaction_version: Some(0),
        };
        let block = self
            .client
            .get_block_with_config(slot, config)
            .await
            .map_err(|e| e.to_string())?;
        Ok(Some(BlockInfo::parse(block)))
    }
}

#[cfg(test)]
mod tests {
    use crate::{Solana, tool::trade};

    use super::*;

    #[tokio::test]
    async fn test_get_block_by_slot() {
        let solana = Solana::new(crate::types::Mode::MAIN).unwrap();
        let service = Block::new(solana.client_arc());
        let block_info = service.get_block_by_slot(387744706).await;
        println!(
            "Transaction Quantity: {:?}",
            block_info.unwrap().unwrap().transaction_signatures.len()
        );
    }

    #[tokio::test]
    async fn test_get_block_by_slot_is_vote_program() {
        let solana = Solana::new(crate::types::Mode::MAIN).unwrap();
        let service = Block::new(solana.client_arc());
        let block_info = service.get_block_by_slot(387744706).await;
        let trade = solana.create_trade();
        for sign in block_info.unwrap().unwrap().transaction_signatures {
            let trade_info = trade
                .get_transaction_display_details(&format!("{:?}", sign))
                .await
                .unwrap();
            trade_info.display().await;
        }
    }

    #[tokio::test]
    async fn test_poll_latest_block() {
        let solana = Solana::new(crate::types::Mode::MAIN).unwrap();
        let service = Block::new(solana.client_arc());
        let trade = solana.create_trade();
        service
            .poll_latest_block(async |block_info| match block_info {
                Some(info) => {
                    for sig in info.transaction_signatures {
                        let trade_info = trade
                            .get_transaction_display_details(&format!("{:?}", sig))
                            .await
                            .unwrap();
                        trade_info.display().await;
                    }
                }
                None => (),
            })
            .await;
    }
}
