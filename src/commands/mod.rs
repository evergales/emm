pub mod init;
pub mod add;
pub mod remove;
pub mod update;
pub mod pin;
pub mod unpin;
pub mod migrate;
pub mod export;
pub mod import;

use clap::Subcommand;
use clap_complete::Shell;

use crate::structs::Mod;

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new modpack in the current folder
    Init,

    /// Add a mod to the current profile
    #[command(alias = "a")]
    Add {
        /// List of mod ids/slugs or names you want to search
        #[arg(required = true)]
        mods: Vec<String>,

        /// ignore minecraft version when checking compatability
        #[arg(long, short = 'V')]
        ignore_version: bool,

        /// ignore mod loader when checking compatability
        #[arg(long, short = 'L')]
        ignore_loader: bool
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

// determine if a mod matches a name or id 
pub fn mod_matches(m: &Mod, s: &String) -> bool {
    // names set to lowercase to make matching less case sensitive
    if m.name.to_lowercase() == s.to_lowercase() { return true; }

    if m.modrinth_id.is_some() {
        return m.modrinth_id.as_ref().unwrap() == s;
    }

    if m.curseforge_id.is_some() {
        if let Ok(id) = s.parse::<i32>() {
            return m.curseforge_id.unwrap() == id;
        }
    }

    false // I guess if it doesnt have a modrinth or curseforge id this is here
}