pub mod app;

#[cfg(feature = "axum")]
pub mod axum;
pub mod cache;
pub mod download;

use fnv::FnvHashSet;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::num::NonZeroUsize;
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

impl Display for CrateVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.krate, self.version)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CrateVersionPath {
    #[serde(flatten)]
    pub crate_version: CrateVersion,
    pub path: Arc<str>,
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone)]
pub struct FileLineRange {
    pub start: Option<NonZeroUsize>,
    pub end: Option<NonZeroUsize>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct Directory {
    pub files: FnvHashSet<PathBuf>,
    pub directories: FnvHashSet<PathBuf>,
}

impl Directory {
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.directories.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::{Crate, CrateCache, CrateTar};
    use crate::download::CrateDownloader;
    use std::num::NonZeroUsize;

    #[tokio::test]
    async fn download_and_read() -> anyhow::Result<()> {
        // let start = Instant::now();
        let crate_version = CrateVersion::from(("tokio", "1.35.1"));
        let downloader = CrateDownloader::default();
        let tar_data = downloader.download_crate_file(&crate_version).await?;
        let cache = CrateCache::new(NonZeroUsize::new(1024).unwrap());
        let crate_tar = CrateTar::from((crate_version.clone(), tar_data));
        let krate = Crate::try_from(crate_tar)?;
        let old = cache.set_crate(crate_version.clone(), krate);
        assert!(old.is_none());

        let crate_ = cache.get_crate(&crate_version).expect("get crate");

        let files = crate_.read_directory("").expect("read directory");
        assert!(!files.is_empty());
        println!("{:#?}", files);

        let lib_rs_content = crate_.get_file_by_line_range("src/lib.rs", ..)?;
        assert!(lib_rs_content.is_some());
        let lib_rs_range_content =
            crate_.get_file_by_line_range("src/lib.rs", ..NonZeroUsize::new(27).unwrap())?;
        assert!(lib_rs_range_content.is_some());
        // println!("{}", lib_rs_range_content.expect("lib.rs"));
        // println!("Elapsed: {}µs", start.elapsed().as_micros());

        let file = crate_
            .get_file_by_line_range("src/lib.rs", ..=NonZeroUsize::new(3).unwrap())?
            .unwrap();
        println!("[{}]", std::str::from_utf8(file.data.as_ref()).unwrap());
        Ok(())
    }
}
