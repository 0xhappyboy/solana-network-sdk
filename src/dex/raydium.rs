use bytemuck::{Pod, Zeroable};

/// raydium v2 moudle
pub mod v2 {
    use std::sync::Arc;

    use solana_client::nonblocking::rpc_client::RpcClient;

    pub struct RaydiumV2 {
        client: Arc<RpcClient>,
    }
    impl RaydiumV2 {
        pub fn new(client: Arc<RpcClient>) -> Self {
            Self { client: client }
        }
    }
}

/// raydium v3 moudle
pub mod v3 {
    use std::sync::Arc;

    use solana_client::nonblocking::rpc_client::RpcClient;
    pub struct RaydiumV3 {
        client: Arc<RpcClient>,
    }
    impl RaydiumV3 {
        pub fn new(client: Arc<RpcClient>) -> Self {
            Self { client: client }
        }
    }
}
