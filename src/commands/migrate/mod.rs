pub mod loader;
pub mod minecraft;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    Loader,
    Minecraft
}