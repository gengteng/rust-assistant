//! The `app` module.
//!
//! This module contains the core application logic for the Rust Assistant.
//! It typically includes structures and functions responsible for initializing
//! and running the application, handling high-level operations, and coordinating
//! between other modules.
//!
use crate::cache::{Crate, CrateCache, CrateTar, FileContent};
use crate::download::CrateDownloader;
use crate::github::{GithubClient, Issue, IssueEvent, Repository};
use crate::{
    CrateVersion, CrateVersionPath, Directory, FileLineRange, Item, ItemQuery, Line, LineQuery,
};

/// The `RustAssistant` struct, providing functionalities to interact with crates and their contents.
///
/// This struct encapsulates methods for downloading crates, reading their content,
/// and performing searches within them.
#[derive(Clone)]
pub struct RustAssistant {
    downloader: CrateDownloader,
    cache: CrateCache,
    github: GithubClient,
}

impl From<(CrateDownloader, CrateCache, GithubClient)> for RustAssistant {
    /// Creates a new `RustAssistant` instance from a tuple of dependencies.
    fn from((downloader, cache, github): (CrateDownloader, CrateCache, GithubClient)) -> Self {
        Self {
            downloader,
            cache,
            github,
        }
    }
}

impl RustAssistant {
    /// Retrieves a crate from the cache or downloads it if not already cached.
    ///
    /// # Arguments
    /// * `crate_version` - A reference to `CrateVersion` specifying the crate to retrieve.
    ///
    /// # Returns
    /// A `Result` wrapping the `Crate`, or an error if the operation fails.
    pub async fn get_crate(&self, crate_version: &CrateVersion) -> anyhow::Result<Crate> {
        Ok(match self.cache.get_crate(crate_version) {
            None => {
                let data = self.downloader.download_crate_file(crate_version).await?;
                let crate_tar = CrateTar::from((crate_version.clone(), data));
                let krate =
                    tokio::task::spawn_blocking(move || Crate::try_from(crate_tar)).await??;
                self.cache.set_crate(crate_version.clone(), krate);
                self.cache
                    .get_crate(crate_version)
                    .ok_or_else(|| anyhow::anyhow!("Failed to get crate: {}", crate_version))?
            }
            Some(crate_tar) => crate_tar,
        })
    }

    /// Retrieves the content of a file within a specified crate and range.
    ///
    /// # Arguments
    /// * `crate_version_path` - A reference to `CrateVersionPath` specifying the crate and file path.
    /// * `file_line_range` - A `FileLineRange` specifying the range of lines to retrieve.
    ///
    /// # Returns
    /// A `Result` wrapping an `Option<CrateFileContent>`, or an error if the operation fails.
    pub async fn get_file_content(
        &self,
        crate_version_path: &CrateVersionPath,
        file_line_range: FileLineRange,
    ) -> anyhow::Result<Option<FileContent>> {
        let krate = self.get_crate(&crate_version_path.crate_version).await?;

        let path = crate_version_path.path.clone();
        tokio::task::spawn_blocking(move || {
            krate.get_file_by_file_line_range(path.as_ref(), file_line_range)
        })
        .await?
    }

    /// Reads the content of a directory within a specified crate.
    ///
    /// # Arguments
    /// * `crate_version_path` - A `CrateVersionPath` specifying the crate and directory path.
    ///
    /// # Returns
    /// A `Result` wrapping an `Option<Directory>`, or an error if the operation fails.
    pub async fn read_directory(
        &self,
        crate_version_path: CrateVersionPath,
    ) -> anyhow::Result<Option<Directory>> {
        let krate = self.get_crate(&crate_version_path.crate_version).await?;
        Ok(krate
            .read_directory(crate_version_path.path.as_ref())
            .cloned())
    }

