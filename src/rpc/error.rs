use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("error code {code} from node: {msg}. Request was: \"{request_json}\"")]
    NodeError {
        msg: String,
        code: isize,
        request_json: String,
    },
    #[error("error deserializing JSON response: {0}. Full text: {1} to request: {2}")]
    JsonDeserialization(serde_json::Error, String, String),
    #[error("base64 decode error: {0}")]
    B64Decode(#[from] base64::DecodeError),
    #[error("solana parse pubkey error: {0}")]
    SolanaParsePubkey(#[from] anchor_lang::solana_program::pubkey::ParsePubkeyError),
    #[error("join error: {0} ")]
    Join(#[from] tokio::task::JoinError),
    #[error("no asset by authority for {0}")]
    NoAssetByAuthority(String),
    #[error("parse int error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("Account not found.")]
    AccountNotFound,
    #[error("try from slice error: {0}")]
    TryFromSlice(#[from] std::array::TryFromSliceError),
}

impl Error {
    pub fn json_deser(erro: serde_json::Error, full_text: String, request: String) -> Self {
        Error::JsonDeserialization(erro, full_text, request)
    }
}
