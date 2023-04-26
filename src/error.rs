use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("solana pubkey parse: {0}")]
    SolanaPubkeyParse(#[from] solana_sdk::pubkey::ParsePubkeyError),
    #[error("solana client error: {0}")]
    SolanaClient(#[from] solana_client::client_error::ClientError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("anchor lang: {0}")]
    AnchorLang(Box<anchor_lang::error::Error>),
    #[error("base64 decode error: {0}")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("invalid subdao: {0}")]
    InvalidSubDao(solana_sdk::pubkey::Pubkey),
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("parse int error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("invali timestamp: {0}")]
    InvalidTimestamp(i64),
    #[error("csv error: {0}")]
    Csv(#[from] csv::Error),
    #[error("rust decimal error: {0}")]
    RustDecimal(#[from] rust_decimal::Error),
    #[error("try from int error: {0}")]
    TryFromInt(#[from] std::num::TryFromIntError),
    #[error("axum error: {0}")]
    Axum(#[from] axum::BoxError),
    #[error("{0}")]
    Custom(&'static str),
}

impl From<anchor_lang::error::Error> for Error {
    fn from(e: anchor_lang::error::Error) -> Self {
        Error::AnchorLang(Box::new(e))
    }
}
