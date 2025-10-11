use std::sync::Arc;

use solana_client::nonblocking::rpc_client::RpcClient;

pub struct Scan {
    client: Arc<RpcClient>,
}

impl Scan {
    /// create scan
    pub async fn new(client: Arc<RpcClient>) -> Self {
        Self { client: client }
    }
}
