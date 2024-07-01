pub mod init;
pub mod add;
pub mod remove;
pub mod update;
pub mod pin;
pub mod unpin;
pub mod list;
pub mod migrate;
pub mod export;
pub mod import;
pub mod modrinth;
pub mod curseforge;

use clap::Subcommand;
use clap_complete::Shell;

use crate::structs::ModLoader;

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new modpack in the current folder
    Init {
        /// The name of your modpack
        #[arg(long, short = 'n')]
        name: Option<String>,

        /// A short description of your modpack, put in "quotes"
        #[arg(long, short = 'd')]
        description: Option<String>,

        /// The authors of this modpack
        #[arg(long, short = 'a')]
        authors: Option<Vec<String>>,

        /// Use the latest minecraft release
        #[arg(long, short = 'l')]
        latest: bool,

        #[arg(long, short = 'L')]
        loader: Option<ModLoader>,

        /// Use the latest minecraft snapshot
        #[arg(long, visible_alias = "ls")]
        latest_snapshot: bool,

        /// Show snapshots in version select
        #[arg(long, short = 's', visible_alias = "snapshots")]
        show_snapshots: bool
    },

    /// Add a mod to the current profile
    #[command(alias = "a")]
    Add {
        /// List of mod ids/slugs or names you want to search
        /// List of Modrinth ids/slugs/names to search and Curseforge ids
        #[arg(required = true)]
        ids: Vec<String>,

        /// The specific version id to use, ignores compatability checks
        #[arg(long)]
        version: Option<String>,
    },

    /// Remove a mod from the current profile (rm/r)
    #[command(aliases = ["rm", "r"])]
    Remove {
        /// List of mod names/ids you want to remove from this modpack
        #[arg(required = true)]
        mods: Vec<String>
    },

    /// Update all mods in this modpack
    #[command(alias = "up")]
    Update,

    /// Pin a mod to exclude it from updates
    Pin {
        #[arg(name="mod")]
        /// A mod name/id you want to pin
        m: String,

        /// Pin the mod to this specific version id
        #[arg(long, short='v')]
        version: Option<String>
    },

    /// Unpin a mod to reinclude it in updates
    Unpin {
        #[arg(name="mod")]
        // A mod name/id you want to unpin
        m: String
    },

    List {
        #[arg(long, short = 'v')]
        verbose: bool
    },

    /// Import modpack from another format
    Import {
        #[command(subcommand)]
        subcommand: import::Commands
    },

    /// Export your modpack
    Export {
        #[command(subcommand)]
        subcommand: export::Commands
    },

    /// Migrate your modpack to a new minecraft or mod loader version
    Migrate {
        #[command(subcommand)]
        subcommand: migrate::Commands
    },

    /// Print shell completions for specified shell
    Completion {
        #[clap(value_enum)]
        shell: Shell
    }

}