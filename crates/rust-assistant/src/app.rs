use crate::cache::{Crate, CrateCache, CrateFileContent, CrateTar};
use crate::download::CrateDownloader;
use crate::search::{Item, ItemType};
use crate::{CrateVersion, CrateVersionPath, Directory, FileLineRange};
use std::path::PathBuf;

#[derive(Clone, Default)]
pub struct RustAssistant {
    downloader: CrateDownloader,
    cache: CrateCache,
}

impl From<(CrateDownloader, CrateCache)> for RustAssistant {
    fn from((downloader, cache): (CrateDownloader, CrateCache)) -> Self {
        Self { downloader, cache }
    }
}

impl RustAssistant {
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

    pub async fn read_directory(
        &self,
        crate_version_path: CrateVersionPath,
    ) -> anyhow::Result<Option<Directory>> {
        let krate = self.get_crate(&crate_version_path.crate_version).await?;
        Ok(krate
            .read_directory(crate_version_path.path.as_ref())
            .cloned())
    }

    pub async fn search(
        &self,
        crate_version: &CrateVersion,
        type_: ItemType,
        query: &str,
        path: Option<PathBuf>,
    ) -> anyhow::Result<Vec<Item>> {
        let krate = self.get_crate(crate_version).await?;
        Ok(krate.search(type_, query, path))
    }
}
