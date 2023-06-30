use base64::Engine;
use serde::{de::DeserializeOwned, Deserialize, Serialize, Serializer};
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
pub type Epoch = u64;

#[derive(Debug)]
pub struct Account {
    /// lamports in the account
    pub lamports: u64,
    /// the program that owns this account. If executable, the program that loads this account.
    pub owner: Pubkey,
    /// data held in this account
    pub data: Vec<u8>,
    /// this account's data contains a loaded program (and is now read-only)
    pub executable: bool,
    /// the epoch at which this account will next owe rent
    pub rent_epoch: Epoch,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GetProgramAccountsFilter<'a> {
    DataSize(u64),
    Memcmp(Memcmp<'a>),
}

#[derive(Clone, Debug, Serialize)]
pub struct Memcmp<'a> {
    /// Data offset to begin match
    pub offset: usize,
    #[serde(serialize_with = "serde_bytes_base58")]
    pub bytes: &'a [u8],
}

fn serde_bytes_base58<S: Serializer>(bytes: &[u8], s: S) -> std::result::Result<S::Ok, S::Error> {
    let str = bs58::encode(bytes).into_string();
    s.serialize_str(&str)
}

impl<'a> Memcmp<'a> {
    pub fn new(offset: usize, bytes: &'a [u8]) -> Self {
        Self { offset, bytes }
    }
}

impl Client {
    pub async fn get_account(&self, pubkey: &Pubkey) -> Result<Account> {
        #[derive(Deserialize, Debug)]
        struct Response {
            pub value: Option<ReceivedAccount>,
        }

        let json = RpcCall::get_account_info(pubkey);
        let account_response: Response = self.post(&json).await?;
        let account = account_response.value.ok_or(Error::AccountNotFound)?;
        Ok(Account {
            lamports: account.lamports,
            owner: account.owner,
            data: base64::engine::general_purpose::STANDARD
                .decode(&account.data[0])
                .map_err(Error::B64Decode)?,
            executable: account.executable,
            rent_epoch: account.rent_epoch,
        })
    }

    async fn get_account_data(&self, pubkey: &Pubkey) -> Result<Vec<u8>> {
        let account = self.get_account(pubkey).await?;
        Ok(account.data)
    }

    pub async fn get_multiple_accounts_data(&self, pubkeys: &[&Pubkey]) -> Result<Vec<Vec<u8>>> {
        #[derive(Deserialize, Debug)]
        pub struct Response {
            pub value: Vec<Value>,
        }
        #[derive(Deserialize, Debug)]
        pub struct Value {
            pub data: Vec<String>,
        }

        let json = RpcCall::get_multiple_accounts(pubkeys);
        let account_response: Response = self.post(&json).await?;
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

    async fn get_token_largest_account(&self, pubkey: &Pubkey) -> Result<Pubkey> {
        #[derive(Deserialize, Debug)]
        struct Response {
            pub value: Vec<Value>,
        }
        #[derive(Deserialize, Debug)]
        struct Value {
            pub address: String,
        }

        let json = RpcCall::get_token_largest_accounts(pubkey);
        let response: Response = self.post(&json).await?;
        Ok(Pubkey::from_str(&response.value[0].address)?)
    }

    pub async fn get_token_supply_amount(&self, pubkey: &Pubkey) -> Result<u128> {
        #[derive(Deserialize, Debug)]
        struct Response {
            pub value: Value,
        }
        #[derive(Deserialize, Debug)]
        struct Value {
            pub amount: String,
        }

        let json = RpcCall::get_token_supply(pubkey);
        let response: Response = self.post(&json).await?;
        Ok(response.value.amount.parse::<u128>()?)
    }

    pub async fn get_assets_by_authority(&self, authority: &Pubkey) -> Result<Option<Pubkey>> {
        #[derive(Deserialize, Debug)]
        pub struct AssetsByAuthorityResponse {
            pub items: Vec<Item>,
        }

        #[derive(Deserialize, Debug)]
        pub struct Item {
            pub id: String,
        }
        let json = RpcCall::get_assets_by_authority(authority);
        let asset_response: AssetsByAuthorityResponse = self.post(&json).await?;
        Ok(if asset_response.items.is_empty() {
            None
        } else {
            Some(Pubkey::from_str(&asset_response.items[0].id)?)
        })
    }

    #[allow(unused)]
    pub async fn get_position_owner(&self, position_id: &Pubkey) -> Result<Pubkey> {
        if let Some(mint) = self.get_assets_by_authority(position_id).await? {
            self.get_owner_by_mint(&mint).await
        } else {
            Err(Error::NoAssetByAuthority(position_id.to_string()))
        }
    }

    #[allow(unused)]
    pub async fn get_owner_by_mint(&self, mint: &Pubkey) -> Result<Pubkey> {
        let token_largest_accounts = self.get_token_largest_account(mint).await?;
        let account_data = self.get_account_data(&token_largest_accounts).await?;
        Ok(Pubkey::new(&account_data[32..64]))
    }

    pub async fn get_all_owners_by_mint(
        &self,
        position_id: &Vec<&Pubkey>,
        chunk_size: usize,
    ) -> Result<Vec<Pubkey>> {
        use futures::future::join_all;
        use std::time::Instant;
        let mut owners = Vec::with_capacity(position_id.len());

        println!("Fetching owners of {} positions", position_id.len());
        let start = Instant::now();
        let mut last_output = start;
        for i in position_id.chunks(chunk_size) {
            let mut futures = Vec::with_capacity(chunk_size);
            for j in i {
                futures.push(self.get_token_largest_account(j));
            }
            let token_accounts: Vec<Result<Pubkey>> = join_all(futures).await;
            let token_accounts: Vec<Pubkey> = token_accounts
                .into_iter()
                .collect::<Result<Vec<Pubkey>>>()?;
            let account_data = self
                .get_multiple_accounts_data(&token_accounts.iter().collect::<Vec<&Pubkey>>())
                .await?;
            let these_owners = account_data
                .into_iter()
                .map(|v| Pubkey::new(&v[32..64]))
                .collect::<Vec<Pubkey>>();
            owners.extend(&these_owners);
            if last_output.elapsed().as_secs() > 5 || owners.len() == position_id.len() {
                last_output = Instant::now();
                println!(
                    "Completed {} / {} positions in {} seconds. ",
                    owners.len(),
                    position_id.len(),
                    start.elapsed().as_secs()
                );
            }
        }
        Ok(owners)
    }

    #[allow(unused)]
    pub async fn get_program_accounts_with_filter<'a>(
        &self,
        program_id: &Pubkey,
        filters: Vec<GetProgramAccountsFilter<'a>>,
    ) -> Result<Vec<(Pubkey, Account)>> {
        #[derive(Deserialize, Debug)]
        struct ReceivedAccountsAndPubkeys {
            account: ReceivedAccount,
            #[serde(deserialize_with = "deserialize_pubkey")]
            pubkey: Pubkey,
        }

        let json = RpcCall::get_program_accounts_with_filters(program_id, filters);
        let account_response: Vec<ReceivedAccountsAndPubkeys> = self.post(&json).await?;
        account_response
            .into_iter()
            .map(|r| {
                Ok((
                    (r.pubkey),
                    Account {
                        lamports: r.account.lamports,
                        owner: r.account.owner,
                        data: base64::engine::general_purpose::STANDARD
                            .decode(&r.account.data[0])
                            .map_err(Error::B64Decode)?,
                        executable: r.account.executable,
                        rent_epoch: r.account.rent_epoch,
                    },
                ))
            })
            .collect::<Result<Vec<(Pubkey, Account)>>>()
    }
}

