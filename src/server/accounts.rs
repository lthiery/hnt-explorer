use super::{positions, *};
use anchor_lang::solana_program::pubkey::Pubkey;
use axum::extract::Path;
use std::str::FromStr;

use crate::cli::accounts::{self, HeliumBalances};
use crate::server::positions::{Dao, LockedBalances};

#[derive(serde::Serialize, Deserialize, Default, PartialEq, Debug, Copy, Clone)]
pub struct Balances {
    pub vehnt: VehntBalance,
    pub hnt: HntBalance,
    pub iot: DntBalance,
    pub mobile: DntBalance,
    pub veiot: u128,
    pub vemobile: u128,
}

impl Add for Balances {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            vehnt: self.vehnt + other.vehnt,
            hnt: self.hnt + other.hnt,
            iot: self.iot + other.iot,
            mobile: self.mobile + other.mobile,
            veiot: self.veiot + other.veiot,
            vemobile: self.vemobile + other.vemobile,
        }
    }
}

impl Balances {
    pub fn absorb_position_balances(&mut self, locked_balances: LockedBalances) {
        self.vehnt = locked_balances.vehnt;
        self.veiot = locked_balances.veiot;
        self.vemobile = locked_balances.vemobile;

        self.hnt.locked_amount = locked_balances.locked_hnt;
        self.hnt.total_amount += locked_balances.locked_hnt;
        self.iot.absorb_pending_amount(locked_balances.pending_iot);
        self.iot.absorb_locked_amount(locked_balances.locked_iot);
        self.mobile
            .absorb_pending_amount(locked_balances.pending_mobile);
        self.mobile
            .absorb_locked_amount(locked_balances.locked_mobile);
    }
}
#[derive(serde::Serialize, Deserialize, Default, Copy, Clone, Debug, PartialEq)]
pub struct DntBalance {
    pub amount: u64,
    pub locked_amount: u64,
    pub pending_amount: u64,
    pub total_amount: u64,
}

impl Add for DntBalance {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            amount: self.amount + other.amount,
            locked_amount: self.locked_amount + other.locked_amount,
            pending_amount: self.pending_amount + other.pending_amount,
            total_amount: self.total_amount + other.total_amount,
        }
    }
}

impl DntBalance {
    pub fn absorb_pending_amount(&mut self, locked: u64) {
        self.total_amount += locked;
        self.pending_amount = locked;
    }

    pub fn absorb_locked_amount(&mut self, locked: u64) {
        self.total_amount += locked;
        self.locked_amount = locked;
    }
}

#[derive(serde::Serialize, Deserialize, Default, Copy, Clone, Debug, PartialEq)]
pub struct HntBalance {
    pub amount: u64,
    pub locked_amount: u64,
    pub total_amount: u64,
}

impl Add for HntBalance {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            amount: self.amount + other.amount,
            locked_amount: self.locked_amount + other.locked_amount,
            total_amount: self.total_amount + other.total_amount,
        }
    }
}

#[derive(serde::Serialize, Deserialize, Default, Copy, Clone, Debug, PartialEq)]
pub struct VehntBalance {
    pub total: u128,
    pub iot_delegated: u128,
    pub mobile_delegated: u128,
    pub undelegated: u128,
}

impl Add for VehntBalance {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            total: self.total + other.total,
            iot_delegated: self.iot_delegated + other.iot_delegated,
            mobile_delegated: self.mobile_delegated + other.mobile_delegated,
            undelegated: self.undelegated + other.undelegated,
        }
    }
}

impl From<HeliumBalances> for Balances {
    fn from(value: HeliumBalances) -> Self {
        Self {
            vehnt: VehntBalance::default(),
            hnt: HntBalance::from(value.hnt),
            iot: DntBalance::from(value.iot),
            mobile: DntBalance::from(value.mobile),
            veiot: 0,
            vemobile: 0,
        }
    }
}

impl From<accounts::Balance> for DntBalance {
    fn from(b: accounts::Balance) -> Self {
        Self {
            amount: b.amount,
            locked_amount: 0,
            pending_amount: 0,
            total_amount: b.amount,
        }
    }
}

impl From<accounts::Balance> for HntBalance {
    fn from(b: accounts::Balance) -> Self {
        Self {
            amount: b.amount,
            locked_amount: 0,
            total_amount: b.amount,
        }
    }
}

#[derive(Deserialize, Default)]
pub struct FullPositions {
    pub vehnt: Vec<positions::Position>,
    pub vemobile: Vec<positions::Position>,
    pub veiot: Vec<positions::Position>,
}
#[derive(Deserialize, Default)]
pub struct Account {
    #[serde(default = "FullPositions::default")]
    pub positions: FullPositions,
    pub balances: Balances,
}

