use chrono::{DateTime, NaiveDateTime, Utc};
use std::str::FromStr;

mod cli;
mod error;
mod rpc;
mod server;
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
    let rpc_client = rpc::Client::default();
    cli.run(rpc_client).await
}
