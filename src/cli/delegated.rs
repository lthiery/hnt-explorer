use super::*;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;

#[derive(Debug, Clone, clap::Args)]
/// Fetches all delegated positions and total HNT, veHNT, and subDAO delegations.
pub struct Delegated {
    #[arg(short, long)]
    verify: bool,
}

use helium_sub_daos::{caclulate_vhnt_info, DelegatedPositionV0, PrecisePosition, SubDaoV0};
use voter_stake_registry::state::{PositionV0, Registrar, PRECISION_FACTOR};

const ANOTHER_DIVIDER: u128 = TOKEN_DIVIDER * PRECISION_FACTOR;

#[allow(unused)]
/// This function can be used when a single query is too big
async fn get_stake_accounts_incremental(
    rpc_client: &RpcClient,
) -> Result<Vec<(Pubkey, solana_sdk::account::Account)>> {
    let mut prefix = [251, 212, 32, 100, 102, 1, 247, 81, 0];
    let mut accounts = Vec::new();
    for i in 0..255 {
        prefix[8] = i;
        accounts.extend(get_accounts_with_prefix(rpc_client, &prefix).await?);
    }
    Ok(accounts)
}

/// This function will work until there's too many to fetch in a single call
pub async fn get_stake_accounts(
    rpc_client: &RpcClient,
) -> Result<Vec<(Pubkey, solana_sdk::account::Account)>> {
    const DELEGATE_POSITION_V0_DESCRIMINATOR: [u8; 8] = [251, 212, 32, 100, 102, 1, 247, 81];
    get_accounts_with_prefix(rpc_client, &DELEGATE_POSITION_V0_DESCRIMINATOR).await
}

async fn get_accounts_with_prefix(
    rpc_client: &RpcClient,
    input: &[u8],
) -> Result<Vec<(Pubkey, solana_sdk::account::Account)>> {
    let helium_dao_id = Pubkey::from_str(HELIUM_DAO_ID)?;
    let mut config = RpcProgramAccountsConfig::default();
    let memcmp = RpcFilterType::Memcmp(Memcmp::new_base58_encoded(0, input));
    config.filters = Some(vec![RpcFilterType::DataSize(196), memcmp]);
    config.account_config.encoding = Some(UiAccountEncoding::Base64);
    let accounts = rpc_client
        .get_program_accounts_with_config(&helium_dao_id, config)
        .await?;
    Ok(accounts)
}

#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct DelegatedData {
    pub timestamp: i64,
    pub positions: Vec<PositionSaved>,
    pub positions_total_len: usize,
    pub network: Data,
    pub mobile: Data,
    pub iot: Data,
}

#[derive(Default, Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Data {
    pub total: Total,
    pub stats: Stats,
}

#[derive(Default, Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Total {
    count: usize,
    vehnt: u128,
    hnt: u64,
    lockup: u128,
    fall_rate: u128,
}

#[derive(Default, Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Stats {
    avg_vehnt: u128,
    median_vehnt: u128,
    avg_hnt: u64,
    median_hnt: u64,
    avg_lockup: u128,
    median_lockup: u128,
}

impl std::fmt::Display for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Stats {{")?;
        write!(f, "count: {}, ", self.total.count)?;
        write!(f, "vehnt: {}, ", format_vehnt(self.total.vehnt))?;
        write!(f, "fall_rate: {}, ", self.total.fall_rate)?;
        write!(
            f,
            "total_duration: {} days, ",
            self.total.lockup / (60 * 60 * 24)
        )?;
        write!(f, "avg_vehnt: {}, ", format_vehnt(self.stats.avg_vehnt))?;
        write!(
            f,
            "median_vehnt: {}, ",
            format_vehnt(self.stats.median_vehnt)
        )?;
        write!(f, "avg_hnt: {}, ", format_hnt(self.stats.avg_hnt))?;
        write!(f, "mediant_hnt: {}, ", format_hnt(self.stats.median_hnt))?;
        write!(
            f,
            "avg_lock_up: {} days, ",
            self.stats.avg_lockup / (60 * 60 * 24)
        )?;
        write!(
            f,
            "median_lock_up: {} days }}",
            self.stats.median_lockup / (60 * 60 * 24)
        )
    }
}

impl Data {
    fn scale_down(&mut self) {
        self.total.vehnt /= PRECISION_FACTOR;
        self.total.fall_rate /= PRECISION_FACTOR;
        self.total.lockup /= PRECISION_FACTOR;
        self.stats.avg_vehnt /= PRECISION_FACTOR;
        self.stats.median_vehnt /= PRECISION_FACTOR;
    }
}

