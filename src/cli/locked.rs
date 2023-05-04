use super::*;

use voter_stake_registry::state::PositionV0;

#[derive(Debug, Clone, clap::Args)]
/// Fetches all delegated positions and total HNT, veHNT, and subDAO delegations.
pub struct Locked {
    #[arg(short, long)]
    verify: bool,
}

/// This function will work until there's too many to fetch in a single call
pub async fn get_all_positions(rpc_client: &RpcClient) -> Result<Vec<(Pubkey, PositionV0)>> {
    const POSITION_V0_DESCRIMINATAOR: [u8; 8] = [152, 131, 154, 46, 158, 42, 31, 233];
    let helium_vsr_id = Pubkey::from_str(HELIUM_VSR_ID)?;
    let mut config = RpcProgramAccountsConfig::default();
    let memcmp = RpcFilterType::Memcmp(Memcmp::new_base58_encoded(0, &POSITION_V0_DESCRIMINATAOR));
    config.filters = Some(vec![RpcFilterType::DataSize(180), memcmp]);
    config.account_config.encoding = Some(UiAccountEncoding::Base64);
    let accounts = rpc_client
        .get_program_accounts_with_config(&helium_vsr_id, config)
        .await?;
    Ok(accounts
        .iter()
        .map(|(pubkey, account)| {
            let mut data = account.data.as_slice();
            (*pubkey, PositionV0::try_deserialize(&mut data).unwrap())
        })
        .collect::<Vec<(Pubkey, PositionV0)>>())
}

impl Locked {
    pub async fn run(self, rpc_client: RpcClient) -> Result {
        let positions = get_all_positions(&rpc_client).await?;
        let mut total_hnt = 0;
        for (_pubkey, position) in positions {
            total_hnt += position.amount_deposited_native;
        }
        println!("Total HNT locked: {}", format_hnt(total_hnt));
        Ok(())
    }
}