#[derive(Deserialize, Debug)]
struct ReceivedAccount {
    pub lamports: u64,
    #[serde(deserialize_with = "deserialize_pubkey")]
    pub owner: Pubkey,
    pub data: Vec<String>,
    pub executable: bool,
    #[serde(rename = "rentEpoch")]
    pub rent_epoch: Epoch,
}

fn deserialize_pubkey<'de, D>(deserializer: D) -> std::result::Result<Pubkey, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let str = String::deserialize(deserializer)?;
    Pubkey::from_str(&str).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;
    use tokio::test;

    #[test]
    async fn test_get_assets_by_authority() {
        let client = Client::default();
        let asset_by_authority = client
            .get_assets_by_authority(
                &Pubkey::from_str("EvXrmwTJaqXvAL5skuyiWRV1X7MPwmZYX8Qp3DCw83RT").unwrap(),
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            asset_by_authority,
            Pubkey::from_str("BQW1ABLREJtw8hA3WqpvfvUZppVWaG127hAoVsNnmNKS").unwrap()
        )
    }

    #[test]
    async fn test_get_token_largest_accounts() {
        let client = Client::default();
        let token_largest_accounts = client
            .get_token_largest_account(
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
        let data = client
            .get_account_data(
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
        let data = client
            .get_multiple_accounts_data(&[
                &Pubkey::from_str("CfGsBm5shwwb5tVpFjdj8zPbYCxJ3LhwNxBSCX7WFCCJ").unwrap(),
                &Pubkey::from_str("GxXXdsdfT4Z7ntYphmBdLRTBTLHPu2cdtCEDK56uGK28").unwrap(),
            ])
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
            Pubkey::from_str("CCUWF8sALfvVtv1EvKwuM4p7ZgUWvsrKVzQfFGmBz6pa").unwrap()
        );
    }

    #[test]
    async fn test_get_position_owner() {
        let client = Client::default();
        let pubkey = client
            .get_position_owner(
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
    async fn test_get_all_owners_by_mint() {
        let client = Client::default();
        let pubkeys = client
            .get_all_owners_by_mint(
                &vec![
                    &Pubkey::from_str("BLt9DipvNYvCddekZn4MuvDwxziuda6wR2Buk3hAbJwF").unwrap(),
                    &Pubkey::from_str("CWyChY7verwn1w2ZgzaRGTqVUxSeKbBqTPgiJJFjJ9LH").unwrap(),
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
