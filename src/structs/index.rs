use serde::{Deserialize, Serialize};

use crate::error::Error;

use super::pack::ModLoader;

#[derive(Debug, Deserialize, Serialize)]
pub struct Index {
    pub addons: Vec<Addon>
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Addon {
    pub name: String,
    #[serde(rename = "type")]
    pub project_type: ProjectType,
    pub side: Side,
    pub source: AddonSource,
    pub options: Option<AddonOptions>,
    #[serde(skip_serializing, default)]
    pub filename: Option<String>
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(tag = "source", rename_all = "lowercase")]
pub enum AddonSource {
    Modrinth(ModrinthSource),
    Curseforge(CurseforgeSource),
    Github(GithubSource),
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[serde(default)]
pub struct AddonOptions {
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub pinned: bool,
    pub mod_loader: Option<ModLoader>, // for mr & cf
    pub game_versions: Option<String>, // for mr & cf
    pub release_channel: Option<ReleaseChannel>
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct ModrinthSource {
    pub id: String,
    pub version: String
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct CurseforgeSource {
    pub id: i32,
    pub version: i32
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct GithubSource {
    pub repo: String,
    pub tag: String,
    pub filter_by: ReleaseFilter,
    pub asset_index: usize
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Project,
    Mod,
    Shader,
    Plugin,
    Modpack,
    Datapack,
    Resourcepack,
    Unknown
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    #[default]
    Both,
    Client,
    Server,
}

impl TryFrom<i32> for ProjectType {
    type Error = crate::error::Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {            
            6 => Ok(Self::Mod),
            6552 => Ok(Self::Shader),
            6945 => Ok(Self::Datapack),
            12 => Ok(Self::Resourcepack),
            _ => Err(Error::UnsupportedProjectType(String::new()))
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseFilter {
    Tag,
    Title,
    None
}

impl std::fmt::Display for ReleaseFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", {
            match self {
                Self::Tag => "tag",
                Self::Title => "title",
                Self::None => "none",
            }
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum ReleaseChannel {
    Release,
    #[serde(alias = "prerelease")]
    Beta,
    Alpha
}