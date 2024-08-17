use std::collections::HashMap;

use lazy_regex::regex_is_match;
use reqwest::{Client, RequestBuilder, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    structs::index::ProjectType,
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Project {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub client_side: SideSupportType,
    pub server_side: SideSupportType,
    pub project_type: ProjectType,
    pub id: String,
    pub license: Option<ProjectLicense>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SideSupportType {
    Required,
    Optional,
    Unsupported,
    Unknown
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProjectLicense {
    pub name: String
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Version {
    pub version_number: String,
    pub dependencies: Vec<VersionDependency>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub id: String,
    pub project_id: String,
    pub files: Vec<VersionFile>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VersionFile {
    pub hashes: HashMap<String, String>,
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: usize,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VersionDependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub file_name: Option<String>,
    pub dependency_type: DependencyType,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SearchResult {
    pub hits: Vec<SearchHit>,
    pub offset: i32,
    pub limit: i32,
    pub total_hits: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SearchHit {
    pub title: String,
    pub project_id: String,
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
    pub fn new(user_agent: &str) -> Self {
        ModrinthAPI {
            client: Client::builder().user_agent(user_agent).build().unwrap(),
        }
    }

    async fn fetch<T: DeserializeOwned>(&self, f: impl FnOnce() -> RequestBuilder) -> Result<T> {
        let res = f().send().await?;
        if let Err(err) = res.error_for_status_ref() {
            let err = match err.status().unwrap() {
                StatusCode::TOO_MANY_REQUESTS => Error::RateLimitExceeded(
                    "modrinth".to_owned(),
                    res.headers()
                        .get("X-Ratelimit-Reset")
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_owned(),
                ),
                StatusCode::NOT_FOUND => Error::NotFound(err.url().unwrap().to_string()),
                StatusCode::GONE => Error::Deprecated(err.to_string()),
                _ => Error::Reqwest(err),
            };
            
            return Err(err);
        }

        Ok(res.json().await?)
    }

    async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        self.fetch(|| self.client.get(url)).await
    }

    async fn post<T, B>(&self, url: &str, body: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        self.fetch(|| self.client.post(url).json(body)).await
    }

    pub async fn get_project(&self, id: &str) -> Result<Project> {
        check_id(id)?;
        self.get(&format!("{API_URL}/project/{id}")).await
    }

    pub async fn get_multiple_projects(&self, ids: &[&str]) -> Result<Vec<Project>> {
        self.get(&format!("{API_URL}/projects?ids={}", serde_json::to_string(ids).unwrap())).await
    }

    pub async fn get_project_versions(&self, id: &str) -> Result<Vec<Version>> {
        check_id(id)?;
        self.get(&format!("{API_URL}/project/{id}/version")).await
    }

    pub async fn get_version(&self, id: &str) -> Result<Version> {
        check_id(id)?;
        self.get(&format!("{API_URL}/version/{id}")).await
    }

    pub async fn get_versions(&self, ids: &[&str]) -> Result<Vec<Version>> {
        self.get(&format!(
            "{API_URL}/versions?ids={}",
            serde_json::to_string(ids).unwrap()
        ))
        .await
    }

    pub async fn versions_from_hashes(&self, hashes: &[&str]) -> Result<HashMap<String, Version>> {
        #[derive(Serialize)]
        struct Body<'a> {
            hashes: &'a [&'a str],
            algorithm: String,
        }

        self.post(
            &format!("{API_URL}/version_files"),
            &Body {
                hashes,
                algorithm: "sha1".to_owned(),
            },
        )
        .await
    }

    pub async fn latest_versions_from_hashes(
        &self,
        hashes: &[&str],
        loaders: Option<&[&str]>,
        game_versions: Option<&[&str]>,
    ) -> Result<HashMap<String, Version>> {
        #[derive(Serialize)]
        struct Body<'a> {
            hashes: &'a [&'a str],
            algorithm: String,
            loaders: Option<&'a [&'a str]>,
            game_versions: Option<&'a [&'a str]>,
        }

        self.post(
            &format!("{API_URL}/version_files/update"),
            &Body {
                hashes,
                algorithm: "sha1".to_owned(),
                loaders,
                game_versions,
            },
        )
        .await
    }

    pub async fn search(
        &self,
        query: &str,
        facets: Vec<Vec<SearchFacet>>,
        limit: &i32,
    ) -> Result<SearchResult> {
        self.get(&format!(
            "{API_URL}/search?query={query}&facets={}&limit={limit}",
            serde_json::to_string(&facets).unwrap()
        ))
        .await
    }
}

// check whether a string is a valid modrinth id/slug
fn check_id(str: &str) -> Result<()> {
    match regex_is_match!(r#"^[\w!@$()`.+,"\-']{3,64}$"#, str) {
        true => Ok(()),
        false => Err(Error::InvalidId(str.to_owned())),
    }
}