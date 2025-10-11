use std::{str::FromStr, sync::Arc};

use solana_client::{
    nonblocking::rpc_client::RpcClient, rpc_client::GetConfirmedSignaturesForAddress2Config,
};
use solana_program::pubkey;
use solana_sdk::{clock::Epoch, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};

pub struct Account {
    client: Arc<RpcClient>,
}
impl Account {
    /// create account
    pub fn new(client: Arc<RpcClient>) -> Self {
        Self { client: client }
    }
    /// refresh account data
    pub fn refresh(&mut self) {}
}
