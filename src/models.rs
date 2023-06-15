#[derive(Default, Debug, serde::Serialize)]
pub struct PositionData {
    pub timestamp: i64,
    pub positions: Vec<Position>,
    pub positions_total_len: usize,
}

#[derive(Clone, Default, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockupType {
    Cliff,
    Constant,
    #[default]
    Unlocked,
}

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
    #[serde(skip_serializing)]
    pub vehnt_info: VehntInfo,
    pub lockup_type: LockupType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegated: Option<DelegatedPosition>,
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

#[derive(Clone, Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct DelegatedPosition {
    pub delegated_position_key: String,
    pub sub_dao: SubDao,
    pub last_claimed_epoch: u64,
    pub pending_rewards: u64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubDao {
    #[default]
    Unknown,
    Iot,
    Mobile,
}

#[cfg(feature = "ingest")]


#[cfg(feature = "ingest")]
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