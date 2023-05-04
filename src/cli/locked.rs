use super::*;

use voter_stake_registry::state::{Registrar, PositionV0};
use std::collections::{HashSet, HashMap};

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
        let raw_positions = get_all_positions(&rpc_client).await?;

        let registrar_keys: Vec<Pubkey> = raw_positions
            .iter()
            .map(|(_, p)| p.registrar)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        for key in &registrar_keys {
            println!("{}", key);
        }

        let registrars_raw = rpc_client.get_multiple_accounts(&registrar_keys).await?;
        let registrars_raw: Vec<Registrar> = registrars_raw
            .iter()
            .map(|registrar| {
                let mut data = registrar.as_ref().unwrap().data.as_slice();
                Registrar::try_deserialize(&mut data)
            })
            .map(|result| result.unwrap())
            .collect();
        


        let mut registrars = HashMap::new();
        for registrar in registrars_raw.iter() {
            let mint = &registrar.voting_mints[0];
            registrars.insert(mint.mint, mint.clone());
            println!("")
        }

        let voting_mint_config =
            &registrars[&Pubkey::from_str("hntyVP6YFm1Hg25TN9WGLqM12b8TQmcknKrdu1oxWux")?];

        let mut total_hnt = 0;
        for (_pubkey, position) in raw_positions {
            let pubkey = Pubkey::from_str("BMnWRWZrWqb6JMKznaDqNxWaWAHoaTzVabM6Qwyh3WKz")?;
            if position.registrar == pubkey {
                total_hnt += position.amount_deposited_native;
            }
            //println!("{}", position.registrar);

        }
        println!("Total HNT locked: {}", format_hnt(total_hnt));
        Ok(())
    }
}
