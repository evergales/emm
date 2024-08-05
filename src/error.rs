#[derive(thiserror::Error, Debug)]
#[error("{0}")]
pub enum Error {
    #[error("{0} not found")]
    NotFound(String),

    #[error("{0} is not a valid modrinth id/slug")]
    InvalidId(String),

    #[error("You exceeded {0}'s ratelimit, try again in {1} seconds")]
    RateLimitExceeded(String, String),

    #[error("The folder you're in doesnt have a modpack, create one with 'emm init'")]
    Uninitialized,

    #[error("{0} loader has no available versions for {1}")]
    NoLoaderSupport(String, String),

    #[error("{0} has no compatible versions with your modpack")]
    NoCompatibleVersions(String),

    #[error("Unable to add {0} because its project type is unsupported")]
    UnsupportedProjectType(String),

    #[error("Unable to import: {0}")]
    BadImport(String),

    #[error("Deprecated api usage: {0}")]
    Deprecated(String),

    #[error("{0}")]
    Other(String),

    Io(#[from] std::io::Error),
    Reqwest(#[from] reqwest::Error),
    JoinError(#[from] tokio::task::JoinError),
    ZipError(#[from] zip::result::ZipError),
    Json(#[from] serde_json::Error)
}

pub type Result<T> = std::result::Result<T, Error>;