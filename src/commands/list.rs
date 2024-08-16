use std::fmt::Write;

use tokio::try_join;

use crate::{
    api::{curseforge::Mod, modrinth::{Project, ProjectLicense}}, cli::ListArgs, error::Result, structs::index::{AddonSource, Index, ProjectType}, CURSEFORGE, MODRINTH
};

pub async fn list(args: ListArgs) -> Result<()> {
    let index = Index::read().await?;

    let mut out = String::new();

    if args.markdown {

    } else {
        
    }

    Ok(())
}