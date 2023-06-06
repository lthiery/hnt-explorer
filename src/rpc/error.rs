use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("error code {1} from node: {0}")]
    NodeError(String, isize),
    #[error("error deserializing JSON response: {0}")]
    JsonDeserialization(#[from] serde_json::Error),
    #[error("base64 decode error: {0}")]
    B64Decode(#[from] base64::DecodeError),
    #[error("solana parse pubkey error: {0}")]
    SolanaParsePubkey(#[from] solana_sdk::pubkey::ParsePubkeyError),
    #[error("join error: {0} ")]
    Join(#[from] tokio::task::JoinError),
}