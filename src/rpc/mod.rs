use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::{Duration, SystemTime};

mod error;
pub use error::Error;
pub type Result<T = ()> = std::result::Result<T, Error>;

/// The default timeout for API requests
pub const DEFAULT_TIMEOUT: u64 = 120;
/// JSON RPC version
pub const JSON_RPC: &str = "2.0";

#[derive(Clone, Debug)]
pub struct Client {
    base_url: String,
    client: reqwest::Client,
}

impl Default for Client {
    fn default() -> Self {
        let url = std::env::var("SOL_RPC_ENDPOINT")
            .unwrap_or("https://api.mainnet-beta.solana.com".to_string());
        Self::new_with_base_url(url)
    }
}

impl Client {
    /// Create a new client using a given base URL and a default
    /// timeout. The library will use absoluate paths based on this
    /// base_url.
    pub fn new_with_base_url(base_url: String) -> Self {
        Self::new_with_timeout(base_url, DEFAULT_TIMEOUT)
    }

    /// Create a new client using a given base URL, and request
    /// timeout value.  The library will use absoluate paths based on
    /// the given base_url.
    pub fn new_with_timeout(base_url: String, timeout: u64) -> Self {
        let client = reqwest::Client::builder()
            .gzip(true)
            .timeout(Duration::from_secs(timeout))
            .build()
            .unwrap();
        Self { base_url, client }
    }

    async fn post<T: DeserializeOwned, D: Serialize>(&self, data: D) -> Result<T> {
        #[derive(Clone, Serialize, Deserialize, Debug)]
        struct ErrorResponse {
            code: isize,
            message: String,
        }

        #[derive(Clone, Serialize, Deserialize, Debug)]
        #[serde(untagged)]
        #[serde(rename_all = "lowercase")]
        enum Response<T> {
            Result { result: T },
            Error { error: ErrorResponse },
        }

        #[derive(Clone, Serialize, Deserialize, Debug)]
        struct FullResponse<T> {
            jsonrpc: String,
            #[serde(flatten)]
            response: Response<T>,
            id: String,
        }

        #[derive(Serialize, Debug)]
        pub struct AssetsByAuthorityResponse {
            pub items: Vec<Item>,
        }

        #[derive(Serialize, Debug)]
        pub struct Item {
            pub id: String,
        }

        let request = self.client.post(&self.base_url).json(&data);
        let response = request.send().await?;
        let body = response.text().await?;
        let v: FullResponse<T> = serde_json::from_str(&body)?;
        match v.response {
            Response::Result { result, .. } => Ok(result),
            Response::Error {
                error: ErrorResponse { code, message },
            } => Err(error::Error::NodeError(message, code)),
        }
    }
}

#[derive(Clone, Deserialize, Debug, Serialize)]
pub(crate) struct NodeCall {
    jsonrpc: String,
    id: String,
    #[serde(flatten)]
    method: Method,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
#[allow(clippy::enum_variant_names)]
#[serde(tag = "method")]
#[serde(rename_all = "camelCase")]
enum Method {
    GetAccountInfo { params: Vec<GetAccountInfoParam> },
    GetTokenLargestAccounts { params: Vec<String> },
    GetAssetsByAuthority { params: GetAssetsByAuthorityParams },
}

#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(untagged)]
enum GetAccountInfoParam {
    Pubkey(String),
    Encoding(Encoding),
}
#[derive(Clone, Deserialize, Debug, Serialize)]
struct Encoding {
    encoding: String,
}

impl NodeCall {
    fn new(request: Method) -> NodeCall {
        NodeCall {
            jsonrpc: JSON_RPC.to_string(),
            id: now_millis(),
            method: request,
        }
    }

    pub(crate) fn get_account_info(address: String) -> Self {
        Self::new(Method::GetAccountInfo {
            params: vec![
                GetAccountInfoParam::Pubkey(address),
                GetAccountInfoParam::Encoding(Encoding {
                    encoding: "base64".to_string(),
                }),
            ],
        })
    }

    fn get_token_largest_accounts(pubkey: String) -> Self {
        Self::new(Method::GetTokenLargestAccounts {
            params: vec![pubkey],
        })
    }