impl DelegatedData {
    fn new() -> Self {
        let curr_ts = Utc::now().timestamp();
        Self {
            timestamp: curr_ts,
            ..Default::default()
        }
    }

    pub fn scale_down(&mut self) {
        self.network.scale_down();
        self.iot.scale_down();
        self.mobile.scale_down();
    }
}

pub async fn get_data(rpc_client: &RpcClient) -> Result<DelegatedData> {
    let mut d = DelegatedData::new();
    let accounts = get_stake_accounts(rpc_client).await?;
    let delegated_positions = accounts
        .iter()
        .map(|(pubkey, account)| {
            let mut data = account.data.as_slice();
            DelegatedPositionV0::try_deserialize(&mut data).map(|position| (pubkey, position))
        })
        .filter(|result| result.is_ok())
        .collect::<AnchorResult<Vec<_>>>()?;

    let position_keys = delegated_positions
        .iter()
        .map(|(_pubkey, delegated_position)| delegated_position.position)
        .collect::<Vec<Pubkey>>();

    let mut positions_raw = Vec::new();
    const BATCH_SIZE: usize = 100;
    for chunk in position_keys.chunks(BATCH_SIZE) {
        let chunk_positions_raw = rpc_client.get_multiple_accounts(chunk).await?;
        positions_raw.extend(chunk_positions_raw);
    }

    let positions_with_delegations: Vec<FullPositionRaw> = delegated_positions
        .iter()
        .zip(positions_raw)
        .map(|((pubkey, delegated_position), position)| {
            let position_unwrapped = position.unwrap();
            let mut data = position_unwrapped.data.as_slice();
            let position_parsed = PositionV0::try_deserialize(&mut data).unwrap();
            FullPositionRaw {
                position_key: delegated_position.position,
                delegated_position_key: **pubkey,
                position: position_parsed,
                delegated_position: delegated_position.clone(),
            }
        })
        .collect();

    let registrar_keys: Vec<Pubkey> = positions_with_delegations
        .iter()
        .map(|p| p.position.registrar)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let registrars_raw = rpc_client.get_multiple_accounts(&registrar_keys).await?;
    let registrars: Vec<Registrar> = registrars_raw
        .iter()
        .map(|registrar| {
            let mut data = registrar.as_ref().unwrap().data.as_slice();
            Registrar::try_deserialize(&mut data)
        })
        .map(|result| result.unwrap())
        .collect();

    let voting_mint_config = registrars[0].voting_mints[0].clone();
    let mut hnt_amounts = vec![];
    let mut vehnt_amounts = vec![];
    let mut lockups = vec![];
    for position in positions_with_delegations {
        let vehnt_info = caclulate_vhnt_info(d.timestamp, &position.position, &voting_mint_config)?;
        let vehnt = position
            .position
            .voting_power_precise(&voting_mint_config, d.timestamp)?;
        let duration =
            (position.position.lockup.end_ts - position.position.lockup.start_ts) as u128;
        d.network.total.hnt += position.delegated_position.hnt_amount;
        d.network.total.fall_rate += vehnt_info.pre_genesis_end_fall_rate;
        d.network.total.vehnt += vehnt;
        d.network.total.count += 1;
        d.network.total.lockup += duration;

        let saved_position = PositionSaved::try_from_full_position_raw(&position, vehnt)?;
        hnt_amounts.push((
            position.delegated_position.hnt_amount,
            saved_position.sub_dao,
        ));
        vehnt_amounts.push((vehnt, saved_position.sub_dao));
        lockups.push((duration, saved_position.sub_dao));
        d.positions.push(saved_position);
        match SubDao::try_from(position.delegated_position.sub_dao).unwrap() {
            SubDao::Mobile => {
                d.mobile.total.hnt += position.delegated_position.hnt_amount;
                d.mobile.total.fall_rate += vehnt_info.pre_genesis_end_fall_rate;
                d.mobile.total.vehnt += vehnt;
                d.mobile.total.count += 1;
                d.mobile.total.lockup += duration;
            }
            SubDao::Iot => {
                d.iot.total.hnt += position.delegated_position.hnt_amount;
                d.iot.total.fall_rate += vehnt_info.pre_genesis_end_fall_rate;
                d.iot.total.vehnt += vehnt;
                d.iot.total.count += 1;
                d.iot.total.lockup += duration;
            }
            SubDao::Unknown => {
                return Err(Error::Custom("Unknown subdao"));
            }
        }
    }
    d.positions_total_len = d.positions.len();

    d.network.stats = get_stats(
        None,
        d.network.total,
        vehnt_amounts.clone(),
        hnt_amounts.clone(),
        lockups.clone(),
    );
    d.iot.stats = get_stats(
        Some(SubDao::Iot),
        d.iot.total,
        vehnt_amounts.clone(),
        hnt_amounts.clone(),
        lockups.clone(),
    );
    d.mobile.stats = get_stats(
        Some(SubDao::Mobile),
        d.mobile.total,
        vehnt_amounts.clone(),
        hnt_amounts.clone(),
        lockups.clone(),
    );

    Ok(d)
}

