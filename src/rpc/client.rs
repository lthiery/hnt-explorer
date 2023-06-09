use super::*;

/// The default timeout for API requests
pub const DEFAULT_TIMEOUT: u64 = 120;

#[derive(Clone, Debug)]
pub struct Client {
    base_url: String,
    client: reqwest::Client,
    max_retries: u64,
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
        Self {
            base_url,
            client,
            max_retries: 5,
        }
    }

    pub(crate) async fn post<T: DeserializeOwned, D: Serialize>(&self, data: &D) -> Result<T> {
        let mut result = self.post_attempt(data).await;
        let mut retries = 0;
        while let Err(Error::NodeError { .. }) = result {
            retries += 1;
            if retries > self.max_retries {
                return result;
            }
            tokio::time::sleep(Duration::from_secs(retries)).await;
            result = self.post_attempt(data).await;
        }
        result
    }

    pub(crate) async fn post_attempt<T: DeserializeOwned, D: Serialize>(
        &self,
        data: &D,
    ) -> Result<T> {
        #[derive(Clone, Serialize, Deserialize, Debug)]
        #[serde(untagged)]
        enum AllResponse<T> {
            Ok(FullResponse<T>),
            Err { error: String },
        }

        #[derive(Clone, Serialize, Deserialize, Debug)]
        struct FullResponse<T> {
            jsonrpc: String,
            #[serde(flatten)]
            response: Response<T>,
            id: Option<String>,
        }
        #[derive(Clone, Serialize, Deserialize, Debug)]
        #[serde(untagged)]
        #[serde(rename_all = "lowercase")]
        enum Response<T> {
            Result { result: T },
            Error { error: ErrorResponse },
        }
        #[derive(Clone, Serialize, Deserialize, Debug)]
        struct ErrorResponse {
            code: isize,
            message: String,
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
        let response: AllResponse<T> = serde_json::from_str(&body)
            .map_err(|e| Error::json_deser(e, body, serde_json::to_string(&data).unwrap()))?;

        match response {
            AllResponse::Ok(response) => match response.response {
                Response::Result { result, .. } => Ok(result),
                Response::Error {
                    error: ErrorResponse { code, message },
                } => Err(Error::NodeError {
                    code,
                    msg: message,
                    request_json: serde_json::to_string(&data).unwrap(),
                }),
            },
            AllResponse::Err { error } => Err(Error::NodeError {
                code: -1,
                msg: error,
                request_json: serde_json::to_string(&data).unwrap(),
            }),
        }
    }
}
