use std::{collections::HashMap, future::Future};

use lazy_regex::regex_is_match;
use reqwest::{Client, Response, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
pub enum ModrinthError {
    #[error("{0} is not a valid modrinth id/slug")]
    InvalidId(String),

    #[error("You exceeded modrinth's ratelimit, try again in {0} seconds")]
    RateLimitExceeded(String),

    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),
}

type Result<T> = std::result::Result<T, ModrinthError>;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Project {
    pub title: String,
    pub client_side: SideSupportType,
    pub server_side: SideSupportType,
    pub project_type: ProjectType,
    pub id: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum SideSupportType {
    Required,
    Optional,
    Unsupported,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ProjectType {
    Project,
    Mod,
    Shader,
    Plugin,
    Modpack,
    Datapack,
    ResourcePack,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Version {
    version_number: String,
    dependencies: Option<VersionDependency>,
    game_versions: Vec<String>,
    loaders: Vec<String>,
    id: String,
    project_id: String,
    files: Vec<VersionFile>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VersionFile {
    hashes: HashMap<String, String>,
    url: String,
    filename: String,
    primary: bool,
    size: usize,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VersionDependency {
    version_id: Option<String>,
    project_id: Option<String>,
    file_name: Option<String>,
    dependency_type: DependencyType,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum DependencyType {
    Required,
    Optional,
    Incompatible,
    Embedded,
    Unsupported,
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchFacet {
    ProjectType(ProjectType),
    Categories(String),
    Versions(String),
}

impl Serialize for SearchFacet {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let string = match self {
            SearchFacet::ProjectType(project_type) => format!(
                "project_type: {}",
                serde_json::to_string(&project_type).unwrap()
            ),
            SearchFacet::Categories(category) => format!("categories: {category}"),
            SearchFacet::Versions(version) => format!("versions: {version}"),
        };

        serializer.collect_str(&string)
    }
}

const API_URL: &str = "https://api.modrinth.com/v2";

pub struct ModrinthAPI {
    client: Client,
}
impl ModrinthAPI {
    // user agent: https://docs.modrinth.com/#section/User-Agents
    pub fn new(user_agent: String) -> Self {
        ModrinthAPI {
            client: Client::builder().user_agent(user_agent).build().unwrap(),
        }
    }

    async fn fetch<T, Fut>(&self, f: impl FnOnce() -> Fut) -> Result<T>
    where
        T: DeserializeOwned,
        Fut: Future<Output = reqwest::Result<Response>>,
    {
        let res = f().await?.error_for_status()?;
        check_ratelimit(&res)?;

        Ok(res.json().await?)
    }

    async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        self.fetch(|| self.client.get(url).send()).await
    }

    pub async fn get_project(&self, id: &str) -> Result<Project> {
        check_id(id)?;
        self.get(&format!("{API_URL}/project/{id}")).await
    }

    pub async fn get_project_versions(&self, id: &str) -> Result<Vec<Version>> {
        check_id(id)?;
        self.get(&format!("{API_URL}/{id}/version")).await
    }

    pub async fn get_version(&self, id: &str) -> Result<Version> {
        check_id(id)?;
        self.get(&format!("{API_URL}/version/{id}")).await
    }

    pub async fn versions_from_hashes(&self, hashes: Vec<String>) -> Result<Vec<Version>> {
        #[derive(Serialize)]
        struct Body {
            hashes: Vec<String>,
            algorithm: String,
        }

        self.fetch(|| {
            self.client
                .post(format!("{API_URL}/version_files"))
                .json(&Body {
                    hashes,
                    algorithm: "sha1".to_owned(),
                })
                .send()
        })
        .await
    }

    pub async fn latest_versions_from_hashes(
        &self,
        hashes: Vec<String>,
        loaders: Option<Vec<String>>,
        game_versions: Option<Vec<String>>,
    ) -> Result<Vec<Version>> {
        #[derive(Serialize)]
        struct Body {
            hashes: Vec<String>,
            algorithm: String,
            loaders: Option<Vec<String>>,
            game_versions: Option<Vec<String>>,
        }

        self.fetch(|| {
            self.client
                .post(format!("{API_URL}/versions_files/update"))
                .json(&Body {
                    hashes,
                    algorithm: "sha1".to_owned(),
                    loaders,
                    game_versions,
                })
                .send()
        })
        .await
    }

    pub async fn search(
        &self,
        query: &str,
        facets: Vec<Vec<SearchFacet>>,
        limit: i32,
    ) -> Result<()> {
        self.fetch(|| {
            self.client
                .get(format!(
                    "{API_URL}/search/?query={query}?facets{}?limit={limit}",
                    serde_json::to_string(&facets).unwrap()
                ))
                .send()
        })
        .await
    }
}

// check whether a string is a valid modrinth id/slug
fn check_id(str: &str) -> Result<()> {
    match regex_is_match!(r#"^[\w!@$()`.+,"\-']{3,64}$"#, str) {
        true => Ok(()),
        false => Err(ModrinthError::InvalidId(str.to_owned())),
    }
}

fn check_ratelimit(res: &Response) -> Result<()> {
    if res.status() == StatusCode::TOO_MANY_REQUESTS {
        Err(ModrinthError::RateLimitExceeded(
            res.headers()
                .get("X-Ratelimit-Reset")
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
        ))
    } else {
        Ok(())
    }
}
