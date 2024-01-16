use crate::cache::{CrateCache, CrateTar};
use crate::download::CrateDownloader;
use crate::{CrateVersion, CrateVersionPath, Directory, FileLineRange};
use std::collections::BTreeSet;
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
    pub async fn get_crate_tar(&self, crate_version: &CrateVersion) -> anyhow::Result<CrateTar> {
        Ok(match self.cache.get(crate_version.clone()) {
            None => {
                let data = self.downloader.download_crate_file(crate_version).await?;
                self.cache.set_data(crate_version.clone(), data.clone());
                CrateTar::from((crate_version.clone(), data))
            }
            Some(crate_tar) => crate_tar,
        })
    }
    pub async fn get_file_content(
        &self,
        crate_version_path: &CrateVersionPath,
        FileLineRange { start, end }: FileLineRange,
    ) -> anyhow::Result<Option<String>> {
        let crate_tar = self
            .get_crate_tar(&crate_version_path.crate_version)
            .await?;

        let path = crate_version_path.path.clone();
        let file = tokio::task::spawn_blocking(move || {
            crate_tar.get_file_by_range(path.as_ref(), start, end)
        })
        .await??;
        Ok(file)
    }

    pub async fn get_file_list(
        &self,
        crate_version: &CrateVersion,
    ) -> anyhow::Result<Option<BTreeSet<PathBuf>>> {
        let crate_tar = self.get_crate_tar(crate_version).await?;
        Ok(tokio::task::spawn_blocking(move || crate_tar.get_all_file_list(..)).await??)
    }

    pub async fn read_directory(
        &self,
        crate_version_path: CrateVersionPath,
    ) -> anyhow::Result<Option<Directory>> {
        let crate_tar = self
            .get_crate_tar(&crate_version_path.crate_version)
            .await?;
        Ok(tokio::task::spawn_blocking(move || {
            crate_tar.read_directory(crate_version_path.path.as_ref())
        })
        .await??)
    }
}
