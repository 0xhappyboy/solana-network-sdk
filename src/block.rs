use serde::{Deserialize, Serialize};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcBlockConfig};
use solana_sdk::{clock::Slot, hash::Hash, signature::Signature};
use solana_transaction_status::{UiConfirmedBlock, UiTransactionEncoding};
use std::collections::VecDeque;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

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

    /// Fetches all transaction information from latest blocks and calls back in batches
    ///
    /// # Parameters
    /// * `interval_time` - Optional delay between block requests in milliseconds (default: 200ms)
    /// * `find_trade_batch_size` - Optional number of transactions to process per batch (default: 50)
    /// * `callback` - Callback function that receives batches of transaction information
    ///
    pub async fn fetch_transactions_from_latest_blocks<F, Fut>(
        self: Arc<Self>,
        interval_time: Option<u64>,
        find_trade_batch_size: Option<u64>,
        callback: F,
    ) -> Result<(), String>
    where
        F: Fn(Vec<crate::trade::info::TransactionInfo>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let trade_batch_size: u64 = find_trade_batch_size.unwrap_or(50);
        let sleep_duration = interval_time.unwrap_or(200);
        let signatures_queue: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
        let trade = crate::trade::Trade::new(self.client.clone());
        let trade_arc = Arc::new(trade);
        let fetch_completed = Arc::new(AtomicBool::new(false));
        let signatures_queue_clone = signatures_queue.clone();
        let trade_clone = trade_arc.clone();
        let callback_arc = Arc::new(callback);
        let self_clone = self.clone();
        let fetch_handle: JoinHandle<()> = tokio::spawn({
            let sleep_duration = sleep_duration;
            let scan = self_clone.clone();
            let queue_clone = signatures_queue_clone.clone();
            async move {
                let mut last_processed_slot: Option<Slot> = None;
                loop {
                    match scan.get_latest_block().await {
                        Ok(Some(block_info)) => {
                            if last_processed_slot
                                .map(|last| block_info.slot > last)
                                .unwrap_or(true)
                            {
                                last_processed_slot = Some(block_info.slot);
                                let mut queue_lock = queue_clone.lock().await;
                                for signature in &block_info.transaction_signatures {
                                    let sig_str = signature.to_string();
                                    if !queue_lock.contains(&sig_str) {
                                        queue_lock.push_back(sig_str);
                                    }
                                }
                            }
                        }
                        Ok(None) => {
                            continue;
                        }
                        Err(e) => {
                            eprintln!("Error fetching block: {:?}", e);
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(sleep_duration)).await;
                }
            }
        });
        let process_handle = tokio::spawn({
            let signatures_queue = signatures_queue.clone();
            let trade = trade_clone.clone();
            let callback = callback_arc.clone();
            let fetch_completed = fetch_completed.clone();
            async move {
                loop {
                    let batch_signatures = {
                        let mut queue_lock = signatures_queue.lock().await;
                        let mut batch = Vec::new();
                        while batch.len() < trade_batch_size.try_into().unwrap()
                            && !queue_lock.is_empty()
                        {
                            if let Some(sig) = queue_lock.pop_front() {
                                batch.push(sig);
                            } else {
                                break;
                            }
                        }
                        batch
                    };
                    if !batch_signatures.is_empty() {
                        let sig_slices: Vec<&str> =
                            batch_signatures.iter().map(|s| s.as_str()).collect();
                        match trade
                            .get_transaction_display_details_batch(sig_slices)
                            .await
                        {
                            Ok(transaction_infos) => {
                                if !transaction_infos.is_empty() {
                                    callback(transaction_infos).await;
                                }
                            }
                            Err(e) => {
                                // On error, put signatures back to queue
                                eprintln!("Error fetching transaction details: {:?}", e);
                                let mut queue_lock = signatures_queue.lock().await;
                                for sig in batch_signatures {
                                    queue_lock.push_front(sig);
                                }
                            }
                        }
                    } else {
                        if fetch_completed.load(Ordering::Relaxed) {
                            let queue_empty = {
                                let queue_lock = signatures_queue.lock().await;
                                queue_lock.is_empty()
                            };
                            if queue_empty {
                                break;
                            }
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
        });
        let _ = tokio::try_join!(fetch_handle, process_handle)
            .map_err(|e| format!("Thread Execution Error: {:?}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Solana;

    use super::*;

    #[tokio::test]
    async fn test_fetch_transactions_from_latest_blocks() {
        let solana = Solana::new(crate::types::Mode::MAIN).unwrap();
        let block_service = Arc::new(Block::new(solana.client_arc()));
        let _ = block_service
            .fetch_transactions_from_latest_blocks(
                Some(500), // the block is polled every 500ms.
                Some(10),  // 10 transactions per batch
                async |transactions| {
                    for tx in &transactions {
                        tx.display().await;
                    }
                },
            )
            .await;
    }

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