fn get_stats(
    filter: Option<SubDao>,
    total: Total,
    mut vehnt_amounts: Vec<(u128, SubDao)>,
    mut hnt_amounts: Vec<(u64, SubDao)>,
    mut lockups: Vec<(u128, SubDao)>,
) -> Stats {
    if let Some(filter) = filter {
        vehnt_amounts.retain(|(_, sub_dao)| *sub_dao == filter);
        hnt_amounts.retain(|(_, sub_dao)| *sub_dao == filter);
        lockups.retain(|(_, sub_dao)| *sub_dao == filter);
    }

    let mut stats = Stats::default();
    vehnt_amounts.sort_by(|a, b| b.0.cmp(&a.0));
    stats.median_vehnt = vehnt_amounts[total.count / 2].0;
    stats.avg_vehnt = total.vehnt / total.count as u128;

    hnt_amounts.sort_by(|a, b| b.0.cmp(&a.0));
    stats.median_hnt = hnt_amounts[total.count / 2].0;
    stats.avg_hnt = total.hnt / total.count as u64;

    lockups.sort_by(|a, b| b.0.cmp(&a.0));
    stats.median_lockup = lockups[total.count / 2].0;
    stats.avg_lockup = total.lockup / total.count as u128;
    stats
}

impl Delegated {
    pub async fn run(self, rpc_client: RpcClient) -> Result {
        let d = get_data(&rpc_client).await?;
        if self.verify {
            let iot_sub_dao_raw = rpc_client
                .get_account(&Pubkey::from_str(IOT_SUBDAO).unwrap())
                .await?;
            let iot_sub_dao = SubDaoV0::try_deserialize(&mut iot_sub_dao_raw.data.as_slice())?;
            let mobile_sub_dao_raw = rpc_client
                .get_account(&Pubkey::from_str(MOBILE_SUBDAO).unwrap())
                .await?;
            let mobile_sub_dao =
                SubDaoV0::try_deserialize(&mut mobile_sub_dao_raw.data.as_slice())?;

            let mobile_vehnt_est = mobile_sub_dao.vehnt_delegated
                + mobile_sub_dao.vehnt_fall_rate
                    * u128::try_from(d.timestamp - mobile_sub_dao.vehnt_last_calculated_ts)
                        .unwrap();

            let iot_vehnt_est = iot_sub_dao.vehnt_delegated
                + iot_sub_dao.vehnt_fall_rate
                    * u128::try_from(d.timestamp - iot_sub_dao.vehnt_last_calculated_ts).unwrap();

            println!("Total MOBILE veHNT : {}", d.mobile.total.vehnt);
            println!("Est MOBILE veHNT   : {}", mobile_vehnt_est);
            println!(
                "MOBILE veHNT Diff  : {}",
                i128::try_from(mobile_vehnt_est).unwrap()
                    - i128::try_from(d.mobile.total.vehnt).unwrap()
            );
            println!("Total IOT veHNT    : {}", d.iot.total.vehnt);
            println!("Est IOT veHNT      : {}", iot_vehnt_est);
            println!(
                "IOT veHNT Diff     : {}",
                i128::try_from(iot_vehnt_est).unwrap() - i128::try_from(d.iot.total.vehnt).unwrap()
            );
            println!("Total veHNT        : {}", d.network.total.vehnt);
            println!("mobile fall        : {}", d.mobile.total.fall_rate);
            println!("mobile est fall    : {}", mobile_sub_dao.vehnt_fall_rate);
            println!(
                "mobile diff        : {}",
                i128::try_from(mobile_sub_dao.vehnt_fall_rate).unwrap()
                    - i128::try_from(d.mobile.total.fall_rate).unwrap()
            );
            println!("iot fall           : {}", d.iot.total.fall_rate);
            println!("iot est fall       : {}", iot_sub_dao.vehnt_fall_rate);
            println!(
                "iot diff           : {}",
                i128::try_from(iot_sub_dao.vehnt_fall_rate).unwrap()
                    - i128::try_from(d.iot.total.fall_rate).unwrap()
            );
        } else {
            println!(
                "Total MOBILE veHNT : {} ({}%)",
                format_vehnt(d.mobile.total.vehnt),
                percentage(d.mobile.total.vehnt, d.network.total.vehnt)
            );
            println!(
                "Total IOT veHNT    : {} ({}%)",
                format_vehnt(d.iot.total.vehnt),
                percentage(d.iot.total.vehnt, d.network.total.vehnt)
            );
            println!(
                "Total veHNT        : {}",
                format_vehnt(d.network.total.vehnt)
            );
        }
        println!("Total positions    :        {}", d.positions_total_len);

        println!("Total HNT delegated:   {}", format_hnt(d.network.total.hnt));
        println!(
            "Average multiple   :         {}",
            (d.network.total.vehnt / ANOTHER_DIVIDER) as u64
                / (d.network.total.hnt / TOKEN_DIVIDER as u64)
        );
        println!(
            "Average veHNT size:      {}",
            format_vehnt(d.network.stats.avg_vehnt)
        );
        println!(
            "Median veHNT size    :      {}",
            format_vehnt(d.network.stats.median_vehnt)
        );

        println!("Network {}", d.network);
        println!("MOBILE {}", d.mobile);
        println!("IOT {}", d.iot);

        Ok(())
    }
}

