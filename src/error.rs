use crate::api::modrinth::ModrinthError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Modrinth(#[from] ModrinthError),

    #[error("{0}")]
    Other(String)
}

pub type Result<T> = std::result::Result<T, Error>;