    /// Searches for items in a crate based on a query.
    ///
    /// # Arguments
    /// * `crate_version` - A reference to `CrateVersion` specifying the crate to search in.
    /// * `query` - An `ItemQuery` specifying the search criteria.
    ///
    /// # Returns
    /// A `Result` wrapping a `Vec<Item>`, or an error if the operation fails.
    pub async fn search_item(
        &self,
        crate_version: &CrateVersion,
        query: impl Into<ItemQuery>,
    ) -> anyhow::Result<Vec<Item>> {
        let krate = self.get_crate(crate_version).await?;
        let query = query.into();
        Ok(tokio::task::spawn_blocking(move || krate.search_item(&query)).await?)
    }

    /// Searches for lines in a crate's files based on a query.
    ///
    /// # Arguments
    /// * `crate_version` - A reference to `CrateVersion` specifying the crate to search in.
    /// * `query` - A `LineQuery` specifying the search criteria.
    ///
    /// # Returns
    /// A `Result` wrapping a `Vec<Line>`, or an error if the operation fails.
    pub async fn search_line(
        &self,
        crate_version: &CrateVersion,
        query: impl Into<LineQuery>,
    ) -> anyhow::Result<Vec<Line>> {
        let krate = self.get_crate(crate_version).await?;
        let query = query.into();
        tokio::task::spawn_blocking(move || krate.search_line(&query)).await?
    }

    /// Reads the content of a file within a specified GitHub repository.
    ///
    /// # Arguments
    /// * `repo` - A reference to `Repository` specifying the GitHub repository.
    /// * `path` - A `&str` specifying the file path.
    /// * `branch` - An optional `&str` specifying the branch name.
    ///
    /// # Returns
    /// A `Result` wrapping a `FileContent`, or an error if the operation fails.
    ///
    pub async fn read_github_repository_file(
        &self,
        repo: &Repository,
        path: &str,
        branch: impl Into<Option<&str>>,
    ) -> anyhow::Result<Option<FileContent>> {
        self.github.get_file(repo, path, branch).await
    }

    /// Reads the content of a directory within a specified GitHub repository.
    ///
    /// # Arguments
    /// * `repo` - A reference to `Repository` specifying the GitHub repository.
    /// * `path` - A `&str` specifying the directory path.
    /// * `branch` - An optional `&str` specifying the branch name.
    ///
    /// # Returns
    /// A `Result` wrapping a `Directory`, or an error if the operation fails.
    ///
    pub async fn read_github_repository_directory(
        &self,
        repo: &Repository,
        path: &str,
        branch: impl Into<Option<&str>>,
    ) -> anyhow::Result<Option<Directory>> {
        self.github.read_dir(repo, path, branch).await
    }

    /// Searches for issues in a specified GitHub repository based on a query.
    ///
    /// # Arguments
    /// * `repo` - A reference to `Repository` specifying the GitHub repository.
    /// * `query` - A `&str` specifying the query string.
    ///
    /// # Returns
    /// A `Result` wrapping a `Vec<Issue>`, or an error if the operation fails.
    ///
    pub async fn search_github_repository_for_issues(
        &self,
        repo: &Repository,
        query: &str,
    ) -> anyhow::Result<Vec<Issue>> {
        self.github.search_for_issues(repo, query).await
    }

    /// Retrieves the timeline of an issue in a specified GitHub repository.
    ///
    /// # Arguments
    /// * `repo` - A reference to `Repository` specifying the GitHub repository.
    /// * `issue_number` - A `u64` specifying the issue number.
    ///
    /// # Returns
    /// A `Result` wrapping a `Vec<IssueEvent>`, or an error if the operation fails.
    ///
    pub async fn get_github_repository_issue_timeline(
        &self,
        repo: &Repository,
        issue_number: u64,
    ) -> anyhow::Result<Vec<IssueEvent>> {
        self.github.get_issue_timeline(repo, issue_number).await
    }

    /// Retrieves the branches of a specified GitHub repository.
    ///
    pub async fn get_github_repository_branches(
        &self,
        repo: &Repository,
    ) -> anyhow::Result<Vec<String>> {
        self.github.get_repo_branches(repo).await
    }
}
