use super::*;
use rust_decimal::prelude::ToPrimitive;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Clone, clap::Args)]
/// Fetches all delegated positions and total HNT, veHNT, and subDAO delegations.
pub struct Positions {
    #[arg(short, long)]
    verify: bool,
}

use helium_anchor_gen::{
    helium_sub_daos::{
        caclulate_vhnt_info, DelegatedPositionV0, PrecisePosition, SubDaoV0,
        VehntInfo as VehntInfoRaw,
    },
    voter_stake_registry::{
        LockupKind, PositionV0, PositionV0Trait, VotingMintConfigV0, PRECISION_FACTOR,
    },
};

#[allow(unused)]
/// This function can be used when a single query is too big
async fn get_stake_accounts_incremental(
    rpc_client: &rpc::Client,
) -> Result<Vec<(Pubkey, rpc::Account)>> {
    let mut prefix = [251, 212, 32, 100, 102, 1, 247, 81, 0];
    let mut accounts = Vec::new();
    for i in 0..255 {
        prefix[8] = i;
        accounts.extend(get_accounts_with_prefix(rpc_client, &prefix).await?);
    }
    Ok(accounts)
}

/// This function will work until there's too many to fetch in a single call
pub async fn get_delegated_positions(
    rpc_client: &rpc::Client,
) -> Result<Vec<(Pubkey, rpc::Account)>> {
    const DELEGATE_POSITION_V0_DESCRIMINATOR: [u8; 8] = [251, 212, 32, 100, 102, 1, 247, 81];
    get_accounts_with_prefix(rpc_client, &DELEGATE_POSITION_V0_DESCRIMINATOR).await
}

