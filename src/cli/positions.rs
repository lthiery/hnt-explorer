use super::*;
use std::collections::HashMap;

#[derive(Debug, Clone, clap::Args)]
/// Fetches all delegated positions and total HNT, veHNT, and subDAO delegations.
pub struct Positions {
    #[arg(short, long)]
    verify: bool,
}

use helium_sub_daos::{
    caclulate_vhnt_info, DelegatedPositionV0, PrecisePosition, SubDaoV0, VehntInfo as VehntInfoRaw,
};
use voter_stake_registry::state::{LockupKind, PositionV0, VotingMintConfigV0, PRECISION_FACTOR};

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
pub struct PositionData {
    pub timestamp: i64,
    pub positions: Vec<Position>,
    #[serde(skip_serializing)]
    pub delegated_positions: Vec<PositionLegacy>,
    pub positions_total_len: usize,
    pub network: Data,
    pub undelegated: Data,
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
    pub vehnt: u128,
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

impl PositionData {
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
        self.undelegated.scale_down();
    }
}

pub async fn get_data(rpc_client: &RpcClient) -> Result<PositionData> {
    let mut d = PositionData::new();
    let positions_data = locked::get_data(rpc_client).await?;

    let voting_mint_config = &positions_data.mint_configs
        [&Pubkey::from_str("hntyVP6YFm1Hg25TN9WGLqM12b8TQmcknKrdu1oxWux")?];
    let mut positions = HashMap::new();
    for (pubkey, position) in positions_data.positions {
        if let Some(mint) = positions_data.registrar_to_mint.get(&position.registrar) {
            if mint.to_string().as_str() == HNT_MINT {
                let position = Position::try_from_positionv0(
                    pubkey,
                    position,
                    d.timestamp,
                    voting_mint_config,
                )?;
                positions.insert(pubkey, position);
            }
        } else {
            println!("No mint found for registrar {}", position.registrar)
        }
    }

    let accounts = get_stake_accounts(rpc_client).await?;
    let delegated_positions = accounts
        .iter()
        .map(|(pubkey, account)| {
            let mut data = account.data.as_slice();
            DelegatedPositionV0::try_deserialize(&mut data).map(|position| (pubkey, position))
        })
        .filter(|result| result.is_ok())
        .collect::<AnchorResult<Vec<_>>>()?;

    for (pubkey, delegated_position) in delegated_positions {
        let delegated_position = delegated_position;
        let mut position = positions.get_mut(&delegated_position.position).unwrap();
        position.delegated = Some(DelegatedPosition::try_from_delegated_position_v0(
            *pubkey,
            delegated_position,
        )?);
        d.delegated_positions
            .push(PositionLegacy::from(position.clone()));
    }

    let mut hnt_amounts = vec![];
    let mut vehnt_amounts = vec![];

    let mut lockups = vec![];
    for (_, position) in positions {
        let duration = (position.end_ts - position.start_ts) as u128;
        d.network.total.hnt += position.hnt_amount;
        d.network.total.fall_rate += position.vehnt_info.pre_genesis_end_fall_rate;
        d.network.total.vehnt += position.vehnt;
        d.network.total.count += 1;
        d.network.total.lockup += duration;

        if let Some(delegated) = &position.delegated {
            hnt_amounts.push((position.hnt_amount, delegated.sub_dao));
            vehnt_amounts.push((position.vehnt, delegated.sub_dao));
            lockups.push((duration, delegated.sub_dao));
            match delegated.sub_dao {
                SubDao::Mobile => {
                    d.mobile.total.hnt += position.hnt_amount;
                    d.mobile.total.fall_rate += position.vehnt_info.pre_genesis_end_fall_rate;
                    d.mobile.total.vehnt += position.vehnt;
                    d.mobile.total.count += 1;
                    d.mobile.total.lockup += duration;
                }
                SubDao::Iot => {
                    d.iot.total.hnt += position.hnt_amount;
                    d.iot.total.fall_rate += position.vehnt_info.pre_genesis_end_fall_rate;
                    d.iot.total.vehnt += position.vehnt;
                    d.iot.total.count += 1;
                    d.iot.total.lockup += duration;
                }
                SubDao::Unknown => {
                    return Err(Error::Custom("Unknown subdao"));
                }
            }
        } else {
            hnt_amounts.push((position.hnt_amount, SubDao::Unknown));
            vehnt_amounts.push((position.vehnt, SubDao::Unknown));
            lockups.push((duration, SubDao::Unknown));
            d.undelegated.total.hnt += position.hnt_amount;
            d.undelegated.total.fall_rate += position.vehnt_info.pre_genesis_end_fall_rate;
            d.undelegated.total.vehnt += position.vehnt;
            d.undelegated.total.count += 1;
            d.undelegated.total.lockup += duration;
        }
        let mut position_copy = position.clone();
        position_copy.vehnt /= PRECISION_FACTOR;
        d.positions.push(position_copy);
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
    d.undelegated.stats = get_stats(
        Some(SubDao::Unknown),
        d.undelegated.total,
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

impl Positions {
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
            let delegated_hnt = d.mobile.total.vehnt + d.iot.total.vehnt;
            let undelegated_hnt = d.network.total.vehnt - delegated_hnt;
            println!(
                "Total MOBILE veHNT     :   {} ({}% of delegated)",
                format_vehnt(d.mobile.total.vehnt),
                percentage(d.mobile.total.vehnt, delegated_hnt)
            );
            println!(
                "Total IOT veHNT        : {} ({}% of delegated)",
                format_vehnt(d.iot.total.vehnt),
                percentage(d.iot.total.vehnt, delegated_hnt)
            );
            println!(
                "Total undelegated veHNT: {} ({}% of total)",
                format_vehnt(undelegated_hnt),
                percentage(undelegated_hnt, d.network.total.vehnt)
            );
            println!(
                "Total veHNT            : {}",
                format_vehnt(d.network.total.vehnt)
            );
        }
        println!("Total positions        :         {}", d.positions_total_len);

        println!(
            "Total HNT locked       :    {}",
            format_hnt(d.network.total.hnt)
        );
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

fn percentage(dao_vhnt: u128, total_vhnt: u128) -> String {
    let percent = (dao_vhnt as f64 / total_vhnt as f64) * 100.0;
    format!("{:.2}", percent)
}

use helium_api::models::Hnt;

#[derive(Clone, Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct Position {
    pub position_key: String,
    pub hnt_amount: u64,
    pub start_ts: i64,
    pub genesis_end_ts: i64,
    pub end_ts: i64,
    pub duration_s: i64,
    pub vehnt: u128,
    #[serde(skip_serializing)]
    pub vehnt_info: VehntInfo,
    pub lockup_type: LockupType,
    pub delegated: Option<DelegatedPosition>,
}

#[derive(Clone, Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct PositionLegacy {
    pub position_key: String,
    pub delegated_position_key: String,
    pub hnt_amount: Hnt,
    pub sub_dao: SubDao,
    pub last_claimed_epoch: u64,
    pub start_ts: i64,
    pub genesis_end_ts: i64,
    pub end_ts: i64,
    pub duration_s: i64,
    pub purged: bool,
    pub vehnt: u128,
    pub lockup_type: LockupType,
}

impl From<Position> for PositionLegacy {
    fn from(value: Position) -> Self {
        let delegated = value.delegated.unwrap();
        Self {
            position_key: value.position_key,
            delegated_position_key: delegated.delegated_position_key,
            last_claimed_epoch: delegated.last_claimed_epoch,
            sub_dao: delegated.sub_dao,
            hnt_amount: Hnt::from(value.hnt_amount),
            start_ts: value.start_ts,
            genesis_end_ts: value.genesis_end_ts,
            end_ts: value.end_ts,
            duration_s: value.duration_s,
            vehnt: value.vehnt / PRECISION_FACTOR,
            lockup_type: value.lockup_type,
            purged: false,
        }
    }
}

#[derive(Clone, Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct DelegatedPosition {
    pub delegated_position_key: String,
    pub sub_dao: SubDao,
    pub last_claimed_epoch: u64,
}

