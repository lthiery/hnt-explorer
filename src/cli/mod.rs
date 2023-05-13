use super::*;

use anchor_lang::prelude::{AccountDeserialize, Result as AnchorResult};
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::RpcProgramAccountsConfig,
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::pubkey::Pubkey;

mod account;
mod epoch_info;
mod locked;
mod positions;
mod server;
mod supply;

#[derive(Debug, clap::Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"))]
#[clap(about = "helium-program-api")]
pub struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Cmd {
    /// View account information
    Account(account::Account),
    /// View locked HNT
    Locked(locked::Locked),
    /// View information about delegated stakes
    Positions(positions::Positions),
    /// Output epochs to CSV file
    EpochInfo(epoch_info::EpochInfo),
    /// Run server that serves the API
    Server(server::Server),
    /// Get supply of Helium tokens
    Supply(supply::Supply),
}

impl Cli {
    pub async fn run(self, rpc_client: RpcClient) -> Result {
        match self.cmd {
            Cmd::Account(cmd) => cmd.run(rpc_client).await,
            Cmd::EpochInfo(cmd) => cmd.run(rpc_client).await,
            Cmd::Locked(cmd) => cmd.run(rpc_client).await,
            Cmd::Positions(cmd) => cmd.run(rpc_client).await,
            Cmd::Server(cmd) => cmd.run(rpc_client).await,
            Cmd::Supply(cmd) => cmd.run(rpc_client).await,
        }
    }
}
