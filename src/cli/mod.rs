use super::*;

use anchor_lang::{
    prelude::{AccountDeserialize, Result as AnchorResult},
    solana_program::pubkey::Pubkey,
};

pub mod accounts;
pub mod epoch_info;
pub mod locked;
pub mod positions;
pub mod supply;

#[derive(Debug, clap::Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"))]
#[clap(about = "hnt-explorer-api")]
pub struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Cmd {
    /// View account information
    Account(accounts::Account),
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
    pub async fn run(self, rpc_client: rpc::Client) -> Result {
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
