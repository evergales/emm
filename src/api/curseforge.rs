use murmur2::murmur2;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, RequestBuilder, StatusCode,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use tokio::task::JoinSet;

use crate::{
    error::{Error, Result},
    structs::pack::ModLoader,
};

const API_URL: &str = "https://api.curseforge.com";

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseResponse<T> {
    data: T,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Mod {
    pub id: i32,
    pub game_id: i32,
    pub name: String,
    pub class_id: Option<i32>,
    pub allow_mod_distribution: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub id: i32,
    pub mod_id: i32,
    pub is_available: bool,
    pub file_name: String,
    pub hashes: Vec<FileHash>,
    pub download_url: Option<String>,
    pub game_versions: Vec<String>,
    pub dependencies: Vec<FileDependency>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileHash {
    pub value: String,
    pub algo: i32
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileDependency {
    pub mod_id: i32,
    pub relation_type: FileRelationType,
}

#[derive(Deserialize_repr, Serialize_repr, Debug, Clone)]
#[repr(u8)]
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
    pub exact_matches: Vec<Match>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Match {
    pub id: i32,
    pub file: File,
}

pub struct CurseAPI {
    client: Client,
}
impl CurseAPI {
    pub fn new(api_key: &str) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_str(api_key).unwrap());

        CurseAPI {
            client: Client::builder().default_headers(headers).build().unwrap(),
        }
    }

    async fn fetch<T: DeserializeOwned>(&self, f: impl FnOnce() -> RequestBuilder) -> Result<T> {
        let res = f().send().await?;
        if let Err(err) = res.error_for_status_ref() {
            let err = match err.status().unwrap() {
                StatusCode::NOT_FOUND => Error::NotFound(err.url().unwrap().to_string()),
                _ => Error::Reqwest(err),
            };

            return Err(err);
        }

        let curse_response: CurseResponse<T> = res.json().await?;
        Ok(curse_response.data)
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

    pub async fn search(
        &self,
        query: &str,
        game_version: &str,
        mod_loader: &ModLoader,
        page_size: &i32,
    ) -> Result<Vec<Mod>> {
        self.get(&format!("{API_URL}/v1/search?gameId=432&classId=6&searchFilter={query}&gameVersion={game_version}&modLoaderType={mod_loader}&pageSize={page_size}")).await
    }

    pub async fn get_mod(&self, id: &i32) -> Result<Mod> {
        self.get(&format!("{API_URL}/v1/mods/{id}")).await
    }

    pub async fn get_mod_by_slug(&self, slug: &str) -> Result<Mod> {
        // gameId=432 == minecraft
        // classId=6 == mod
        let res: Vec<Mod> = self
            .get(&format!(
                "{API_URL}/v1/mods/search?gameId=432&classId=6&slug={slug}"
            ))
            .await?;
        match res.is_empty() {
            true => Err(Error::InvalidId(slug.to_owned())),
            false => Ok(res[0].clone()),
        }
    }

    pub async fn get_mods(&self, ids: Vec<i32>) -> Result<Vec<Mod>> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body {
            mod_ids: Vec<i32>,
        }

        self.post(&format!("{API_URL}/v1/mods"), &Body { mod_ids: ids }).await
    }

    pub async fn get_mod_file(&self, mod_id: &i32, file_id: &i32) -> Result<File> {
        self.get(&format!("{API_URL}/v1/mods/{mod_id}/files/{file_id}")).await
    }

    pub async fn get_mod_files(&self, id: &i32) -> Result<Vec<File>> {
        self.get(&format!("{API_URL}/v1/mods/{id}/files")).await
    }

    // using this instead of cf's v1/mods/files path
    // because that has inconsistencies
    pub async fn get_files(&'static self, id_pairs: Vec<(i32, i32)>) -> Result<Vec<File>> {
        let mut tasks: JoinSet<Result<File>> = JoinSet::new();
        for pair in id_pairs {
            let task = async move {
                self.get_mod_file(&pair.0, &pair.1).await
            };
            tasks.spawn(task);
        }
        let mut files = Vec::new();
        while let Some(res) = tasks.join_next().await { files.push(res??) };
        Ok(files)
    }

    // curseforge uses its own modified version of murmur2
    // some bytes get stripped and the hash is calculated with seed 1
    // I could not find this documented anywhere..
    // this implementation is from https://github.com/gorilla-devs/furse
    pub fn hash_curse(bytes: &[u8]) -> u32 {
        let bytes: Vec<u8> = bytes
            .iter()
            .filter(|b| !matches!(b, 9 | 10 | 13 | 32))
            .copied()
            .collect();
        murmur2(&bytes, 1)
    }

    pub async fn get_fingerprint_matches(
        &self,
        fingerprints: &[u32],
    ) -> Result<FingerprintMatches> {
        #[derive(Serialize)]
        struct Body<'a> {
            fingerprints: &'a [u32],
        }

        self.post(
            &format!("{API_URL}/v1/fingerprints"),
            &Body { fingerprints },
        )
        .await
    }
}
