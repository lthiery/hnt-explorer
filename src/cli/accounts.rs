use super::*;

#[derive(Debug, Clone, clap::Args)]
/// Get account balance for HNT, MOBILE, and IOT tokens
pub struct Account {
    account: Pubkey,
}

impl Account {
    pub async fn run(self, rpc_client: rpc::Client) -> Result {
        let rpc_client = Arc::new(rpc_client);
        let helium_balances = HeliumBalances::fetch(&rpc_client, &self.account).await?;
        let output = serde_json::to_string_pretty(&helium_balances)?;
        println!("{}", output);
        Ok(())
    }
}

#[derive(serde::Serialize)]
pub struct HeliumBalances {
    pub hnt: Balance,
    pub iot: Balance,
    pub mobile: Balance,
}

#[derive(serde::Serialize)]
pub struct Balance {
    pub mint: String,
    pub amount: u64,
    pub decimals: u8,
}

impl HeliumBalances {
    pub async fn fetch(rpc_client: &Arc<rpc::Client>, account: &Pubkey) -> Result<Self> {
        let hnt_mint = Pubkey::from_str(HNT_MINT)?;
        let iot_mint = Pubkey::from_str(IOT_MINT)?;
        let mobile_mint = Pubkey::from_str(MOBILE_MINT)?;
        let (hnt_account, iot_account, mobile_account) = tokio::join!(
            get_account(rpc_client, &hnt_mint, account),
            get_account(rpc_client, &iot_mint, account),
            get_account(rpc_client, &mobile_mint, account)
        );

        Ok(Self {
            hnt: Balance {
                mint: HNT_MINT.to_string(),
                amount: hnt_account?.token,
                decimals: 8,
            },
            iot: Balance {
                mint: IOT_MINT.to_string(),
                amount: iot_account?.token,
                decimals: 6,
            },
            mobile: Balance {
                mint: MOBILE_MINT.to_string(),
                amount: mobile_account?.token,
                decimals: 6,
            },
        })
    }
}

use std::sync::Arc;

#[derive(Debug)]
struct SplAccount {
    #[allow(unused)]
    pub mint: Pubkey,
    #[allow(unused)]
    pub lamports: u64,
    pub token: u64,
}

use spl_token_2022::extension::StateWithExtensions;

async fn get_account(
    rpc_client: &Arc<rpc::Client>,
    mint: &Pubkey,
    pubkey: &Pubkey,
) -> Result<SplAccount> {
    let spl_atc = spl_associated_token_account::get_associated_token_address(pubkey, mint);
    let account = rpc_client.get_account(&spl_atc).await;
    if let Err(rpc::Error::AccountNotFound) = account {
        return Ok(SplAccount {
            mint: *mint,
            lamports: 0,
            token: 0,
        });
    };
    let account = account?;
    let account_data =
        StateWithExtensions::<spl_token_2022::state::Account>::unpack(&account.data)?;
    Ok(SplAccount {
        mint: *mint,
        lamports: account.lamports,
        token: account_data.base.amount,
    })
}
