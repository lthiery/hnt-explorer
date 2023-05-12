use super::*;

#[derive(Debug, Clone, clap::Args)]
/// Get account balance for HNT, MOBILE, and IOT tokens
pub struct Account {
    account: Pubkey,
}

impl Account {
    pub async fn run(self, rpc_client: RpcClient) -> Result {
        let rpc_client = Arc::new(rpc_client);
        let hnt_account =
            get_account(&rpc_client, &Pubkey::from_str(HNT_MINT)?, &self.account).await?;
        if let Some(account) = hnt_account {
            println!("HNT    : {}", format_hnt(account.token));
        } else {
            println!("HNT    : 0");
        }
        let iot_account =
            get_account(&rpc_client, &Pubkey::from_str(IOT_MINT)?, &self.account).await?;
        if let Some(account) = iot_account {
            println!("IOT    : {}", format_dnt(account.token));
        } else {
            println!("IOT    : 0");
        }
        let mobile_account =
            get_account(&rpc_client, &Pubkey::from_str(MOBILE_MINT)?, &self.account).await?;
        if let Some(account) = mobile_account {
            println!("MOBILE : {:?}", format_dnt(account.token));
        } else {
            println!("MOBILE : 0");
        }
        Ok(())
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
) -> Result<Option<SplAccount>> {
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
                    Ok(Some(SplAccount {
                        mint: *mint,
                        lamports: account.lamports,
                        token: account_data.base.amount,
                    }))
                }
                _ => Err(Error::UnexpectedProgramName),
            }
        }
        None => Ok(None),
    }
}
