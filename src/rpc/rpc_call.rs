use super::*;
use serde::Serializer;

/// JSON RPC version
pub const JSON_RPC: &str = "2.0";

#[derive(Clone, Debug, Serialize)]
pub(crate) struct RpcCall<'se> {
    jsonrpc: String,
    id: String,
    #[serde(flatten)]
    method: Method<'se>,
}

#[derive(Clone, Debug)]
struct InnerPubkey<'se> {
    pubkey: &'se Pubkey,
}

impl<'se> Serialize for InnerPubkey<'se> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.pubkey.to_string())
    }
}

impl<'se> From<&'se Pubkey> for InnerPubkey<'se> {
    fn from(pubkey: &'se Pubkey) -> Self {
        InnerPubkey { pubkey }
    }
}

#[derive(Clone, Debug, Serialize)]
#[allow(clippy::enum_variant_names)]
#[serde(tag = "method")]
#[serde(rename_all = "camelCase")]
enum Method<'se> {
    GetMultipleAccounts {
        params: Vec<GetAccountInfoParam<'se>>,
    },
    #[allow(unused)]
    GetAccountInfo {
        params: Vec<GetAccountInfoParam<'se>>,
    },
    GetTokenLargestAccounts {
        params: Vec<InnerPubkey<'se>>,
    },
    GetTokenSupply {
        params: Vec<InnerPubkey<'se>>,
    },
    GetAssetsByAuthority {
        params: GetAssetsByAuthorityParams<'se>,
    },
    GetProgramAccounts {
        params: Vec<GetProgramAccountsParams<'se>>,
    },
}

#[derive(Clone, Debug, Serialize)]
struct GetMultipleAccounts<'se> {
    array: Vec<&'se Pubkey>,
    encoding: EncodingType,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
enum GetAccountInfoParam<'se> {
    #[allow(unused)]
    Pubkey(InnerPubkey<'se>),
    Pubkeys(Vec<InnerPubkey<'se>>),
    Encoding(Encoding),
}

#[derive(Clone, Debug, Serialize)]
struct Encoding {
    encoding: EncodingType,
}

#[derive(Copy, Clone, Debug, Serialize, Default)]
#[serde(rename_all = "lowercase")]
enum EncodingType {
    #[default]
    Base64,
}

impl<'se> RpcCall<'se> {
    fn new(request: Method) -> RpcCall {
        RpcCall {
            jsonrpc: JSON_RPC.to_string(),
            id: now_millis(),
            method: request,
        }
    }

    pub(crate) fn get_multiple_accounts(array: &Vec<&'se Pubkey>) -> Self {
        Self::new(Method::GetMultipleAccounts {
            params: vec![
                GetAccountInfoParam::Pubkeys(array.into_iter().map(|p| (*p).into()).collect()),
                GetAccountInfoParam::Encoding(Encoding {
                    encoding: EncodingType::Base64,
                }),
            ],
        })
    }

    pub(crate) fn get_account_info(address: &'se Pubkey) -> Self {
        Self::new(Method::GetAccountInfo {
            params: vec![
                GetAccountInfoParam::Pubkey(address.into()),
                GetAccountInfoParam::Encoding(Encoding {
                    encoding: EncodingType::Base64,
                }),
            ],
        })
    }

    pub(crate) fn get_token_largest_accounts(pubkey: &'se Pubkey) -> Self {
        Self::new(Method::GetTokenLargestAccounts {
            params: vec![pubkey.into()],
        })
    }

    pub(crate) fn get_token_supply(pubkey: &'se Pubkey) -> Self {
        Self::new(Method::GetTokenSupply {
            params: vec![pubkey.into()],
        })
    }

    pub(crate) fn get_assets_by_authority(authority_address: &'se Pubkey) -> Self {
        Self::new(Method::GetAssetsByAuthority {
            params: GetAssetsByAuthorityParams {
                authority_address: authority_address.into(),
                page: 1,
                limit: 100,
            },
        })
    }

    #[allow(unused)]
    pub(crate) fn get_program_accounts(program_id: &'se Pubkey) -> Self {
        Self::get_program_accounts_with_filters(program_id, vec![])
    }

    pub(crate) fn get_program_accounts_with_filters(
        program_id: &'se Pubkey,
        filters: Vec<GetProgramAccountsFilter<'se>>,
    ) -> Self {
        Self::new(Method::GetProgramAccounts {
            params: vec![
                GetProgramAccountsParams::Pubkey(program_id.into()),
                GetProgramAccountsParams::Object(GetProgramAccountsObject {
                    filters,
                    encoding: Encoding {
                        encoding: EncodingType::Base64,
                    },
                }),
            ],
        })
    }
}

#[derive(Clone, Debug, Serialize)]
struct GetAssetParams {
    id: String,
}

#[derive(Clone, Debug, Serialize)]
struct GetAccountInfoParams {
    address: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GetAssetsByAuthorityParams<'se> {
    authority_address: InnerPubkey<'se>,
    page: usize,
    limit: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
enum GetProgramAccountsParams<'se> {
    #[allow(unused)]
    Pubkey(InnerPubkey<'se>),
    Object(GetProgramAccountsObject<'se>),
}
#[derive(Clone, Debug, Serialize)]
struct GetProgramAccountsObject<'se> {
    filters: Vec<GetProgramAccountsFilter<'se>>,
    #[serde(flatten)]
    encoding: Encoding,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataSlice {
    pub offset: usize,
    pub length: usize,
}

fn now_millis() -> String {
    let ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    ms.as_millis().to_string()
}
