use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("error code {1} from node: {0}")]
    NodeError(String, isize),
    #[error("error deserializing JSON response")]
    JsonDeserialization(#[from] serde_json::Error),
}
