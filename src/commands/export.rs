use std::path::PathBuf;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Export to mrpack format
    Modrinth {
        /// Path to mrpack overrides
        overrides_path: Option<PathBuf>
    },

    /// Export to a curseforge pack
    Curseforge {
        /// Path to overrides
        overrides_path: Option<PathBuf>
    }
}