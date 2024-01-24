pub mod app;

#[cfg(feature = "axum")]
pub mod axum;
pub mod cache;
pub mod download;
pub mod search;

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::num::NonZeroUsize;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

pub use app::*;
pub use search::*;

/// The name and version of the crate.
#[derive(Debug, Deserialize, Serialize, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CrateVersion {
    /// The exact name of the crate
    #[serde(rename = "crate")]
    pub krate: Arc<str>,
    /// The semantic version number of the specified crate, following the Semantic versioning specification.
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

/// The path of a specific file or directory within a crate's directory structure.
#[derive(Debug, Deserialize, Serialize)]
pub struct CrateVersionPath {
    #[serde(flatten)]
    pub crate_version: CrateVersion,
    /// The path.
    pub path: Arc<str>,
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone)]
pub struct FileLineRange {
    pub start: Option<NonZeroUsize>,
    pub end: Option<NonZeroUsize>,
}

/// Represents the contents of a directory, including files and subdirectories.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Directory {
    /// Files in the directory.
    #[cfg_attr(feature = "utoipa", schema(value_type = BTreeSet<String>))]
    pub files: Arc<BTreeSet<PathBuf>>,
    /// Subdirectories in the directory.
    #[cfg_attr(feature = "utoipa", schema(value_type = BTreeSet<String>))]
    pub directories: Arc<BTreeSet<PathBuf>>,
}

impl Directory {
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.directories.is_empty()
    }
}

#[derive(Debug, Default)]
pub struct DirectoryMut {
    files: BTreeSet<PathBuf>,
    directories: BTreeSet<PathBuf>,
}

impl DirectoryMut {
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.directories.is_empty()
    }

    pub fn freeze(self) -> Directory {
        Directory {
            files: Arc::new(self.files),
            directories: Arc::new(self.directories),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ItemQuery {
    #[serde(rename = "type")]
    pub type_: ItemType,
    pub query: String,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct LineQuery {
    pub query: String,
    pub mode: SearchMode,
    #[serde(default)]
    pub case_sensitive: bool,
    pub whole_word: bool,
    #[cfg_attr(feature = "utoipa", schema(value_type = usize))]
    pub max_results: NonZeroUsize,
    pub file_ext: Vec<String>,
    #[cfg_attr(feature = "utoipa", schema(value_type = Option<String>))]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(rename = "kebab-case")]
pub enum SearchMode {
    PlainText,
    Regex,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Line {
    pub line: String,
    pub file: PathBuf,
    pub line_number: NonZeroUsize,
    pub column_range: Range<NonZeroUsize>,
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

        let lines = crate_.search_line(&LineQuery {
            query: "Sleep".to_string(),
            mode: SearchMode::PlainText,
            case_sensitive: true,
            whole_word: true,
            max_results: 6.try_into().expect("6"),
            file_ext: vec!["rs".into()],
            path: Some(PathBuf::from("src")),
        })?;
        println!("{:#?}", lines);
        Ok(())
    }
}
