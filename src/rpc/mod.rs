use base64::Engine;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

mod client;
mod error;
mod rpc_call;

pub use client::Client;
pub use error::Error;
use rpc_call::RpcCall;

pub type Result<T = ()> = std::result::Result<T, Error>;

#[allow(unused)]
async fn get_account_data(client: &Client, pubkey: &Pubkey) -> Result<Vec<u8>> {
    #[derive(Deserialize, Debug)]
    pub struct Response {
        pub value: Value,
    }
    #[derive(Deserialize, Debug)]
    pub struct Value {
        pub data: Vec<String>,
    }

    let json = RpcCall::get_account_info(pubkey);
    let account_response: Response = client.post(&json).await?;
    Ok(base64::engine::general_purpose::STANDARD.decode(&account_response.value.data[0])?)
}

pub async fn get_multiple_accounts_data(
    client: &Client,
    pubkeys: Vec<&Pubkey>,
) -> Result<Vec<Vec<u8>>> {
    #[derive(Deserialize, Debug)]
    pub struct Response {
        pub value: Vec<Value>,
    }
    #[derive(Deserialize, Debug)]
    pub struct Value {
        pub data: Vec<String>,
    }

    let json = RpcCall::get_multiple_accounts(pubkeys);
    let account_response: Response = client.post(&json).await?;
    account_response
        .value
        .into_iter()
        .map(|v| {
            base64::engine::general_purpose::STANDARD
                .decode(&v.data[0])
                .map_err(Error::B64Decode)
        })
        .collect()
}

async fn get_token_largest_account(client: &Client, pubkey: &Pubkey) -> Result<Pubkey> {
    #[derive(Deserialize, Debug)]
    struct Response {
        pub value: Vec<Value>,
    }
    #[derive(Deserialize, Debug)]
    struct Value {
        pub address: String,
    }

    let json = RpcCall::get_token_largest_accounts(pubkey);
    let response: Response = client.post(&json).await?;
    Ok(Pubkey::from_str(&response.value[0].address)?)
}

pub async fn get_assets_by_authority(client: &Client, authority: &Pubkey) -> Result<Pubkey> {
    #[derive(Deserialize, Debug)]
    pub struct AssetsByAuthorityResponse {
        pub items: Vec<Item>,
    }

    #[derive(Deserialize, Debug)]
    pub struct Item {
        pub id: String,
    }
    let json = RpcCall::get_assets_by_authority(authority);
    let asset_response: AssetsByAuthorityResponse = client.post(&json).await?;
    Ok(Pubkey::from_str(&asset_response.items[0].id)?)
}

#[allow(unused)]
pub async fn get_position_owner(client: &Client, position_id: &Pubkey) -> Result<Pubkey> {
    let asset_by_authority = get_assets_by_authority(client, position_id).await?;
    let token_largest_accounts = get_token_largest_account(client, &asset_by_authority).await?;
    let account_data = get_account_data(client, &token_largest_accounts).await?;
    Ok(Pubkey::new(&account_data[32..64]))
}

