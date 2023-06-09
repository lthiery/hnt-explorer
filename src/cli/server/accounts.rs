use super::{*, positions};
use axum::extract::Path;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::cli::accounts::{self, HeliumBalances};
use crate::SubDao;

#[derive(serde::Serialize)]
pub struct Balances {
    vehnt: VehntBalance,
    hnt: HntBalance,
    iot: DntBalance,
    mobile: DntBalance,
}

impl Balances {
    pub fn absorb_position_balances(&mut self, locked_balances: positions::LockedBalances) {
        self.vehnt = locked_balances.vehnt;
        self.hnt.locked_amount = locked_balances.locked_hnt;
        self.hnt.total_amount += locked_balances.locked_hnt;
        self.iot.absorb_pending_amount(locked_balances.pending_iot);
        self.mobile.absorb_pending_amount(locked_balances.pending_mobile);
    }
}
#[derive(serde::Serialize)]
pub struct DntBalance {
    amount: u64,
    pending_amount: u64,
    total_amount: u64,
    mint: String,
    decimals: u8,
}

impl DntBalance {
    pub fn absorb_pending_amount(&mut self, pending: u64)  {
        self.total_amount += pending;
        self.pending_amount = pending;
    }
}

#[derive(serde::Serialize)]
pub struct HntBalance {
    amount: u64,
    locked_amount: u64,
    total_amount: u64,
    mint: String,
    decimals: u8,
}

#[derive(serde::Serialize, Default, Copy, Clone, Debug)]
pub struct VehntBalance {
    pub total: u128,
    pub iot_delegated: u128,
    pub mobile_delegated: u128,
    pub undelegated: u128,
}

impl From<HeliumBalances> for Balances {
    fn from(value: HeliumBalances) -> Self {
        Self {
            vehnt: VehntBalance::default(),
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
    Extension(positions): Extension<Arc<Mutex<Option<positions::Memory>>>>,
    Path(account): Path<String>,
) -> HandlerResult {
    if let Ok(pubkey) = Pubkey::from_str(&account) {
        match HeliumBalances::fetch(&rpc_client, &pubkey).await {
            Err(e) => {
                println!("Error fetching account: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, "Error fetching account".to_string()))
            }
            Ok(balances) => {
                let mut balances = Balances::from(balances);
                let positions = positions.lock().await;
                if positions.is_none() {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        DATA_NOT_INIT_MSG.to_string(),
                    ));
                }
                let positions = positions.as_ref().unwrap();

                if let Some(account) = positions.positions_by_owner.get(&pubkey) {
                    balances.absorb_position_balances(account.balances);
                    let mut list_of_positions = Vec::new();
                    for p in account.positions.iter() {
                        match positions.position.get(p) {
                            None => {
                                let error = format!("Expected to find position {p} for account {pubkey} but none found!");
                                println!("{error}");
                                return Err((StatusCode::INTERNAL_SERVER_ERROR, error));
                            }
                            Some(p) => {
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

// pub async fn get_vehnt_richlist(
//     Extension(rpc_client): Extension<Arc<RpcClient>>,
//     Extension(positions): Extension<Arc<Mutex<Option<positions::Memory>>>>,
//     Path(account): Path<String>,
// ) -> HandlerResult {
//
//     let positions = positions.lock().await;
//     if positions.is_none() {
//         return Err((
//             StatusCode::INTERNAL_SERVER_ERROR,
//             DATA_NOT_INIT_MSG.to_string(),
//         ));
//     }
//     let positions = positions.as_ref().unwrap();
//     let positions_by_owner = positions.positions_by_owner.iter();
//     let balances_by_owner = positions_by_owner.map(
//         |owner, positions| {
//             let vehnt_balance = VehntBalance::default();
//             for p in positions {
//
//             }
//
//
//             (k, total)
//         }
//     )
//
//     Err((
//         StatusCode::BAD_REQUEST,
//         format!("\"{account}\" is not a valid base58 encoded Solana pubkey"),
//     ))
// }