async fn get_accounts_with_prefix(
    rpc_client: &rpc::Client,
    input: &[u8],
) -> Result<Vec<(Pubkey, rpc::Account)>> {
    let helium_dao_id = Pubkey::from_str(HELIUM_DAO_ID)?;
    let memcmp = rpc::GetProgramAccountsFilter::Memcmp(rpc::Memcmp::new(0, input));
    let accounts = rpc_client
        .get_program_accounts_with_filter(&helium_dao_id, vec![memcmp])
        .await?;
    Ok(accounts)
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DaoPositionData {
    pub timestamp: i64,
    pub positions: Vec<Position>,
    pub positions_total_len: usize,
    #[serde(skip_serializing)]
    pub delegated_positions: Vec<PositionLegacy>,
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AllPositionsData {
    pub stats: Metadata,
    pub vehnt: DaoPositionData,
    pub vemobile: DaoPositionData,
    pub veiot: DaoPositionData,
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Metadata {
    pub timestamp: i64,
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

impl AllPositionsData {
    pub fn new() -> Self {
        let curr_ts = Utc::now().timestamp();
        Self {
            stats: Metadata::new(curr_ts),
            vehnt: DaoPositionData::new(curr_ts),
            vemobile: DaoPositionData::new(curr_ts),
            veiot: DaoPositionData::new(curr_ts),
        }
    }

    pub fn scale_down(&mut self) {
        self.stats.scale_down();
    }
}

impl Metadata {
    pub fn new(timestamp: i64) -> Self {
        Self {
            timestamp,
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

impl DaoPositionData {
    pub fn new(timestamp: i64) -> Self {
        Self {
            timestamp,
            ..Default::default()
        }
    }
}

pub async fn get_positions_of_mint(
    positions_data: &locked::Data,
    position_owners_map: &mut HashMap<Pubkey, Pubkey>,
    mint: &str,
    timestamp: i64,
) -> Result<(HashMap<Pubkey, PositionV0>, HashMap<Pubkey, Position>)> {
    let voting_mint_config = &positions_data.mint_configs[&Pubkey::from_str(mint)?];
    let voting_mint_config = VotingMintConfigV0 {
        mint: voting_mint_config.mint,
        baseline_vote_weight_scaled_factor: voting_mint_config.baseline_vote_weight_scaled_factor,
        max_extra_lockup_vote_weight_scaled_factor: voting_mint_config
            .max_extra_lockup_vote_weight_scaled_factor,
        genesis_vote_power_multiplier: voting_mint_config.genesis_vote_power_multiplier,
        genesis_vote_power_multiplier_expiration_ts: voting_mint_config
            .genesis_vote_power_multiplier_expiration_ts,
        lockup_saturation_secs: voting_mint_config.lockup_saturation_secs,
        digit_shift: voting_mint_config.digit_shift,
    };

    let mut positions_raw = HashMap::new();
    let mut positions = HashMap::new();

    for (pubkey, position) in positions_data.positions.iter() {
        if let Some(position_mint) = positions_data.registrar_to_mint.get(&position.registrar) {
            if position_mint.to_string().as_str() == mint {
                positions_raw.insert(*pubkey, *position);
                let owner: Result<Pubkey> = match position_owners_map.get(pubkey) {
                    Some(owner) => Ok(*owner),
                    None => {
                        let client = rpc::Client::default();
                        match client.get_owner_by_mint(&position.mint).await {
                            Ok(owner) => {
                                position_owners_map.insert(*pubkey, owner);
                                Ok(owner)
                            }
                            Err(e) => Err(e.into()),
                        }
                    }
                };
                match owner {
                    Err(e) => println!("Warning: could not get owner for position {pubkey}: {e}"),
                    Ok(owner) => {
                        let position = Position::try_from_positionv0(
                            owner,
                            *pubkey,
                            *position,
                            timestamp,
                            &voting_mint_config,
                        )
                        .await?;
                        positions.insert(*pubkey, position);
                    }
                }
            }
        } else {
            println!("No mint found for registrar {}", position.registrar)
        }
    }
    Ok((positions_raw, positions))
}
pub type PositionOwnersMap = HashMap<Pubkey, Pubkey>;
#[derive(Default)]
pub struct PositionOwners {
    pub vehnt: PositionOwnersMap,
    pub veiot: PositionOwnersMap,
    pub vemobile: PositionOwnersMap,
}

impl PositionOwners {
    pub fn is_empty(&self) -> bool {
        self.vehnt.is_empty() && self.veiot.is_empty() && self.vemobile.is_empty()
    }
}

pub async fn get_data(
    rpc_client: &rpc::Client,
    epoch_info: Arc<Vec<epoch_info::EpochSummary>>,
    position_owners_map: &mut PositionOwners,
) -> Result<AllPositionsData> {
    let mut all_data = AllPositionsData::new();
    let d = &mut all_data.vehnt;
    let s = &mut all_data.stats;

    let positions_data = locked::get_data(rpc_client).await?;
    // if the map is empty, we assume it hasn't been initialized and so we initialize it
    if position_owners_map.is_empty() {
        println!("Initializing position owners map");
        let client = rpc::Client::default();
        let position_keys = positions_data
            .positions
            .iter()
            .map(|p| &p.1.mint)
            .collect::<Vec<&Pubkey>>();
        let owners = client.get_all_owners_by_mint(&position_keys, 100).await?;
        positions_data
            .positions
            .iter()
            .zip(owners.iter())
            .for_each(|(p, k)| {
                if let Some(mint) = positions_data.registrar_to_mint.get(&p.1.registrar) {
                    match mint.to_string().as_ref() {
                        HNT_MINT => {
                            position_owners_map.vehnt.insert(p.0, *k);
                        }
                        IOT_MINT => {
                            position_owners_map.veiot.insert(p.0, *k);
                        }
                        MOBILE_MINT => {
                            position_owners_map.vemobile.insert(p.0, *k);
                        }
                        _ => println!("Warning: Unknown mint {} for position {}", mint, p.0),
                    }
                } else {
                    println!("Warning: No mint found for registrar {}", p.1.registrar)
                }
            });
    }

    let (_veiot_positions_raw, veiot_positions) = get_positions_of_mint(
        &positions_data,
        &mut position_owners_map.veiot,
        IOT_MINT,
        d.timestamp,
    )
    .await?;
    all_data.veiot.positions = veiot_positions
        .into_values()
        .map(|mut p| {
            p.voting_weight /= PRECISION_FACTOR;
            p
        })
        .collect();
    all_data.veiot.positions_total_len = all_data.veiot.positions.len();
    println!("veiot    positions: {:>#5}", all_data.veiot.positions.len());

    let (_vemobile_positions_raw, vemobile_positions) = get_positions_of_mint(
        &positions_data,
        &mut position_owners_map.vemobile,
        MOBILE_MINT,
        d.timestamp,
    )
    .await?;
    all_data.vemobile.positions = vemobile_positions
        .into_values()
        .map(|mut p| {
            p.voting_weight /= PRECISION_FACTOR;
            p
        })
        .collect();
    all_data.vemobile.positions_total_len = all_data.vemobile.positions.len();
    println!(
        "vemobile positions: {:>#5}",
        all_data.vemobile.positions.len()
    );

    let (vehnt_positions_raw, mut vehnt_positions) = get_positions_of_mint(
        &positions_data,
        &mut position_owners_map.vehnt,
        HNT_MINT,
        d.timestamp,
    )
    .await?;

    // this next section only applies to veHNT since veHNT can delegate towards subDAOs
    let delegated_positions = get_delegated_positions(rpc_client).await?;
    let delegated_positions = delegated_positions
        .iter()
        .map(|(pubkey, account)| {
            let mut data = account.data.as_slice();
            DelegatedPositionV0::try_deserialize(&mut data).map(|position| (pubkey, position))
        })
        .filter(|result| result.is_ok())
        .collect::<AnchorResult<Vec<_>>>()?;

    let voting_mint_config = &positions_data.mint_configs[&Pubkey::from_str(HNT_MINT)?];
    let voting_mint_config = VotingMintConfigV0 {
        mint: voting_mint_config.mint,
        baseline_vote_weight_scaled_factor: voting_mint_config.baseline_vote_weight_scaled_factor,
        max_extra_lockup_vote_weight_scaled_factor: voting_mint_config
            .max_extra_lockup_vote_weight_scaled_factor,
        genesis_vote_power_multiplier: voting_mint_config.genesis_vote_power_multiplier,
        genesis_vote_power_multiplier_expiration_ts: voting_mint_config
            .genesis_vote_power_multiplier_expiration_ts,
        lockup_saturation_secs: voting_mint_config.lockup_saturation_secs,
        digit_shift: voting_mint_config.digit_shift,
    };

    for (pubkey, delegated_position) in delegated_positions {
        let position_v0 = vehnt_positions_raw.get(&delegated_position.position);
        let position = vehnt_positions.get_mut(&delegated_position.position);
        match (position_v0, position) {
            (Some(position_v0), Some(position)) => {
                position.delegated = Some(DelegatedPosition::try_from_delegated_position_v0(
                    *pubkey,
                    delegated_position,
                    &epoch_info,
                    position_v0,
                    &voting_mint_config,
                )?);
                d.delegated_positions
                    .push(PositionLegacy::from(position.clone()));
            }
            (None, Some(_)) => println!(
                "Warning: could not find position_v0 for delegated position {}",
                delegated_position.position
            ),
            (Some(_), None) => println!(
                "Warning: could not find position for delegated position {}",
                delegated_position.position
            ),
            (None, None) => println!(
                "Warning: could find neither position nor position_v0 for delegated position {}. {}",
                delegated_position.position, delegated_position.purged,
            ),
        }
    }

    let mut hnt_amounts = vec![];
    let mut vehnt_amounts = vec![];
    let mut lockups = vec![];

    // stats for veHNT positions
    for (_, position) in vehnt_positions {
        let duration = (position.end_ts - position.start_ts) as u128;
        s.network.total.hnt += position.locked_tokens;
        s.network.total.fall_rate += position.vehnt_info.pre_genesis_end_fall_rate;
        s.network.total.vehnt += position.voting_weight;
        s.network.total.count += 1;
        s.network.total.lockup += duration;

        if let Some(delegated) = &position.delegated {
            hnt_amounts.push((position.locked_tokens, delegated.sub_dao));
            vehnt_amounts.push((position.voting_weight, delegated.sub_dao));
            lockups.push((duration, delegated.sub_dao));
            match delegated.sub_dao {
                SubDao::Mobile => {
                    s.mobile.total.hnt += position.locked_tokens;
                    s.mobile.total.fall_rate += position.vehnt_info.pre_genesis_end_fall_rate;
                    s.mobile.total.vehnt += position.voting_weight;
                    s.mobile.total.count += 1;
                    s.mobile.total.lockup += duration;
                }
                SubDao::Iot => {
                    s.iot.total.hnt += position.locked_tokens;
                    s.iot.total.fall_rate += position.vehnt_info.pre_genesis_end_fall_rate;
                    s.iot.total.vehnt += position.voting_weight;
                    s.iot.total.count += 1;
                    s.iot.total.lockup += duration;
                }
                SubDao::Unknown => {
                    return Err(Error::Custom("Unknown subdao"));
                }
            }
        } else {
            hnt_amounts.push((position.locked_tokens, SubDao::Unknown));
            vehnt_amounts.push((position.voting_weight, SubDao::Unknown));
            lockups.push((duration, SubDao::Unknown));
            s.undelegated.total.hnt += position.locked_tokens;
            s.undelegated.total.fall_rate += position.vehnt_info.pre_genesis_end_fall_rate;
            s.undelegated.total.vehnt += position.voting_weight;
            s.undelegated.total.count += 1;
            s.undelegated.total.lockup += duration;
        }
        let mut position_copy = position.clone();
        position_copy.voting_weight /= PRECISION_FACTOR;
        d.positions.push(position_copy);
    }
    d.positions_total_len = d.positions.len();
    println!("vehnt    positions: {:>#5}", d.positions.len());

    s.network.stats = get_stats(
        None,
        s.network.total,
        vehnt_amounts.clone(),
        hnt_amounts.clone(),
        lockups.clone(),
    );
    s.iot.stats = get_stats(
        Some(SubDao::Iot),
        s.iot.total,
        vehnt_amounts.clone(),
        hnt_amounts.clone(),
        lockups.clone(),
    );
    s.mobile.stats = get_stats(
        Some(SubDao::Mobile),
        s.mobile.total,
        vehnt_amounts.clone(),
        hnt_amounts.clone(),
        lockups.clone(),
    );
    s.undelegated.stats = get_stats(
        Some(SubDao::Unknown),
        s.undelegated.total,
        vehnt_amounts.clone(),
        hnt_amounts.clone(),
        lockups.clone(),
    );

    let total_positions = all_data.vehnt.positions_total_len
        + all_data.vemobile.positions_total_len
        + all_data.veiot.positions_total_len;
    println!("Organized data for positions {} positions", total_positions);

    Ok(all_data)
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
    pub async fn run(self, rpc_client: rpc::Client) -> Result {
        let epoch_summaries = epoch_info::get_epoch_summaries(&rpc_client).await?;
        let all_data = get_data(
            &rpc_client,
            epoch_summaries.into(),
            &mut PositionOwners::default(),
        )
        .await?;
        let d = all_data.vehnt;
        let s = all_data.stats;

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

            println!("Total MOBILE veHNT : {}", s.mobile.total.vehnt);
            println!("Est MOBILE veHNT   : {}", mobile_vehnt_est);
            println!(
                "MOBILE veHNT Diff  : {}",
                i128::try_from(mobile_vehnt_est).unwrap()
                    - i128::try_from(s.mobile.total.vehnt).unwrap()
            );
            println!("Total IOT veHNT    : {}", s.iot.total.vehnt);
            println!("Est IOT veHNT      : {}", iot_vehnt_est);
            println!(
                "IOT veHNT Diff     : {}",
                i128::try_from(iot_vehnt_est).unwrap() - i128::try_from(s.iot.total.vehnt).unwrap()
            );
            println!("Total veHNT        : {}", s.network.total.vehnt);
            println!("mobile fall        : {}", s.mobile.total.fall_rate);
            println!("mobile est fall    : {}", mobile_sub_dao.vehnt_fall_rate);
            println!(
                "mobile diff        : {}",
                i128::try_from(mobile_sub_dao.vehnt_fall_rate).unwrap()
                    - i128::try_from(s.mobile.total.fall_rate).unwrap()
            );
            println!("iot fall           : {}", s.iot.total.fall_rate);
            println!("iot est fall       : {}", iot_sub_dao.vehnt_fall_rate);
            println!(
                "iot diff           : {}",
                i128::try_from(iot_sub_dao.vehnt_fall_rate).unwrap()
                    - i128::try_from(s.iot.total.fall_rate).unwrap()
            );
        } else {
            let delegated_hnt = s.mobile.total.vehnt + s.iot.total.vehnt;
            let undelegated_hnt = s.network.total.vehnt - delegated_hnt;
            println!(
                "Total MOBILE veHNT     :   {} ({}% of delegated)",
                format_vehnt(s.mobile.total.vehnt),
                percentage(s.mobile.total.vehnt, delegated_hnt)
            );
            println!(
                "Total IOT veHNT        : {} ({}% of delegated)",
                format_vehnt(s.iot.total.vehnt),
                percentage(s.iot.total.vehnt, delegated_hnt)
            );
            println!(
                "Total undelegated veHNT:   {} ({}% of total)",
                format_vehnt(undelegated_hnt),
                percentage(undelegated_hnt, s.network.total.vehnt)
            );
            println!(
                "Total veHNT            : {}",
                format_vehnt(s.network.total.vehnt)
            );
        }
        println!("Total positions        :         {}", d.positions_total_len);

        println!(
            "Total HNT locked       :    {}",
            format_hnt(s.network.total.hnt)
        );
        println!(
            "Average multiple       :         {}",
            (s.network.total.vehnt / ANOTHER_DIVIDER) as u64
                / (s.network.total.hnt / TOKEN_DIVIDER as u64)
        );
        println!(
            "Average veHNT size     :      {}",
            format_vehnt(s.network.stats.avg_vehnt)
        );
        println!(
            "Median veHNT size      :      {}",
            format_vehnt(s.network.stats.median_vehnt)
        );

        println!("Network {}", s.network);
        println!("MOBILE {}", s.mobile);
        println!("IOT {}", s.iot);

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
    pub owner: String,
    pub mint: String,
    pub position_key: String,
    pub locked_tokens: u64,
    pub start_ts: i64,
    pub genesis_end_ts: i64,
    pub end_ts: i64,
    pub duration_s: i64,
    pub voting_weight: u128,
    #[serde(skip_serializing, skip_deserializing)]
    pub vehnt_info: VehntInfo,
    pub lockup_type: LockupType,
    #[serde(skip_serializing_if = "Option::is_none")]
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
            hnt_amount: Hnt::from(value.locked_tokens),
            start_ts: value.start_ts,
            genesis_end_ts: value.genesis_end_ts,
            end_ts: value.end_ts,
            duration_s: value.duration_s,
            vehnt: value.voting_weight / PRECISION_FACTOR,
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
    pub pending_rewards: u64,
}

impl DelegatedPosition {
    fn try_from_delegated_position_v0(
        delegated_position_key: Pubkey,
        delegated_position: DelegatedPositionV0,
        epochs: &[epoch_info::EpochSummary],
        position: &PositionV0,
        voting_mint_config: &VotingMintConfigV0,
    ) -> Result<Self> {
        let mut pending_rewards = 0;
        const FIRST_EPOCH: usize = 19465;
        const FIRST_EPOCH_WITH_VEHNT: usize = 19467;

        let first_unclaimed_epoch = std::cmp::max(
            (delegated_position.last_claimed_epoch + 1) as usize,
            FIRST_EPOCH_WITH_VEHNT,
        );
        // the last epochs is initialized but incomplete
        let last_reward_epoch = epochs.len() - 1 + FIRST_EPOCH;
        let sub_dao = SubDao::try_from(delegated_position.sub_dao)?;

        for i in first_unclaimed_epoch..last_reward_epoch {
            let epoch_summary = &epochs[i - FIRST_EPOCH];
            assert_eq!(epoch_summary.epoch, i as u64);
            let ts = epoch_summary.epoch_start_at_ts.unwrap();
            let delegated_vehnt_at_epoch = position.voting_power(voting_mint_config, ts)? as u128;

            let (delegation_rewards_issued, vehnt_at_epoch_start) = match sub_dao {
                SubDao::Mobile => (epoch_summary.mobile_delegation_rewards_issued as u128, {
                    let mut mobile_vehnt = *epoch_summary.mobile_vehnt_at_epoch_start.get_decimal();
                    mobile_vehnt.set_scale(0)?;
                    mobile_vehnt.to_u128().unwrap()
                }),
                SubDao::Iot => (epoch_summary.iot_delegation_rewards_issued as u128, {
                    let mut mobile_vehnt = *epoch_summary.iot_vehnt_at_epoch_start.get_decimal();
                    mobile_vehnt.set_scale(0)?;
                    mobile_vehnt.to_u128().unwrap()
                }),
                _ => panic!("Error: calculating earnings for undelegated position?!"),
            };

            pending_rewards += u64::try_from(
                delegated_vehnt_at_epoch
                    .checked_mul(delegation_rewards_issued)
                    .unwrap()
                    .checked_div(vehnt_at_epoch_start)
                    .unwrap(),
            )
            .unwrap();
        }

        Ok(Self {
            pending_rewards,
            delegated_position_key: delegated_position_key.to_string(),
            sub_dao,
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
    pub async fn try_from_positionv0(
        owner: Pubkey,
        position_key: Pubkey,
        position: PositionV0,
        timestamp: i64,
        voting_mint_config: &VotingMintConfigV0,
    ) -> Result<Self> {
        let vehnt_info = caclulate_vhnt_info(timestamp, &position, voting_mint_config)?;
        let vehnt = position.voting_power_precise(voting_mint_config, timestamp)?;

        Ok(Self {
            owner: owner.to_string(),
            position_key: position_key.to_string(),
            mint: position.mint.to_string(),
            locked_tokens: position.amount_deposited_native,
            start_ts: position.lockup.start_ts,
            end_ts: position.lockup.end_ts,
            genesis_end_ts: position.genesis_end,
            duration_s: position.lockup.end_ts - position.lockup.start_ts,
            voting_weight: vehnt,
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