pub async fn get_positions_owner(
    client: &Client,
    position_id: &Vec<&Pubkey>,
    chunk_size: usize,
) -> Result<Vec<Pubkey>> {
    use futures::future::join_all;
    let mut owners = Vec::with_capacity(position_id.len());
    for i in position_id.chunks(chunk_size) {
        let mut futures = Vec::with_capacity(chunk_size);
        for j in i {
            futures.push(async move {
                let asset_by_authority = get_assets_by_authority(client, j).await.unwrap();
                get_token_largest_account(client, &asset_by_authority)
                    .await
                    .unwrap()
            });
        }
        let token_accounts = join_all(futures).await;
        let account_data =
            get_multiple_accounts_data(client, token_accounts.iter().collect()).await?;
        let these_owners = account_data
            .into_iter()
            .map(|v| Pubkey::new(&v[32..64]))
            .collect::<Vec<Pubkey>>();
        owners.extend(&these_owners)
    }
    Ok(owners)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;
    use tokio::test;

    #[test]
    async fn test_get_assets_by_authority() {
        let client = Client::default();
        let asset_by_authority = get_assets_by_authority(
            &client,
            &Pubkey::from_str("EvXrmwTJaqXvAL5skuyiWRV1X7MPwmZYX8Qp3DCw83RT").unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(
            asset_by_authority,
            Pubkey::from_str("BQW1ABLREJtw8hA3WqpvfvUZppVWaG127hAoVsNnmNKS").unwrap()
        )
    }

    #[test]
    async fn test_get_token_largest_accounts() {
        let client = Client::default();
        let token_largest_accounts = get_token_largest_account(
            &client,
            &Pubkey::from_str("BLt9DipvNYvCddekZn4MuvDwxziuda6wR2Buk3hAbJwF").unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(
            token_largest_accounts,
            Pubkey::from_str("CfGsBm5shwwb5tVpFjdj8zPbYCxJ3LhwNxBSCX7WFCCJ").unwrap()
        )
    }

    #[test]
    async fn test_get_account_data() {
        let client = Client::default();
        let data = get_account_data(
            &client,
            &Pubkey::from_str("CfGsBm5shwwb5tVpFjdj8zPbYCxJ3LhwNxBSCX7WFCCJ").unwrap(),
        )
        .await
        .unwrap();
        let pubkey = Pubkey::new(&data[32..64]);
        assert_eq!(
            pubkey,
            Pubkey::from_str("ADqp77vvKapHsU2ymsaoMojXpHjdhxLcfrgWWtaxYCVU").unwrap()
        )
    }

    #[test]
    async fn test_get_multiple_accounts() {
        let client = Client::default();
        let data = get_multiple_accounts_data(
            &client,
            vec![
                &Pubkey::from_str("CfGsBm5shwwb5tVpFjdj8zPbYCxJ3LhwNxBSCX7WFCCJ").unwrap(),
                &Pubkey::from_str("GxXXdsdfT4Z7ntYphmBdLRTBTLHPu2cdtCEDK56uGK28").unwrap(),
            ],
        )
        .await
        .unwrap();
        let pubkey = Pubkey::new(&data[0][32..64]);
        assert_eq!(
            pubkey,
            Pubkey::from_str("ADqp77vvKapHsU2ymsaoMojXpHjdhxLcfrgWWtaxYCVU").unwrap()
        );
        let pubkey = Pubkey::new(&data[1][32..64]);
        assert_eq!(
            pubkey,
            solana_sdk::pubkey::Pubkey::from_str("CCUWF8sALfvVtv1EvKwuM4p7ZgUWvsrKVzQfFGmBz6pa")
                .unwrap()
        );
    }

    #[test]
    async fn test_get_position_owner() {
        let client = Client::default();
        let pubkey = get_position_owner(
            &client,
            &Pubkey::from_str("E6ELFUZMahhCgsPCeEYXtQT51Yq24Yg4WAhmQJBsGxhg").unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(
            pubkey,
            Pubkey::from_str("ADqp77vvKapHsU2ymsaoMojXpHjdhxLcfrgWWtaxYCVU").unwrap()
        )
    }

    #[test]
    async fn test_get_positions_owner() {
        let client = Client::default();
        let pubkeys = get_positions_owner(
            &client,
            &vec![
                &Pubkey::from_str("E6ELFUZMahhCgsPCeEYXtQT51Yq24Yg4WAhmQJBsGxhg").unwrap(),
                &Pubkey::from_str("3NW4DzSNeS72pJXCpAszC6r4Su1Ku6RHEDwRmzVXMVxo").unwrap(),
            ],
            2,
        )
        .await
        .unwrap();
        assert_eq!(
            pubkeys[0],
            Pubkey::from_str("ADqp77vvKapHsU2ymsaoMojXpHjdhxLcfrgWWtaxYCVU").unwrap()
        );
        assert_eq!(
            pubkeys[1],
            Pubkey::from_str("BKw4D8sv6Wt67LmUqVN1gLpe2XUDicifdrSBuGcYvPz2").unwrap()
        );
    }
}
