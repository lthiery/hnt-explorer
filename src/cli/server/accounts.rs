use super::*;
use axum::extract::Path;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::cli::accounts::{self, HeliumBalances};
use crate::SubDao;

#[derive(serde::Serialize)]
pub struct Balances {
    hnt: HntBalance,
    iot: DntBalance,
    mobile: DntBalance,
}

#[derive(serde::Serialize)]
struct DntBalance {
    amount: u64,
    pending_amount: u64,
    total_amount: u64,
    mint: String,
    decimals: u8,
}

#[derive(serde::Serialize)]
struct HntBalance {
    amount: u64,
    locked_amount: u64,
    total_amount: u64,
    mint: String,
    decimals: u8,
}

impl From<HeliumBalances> for Balances {
    fn from(value: HeliumBalances) -> Self {
        Self {
            hnt: HntBalance::from(value.hnt),
            iot: DntBalance::from(value.iot),
            mobile: DntBalance::from(value.mobile),
        }
    }
}

impl From<accounts::Balance> for DntBalance {
    fn from(b: accounts::Balance) -> Self {
        Self {
            amount: b.amount,
            pending_amount: 0,
            total_amount: b.amount,
            mint: b.mint,
            decimals: b.decimals,
        }
    }
}

impl From<accounts::Balance> for HntBalance {
    fn from(b: accounts::Balance) -> Self {
        Self {
            amount: b.amount,
            locked_amount: 0,
            total_amount: b.amount,
            mint: b.mint,
            decimals: b.decimals,
        }
    }
}
pub async fn get_account(
    Extension(rpc_client): Extension<Arc<RpcClient>>,
    Extension(positions): Extension<Arc<Mutex<positions::Memory>>>,
    Path(account): Path<String>,
) -> HandlerResult {
    if let Ok(pubkey) = Pubkey::from_str(&account) {
        match HeliumBalances::fetch(&rpc_client, &pubkey).await {
            Err(e) => {
                println!("Error fetching account: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
            Ok(balances) => {
                let mut balances = Balances::from(balances);
                let positions = positions.lock().await;
                if let Some(owned_positions) = positions.positions_by_owner.get(&pubkey) {
                    let mut list_of_positions = Vec::new();
                    for p in owned_positions {
                        match positions.position.get(p) {
                            None => {
                                let error = format!("Expected to find position {p} for account {pubkey} but none found!");
                                println!("{error}");
                                return Err((StatusCode::INTERNAL_SERVER_ERROR, error));
                            }
                            Some(p) => {
                                balances.hnt.locked_amount += p.hnt_amount;
                                if let Some(delegated) = &p.delegated {
                                    match delegated.sub_dao {
                                        SubDao::Iot => {
                                            balances.iot.pending_amount +=
                                                delegated.pending_rewards;
                                            balances.iot.total_amount += delegated.pending_rewards;
                                        }
                                        SubDao::Mobile => {
                                            balances.mobile.pending_amount +=
                                                delegated.pending_rewards;
                                            balances.mobile.total_amount +=
                                                delegated.pending_rewards;
                                        }
                                        SubDao::Unknown => {
                                            println!("Unknown subdao for delegated position {} for account {pubkey}!", p.position_key);
                                        }
                                    }
                                }
                                list_of_positions.push(p)
                            }
                        }
                    }
                    Ok(response::Json(json!({
                        "balances": balances,
                        "positions": list_of_positions,
                    })))
                } else {
                    Ok(response::Json(json!({
                        "balances": balances,
                    })))
                }
            }
        }
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            format!("\"{account}\" is not a valid base58 encoded Solana pubkey"),
        ))
    }
}
