use super::*;

#[derive(Debug, Clone, clap::Args)]
/// Get account balance for HNT, MOBILE, and IOT tokens
pub struct Account {
    account: Pubkey,
}

impl Account {
    pub async fn run(self, rpc_client: RpcClient) -> Result {
        let rpc_client = Arc::new(rpc_client);
        let helium_balances = HeliumBalances::fetch(&rpc_client, &self.account).await?;
        let output = serde_json::to_string_pretty(&helium_balances)?;
        println!("{}", output);
        Ok(())
    }
}

#[derive(serde::Serialize)]
pub struct HeliumBalances {
    hnt: Balance,
    iot: Balance,
    mobile: Balance,
}

#[derive(serde::Serialize)]
struct Balance {
    mint: String,
    value: u64,
    decimals: u8,
}

impl HeliumBalances {
    pub async fn fetch(rpc_client: &Arc<RpcClient>, account: &Pubkey) -> Result<Self> {
        let hnt_account = get_account(rpc_client, &Pubkey::from_str(HNT_MINT)?, account).await?;

        let iot_account = get_account(rpc_client, &Pubkey::from_str(IOT_MINT)?, account).await?;

        let mobile_account =
            get_account(rpc_client, &Pubkey::from_str(MOBILE_MINT)?, account).await?;

        Ok(Self {
            hnt: Balance {
                mint: HNT_MINT.to_string(),
                value: hnt_account.token,
                decimals: 8,
            },
            iot: Balance {
                mint: IOT_MINT.to_string(),
                value: iot_account.token,
                decimals: 6,
            },
            mobile: Balance {
                mint: MOBILE_MINT.to_string(),
                value: mobile_account.token,
                decimals: 6,
            },
        })
    }
}

use solana_account_decoder::parse_account_data::{ParsableAccount, PARSABLE_PROGRAM_IDS};
use spl_token_2022::extension::StateWithExtensions;
use spl_token_client::client::{ProgramClient, ProgramRpcClient, ProgramRpcClientSendTransaction};
use std::sync::Arc;

#[derive(Debug)]
struct SplAccount {
    #[allow(unused)]
    pub mint: Pubkey,
    #[allow(unused)]
    pub lamports: u64,
    pub token: u64,
}

async fn get_account(
    rpc_client: &Arc<RpcClient>,
    mint: &Pubkey,
    pubkey: &Pubkey,
) -> Result<SplAccount> {
    let spl_atc = spl_associated_token_account::get_associated_token_address(pubkey, mint);
    let program_id = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();

    let program_rpc_client =
        ProgramRpcClient::new(rpc_client.clone(), ProgramRpcClientSendTransaction);

    match program_rpc_client.get_account(spl_atc).await? {
        Some(account) => {
            let program_name = PARSABLE_PROGRAM_IDS
                .get(&program_id)
                .ok_or(Error::SolanaProgramIdNotParsable(program_id.to_string()))?;

            match program_name {
                ParsableAccount::SplToken | ParsableAccount::SplToken2022 => {
                    let account_data =
                        StateWithExtensions::<spl_token_2022::state::Account>::unpack(
                            &account.data,
                        )?;
                    Ok(SplAccount {
                        mint: *mint,
                        lamports: account.lamports,
                        token: account_data.base.amount,
                    })
                }
                _ => Err(Error::UnexpectedProgramName),
            }
        }
        None => Ok(SplAccount {
            mint: *mint,
            lamports: 0,
            token: 0,
        }),
    }
}
