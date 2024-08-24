use reqwest::{
    header::{HeaderMap, HeaderValue}, Client, RequestBuilder, Response
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::task::JoinSet;

use crate::error::{Error, Result};

const API_URL: &str = "https://api.github.com";
const API_VERSION: &str = "2022-11-28";

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Repository {
    pub full_name: String,
    pub description: String
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GithubRelease {
    pub name: String,
    pub tag_name: String,
    pub prerelease: bool,
    pub assets: Vec<ReleaseAsset>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReleaseAsset {
    pub browser_download_url: String,
    pub name: String,
    pub size: usize
}

pub struct GithubApi {
    client: Client,
}
impl GithubApi {
    pub fn default() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("User-Agent", HeaderValue::from_str(&format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))).unwrap());
        headers.insert("Accept", HeaderValue::from_str("application/vnd.github+json").unwrap());
        headers.insert("X-GitHub-Api-Version", HeaderValue::from_str(API_VERSION).unwrap());

        GithubApi {
            client: Client::builder().default_headers(headers).build().unwrap(),
        }
    }

    async fn fetch<T: DeserializeOwned>(&self, f: impl FnOnce() -> RequestBuilder) -> Result<T> {
        let res = f().send().await?;
        check_ratelimit(&res)?;

        Ok(res.error_for_status()?.json().await?)
    }

    async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        self.fetch(|| self.client.get(url)).await
    }

    pub async fn get_repo(&self, owner: &str, repo: &str) -> Result<Repository> {
        self.get(&format!("{API_URL}/repos/{owner}/{repo}")).await
    }

    pub async fn get_repos(&'static self, repos: Vec<(String, String)>) -> Result<Vec<Repository>> {
        let mut tasks: JoinSet<Result<Repository>> = JoinSet::new();
        for repo in repos {
            let task = async move {
                self.get(&format!("{API_URL}/repos/{}/{}", repo.0, repo.1)).await
            };
            tasks.spawn(task);
        }

        let mut return_repos = Vec::new();
        while let Some(res) = tasks.join_next().await { return_repos.push(res??) }

        Ok(return_repos)
    }

    pub async fn list_releases(&self, owner: &str, repo: &str) -> Result<Vec<GithubRelease>> {
        self.get(&format!("{API_URL}/repos/{owner}/{repo}/releases")).await
    }

    pub async fn get_release_by_tag(&self, owner: &str, repo: &str, tag: &str) -> Result<GithubRelease> {
        self.get(&format!("{API_URL}/repos/{owner}/{repo}/releases/tags/{tag}")).await
    }
}

fn check_ratelimit(res: &Response) -> Result<()> {
    let ratelimit_remaining = match res.headers().get("x-ratelimit-remaining") {
        Some(val) => val.to_str().unwrap(),
        None => return Ok(()),
    };
    
    if ratelimit_remaining == "0" {
        Err(Error::RateLimitExceeded(
            "github".to_owned(),
            res.headers()
                .get("x-ratelimit-reset")
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
        ))
    } else {
        Ok(())
    }
}
