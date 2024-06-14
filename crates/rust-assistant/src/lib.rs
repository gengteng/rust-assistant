//! # Rust Assistant Library
//!
//! `rust_assistant` is a comprehensive library designed to enhance the Rust development experience,
//! offering a suite of tools and functionalities for downloading, caching, searching, and analyzing Rust crates.
//!
//! This library encapsulates a range of modules, each specializing in different aspects of crate management
//! and code analysis. It aims to streamline the process of working with Rust crates, providing developers
//! with efficient access to crate data, advanced search capabilities, and more.
//!
//! ## Features
//!
//! - **Crate Downloading**: Facilitates the downloading of crates from sources like crates.io,
//!   handling network requests and data processing.
//!
//! - **Crate Caching**: Implements caching mechanisms to store downloaded crates, optimizing
//!   performance and reducing redundant operations.
//!
//! - **Search Functionality**: Provides advanced search functionalities within crate contents,
//!   including source code, documentation, and other relevant data.
//!
//! ## Modules
//!
//! - `app`: Contains the core application logic for the Rust Assistant.
//! - `cache`: Provides caching functionalities for crates.
//! - `download`: Handles the downloading of crates and their contents.
//! - `search`: Implements search algorithms and data structures for efficient crate content search.
//!
pub mod app;

#[cfg(feature = "axum")]
pub mod axum;
pub mod cache;
pub mod download;
pub mod github;
pub mod search;

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::num::NonZeroUsize;
use std::ops::{Range, RangeInclusive};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

pub use app::*;
pub use github::*;
pub use search::*;

/// Represents the name and version of a crate.
///
/// This struct is used to uniquely identify a crate with its name and version number.
#[derive(Debug, Deserialize, Serialize, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CrateVersion {
    /// The exact name of the crate
    #[serde(rename = "crate")]
    pub krate: Arc<str>,
    /// The semantic version number of the specified crate, following the Semantic versioning specification.
    pub version: Arc<str>,
}

impl CrateVersion {
    /// Computes the root directory for the specified crate version.
    ///
    /// This method concatenates the crate name and version number to form a path-like string,
    /// which can be used as a directory name to store crate-related data.
    ///
    pub fn root_dir(&self) -> PathBuf {
        PathBuf::from(format!("{}-{}", self.krate, self.version))
    }
}

impl<C, V> From<(C, V)> for CrateVersion
where
    C: AsRef<str>,
    V: AsRef<str>,
{
    /// Creates a `CrateVersion` instance from a tuple of crate name and version.
    ///
    /// This method allows for a convenient way to construct a `CrateVersion` from separate
    /// name and version strings.
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

/// Represents a path within a specific crate's directory structure.
///
/// It combines the crate version information with the relative path within the crate.
#[derive(Debug, Deserialize, Serialize)]
pub struct CrateVersionPath {
    /// The name and version of a crate.
    #[serde(flatten)]
    pub crate_version: CrateVersion,
    /// The path.
    pub path: Arc<str>,
}

/// Represents a range of lines in a file.
///
/// This struct is used to specify a start and end line for operations that work with line ranges.
#[derive(Debug, Deserialize, Serialize, Copy, Clone)]
pub struct FileLineRange {
    /// The start line number.
    pub start: Option<NonZeroUsize>,
    /// The end line number.
    pub end: Option<NonZeroUsize>,
}

/// Represents the contents of a directory, including files and subdirectories.
///
/// This is used to provide a snapshot of a directory's contents, listing all files and directories.
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
    /// Checks whether the directory is empty.
    ///
    /// This method returns `true` if both the `files` and `directories` sets are empty,
    /// indicating that the directory has no contents.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.directories.is_empty()
    }
}

#[derive(Debug, Default)]
pub struct DirectoryMut {
    pub files: BTreeSet<PathBuf>,
    pub directories: BTreeSet<PathBuf>,
}

impl DirectoryMut {
    /// Checks whether the mutable directory is empty.
    ///
    /// Similar to `Directory::is_empty`, but for the mutable version of the directory.
    /// It's useful for scenarios where directory contents are being modified.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.directories.is_empty()
    }

    /// Freezes the directory, converting it into an immutable `Directory`.
    ///
    /// This method converts `DirectoryMut` into `Directory` by wrapping its contents
    /// in `Arc`, thus allowing for safe shared access.
    ///
    pub fn freeze(self) -> Directory {
        Directory {
            files: Arc::new(self.files),
            directories: Arc::new(self.directories),
        }
    }
}

