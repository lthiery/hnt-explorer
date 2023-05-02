use super::error::Error;
use std::collections::HashMap;

use axum::{
    extract::{Extension, Query},
    http::StatusCode,
    response,
    routing::get,
    Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::time;

pub type Result<T = ()> = std::result::Result<T, Error>;
pub type HandlerResult = std::result::Result<response::Json<Value>, (StatusCode, String)>;

mod epoch_info;
mod positions;

use std::sync::Arc;
use tokio::sync::Mutex;

use solana_client::nonblocking::rpc_client::RpcClient;

#[derive(Debug, Clone, clap::Args)]
pub struct Server {}

impl Server {
    pub async fn run(self, rpc_client: RpcClient) -> Result {
        let rpc_client = Arc::new(rpc_client);

        println!("Initializing server with data...");

        let epoch_info_memory = epoch_info::Memory::new(&rpc_client).await?;
        let epoch_info_memory = Arc::new(Mutex::new(epoch_info_memory));
        println!("epoch_info data intialized...");
        let delegated_memory = positions::Memory::new(&rpc_client).await?;
        let delegated_memory = Arc::new(Mutex::new(delegated_memory));
        println!("delegated_memory data initialized!");

        println!("Server initialized!");

        // build our application with a route
        let app = Router::new()
            .route("/v1/delegated_stakes", get(positions::delegated_stakes))
            .route(
                "/v1/delegated_stakes/csv",
                get(positions::server_latest_delegated_positions_as_csv),
            )
            .route(
                "/v1/delegated_stakes/info",
                get(positions::positions_metadata),
            )
            .route("/v1/positions", get(positions::positions))
            .route("/v1/positions/info", get(positions::positions_metadata))
            .route(
                "/v1/positions/csv",
                get(positions::server_latest_positions_as_csv),
            )
            .route("/v1/epoch/info", get(epoch_info::get))
            .layer(Extension(delegated_memory.clone()))
            .layer(Extension(epoch_info_memory.clone()));

        let server_endpoint = std::env::var("PORT").unwrap_or("3000".to_string());
        println!("Binding to port {}...", server_endpoint);
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], server_endpoint.parse().unwrap()));

        // run it with hyper on localhost:3000
        tokio::select!(
            result = positions::get_positions(rpc_client.clone(), delegated_memory) => result,
            result = epoch_info::get_epoch_info(rpc_client, epoch_info_memory) => result,
            result = axum::Server::bind(&addr)
                .serve(app.into_make_service()) =>
                    result.map_err(|e| Error::Axum(e.into())),
        )
    }
}
