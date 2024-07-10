use std::future::Future;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, Response,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

const API_URL: &str = "api.curseforge.com";

#[derive(thiserror::Error, Debug)]
pub enum CurseError {
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),
}

type Result<T> = std::result::Result<T, CurseError>;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Mod {
    id: i32,
    game_id: i32,
    name: String,
    class_id: i32,
    allow_mod_distribution: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct File {
    id: i32,
    mod_id: i32,
    file_name: String,
    dependencies: Vec<FileDependency>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileDependency {
    mod_id: i32,
    relation_type: FileRelationType
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum FileRelationType {
    EmbeddedLibrary = 1,
    OptionalDependency = 2,
    RequiredDependency = 3,
    Tool = 4,
    Incompatible = 5,
    Include = 6,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintMatches {
    exact_matches: Vec<Match>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Match {
    id: i32,
    file: File,
}

pub struct CurseAPI {
    client: Client,
}
impl CurseAPI {
    pub fn new(api_key: String) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_str(&api_key).unwrap());

        CurseAPI {
            client: Client::builder().default_headers(headers).build().unwrap(),
        }
    }

    async fn fetch<T, Fut>(&self, f: impl FnOnce() -> Fut) -> Result<T>
    where
        T: DeserializeOwned,
        Fut: Future<Output = reqwest::Result<Response>>,
    {
        Ok(f().await?.error_for_status()?.json().await?)
    }

    async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        self.fetch(|| self.client.get(url).send()).await
    }

    pub async fn get_mod(&self, id: &i32) -> Result<Mod> {
        self.get(&format!("{API_URL}/v1/mods/{id}")).await
    }

    pub async fn get_mods(&self, ids: Vec<i32>) -> Result<Mod> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body {
            mod_ids: Vec<i32>,
        }

        self.fetch(|| {
            self.client
                .post(format!("{API_URL}/v1/mods"))
                .json(&Body { mod_ids: ids })
                .send()
        })
        .await
    }

    pub async fn get_mod_file(&self, mod_id: &i32, file_id: &i32) -> Result<File> {
        self.get(&format!("{API_URL}/v1/mods/{mod_id}/files/{file_id}")).await
    }

    pub async fn get_mod_files(&self, id: &i32) -> Result<Vec<File>> {
        self.get(&format!("{API_URL}/v1/mods/{id}/files")).await
    }

    pub async fn get_fingerprint_matches(&self, fingerprints: Vec<i32>) -> Result<FingerprintMatches> {
        #[derive(Serialize)]
        struct Body {
            fingerprints: Vec<i32>
        }

        self.fetch(|| {
            self.client
                .post(format!("{API_URL}/v1/fingerprints"))
                .json(&Body { fingerprints })
                .send()
        }).await
    }
    //get_fingerprint_matches
}
