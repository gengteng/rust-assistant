pub mod app;

#[cfg(feature = "axum")]
pub mod axum;
pub mod cache;
pub mod download;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CrateVersion {
    #[serde(rename = "crate")]
    pub krate: Arc<str>,
    pub version: Arc<str>,
}

impl CrateVersion {
    pub fn root_dir(&self) -> PathBuf {
        PathBuf::from(format!("{}-{}", self.krate, self.version))
    }
}

impl<C, V> From<(C, V)> for CrateVersion
where
    C: AsRef<str>,
    V: AsRef<str>,
{
    fn from(value: (C, V)) -> Self {
        Self {
            krate: Arc::from(value.0.as_ref()),
            version: Arc::from(value.1.as_ref()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CrateVersionPath {
    #[serde(flatten)]
    pub crate_version: CrateVersion,
    pub path: Arc<str>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileLineRange {
    pub start: Option<u64>,
    pub end: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::CrateCache;
    use crate::download::CrateDownloader;
    use std::io::Read;
    use std::num::{NonZeroU64, NonZeroUsize};
    use std::time::Instant;

    #[tokio::test]
    async fn download_and_read() -> anyhow::Result<()> {
        let start = Instant::now();
        let crate_version = CrateVersion::from(("tokio", "1.35.1"));
        let downloader = CrateDownloader::default();
        let data = downloader.download_crate_file(&crate_version).await?;
        let cache = CrateCache::new(NonZeroUsize::new(1024).unwrap());
        let mut dc = flate2::bufread::GzDecoder::new(data.as_ref());
        let mut tar_data = Vec::new();
        dc.read_to_end(&mut tar_data).expect("decompress gzip data");

        let old = cache.set_raw(crate_version.clone(), tar_data);
        assert!(old.is_none());
        let files = cache.list_files(&crate_version)?;
        assert!(files.is_some());

        let lib_rs_content = cache.get_file(&crate_version, "src/lib.rs")?;
        assert!(lib_rs_content.is_some());
        let lib_rs_range_content = cache.get_file_by_range(
            &crate_version,
            "src/lib.rs",
            None,
            NonZeroU64::new(27).unwrap(),
        )?;
        println!("{}", lib_rs_range_content.expect("lib.rs"));
        println!("Elapsed: {}µs", start.elapsed().as_micros());
        Ok(())
    }
}
