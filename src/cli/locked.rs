use super::*;

use helium_anchor_gen::voter_stake_registry::{PositionV0, Registrar, VotingMintConfigV0};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, clap::Args)]
/// Fetches all delegated positions and total HNT, veHNT, and subDAO delegations.
pub struct Locked {
    #[arg(short, long)]
    verify: bool,
}

pub struct Data {
    pub positions: Vec<(Pubkey, PositionV0)>,
    pub mint_configs: HashMap<Pubkey, VotingMintConfigV0>,
    pub registrar_to_mint: HashMap<Pubkey, Pubkey>,
}

/// This function will work until there's too many to fetch in a single call
pub async fn get_data(rpc_client: &rpc::Client) -> Result<Data> {
    const POSITION_V0_DESCRIMINATAOR: [u8; 8] = [152, 131, 154, 46, 158, 42, 31, 233];
    let helium_vsr_id = Pubkey::from_str(HELIUM_VSR_ID)?;
    let memcmp =
        rpc::GetProgramAccountsFilter::Memcmp(rpc::Memcmp::new(0, &POSITION_V0_DESCRIMINATAOR));
    let accounts = rpc_client
        .get_program_accounts_with_filter(
            &helium_vsr_id,
            vec![rpc::GetProgramAccountsFilter::DataSize(180), memcmp],
        )
        .await?;
    let positions = accounts
        .iter()
        .map(|(pubkey, account)| {
            let mut data = account.data.as_slice();
            (*pubkey, PositionV0::try_deserialize(&mut data).unwrap())
        })
        .collect::<Vec<(Pubkey, PositionV0)>>();

    let registrar_keys: Vec<&Pubkey> = positions
        .iter()
        .map(|(_, p)| &p.registrar)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let registrars_raw = rpc_client
        .get_multiple_accounts_data(&registrar_keys)
        .await?;
    let registrars_raw: Vec<Registrar> = registrars_raw
        .iter()
        .map(|data| {
            let data = data.clone();
            Registrar::try_deserialize(&mut data.as_slice())
        })
        .map(|result| result.unwrap())
        .collect();

    let mut mint_configs = HashMap::new();
    let mut registrar_to_mint = HashMap::new();
    for (pubkey, registrar) in registrar_keys.iter().zip(registrars_raw.iter()) {
        let mint = &registrar.voting_mints[0];
        mint_configs.insert(mint.mint, *mint);
        registrar_to_mint.insert(**pubkey, mint.mint);
    }

    Ok(Data {
        positions,
        mint_configs,
        registrar_to_mint,
    })
}

impl Locked {
    pub async fn run(self, rpc_client: rpc::Client) -> Result {
        let data = get_data(&rpc_client).await?;

        let mut total_hnt = 0;
        let mut total_mobile = 0;
        let mut total_iot = 0;
        for (_pubkey, position) in data.positions.iter() {
            if let Some(mint) = data.registrar_to_mint.get(&position.registrar) {
                match mint.to_string().as_str() {
                    HNT_MINT => total_hnt += position.amount_deposited_native,
                    MOBILE_MINT => total_mobile += position.amount_deposited_native,
                    IOT_MINT => total_iot += position.amount_deposited_native,
                    _ => println!("Unknown mint {}", mint),
                }
            } else {
                println!("No mint found for registrar {}", position.registrar)
            }
        }
        println!("Total HNT locked   : {}", format_hnt(total_hnt));
        println!("Total MOBILE locked: {}", format_dnt(total_mobile));
        println!("Total IOT locked   : {}", format_dnt(total_iot));

        Ok(())
    }
}
