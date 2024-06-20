use std::path::PathBuf;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Import from an mrpack file
    Modrinth {
        /// Path to mrpack file
        #[arg(required = true)]
        path: PathBuf
    }
}