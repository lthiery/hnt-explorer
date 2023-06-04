use chrono::{DateTime, NaiveDateTime, Utc};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use std::str::FromStr;

mod cli;
mod error;
mod rpc;
mod types;
mod utils;

pub use error::Error;
pub use types::*;
pub use utils::*;
pub type Result<T = ()> = std::result::Result<T, Error>;

#[tokio::main]
async fn main() -> Result {
    use clap::Parser;
    let cli = cli::Cli::parse();
    let rpc_endpoint = std::env::var("SOL_RPC_ENDPOINT")
        .unwrap_or("https://api.mainnet-beta.solana.com".to_string());
    let rpc_client = RpcClient::new_with_commitment(rpc_endpoint, CommitmentConfig::confirmed());
    cli.run(rpc_client).await
}