    fn get_assets_by_authority(authority_address: String) -> Self {
        Self::new(Method::GetAssetsByAuthority {
            params: GetAssetsByAuthorityParams {
                authority_address,
                page: 1,
                limit: 100,
            },
        })
    }
}

#[derive(Clone, Deserialize, Debug, Serialize)]
struct GetAssetParams {
    id: String,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
struct GetAccountInfoParams {
    address: String,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GetAssetsByAuthorityParams {
    authority_address: String,
    page: usize,
    limit: usize,
}

fn now_millis() -> String {
    let ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    ms.as_millis().to_string()
}

async fn get_account_info(client: &Client, pubkey: &str) -> Result<String> {
    #[derive(Deserialize, Debug)]
    pub struct Response {
        pub value: Value,
    }

    #[derive(Deserialize, Debug)]
    pub struct Value {
        pub data: Vec<String>,
    }

    let json = NodeCall::get_account_info(pubkey.to_string());
    let account_response: Response = client.post(&json).await?;
    Ok(account_response.value.data[0].clone())
}

async fn get_token_largest_accounts(client: &Client, pubkey: &str) -> Result<String> {
    #[derive(Deserialize, Debug)]
    struct Response {
        pub value: Vec<Value>,
    }
    #[derive(Deserialize, Debug)]
    struct Value {
        pub address: String,
    }
    let json = NodeCall::get_token_largest_accounts(pubkey.to_string());
    let response: Response = client.post(&json).await?;
    Ok(response.value[0].address.clone())
}

pub async fn get_assets_by_authority(client: &Client, authority: &str) -> Result<String> {
    #[derive(Deserialize, Debug)]
    pub struct AssetsByAuthorityResponse {
        pub items: Vec<Item>,
    }

    #[derive(Deserialize, Debug)]
    pub struct Item {
        pub id: String,
    }
    let json = NodeCall::get_assets_by_authority(authority.to_string());
    let asset_response: AssetsByAuthorityResponse = client.post(&json).await?;
    Ok(asset_response.items[0].id.clone())
}

pub async fn get_position_owner(
    client: &Client,
    position_id: &str,
) -> Result<solana_sdk::pubkey::Pubkey> {
    use base64::Engine;
    let asset_by_authority = get_assets_by_authority(client, position_id).await?;
    let token_largest_accounts = get_token_largest_accounts(client, &asset_by_authority).await?;
    let account_info = get_account_info(client, &token_largest_accounts).await?;
    let data = base64::engine::general_purpose::STANDARD
        .decode(account_info)
        .unwrap();
    Ok(solana_sdk::pubkey::Pubkey::new(&data[32..64]))
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;
    use tokio::test;

    #[test]
    async fn test_get_assets_by_authority() {
        let client = Client::default();
        let asset_by_authority =
            get_assets_by_authority(&client, "E6ELFUZMahhCgsPCeEYXtQT51Yq24Yg4WAhmQJBsGxhg")
                .await
                .unwrap();
        assert_eq!(
            asset_by_authority,
            "BLt9DipvNYvCddekZn4MuvDwxziuda6wR2Buk3hAbJwF"
        )
    }

    #[test]
    async fn test_get_token_largest_accounts() {
        let client = Client::default();
        let token_largest_accounts =
            get_token_largest_accounts(&client, "BLt9DipvNYvCddekZn4MuvDwxziuda6wR2Buk3hAbJwF")
                .await
                .unwrap();
        assert_eq!(
            token_largest_accounts,
            "CfGsBm5shwwb5tVpFjdj8zPbYCxJ3LhwNxBSCX7WFCCJ"
        )
    }

    #[test]
    async fn test_get_account_info() {
        use base64::Engine;
        let client = Client::default();
        let get_asset = get_account_info(&client, "CfGsBm5shwwb5tVpFjdj8zPbYCxJ3LhwNxBSCX7WFCCJ")
            .await
            .unwrap();
        let data = base64::engine::general_purpose::STANDARD
            .decode(&get_asset)
            .unwrap();
        let pubkey = solana_sdk::pubkey::Pubkey::new(&data[32..64]);
        assert_eq!(
            pubkey,
            solana_sdk::pubkey::Pubkey::from_str("ADqp77vvKapHsU2ymsaoMojXpHjdhxLcfrgWWtaxYCVU")
                .unwrap()
        )
    }

    #[test]
    async fn test_get_position_owner() {
        let client = Client::default();
        let pubkey = get_position_owner(&client, "E6ELFUZMahhCgsPCeEYXtQT51Yq24Yg4WAhmQJBsGxhg")
            .await
            .unwrap();
        assert_eq!(
            pubkey,
            solana_sdk::pubkey::Pubkey::from_str("ADqp77vvKapHsU2ymsaoMojXpHjdhxLcfrgWWtaxYCVU")
                .unwrap()
        )
    }
}
