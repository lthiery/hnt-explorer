use super::*;

#[derive(Debug, Clone, clap::Args)]
/// Scrape all SubDao epoch info
pub struct Supply {}

pub async fn get_token_supply(rpc_client: &RpcClient, mint: Pubkey) -> Result<u128> {
    let supply = rpc_client.get_token_supply(&mint).await?;
    Ok(u128::from_str(&supply.amount)?)
}

impl Supply {
    pub async fn run(self, rpc_client: RpcClient) -> Result {
        let hnt_supply = get_token_supply(&rpc_client, Pubkey::from_str(HNT_MINT)?).await?;
        println!("HNT supply   : {}", format_hnt(hnt_supply as u64));
        let mobile_supply = get_token_supply(&rpc_client, Pubkey::from_str(MOBILE_MINT)?).await?;
        println!("MOBILE supply: {}", format_dnt(mobile_supply as u64));
        let iot_supply = get_token_supply(&rpc_client, Pubkey::from_str(IOT_MINT)?).await?;
        println!("IOT supply   : {}", format_dnt(iot_supply as u64));
        Ok(())
    }
}
