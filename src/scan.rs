use solana_client::{
    nonblocking::rpc_client::RpcClient, rpc_client::GetConfirmedSignaturesForAddress2Config,
};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use std::collections::VecDeque;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

/// Scanner for retrieving transaction signatures from Solana blockchain
/// Provides methods to fetch historical and recent transaction signatures for given addresses
pub struct Scan {
    /// Solana RPC client for making network requests
    client: Arc<RpcClient>,
    /// Optional stop flag for early termination
    poll_all_signatures_by_address_stop_flag: Arc<AtomicBool>,
}

impl Scan {
    /// Creates a new Scan instance with the provided RPC client
    ///
    /// # Params
    /// * `client` - Arc-wrapped RpcClient for making Solana RPC calls
    ///
    /// # Returns
    /// New Scan instance
    pub fn new(client: Arc<RpcClient>) -> Self {
        Self {
            client: client,
            poll_all_signatures_by_address_stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Fetches all historical transaction signatures for a given address
    /// Continues pagination until no more signatures are available
    ///
    /// # Params
    /// * `address` - Solana address (base58 encoded) to fetch signatures for
    /// * `interval_time` - Optional delay between requests in milliseconds (default: 200ms)
    /// * `batch_size` - Optional number of signatures to fetch per batch (default: 1000)
    /// * `callback` - Callback function for signature processing. f(sign: String)
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - Vector of transaction signatures
    /// * `Err(String)` - Error message if address parsing or RPC call fails
    ///
    pub async fn poll_all_signatures_by_address<F>(
        &self,
        address: &str,
        interval_time: Option<u64>,
        batch_size: Option<u64>,
        mut callback: F,
    ) -> Result<(), String>
    where
        F: AsyncFnMut(String),
    {
        let pubkey = Pubkey::from_str(address).map_err(|e| format!("address error:{:?}", e))?;
        let mut all_signatures = Vec::new();
        let mut before: Option<Signature> = None;
        let sleep_duration = interval_time.unwrap_or(200);
        let batch_limit = batch_size.unwrap_or(1000);
        let mut history_completed = false;
        loop {
            if self
                .poll_all_signatures_by_address_stop_flag
                .load(Ordering::Relaxed)
            {
                return Ok(());
            }
            let config = GetConfirmedSignaturesForAddress2Config {
                before,
                until: None,
                limit: Some(batch_limit.try_into().unwrap()),
                commitment: None,
            };
            let signatures = match self
                .client
                .get_signatures_for_address_with_config(&pubkey, config)
                .await
            {
                Ok(sigs) => sigs,
                Err(e) => {
                    if e.to_string().contains("rate limit") || e.to_string().contains("429") {
                        tokio::time::sleep(Duration::from_millis(2000)).await;
                    } else {
                        eprintln!("RPC error (retrying): {:?}", e);
                        tokio::time::sleep(Duration::from_millis(sleep_duration)).await;
                    }
                    continue;
                }
            };
            if signatures.is_empty() {
                if !history_completed {
                    history_completed = true;
                    before = None;
                }
                tokio::time::sleep(Duration::from_millis(sleep_duration)).await;
                continue;
            }
            let signature_strings: Vec<String> =
                signatures.iter().map(|sig| sig.signature.clone()).collect();
            let mut new_signatures_found = false;
            for sig in &signature_strings {
                if !all_signatures.contains(sig) {
                    all_signatures.push(sig.clone());
                    new_signatures_found = true;
                    callback(sig.clone()).await;
                }
            }
            if let Some(last_sig) = signatures.last() {
                before = match Signature::from_str(&last_sig.signature) {
                    Ok(sig) => Some(sig),
                    Err(e) => {
                        eprintln!("Error parsing signature: {:?}", e);
                        continue;
                    }
                };
            } else {
                tokio::time::sleep(Duration::from_millis(sleep_duration)).await;
                continue;
            }
            if !new_signatures_found {
                tokio::time::sleep(Duration::from_millis(sleep_duration * 2)).await;
            } else {
                tokio::time::sleep(Duration::from_millis(sleep_duration)).await;
            }
        }
    }

    /// stop poll all signatures by address
    pub fn stop_poll_all_signatures_by_address(&self) {
        self.poll_all_signatures_by_address_stop_flag
            .store(true, Ordering::SeqCst);
    }

    /// Fetches a limited number of transaction signatures for a given address
    /// Stops when the specified limit is reached or no more signatures are available
    ///
    /// # Params
    /// * `address` - Solana address (base58 encoded) to fetch signatures for
    /// * `limit` - Maximum number of signatures to return
    /// * `interval_time` - Optional delay between requests in milliseconds (default: 200ms)
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - Vector of transaction signatures (up to the specified limit)
    /// * `Err(String)` - Error message if address parsing fails
    pub async fn get_signatures_with_limit(
        &self,
        address: &str,
        limit: usize,
        interval_time: Option<u64>,
    ) -> Result<Vec<String>, String> {
        let pubkey = Pubkey::from_str(address).map_err(|e| format!("address error:{:?}", e))?;
        let mut all_signatures = Vec::new();
        let mut before: Option<Signature> = None;
        let sleep_duration = interval_time.unwrap_or(200);
        while all_signatures.len() < limit {
            let remaining = limit - all_signatures.len();
            let batch_size = (remaining.min(1000)).min(u64::MAX.try_into().unwrap());
            let config = GetConfirmedSignaturesForAddress2Config {
                before,
                until: None,
                limit: Some(batch_size),
                commitment: None,
            };
            let signatures = match self
                .client
                .get_signatures_for_address_with_config(&pubkey, config)
                .await
            {
                Ok(sigs) => sigs,
                Err(e) => {
                    break;
                }
            };
            if signatures.is_empty() {
                break;
            }
            let signature_strings: Vec<String> =
                signatures.iter().map(|sig| sig.signature.clone()).collect();
            for sig in signature_strings {
                if !all_signatures.contains(&sig) && all_signatures.len() < limit {
                    all_signatures.push(sig);
                }
            }
            if all_signatures.len() >= limit {
                break;
            }
            if let Some(last_sig) = signatures.last() {
                before = match Signature::from_str(&last_sig.signature) {
                    Ok(sig) => Some(sig),
                    Err(_) => break,
                };
            } else {
                break;
            }
            tokio::time::sleep(Duration::from_millis(sleep_duration)).await;
        }
        Ok(all_signatures)
    }

    /// Quickly fetches the most recent transaction signatures for a given address
    /// Returns only the latest signatures without pagination
    ///
    /// # Params
    /// * `address` - Solana address (base58 encoded) to fetch signatures for
    /// * `count` - Number of recent signatures to return
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - Vector of the most recent transaction signatures
    /// * `Err(String)` - Error message if address parsing or RPC call fails
    pub async fn get_last_signatures(
        &self,
        address: &str,
        count: usize,
    ) -> Result<Vec<String>, String> {
        let pubkey = Pubkey::from_str(address).map_err(|e| format!("address error:{:?}", e))?;
        let config = GetConfirmedSignaturesForAddress2Config {
            before: None,
            until: None,
            limit: Some(count),
            commitment: None,
        };
        let signatures = self
            .client
            .get_signatures_for_address_with_config(&pubkey, config)
            .await
            .map_err(|e| format!("get signatures error:{:?}", e))?;
        let signature_strings: Vec<String> =
            signatures.iter().map(|sig| sig.signature.clone()).collect();
        Ok(signature_strings)
    }

    /// Fetches all transaction information for the specified address and calls back in batches
    ///
    /// # Parameters
    /// * `address` - Solana address (base58 encoded)
    /// * `callback` - Callback function that receives batches of transaction information
    ///
    pub async fn fetch_all_transactions_by_address<F, Fut>(
        self: Arc<Self>,
        address: &str,
        interval_time: Option<u64>,
        signs_batch_size: Option<u64>,
        find_trade_batch_size: Option<u64>,
        callback: F,
    ) -> Result<(), String>
    where
        F: Fn(Vec<crate::trade::info::TransactionInfo>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let trade_batch_size: u64 = find_trade_batch_size.unwrap_or(50);
        let sleep_duration = interval_time.unwrap_or(200);
        let batch_limit = signs_batch_size.unwrap_or(1000);
        let signatures_queue: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
        let trade = crate::trade::Trade::new(self.client.clone());
        let trade_arc = Arc::new(trade);
        let fetch_completed = Arc::new(AtomicBool::new(false));
        let signatures_queue_clone = signatures_queue.clone();
        let trade_clone = trade_arc.clone();
        let callback_arc = Arc::new(callback);
        let fetch_completed_clone = fetch_completed.clone();
        let self_clone = self.clone();
        let fetch_handle = tokio::spawn({
            let address = address.to_string();
            let queue_clone = signatures_queue_clone.clone();
            let fetch_completed = fetch_completed_clone.clone();
            let scan = self_clone.clone();
            let sleep_duration = sleep_duration;
            async move {
                let pubkey = match Pubkey::from_str(&address) {
                    Ok(p) => p,
                    Err(e) => {
                        fetch_completed.store(true, Ordering::Relaxed);
                        return;
                    }
                };
                let mut before: Option<Signature> = None;
                let mut history_completed = false;
                loop {
                    let config = GetConfirmedSignaturesForAddress2Config {
                        before,
                        until: None,
                        limit: Some(batch_limit.try_into().unwrap()),
                        commitment: None,
                    };
                    let signatures = match scan
                        .client
                        .get_signatures_for_address_with_config(&pubkey, config)
                        .await
                    {
                        Ok(sigs) => sigs,
                        Err(e) => {
                            if e.to_string().contains("rate limit") || e.to_string().contains("429")
                            {
                                tokio::time::sleep(Duration::from_millis(2000)).await;
                            } else {
                                eprintln!("RPC error: {:?}", e);
                                tokio::time::sleep(Duration::from_millis(sleep_duration)).await;
                            }
                            continue;
                        }
                    };
                    if signatures.is_empty() {
                        if !history_completed {
                            history_completed = true;
                            before = None;
                            fetch_completed.store(true, Ordering::Relaxed);
                        }
                        tokio::time::sleep(Duration::from_millis(sleep_duration)).await;
                        continue;
                    }
                    let mut queue_lock = queue_clone.lock().await;
                    for sig in &signatures {
                        queue_lock.push_back(sig.signature.clone());
                    }
                    if let Some(last_sig) = signatures.last() {
                        before = match Signature::from_str(&last_sig.signature) {
                            Ok(sig) => Some(sig),
                            Err(_) => continue,
                        };
                    }
                    tokio::time::sleep(Duration::from_millis(sleep_duration)).await;
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
                                let mut queue_lock = signatures_queue.lock().await;
                                for sig in batch_signatures {
                                    queue_lock.push_front(sig);
                                }
                            }
                        }
                    } else if fetch_completed.load(Ordering::Relaxed) {
                        let queue_empty = {
                            let queue_lock = signatures_queue.lock().await;
                            queue_lock.is_empty()
                        };

                        if queue_empty {
                            break;
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
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn test_get_last_signatures() -> Result<(), ()> {
        let solana = Solana::new(crate::types::Mode::MAIN).unwrap();
        let trade = solana.create_trade();
        let scan = Arc::new(solana.create_scan());
        let signs = scan
            .get_last_signatures("AmK2hPHoHktE2tcJWKbfMpYR3JiMdS3J19xGdHX4ZCLK", 10)
            .await
            .unwrap();
        for sign in signs {
            let trade_info = trade.get_transaction_display_details(&sign).await.unwrap();
            trade_info.display().await;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_get_all_signatures_by_address_and_batch_find_transaction() -> Result<(), ()> {
        let solana = Solana::new(crate::types::Mode::MAIN).unwrap();
        let scan = Arc::new(solana.create_scan());
        let _ = scan
            .fetch_all_transactions_by_address(
                "CzVqatmaK6GfyEWZUcWromDvpq3MFxqSrUweZgbjHngh",
                Some(100),
                Some(100),
                Some(10),
                async |trades| {
                    for trade in trades {
                        if (trade.is_swap()) {
                            trade.display().await;
                        }
                    }
                },
            )
            .await;
        Ok(())
    }

    #[tokio::test]
    async fn test_get_all_signatures_by_address_stop() -> Result<(), ()> {
        let solana = Solana::new(crate::types::Mode::MAIN).unwrap();
        let scan = Arc::new(solana.create_scan());
        let test_address = "CzVqatmaK6GfyEWZUcWromDvpq3MFxqSrUweZgbjHngh";
        let scan_1 = scan.clone();
        let handle1 = tokio::spawn(async move {
            let result = scan_1
                .poll_all_signatures_by_address(
                    test_address,
                    Some(100),
                    Some(100),
                    |sig| async move {
                        println!("Signature: {:?}", sig);
                    },
                )
                .await;
            println!("Stop polling: {:?}", result);
        });
        let scan_2 = scan.clone();
        let handle2 = tokio::spawn(async move {
            println!("Stop after 5 seconds...");
            tokio::time::sleep(Duration::from_secs(5)).await;
            println!("Stopping...");
            scan_2.stop_poll_all_signatures_by_address();
            println!("Stop signal sent");
        });
        tokio::join!(handle1, handle2);
        Ok(())
    }
}
