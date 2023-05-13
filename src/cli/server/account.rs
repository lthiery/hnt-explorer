use super::*;
use axum::extract::Path;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::cli::account::HeliumBalances;

pub async fn get_account(
    Extension(rpc_client): Extension<Arc<RpcClient>>,
    Path(account): Path<String>,
) -> HandlerResult {
    if let Ok(pubkey) = Pubkey::from_str(&account) {
        match HeliumBalances::fetch(&rpc_client, &pubkey).await {
            Err(e) => {
                println!("Error fetching account: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
            Ok(account) => Ok(response::Json(json!(account))),
        }
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            format!("\"{account}\" is not a valid base58 encoded Solana pubkey"),
        ))
    }
}