/// Represents a query for searching items in a crate.
///
/// This struct is used to specify the criteria for searching items like structs, enums, traits, etc., within a crate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ItemQuery {
    /// The type of item to search for.
    #[serde(rename = "type")]
    pub type_: ItemType,
    /// The query string used for searching.
    pub query: String,
    /// Optional path within the crate to narrow down the search scope.
    pub path: Option<PathBuf>,
}

/// Represents an item found in a crate.
///
/// This struct describes an item, such as a struct or function, including its location within the crate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Item {
    /// The name of the item.
    pub name: String,
    /// The type of the item.
    #[serde(rename = "type")]
    pub type_: ItemType,
    /// The file path where the item is located.
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub file: Arc<Path>,
    /// The range of lines in the file where the item is defined.
    #[cfg_attr(feature = "utoipa", schema(value_type = RangeSchema))]
    pub line_range: RangeInclusive<NonZeroUsize>,
}

/// Defines various types of items that can be searched for in a crate.
///
/// This enum lists different types of code constructs like structs, enums, traits, etc.
#[derive(Debug, Default, Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ItemType {
    /// Represents all item types.
    #[default]
    All,
    /// A struct definition.
    Struct,
    /// An enum definition.
    Enum,
    /// A trait definition.
    Trait,
    /// Type implementation.
    ImplType,
    /// Trait implementation for a type.
    ImplTraitForType,
    /// A macro definition.
    Macro,
    /// An attribute macro.
    AttributeMacro,
    /// A standalone function.
    Function,
    /// A type alias.
    TypeAlias,
}

/// Represents a query for searching lines within files in a crate.
///
/// This struct is used for specifying criteria for line-based searches, such as finding specific text within files.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct LineQuery {
    /// The text or pattern to search for.
    pub query: String,
    /// The search mode (e.g., plain text or regular expression).
    pub mode: SearchMode,
    /// Indicates if the search should be case-sensitive.
    #[serde(default)]
    pub case_sensitive: bool,
    /// Indicates if the search should match whole words only.
    #[serde(default)]
    pub whole_word: bool,
    /// The maximum number of results to return.
    #[cfg_attr(feature = "utoipa", schema(value_type = usize))]
    pub max_results: Option<NonZeroUsize>,
    /// A comma-separated string specifying file extensions to include in the search.
    /// Each segment represents a file extension, e.g., "rs,txt" for Rust and text files.
    #[serde(default)]
    pub file_ext: String,
    /// Optional path within the crate to limit the search scope.
    #[cfg_attr(feature = "utoipa", schema(value_type = Option<String>))]
    pub path: Option<PathBuf>,
}

/// Defines different modes for searching text.
///
/// This enum distinguishes between plain text searches and regular expression searches.
#[derive(Debug, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(rename_all = "kebab-case")]
pub enum SearchMode {
    /// A plain text search.
    PlainText,
    /// A regular expression search.
    Regex,
}

/// Represents a specific line found in a search operation.
///
/// This struct contains details about a line of text found in a file, including its content and location.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Line {
    /// The content of the line.
    pub line: String,
    /// The file path where the line is located.
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub file: PathBuf,
    /// The line number within the file.
    #[cfg_attr(feature = "utoipa", schema(value_type = usize))]
    pub line_number: NonZeroUsize,
    /// The range of columns in the line where the text was found.
    #[cfg_attr(feature = "utoipa", schema(value_type = RangeSchema))]
    pub column_range: Range<NonZeroUsize>,
}

/// Schema for representing a range, used in other structs to describe line and column ranges.
#[cfg(feature = "utoipa")]
#[derive(ToSchema)]
pub struct RangeSchema {
    /// The start line number.
    pub start: usize,
    /// The end line number.
    pub end: usize,
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
            max_results: Some(6.try_into().expect("6")),
            file_ext: "rs".into(),
            path: Some(PathBuf::from("src")),
        })?;
        println!("{:#?}", lines);
        Ok(())
    }
}
