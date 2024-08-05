use std::path::PathBuf;
use clap::{Parser, Subcommand};
use clap_complete::Shell;

use crate::structs::pack::ModLoader;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub subcommand: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new modpack in the current folder
    #[command(alias = "i")]
    Init(InitArgs),

    /// Add a mod to the current pack
    #[command(alias = "a")]
    Add(AddArgs),

    /// Remove a mod from the current pack
    #[command(aliases = ["rm", "r"])]
    Remove(RemoveArgs),

    /// Update all mods in this modpack
    #[command(alias = "up")]
    Update,

    /// Pin a mod to exclude it from updates
    Pin(PinArgs),

    /// Unpin a mod to reinclude it in updates
    Unpin(UnpinArgs),

    /// List all addons in this modpack
    List(ListArgs),

    /// Import modpack from another format
    Import(ImportArgs),

    /// Export your modpack
    Export(ExportArgs),

    /// Migrate your modpack to a new minecraft or mod loader version
    Migrate(MigrateArgs),

    /// Print shell completions for specified shell
    Completion {
        #[clap(value_enum)]
        shell: Shell
    }

}

#[derive(clap::Args)]
pub struct InitArgs {
    /// The name of your modpack
    #[arg(long, short = 'n')]
    pub name: Option<String>,

    /// A short description of your modpack, put in "quotes"
    #[arg(long, short = 'd')]
    pub description: Option<String>,

    /// The authors of this modpack
    #[arg(long, short = 'a')]
    pub authors: Option<Vec<String>>,

    /// Use the latest minecraft release
    #[arg(long, short = 'l')]
    pub latest: bool,

    #[arg(long, short = 'L')]
    pub loader: Option<ModLoader>,

    /// Use the latest minecraft snapshot
    #[arg(long, visible_alias = "ls")]
    pub latest_snapshot: bool,

    /// Show snapshots in version select
    #[arg(long, short = 's', visible_alias = "snapshots")]
    pub show_snapshots: bool
}

#[derive(clap::Args)]
pub struct RemoveArgs {
    /// List of mod names/ids you want to remove from this modpack
    #[arg(required = true)]
    pub mods: Vec<String>
}

#[derive(clap::Args)]
pub struct AddArgs {
    #[command(subcommand)]
    pub subcommand: AddCommands
}

#[derive(clap::Args)]
pub struct PinArgs {
    /// A mod name/id you want to pin
    #[arg(name = "mod")]
    pub m: String,

    /// Pin the mod to this specific version id
    #[arg(long, short = 'v')]
    pub version: Option<String>
}

#[derive(clap::Args)]
pub struct UnpinArgs {
    /// A mod name/id you want to unpin
    #[arg(name = "mod")]
    pub m: String
}

#[derive(clap::Args)]
pub struct ListArgs {
    /// List mods in more detail
    #[arg(long, short = 'v')]
    pub verbose: bool
}

#[derive(clap::Args)]
pub struct ImportArgs {
    #[command(subcommand)]
    pub subcommand: ImportCommmands
}

#[derive(clap::Args)]
pub struct ExportArgs {
    #[command(subcommand)]
    pub subcommand: ExportCommands
}

#[derive(clap::Args)]
pub struct MigrateArgs {
    #[command(subcommand)]
    pub subcommand: MigrateCommands
}

#[derive(Subcommand)]
pub enum AddCommands {
    /// Add mods from modrinth
    #[command(visible_alias = "mr")]
    Modrinth {
        /// Project ids/slugs or search terms
        #[arg(required = true)]
        ids: Vec<String>,

        /// The version id of the mod to add, ignores compatability checks
        #[arg(long, short = 'v')]
        version: Option<String>
    },

    /// Add mods from curseforge
    #[command(visible_alias = "cf")]
    Curseforge {
        /// Project ids/slugs or search terms
        #[arg(required = true)]
        ids: Vec<String>,

        /// The version id of the mod to add, ignores compatability checks
        #[arg(long, short = 'v')]
        version: Option<i32>
    },

    /// Add mods from a github repo's releases
    #[command(visible_alias = "gh")]
    Github {
        /// The repository to add mods from,
        /// github url or owner/repo accepted
        #[arg(required = true)]
        repo: String,
        
        /// The release tag to use
        #[arg(long, short = 't')]
        tag: Option<String>,

        /// Use the first release asset
        #[arg(long, short = 'f')]
        first_asset: bool
    }
}

#[derive(Subcommand)]
pub enum ImportCommmands {
    /// Import from an mrpack file
    #[command(visible_aliases = ["mr", "mrpack"])]
    Modrinth {
        /// Path to mrpack file
        path: PathBuf
    },

    /// Import from a curseforge pack zip file
    #[command(visible_alias = "cf")]
    Curseforge {
        /// Path to pack zip file
        path: PathBuf
    },

    /// Import from a packwiz pack
    #[command(visible_alias = "pw")]
    Packwiz {
        /// Url or file path to a packwiz pack.toml file
        source: String
    }
}

#[derive(Subcommand)]
pub enum ExportCommands {
    /// Export to mrpack format
    #[command(visible_aliases = ["mr", "mrpack"])]
    Modrinth {
        /// Path to mrpack overrides
        #[arg(long, short = 'o')]
        overrides_path: Option<PathBuf>
    },

    /// Export to a curseforge pack
    #[command(visible_alias = "cf")]
    Curseforge {
        /// Path to overrides
        #[arg(long, short = 'o')]
        overrides_path: Option<PathBuf>
    },

    /// Export to a packwiz pack
    #[command(visible_alias = "pw")]
    Packwiz {
        /// Output folder path
        export_path: PathBuf
    }
}

#[derive(Subcommand)]
pub enum MigrateCommands {
    /// Migrate to a new loader version
    #[command(visible_alias = "l")]
    Loader,

    /// Migrate to a new minecraft version
    #[command(visible_alias = "mc")]
    Minecraft
}