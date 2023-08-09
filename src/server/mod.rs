use super::{error::Error, rpc};
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
pub type HandlerResult = std::result::Result<MyResponse, (StatusCode, String)>;

pub enum MyResponse {
    Json(response::Json<Value>),
    Redirect(response::Redirect),
}

impl response::IntoResponse for MyResponse {
    fn into_response(self) -> Response {
        match self {
            Self::Json(j) => j.into_response(),
            Self::Redirect(r) => r.into_response(),
        }
    }
}

impl From<response::Json<Value>> for MyResponse {
    fn from(j: response::Json<Value>) -> Self {
        Self::Json(j)
    }
}
impl From<response::Redirect> for MyResponse {
    fn from(r: response::Redirect) -> Self {
        Self::Redirect(r)
    }
}

mod accounts;
mod epoch_info;
mod positions;

use axum::response::Response;
use std::sync::Arc;
use tokio::sync::Mutex;

const DATA_NOT_INIT_MSG: &str = "Data not initialized yet. Please try again in a few minutes.";

#[derive(Debug, Clone, clap::Args)]
pub struct Server {}

impl Server {
    pub async fn run(self, rpc_client: rpc::Client) -> Result {
        let rpc_client = Arc::new(rpc_client);

        println!("Initializing server with data...");

        let epoch_info_memory = epoch_info::Memory::new(&rpc_client).await?;
        let epoch_info_memory = Arc::new(Mutex::new(epoch_info_memory));
        println!("epoch_info data intialized...");
        // Initializing positions can take up to 3 minutes and not binding to the port upsets heroku
        // Therefore, we use an Option<positions::Memory> and it gets initialized after server is up
        let positions_memory = Arc::new(Mutex::new(None));
        println!("positions_memory initialized as empty...");
        println!("Server initialized!");

        // build our application with a route
        let app = Router::new()
            .route("/v1/accounts/:account", get(accounts::get_account))
            .route(
                "/v1/accounts/vehnt/top",
                get(accounts::get_top_vehnt_accounts),
            )
            .route(
                "/v1/accounts/veiot/top",
                get(accounts::get_top_veiot_accounts),
            )
            .route(
                "/v1/accounts/vemobile/top",
                get(accounts::get_top_vemobile_accounts),
            )
            .route("/v1/delegated_stakes", get(positions::delegated_stakes))
            .route(
                "/v1/delegated_stakes/csv",
                get(positions::server_latest_delegated_positions_as_csv),
            )
            .route(
                "/v1/delegated_stakes/info",
                get(positions::vehnt_positions_metadata),
            )
            .route("/v1/positions", get(positions::vehnt_positions))
            .route("/v1/positions/:position", get(positions::vehnt_position))
            .route(
                "/v1/positions/info",
                get(positions::vehnt_positions_metadata),
            )
            .route("/v1/positions/vehnt", get(positions::vehnt_positions))
            .route(
                "/v1/positions/vehnt/:position",
                get(positions::vehnt_position),
            )
            .route(
                "/v1/positions/vehnt/metadata",
                get(positions::vehnt_positions_metadata),
            )
            .route("/v1/positions/veiot", get(positions::veiot_positions))
            .route(
                "/v1/positions/veiot/:position",
                get(positions::veiot_position),
            )
            .route("/v1/positions/vemobile", get(positions::vemobile_positions))
            .route(
                "/v1/positions/vemobile/:position",
                get(positions::vemobile_position),
            )
            .route(
                "/v1/positions/csv",
                get(positions::server_latest_positions_as_csv),
            )
            .route("/v1/epoch/info", get(epoch_info::get))
            .layer(Extension(rpc_client.clone()))
            .layer(Extension(positions_memory.clone()))
            .layer(Extension(epoch_info_memory.clone()));

        let server_endpoint = std::env::var("PORT").unwrap_or("3000".to_string());
        println!("Binding to port {}...", server_endpoint);
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], server_endpoint.parse().unwrap()));
        tokio::select!(
            result = positions::get_positions(rpc_client.clone(), positions_memory,
                epoch_info_memory.clone()) => result,
            result = epoch_info::get_epoch_info(rpc_client, epoch_info_memory) => result,
            result = axum::Server::bind(&addr)
                .serve(app.into_make_service()) =>
                    result.map_err(|e| Error::Axum(e.into())),
        )
    }
}
