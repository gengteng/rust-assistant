use crate::cache::FileContent;
use crate::{Directory, DirectoryMut};
use reqwest::header::HeaderMap;
use reqwest::{Client, Proxy, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

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

/// A struct representing a GitHub repository and an issue number.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryIssue {
    /// The repository.
    #[serde(flatten)]
    pub repo: Repository,
    /// The path.
    pub number: u64,
}

/// The Query string for searching issues.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueQuery {
    /// The query string.
    pub query: String,
}

impl AsRef<str> for IssueQuery {
    fn as_ref(&self) -> &str {
        self.query.as_str()
    }
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

    /// Search for issues.
    ///
    /// # Arguments
    ///
    /// * `query` - The query string to search for.
    ///
    /// # Returns
    ///
    /// A vector of issues matching the query.
    ///
    pub async fn search_for_issues(
        &self,
        Repository { owner, repo }: &Repository,
        keyword: &str,
    ) -> anyhow::Result<Vec<Issue>> {
        let url = format!("https://api.github.com/search/issues?q={keyword}+repo:{owner}/{repo}",);
        let resp = self.client.get(url).send().await?;
        let status = resp.status();
        if status != StatusCode::OK {
            anyhow::bail!(
                "The server returned a non-200 status code when fetching the file download URL ({status}): {}",
                resp.text().await?
            );
        }

        let body = resp.json::<SearchIssuesResponse>().await?;
        Ok(body.items)
    }

    pub async fn get_issue_timeline(
        &self,
        Repository { owner, repo }: &Repository,
        issue_number: u64,
    ) -> anyhow::Result<Vec<IssueEvent>> {
        let url = format!(
            "https://api.github.com/repos/{owner}/{repo}/issues/{issue_number}/timeline",
            owner = owner,
            repo = repo,
            issue_number = issue_number
        );
        let resp = self.client.get(url).send().await?;
        let status = resp.status();
        if status != StatusCode::OK {
            anyhow::bail!(
                "The server returned a non-200 status code when fetching the file download URL ({status}): {}",
                resp.text().await?
            );
        }

        let body = resp.json::<Vec<IssueEvent>>().await?;
        Ok(body)
    }
}

#[derive(Deserialize, Serialize, Debug)]
struct Item {
    r#type: String,
    name: String,
}

/// A struct representing a GitHub issue.
#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub state: String,
    pub body: Option<String>,
}

/// A struct representing a response from a GitHub issue search.
#[derive(Deserialize, Debug)]
pub struct SearchIssuesResponse {
    pub items: Vec<Issue>,
}

/// A struct representing a GitHub issue event.
/// https://docs.github.com/en/rest/reference/issues#list-issue-events
/// https://docs.github.com/en/rest/reference/issues#events
///
#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct IssueEvent {
    /// The event type.
    pub event: String,
    /// The actor of the event.
    pub actor: Option<Actor>,
    /// The author of the event.
    pub author: Option<Author>,
    /// The time the event was created.
    pub created_at: Option<String>,
    /// The body of the event.
    pub body: Option<String>,
}

/// A struct representing a GitHub actor.
///
#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Actor {
    pub login: String,
    pub avatar_url: String,
}

/// A struct representing a GitHub author.
#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Author {
    /// The author's email.
    pub email: String,
    /// The author's name.
    pub name: String,
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
//         let repo = Repository::from(("gengteng", "axum-valid"));
//         // https://github.com/rust-lang/crates.io-index
//         let client = GithubClient::new(token.as_str(), proxy)?;
//         let content = client.get_file(&repo, "Cargo.toml").await?;
//         println!("content: {content:?}");
//
//         let dir = client.read_dir(&repo, "lib.rs").await?;
//         println!("dir crates: {dir:#?}");
//
//         let issues = client.search_for_issues(&repo, "test").await?;
//         println!("issues: {issues:#?}");
//
//         for issue in issues {
//             let timeline = client.get_issue_timeline(&repo, issue.number).await?;
//             println!("timeline: {timeline:#?}");
//         }
//         Ok(())
//     }
// }
