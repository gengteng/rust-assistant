use crate::cache::FileContent;
use crate::{Directory, DirectoryMut};
use reqwest::header::HeaderMap;
use reqwest::{Client, Proxy, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct GithubClient {
    client: Client,
}

/// A struct representing a GitHub repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    /// The owner of the repository.
    pub owner: Arc<str>,
    /// The name of the repository.
    pub repo: Arc<str>,
}

/// A struct representing a GitHub repository and a path within it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryPath {
    /// The repository.
    #[serde(flatten)]
    pub repo: Repository,
    /// The path.
    pub path: Arc<str>,
}

impl<O, R> From<(O, R)> for Repository
where
    O: AsRef<str>,
    R: AsRef<str>,
{
    fn from((owner, repo): (O, R)) -> Self {
        Self {
            owner: Arc::from(owner.as_ref()),
            repo: Arc::from(repo.as_ref()),
        }
    }
}

impl GithubClient {
    pub fn new(token: &str, proxy: impl Into<Option<Proxy>>) -> anyhow::Result<Self> {
        let authorization = format!("token {token}");
        let mut headers = HeaderMap::new();
        headers.insert(reqwest::header::AUTHORIZATION, authorization.parse()?);
        headers.insert(reqwest::header::USER_AGENT, "Rust Assistant".parse()?);

        let mut builder = reqwest::ClientBuilder::default().default_headers(headers);
        if let Some(proxy) = proxy.into() {
            builder = builder.proxy(proxy);
        }

        Ok(Self {
            client: builder.build()?,
        })
    }

    pub fn build_file_url(&self, repo: &Repository, path: &str) -> String {
        format!(
            "https://api.github.com/repos/{}/{}/contents/{path}",
            repo.owner, repo.repo
        )
    }

    pub async fn get_file(
        &self,
        repo: &Repository,
        path: &str,
    ) -> anyhow::Result<Option<FileContent>> {
        let file_path = self.build_file_url(repo, path);

        let resp = self.client.get(file_path).send().await?;
        let status = resp.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if status != StatusCode::OK {
            anyhow::bail!(
                "The server returned a non-200 status code when fetching the file download URL ({status}): {}",
                resp.text().await?
            );
        }

        let body = resp.json::<serde_json::Value>().await?;
        if body.is_array() || body.get("type") != Some(&json!("file")) {
            anyhow::bail!("The path is not a regular file.");
        }
        let Some(download_url) = body.get("download_url").map(|u| u.as_str()).flatten() else {
            anyhow::bail!("Failed to get download url from response body: {body}");
        };

        let resp = self.client.get(download_url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!(
                "The server returned a non-200 status code when fetching file content ({status}): {}",
                resp.text().await?
            );
        }
        let bytes = resp.bytes().await?;
        Ok(Some(crate::cache::FileContent::from(bytes)))
    }

    pub async fn read_dir(
        &self,
        repo: &Repository,
        path: &str,
    ) -> anyhow::Result<Option<Directory>> {
        let file_path = self.build_file_url(repo, path);
        let resp = self.client.get(file_path).send().await?;
        let status = resp.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if status != StatusCode::OK {
            anyhow::bail!(
                "The server returned a non-200 status code when fetching the file download URL ({status}): {}",
                resp.text().await?
            );
        }

        let items = resp.json::<Vec<Item>>().await?;
        let mut directories = DirectoryMut::default();
        for item in items {
            match item.r#type.as_str() {
                "file" => {
                    directories.files.insert(PathBuf::from(item.name));
                }
                "dir" => {
                    directories.directories.insert(PathBuf::from(item.name));
                }
                _ => {
                    continue;
                }
            }
        }
        Ok(Some(directories.freeze()))
    }
}

#[derive(Deserialize, Serialize, Debug)]
struct Item {
    r#type: String,
    name: String,
}
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[tokio::test]
//     async fn test_get_file() -> anyhow::Result<()> {
//         let token = dotenv::var("GITHUB_ACCESS_TOKEN")?;
//         let proxy = if tokio::net::TcpStream::connect("127.0.0.1:7890")
//             .await
//             .is_ok()
//         {
//             Some(Proxy::all("http://127.0.0.1:7890")?)
//         } else {
//             None
//         };
//         let repo = Repository::from(("rust-lang", "crates.io-index"));
//         // https://github.com/rust-lang/crates.io-index
//         let client = GithubClient::new(token.as_str(), proxy)?;
//         let content = client.get_file(&repo, "config.json").await?;
//         println!("content: {content}");
//
//         println!("{:?}", client.file_type(&repo, "a-").await?);
//
//         let dir = client.read_dir(&repo, "a-").await?;
//         println!("{:?}", dir);
//         Ok(())
//     }
// }
