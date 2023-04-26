use super::*;
use anchor_lang::AccountDeserialize;

#[derive(Debug, Clone, clap::Args)]
/// Scrape all SubDao epoch info
pub struct EpochInfo {}

use helium_sub_daos::SubDaoEpochInfoV0;
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::HashMap;

use voter_stake_registry::state::PRECISION_FACTOR;

pub async fn get_epoch_summaries(rpc_client: &RpcClient) -> Result<Vec<EpochSummary>> {
    let helium_dao_id = Pubkey::from_str(HELIUM_DAO_ID)?;
    const SUB_DAO_EPOCH_INFO_DESCRIMINATOR: [u8; 8] = [45, 249, 177, 20, 170, 251, 37, 37];

    let mut config = RpcProgramAccountsConfig::default();
    let memcmp = RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
        0,
        &SUB_DAO_EPOCH_INFO_DESCRIMINATOR,
    ));
    config.filters = Some(vec![RpcFilterType::DataSize(204), memcmp]);
    config.account_config.encoding = Some(UiAccountEncoding::Base64);
    let accounts = rpc_client
        .get_program_accounts_with_config(&helium_dao_id, config)
        .await?;

    let mut iot_epochs = HashMap::new();
    let mut mobile_epochs = HashMap::new();

    for (_pubkey, account) in &accounts {
        let mut data = account.data.as_slice();
        if let Ok(sub_dao_epoch_info) = SubDaoEpochInfoV0::try_deserialize(&mut data) {
            let sub_dao_epoch_info = SubDaoEpochInfo::try_from(sub_dao_epoch_info)?;
            match sub_dao_epoch_info.sub_dao {
                SubDao::Iot => {
                    iot_epochs.insert(sub_dao_epoch_info.epoch, sub_dao_epoch_info);
                }
                SubDao::Mobile => {
                    mobile_epochs.insert(sub_dao_epoch_info.epoch, sub_dao_epoch_info);
                }
                SubDao::Unknown => {
                    return Err(Error::Custom("Unknown subdao"));
                }
            }
        }
    }

    let mut output = Vec::new();
    for (i, iot) in iot_epochs {
        if let Some(mobile) = mobile_epochs.get(&i) {
            if iot.utility_score.is_some() && mobile.utility_score.is_some() {
                let epoch_summary = {
                    EpochSummary {
                        epoch: i,
                        iot_dc_burned: iot.dc_burned,
                        mobile_dc_burned: mobile.dc_burned,
                        iot_vehnt_at_epoch_start: iot.vehnt_at_epoch_start.try_into()?,
                        mobile_vehnt_at_epoch_start: mobile.vehnt_at_epoch_start.try_into()?,
                        iot_vehnt_in_closing_positions: iot
                            .vehnt_in_closing_positions
                            .try_into()?,
                        mobile_vehnt_in_closing_positions: mobile
                            .vehnt_in_closing_positions
                            .try_into()?,
                        iot_delegation_rewards_issued: iot.delegation_rewards_issued,
                        mobile_delegation_rewards_issued: mobile.delegation_rewards_issued,
                        iot_utility_score: iot.utility_score.unwrap(),
                        mobile_utility_score: mobile.utility_score.unwrap(),
                        iot_rewards_issued_at: iot.rewards_issued_at,
                        mobile_rewards_issued_at: mobile.rewards_issued_at,
                    }
                };
                output.push(epoch_summary);
            }
        }
    }
    output.sort_by(|a, b| a.epoch.cmp(&b.epoch));

    Ok(output)
}

impl EpochInfo {
    pub async fn run(self, rpc_client: RpcClient) -> Result {
        let summaries = get_epoch_summaries(&rpc_client).await?;
        use csv::Writer;
        let mut wtr = Writer::from_path("epochs.csv")?;
        for record in summaries {
            wtr.serialize(record)?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct SubDaoEpochInfo {
    pub epoch: u64,
    pub sub_dao: SubDao,
    pub dc_burned: u64,
    pub vehnt_at_epoch_start: u64,
    pub vehnt_in_closing_positions: u128,
    /// The vehnt amount that is decaying per second, with 12 decimals of extra precision. Associated with positions that are closing this epoch,
    /// which means they must be subtracted from the total fall rate on the subdao after this epoch passes
    pub fall_rates_from_closing_positions: u128,
    /// The number of delegation rewards issued this epoch, so that delegators can claim their share of the rewards
    pub delegation_rewards_issued: u64,
    /// Precise number with 12 decimals
    pub utility_score: Option<u128>,
    /// The program only needs to know whether or not rewards were issued, however having a history of when they were issued could prove
    /// useful in the future, or at least for debugging purposes
    pub rewards_issued_at: Option<DateTime<Utc>>,
    pub bump_seed: u8,
    pub initialized: bool,
}

#[derive(Debug, Serialize)]
pub struct EpochSummary {
    pub epoch: u64,
    pub iot_dc_burned: u64,
    pub mobile_dc_burned: u64,

    pub iot_vehnt_at_epoch_start: VeHnt,
    pub iot_vehnt_in_closing_positions: VeHnt,

    pub mobile_vehnt_at_epoch_start: VeHnt,
    pub mobile_vehnt_in_closing_positions: VeHnt,

    pub iot_delegation_rewards_issued: u64,
    pub mobile_delegation_rewards_issued: u64,

    pub iot_utility_score: u128,
    pub mobile_utility_score: u128,

    pub iot_rewards_issued_at: Option<DateTime<Utc>>,
    pub mobile_rewards_issued_at: Option<DateTime<Utc>>,
}

impl EpochSummary {
    pub fn scale_down(&mut self) {
        self.iot_utility_score /= PRECISION_FACTOR;
        self.mobile_utility_score /= PRECISION_FACTOR;
    }
}

impl TryFrom<SubDaoEpochInfoV0> for SubDaoEpochInfo {
    type Error = Error;
    fn try_from(value: SubDaoEpochInfoV0) -> Result<Self> {
        Ok(Self {
            epoch: value.epoch,
            sub_dao: SubDao::try_from(value.sub_dao)?,
            dc_burned: value.dc_burned,
            vehnt_at_epoch_start: value.vehnt_at_epoch_start,
            vehnt_in_closing_positions: value.vehnt_in_closing_positions,
            fall_rates_from_closing_positions: value.fall_rates_from_closing_positions,
            delegation_rewards_issued: value.delegation_rewards_issued,
            utility_score: value.utility_score,
            rewards_issued_at: value
                .rewards_issued_at
                .map(|t| DateTime::from_utc(NaiveDateTime::from_timestamp_opt(t, 0).unwrap(), Utc)),
            bump_seed: value.bump_seed,
            initialized: value.initialized,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct VeHnt(Decimal);

impl TryFrom<u128> for VeHnt {
    type Error = Error;

    fn try_from(value: u128) -> Result<Self> {
        let value = Decimal::try_from_i128_with_scale(value.try_into()?, 8)?;
        Ok(Self(value))
    }
}

impl TryFrom<u64> for VeHnt {
    type Error = Error;

    fn try_from(value: u64) -> Result<Self> {
        use rust_decimal::prelude::FromPrimitive;
        let mut value = Decimal::from_u64(value).unwrap();
        value.set_scale(8).unwrap();
        Ok(Self(value))
    }
}