fn format_vehnt(vehnt: u128) -> String {
    let vehnt = vehnt / ANOTHER_DIVIDER;
    vehnt
        .to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<std::result::Result<Vec<&str>, _>>()
        .unwrap()
        .join(",")
}

fn percentage(dao_vhnt: u128, total_vhnt: u128) -> String {
    let percent = (dao_vhnt as f64 / total_vhnt as f64) * 100.0;
    format!("{:.2}", percent)
}

fn format_hnt(vehnt: u64) -> String {
    let vehnt = vehnt / TOKEN_DIVIDER as u64;
    vehnt
        .to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<std::result::Result<Vec<&str>, _>>()
        .unwrap()
        .join(",")
}

use helium_api::models::Hnt;

#[derive(Debug)]
pub struct DelegatedPosition {
    pub mint: Pubkey,
    pub position: Pubkey,
    pub hnt_amount: Hnt,
    pub sub_dao: SubDao,
    pub last_claimed_epoch: u64,
    pub start_ts: i64,
    pub purged: bool,
}

#[derive(Clone, Default, Debug, serde::Serialize, serde::Deserialize)]
/// Saves some high level info about the position
pub struct PositionSaved {
    pub position_key: String,
    pub delegated_position_key: String,
    pub hnt_amount: Hnt,
    pub sub_dao: SubDao,
    pub last_claimed_epoch: u64,
    pub start_ts: i64,
    pub purged: bool,
    pub vehnt: Hnt,
}

impl PositionSaved {
    pub fn try_from_full_position_raw(position: &FullPositionRaw, vehnt: u128) -> Result<Self> {
        let delegated_position = DelegatedPosition::try_from(position.delegated_position.clone())?;
        Ok(Self {
            position_key: position.delegated_position_key.to_string(),
            delegated_position_key: position.delegated_position_key.to_string(),
            hnt_amount: delegated_position.hnt_amount,
            sub_dao: delegated_position.sub_dao,
            last_claimed_epoch: delegated_position.last_claimed_epoch,
            start_ts: delegated_position.start_ts,
            purged: delegated_position.purged,
            vehnt: Hnt::from(u64::try_from(vehnt / PRECISION_FACTOR)?),
        })
    }
}

impl TryFrom<DelegatedPositionV0> for DelegatedPosition {
    type Error = Error;
    fn try_from(position: DelegatedPositionV0) -> Result<Self> {
        Ok(Self {
            mint: position.mint,
            position: position.position,
            hnt_amount: Hnt::from(position.hnt_amount),
            sub_dao: SubDao::try_from(position.sub_dao)?,
            last_claimed_epoch: position.last_claimed_epoch,
            start_ts: position.start_ts,
            purged: position.purged,
        })
    }
}

pub struct FullPositionRaw {
    pub position_key: Pubkey,
    pub delegated_position_key: Pubkey,
    pub position: PositionV0,
    pub delegated_position: DelegatedPositionV0,
}