pub async fn get_account(
    Extension(rpc_client): Extension<Arc<rpc::Client>>,
    Extension(positions): Extension<Arc<Mutex<Option<positions::Memory>>>>,
    Path(account): Path<String>,
) -> HandlerResult {
    if let Ok(pubkey) = Pubkey::from_str(&account) {
        match HeliumBalances::fetch(&rpc_client, &pubkey).await {
            Err(e) => {
                println!("Error fetching account: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Error fetching account".to_string(),
                ))
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

                #[derive(serde::Serialize, Default)]
                pub struct Positions<'a> {
                    vehnt: Vec<&'a positions::Position>,
                    vemobile: Vec<&'a positions::Position>,
                    veiot: Vec<&'a positions::Position>,
                }

                if let Some(account) = positions.positions_by_owner.get(&pubkey) {
                    balances.absorb_position_balances(account.balances);

                    fn position_keys_to_positions<'a>(
                        account: &Pubkey,
                        account_positions: &Vec<Pubkey>,
                        positions: &'a HashMap<Pubkey, positions::Position>,
                    ) -> std::result::Result<Vec<&'a positions::Position>, (StatusCode, String)>
                    {
                        let mut list_of_positions: Vec<&'a positions::Position> = Vec::new();
                        for p in account_positions {
                            match positions.get(p) {
                                None => {
                                    let error = format!("Expected to find position {p} for account {account} but none found!");
                                    println!("{error}");
                                    return Err((StatusCode::INTERNAL_SERVER_ERROR, error));
                                }
                                Some(p) => list_of_positions.push(p),
                            }
                        }
                        Ok(list_of_positions)
                    }
                    let list_of_positions = Positions {
                        vehnt: position_keys_to_positions(
                            &pubkey,
                            &account.positions.vehnt,
                            &positions.vehnt_positions,
                        )?,
                        veiot: position_keys_to_positions(
                            &pubkey,
                            &account.positions.veiot,
                            &positions.veiot_positions,
                        )?,
                        vemobile: position_keys_to_positions(
                            &pubkey,
                            &account.positions.vemobile,
                            &positions.vemobile_positions,
                        )?,
                    };
                    Ok(response::Json(json!({
                        "balances": balances,
                        "positions": list_of_positions,
                    }))
                    .into())
                } else {
                    Ok(response::Json(json!({
                        "balances": balances,
                    }))
                    .into())
                }
            }
        }
    } else {
        match  helium_crypto::public_key::PublicKey::from_str(&account) {
            Ok(helium_pubkey) => {
                match Pubkey::try_from(helium_pubkey) {
                    Ok(solana_pubkey) => {
                        Ok(response::Redirect::to(&format!("/v1/accounts/{}", solana_pubkey)).into())
                    }
                    Err(e) => {
                        Err((
                            StatusCode::BAD_REQUEST,
                            format!("\"{account}\" is a valid Helium pubkey but cannot be converted to Solana: {e}"),
                        ))
                    }
                }
            }
            Err(_) => {
                Err((
                    StatusCode::BAD_REQUEST,
                    format!("\"{account}\" is neither a valid base58 encoded Solana or Helium pubkey."),
                ))
            }
        }
    }
}

#[derive(serde::Serialize)]
struct TopResult {
    #[serde(skip_serializing)]
    pub dao: Dao,
    pub pubkey: String,
    pub positions: Positions,
    #[serde(flatten)]
    pub locked_balances: LockedBalances,
}

#[derive(serde::Serialize, Deserialize)]
pub struct Positions {
    vehnt: Vec<String>,
    veiot: Vec<String>,
    vemobile: Vec<String>,
}

impl From<&positions::account::Positions> for Positions {
    fn from(value: &positions::account::Positions) -> Self {
        Positions {
            vehnt: value.vehnt.iter().map(|p| p.to_string()).collect(),
            veiot: value.veiot.iter().map(|p| p.to_string()).collect(),
            vemobile: value.vemobile.iter().map(|p| p.to_string()).collect(),
        }
    }
}

use std::cmp::{Ord, Ordering};
impl Ord for TopResult {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.dao {
            Dao::Hnt => self
                .locked_balances
                .vehnt
                .total
                .cmp(&other.locked_balances.vehnt.total),
            Dao::Mobile => self
                .locked_balances
                .vemobile
                .cmp(&other.locked_balances.vemobile),
            Dao::Iot => self.locked_balances.veiot.cmp(&other.locked_balances.veiot),
        }
    }
}
impl PartialEq for TopResult {
    fn eq(&self, other: &Self) -> bool {
        self.pubkey == other.pubkey
    }
}
impl Eq for TopResult {}
impl PartialOrd for TopResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub async fn get_top_vehnt_accounts(
    positions: Extension<Arc<Mutex<Option<positions::Memory>>>>,
) -> HandlerResult {
    get_top_dao_accounts(positions, Dao::Hnt).await
}

pub async fn get_top_vemobile_accounts(
    positions: Extension<Arc<Mutex<Option<positions::Memory>>>>,
) -> HandlerResult {
    get_top_dao_accounts(positions, Dao::Mobile).await
}

pub async fn get_top_veiot_accounts(
    positions: Extension<Arc<Mutex<Option<positions::Memory>>>>,
) -> HandlerResult {
    get_top_dao_accounts(positions, Dao::Iot).await
}

pub async fn get_top_dao_accounts(
    Extension(positions): Extension<Arc<Mutex<Option<positions::Memory>>>>,
    dao: Dao,
) -> HandlerResult {
    let positions = positions.lock().await;
    if positions.is_none() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            DATA_NOT_INIT_MSG.to_string(),
        ));
    }
    let positions = positions.as_ref().unwrap();

    let mut owners_and_balances: Vec<TopResult> = positions
        .positions_by_owner
        .iter()
        .map(|(owner, account)| TopResult {
            dao,
            pubkey: owner.to_string(),
            positions: Positions::from(&account.positions),
            locked_balances: account.balances,
        })
        .collect();
    owners_and_balances.sort();
    owners_and_balances.reverse();
    owners_and_balances.truncate(100);

    Ok(response::Json(json!({
        "top": owners_and_balances,
    }))
    .into())
}