impl DelegatedPosition {
    fn try_from_delegated_position_v0(
        delegated_position_key: Pubkey,
        delegated_position: DelegatedPositionV0,
    ) -> Result<Self> {
        Ok(Self {
            delegated_position_key: delegated_position_key.to_string(),
            sub_dao: SubDao::try_from(delegated_position.sub_dao)?,
            last_claimed_epoch: delegated_position.last_claimed_epoch,
        })
    }
}

#[derive(Clone, Default, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockupType {
    Cliff,
    Constant,
    #[default]
    Unlocked,
}

impl Position {
    pub fn try_from_positionv0(
        position_key: Pubkey,
        position: PositionV0,
        timestamp: i64,
        voting_mint_config: &VotingMintConfigV0,
    ) -> Result<Self> {
        let vehnt_info = caclulate_vhnt_info(timestamp, &position, voting_mint_config)?;
        let vehnt = position.voting_power_precise(voting_mint_config, timestamp)?;

        Ok(Self {
            position_key: position_key.to_string(),
            hnt_amount: position.amount_deposited_native,
            start_ts: position.lockup.start_ts,
            end_ts: position.lockup.end_ts,
            genesis_end_ts: position.genesis_end,
            duration_s: position.lockup.end_ts - position.lockup.start_ts,
            vehnt,
            vehnt_info: vehnt_info.into(),
            delegated: None,
            lockup_type: match position.lockup.kind {
                LockupKind::Constant => LockupType::Constant,
                LockupKind::Cliff => LockupType::Cliff,
                LockupKind::None => LockupType::Unlocked,
            },
        })
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct VehntInfo {
    pub has_genesis: bool,
    pub vehnt_at_curr_ts: u128,
    pub pre_genesis_end_fall_rate: u128,
    pub post_genesis_end_fall_rate: u128,
    pub genesis_end_vehnt_correction: u128,
    pub genesis_end_fall_rate_correction: u128,
    pub end_vehnt_correction: u128,
    pub end_fall_rate_correction: u128,
}

impl From<VehntInfoRaw> for VehntInfo {
    fn from(value: VehntInfoRaw) -> Self {
        Self {
            has_genesis: value.has_genesis,
            vehnt_at_curr_ts: value.vehnt_at_curr_ts,
            pre_genesis_end_fall_rate: value.pre_genesis_end_fall_rate,
            post_genesis_end_fall_rate: value.post_genesis_end_fall_rate,
            genesis_end_vehnt_correction: value.genesis_end_vehnt_correction,
            genesis_end_fall_rate_correction: value.genesis_end_fall_rate_correction,
            end_vehnt_correction: value.end_vehnt_correction,
            end_fall_rate_correction: value.end_fall_rate_correction,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct VotingMintConfig {
    pub mint: Pubkey,
    pub baseline_vote_weight_scaled_factor: u64,
    pub max_extra_lockup_vote_weight_scaled_factor: u64,
    pub genesis_vote_power_multiplier: u8,
    pub genesis_vote_power_multiplier_expiration_ts: i64,
    pub lockup_saturation_secs: u64,
    pub digit_shift: i8,
}
