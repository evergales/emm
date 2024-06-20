use std::path::PathBuf;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Export to mrpack format
    Modrinth {
        /// Path to mrpack overrides
        overrides_path: Option<PathBuf>
    }
}