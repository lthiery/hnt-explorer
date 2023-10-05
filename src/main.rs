use hnt_explorer::*;

#[tokio::main]
async fn main() -> Result {
    use clap::Parser;
    let cli = cli::Cli::parse();
    let rpc_client = rpc::Client::default();
    cli.run(rpc_client).await
}
