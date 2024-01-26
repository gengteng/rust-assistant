//! The `app` module.
//!
//! This module contains the core application logic for the Rust Assistant.
//! It typically includes structures and functions responsible for initializing
//! and running the application, handling high-level operations, and coordinating
//! between other modules.
//!
use crate::cache::{Crate, CrateCache, CrateFileContent, CrateTar};
use crate::download::CrateDownloader;
use crate::{
    CrateVersion, CrateVersionPath, Directory, FileLineRange, Item, ItemQuery, Line, LineQuery,
};

/// The `RustAssistant` struct, providing functionalities to interact with crates and their contents.
///
/// This struct encapsulates methods for downloading crates, reading their content,
/// and performing searches within them.
#[derive(Clone, Default)]
pub struct RustAssistant {
    downloader: CrateDownloader,
    cache: CrateCache,
}

impl From<(CrateDownloader, CrateCache)> for RustAssistant {
    /// Constructs a `RustAssistant` from a `CrateDownloader` and `CrateCache`.
    ///
    fn from((downloader, cache): (CrateDownloader, CrateCache)) -> Self {
        Self { downloader, cache }
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
    ) -> anyhow::Result<Option<CrateFileContent>> {
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
}
