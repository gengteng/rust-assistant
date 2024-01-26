//! The `cache` module.
//!
//! This module provides caching functionalities to optimize performance and reduce
//! redundant operations, particularly in the context of downloading and storing crate data.
//! It may include structures like `CrateCache` to store downloaded crates and their metadata
//! for quick retrieval.
//!
use crate::search::{SearchIndex, SearchIndexBuilder};
use crate::{
    CrateVersion, Directory, DirectoryMut, FileLineRange, Item, ItemQuery, Line, LineQuery,
    SearchMode,
};
use bytes::{Bytes, BytesMut};
use fnv::FnvHashMap;
use lru::LruCache;
use parking_lot::Mutex;
use regex::RegexBuilder;
use std::collections::BTreeSet;
use std::io::{BufRead, Cursor, Read};
use std::num::NonZeroUsize;
use std::ops::{Bound, Range, RangeBounds};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tar::EntryType;

/// Represents a tarball of a crate, including version information and tar data.
#[derive(Clone)]
pub struct CrateTar {
    pub crate_version: CrateVersion,
    pub tar_data: Vec<u8>,
}

impl<C, D> From<(C, D)> for CrateTar
where
    C: Into<CrateVersion>,
    D: Into<Vec<u8>>,
{
    fn from((c, d): (C, D)) -> Self {
        CrateTar {
            crate_version: c.into(),
            tar_data: d.into(),
        }
    }
}

impl CrateTar {
    /// Retrieves the content of a specified file within the crate tarball.
    ///
    pub fn get_file(&self, file: &str) -> anyhow::Result<Option<String>> {
        let mut archive = tar::Archive::new(self.tar_data.as_slice());
        let entries = archive.entries()?;
        for entry in entries {
            let Ok(mut entry) = entry else {
                continue;
            };

            let Ok(path) = entry.path() else {
                continue;
            };

            if self.crate_version.root_dir().join(file).eq(path.as_ref()) {
                let mut content = String::with_capacity(entry.size() as usize);
                entry.read_to_string(&mut content)?;
                return Ok(Some(content));
            }
        }

        Ok(None)
    }

    /// Retrieves the content of a specified file within a range.
    ///
    pub fn get_file_by_range(
        &self,
        file: &str,
        start: impl Into<Option<NonZeroUsize>>,
        end: impl Into<Option<NonZeroUsize>>,
    ) -> anyhow::Result<Option<String>> {
        let mut archive = tar::Archive::new(self.tar_data.as_slice());
        let entries = archive.entries()?;
        for entry in entries {
            let Ok(mut entry) = entry else {
                continue;
            };

            let Ok(path) = entry.path() else {
                continue;
            };

            if self.crate_version.root_dir().join(file).eq(path.as_ref()) {
                let mut content = String::with_capacity(entry.size() as usize);
                entry.read_to_string(&mut content)?;
                let lines: Vec<&str> = content.lines().collect();

                let start = start.into();
                let end = end.into();

                let start_line = start.map_or(0, |n| n.get() - 1);
                let end_line = end.map_or(lines.len(), |n| n.get());

                if start_line > lines.len() {
                    return Ok(Some(String::new()));
                }

                return Ok(Some(
                    lines[start_line..end_line.min(lines.len())].join("\n"),
                ));
            }
        }

        Ok(None)
    }

    /// Lists all files in the crate within a specified range.
    ///
    pub fn get_all_file_list(
        &self,
        range: impl RangeBounds<usize>,
    ) -> anyhow::Result<Option<BTreeSet<PathBuf>>> {
        let mut archive = tar::Archive::new(self.tar_data.as_slice());
        let root_dir = self.crate_version.root_dir();
        let entries = archive.entries()?;
        let mut list = BTreeSet::default();
        for (i, entry) in entries.enumerate() {
            if !range.contains(&i) {
                continue;
            }
            let Ok(entry) = entry else {
                continue;
            };

            let Ok(path) = entry.path() else {
                continue;
            };

            let Ok(path) = path.strip_prefix(&root_dir) else {
                continue;
            };
            list.insert(path.to_path_buf());
        }
        Ok(Some(list))
    }

    /// Reads the contents of a directory within the crate.
    ///
    pub fn read_directory<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<Option<Directory>> {
        let mut archive = tar::Archive::new(self.tar_data.as_slice());
        let base_dir = self.crate_version.root_dir().join(path);
        let entries = archive.entries()?;
        let mut dir = DirectoryMut::default();
        for entry in entries {
            let Ok(entry) = entry else {
                continue;
            };

            let Ok(path) = entry.path() else {
                continue;
            };

            let Ok(path) = path.strip_prefix(&base_dir) else {
                continue;
            };

            let mut components = path.components();
            if let Some(path) = components
                .next()
                .map(|comp| PathBuf::from(comp.as_os_str()))
            {
                if components.next().is_none() {
                    dir.files.insert(path.to_path_buf());
                } else {
                    dir.directories.insert(path.to_path_buf());
                }
            }
        }

        Ok(Some(dir.freeze()))
    }
}

/// Enumerates the possible data formats of a crate file.
///
/// This enum helps in distinguishing between different text encoding formats of the files contained in a crate.
#[derive(Debug, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum CrateFileDataType {
    /// Represents a UTF-8 formatted file.
    Utf8,
    /// Represents a non-UTF-8 formatted file.
    #[default]
    NonUtf8,
}

/// Describes a crate file with its data type and range in the crate's data buffer.
///
/// This struct is used to quickly access the file's content and its encoding format.
///
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct CrateFileDataDesc {
    /// The data type of the file (UTF-8 or Non-UTF-8).
    pub data_type: CrateFileDataType,
    /// The byte range of the file content within the crate's data buffer.
    pub range: Range<usize>,
}

/// Contains the actual content of a file within a crate.
///
/// This struct holds the file data and its data type, which is useful for encoding-specific operations.
#[derive(Debug, Clone)]
pub struct CrateFileContent {
    /// The data type of the file.
    pub data_type: CrateFileDataType,
    /// The byte content of the file.
    pub data: Bytes,
}

/// Represents a crate with its data and indexes for quick access to its contents.
///
/// This struct stores the complete data of a crate and provides indexes for accessing individual files,
/// directories, and search functionalities within the crate.
///
#[derive(Debug, Clone)]
pub struct Crate {
    data: Bytes,
    files_index: Arc<FnvHashMap<PathBuf, CrateFileDataDesc>>,
    directories_index: Arc<FnvHashMap<PathBuf, Directory>>,
    item_search_index: SearchIndex,
}

impl Crate {
    /// Retrieves the content of a file by specifying a line range.
    ///
    pub fn get_file_by_file_line_range<P: AsRef<Path>>(
        &self,
        file: P,
        FileLineRange { start, end }: FileLineRange,
    ) -> anyhow::Result<Option<CrateFileContent>> {
        match (start, end) {
            (Some(start), Some(end)) => self.get_file_by_line_range(file, start..=end),
            (Some(start), None) => self.get_file_by_line_range(file, start..),
            (None, Some(end)) => self.get_file_by_line_range(file, ..=end),
            (None, None) => self.get_file_by_line_range(file, ..),
        }
    }

    /// Retrieves the content of a file by specifying a line range.
    ///
    /// This method is used to extract a specific range of lines from a file in the crate.
    ///
    pub fn get_file_by_line_range<P: AsRef<Path>>(
        &self,
        file: P,
        line_range: impl RangeBounds<NonZeroUsize>,
    ) -> anyhow::Result<Option<CrateFileContent>> {
        let file = file.as_ref();
        let Some(CrateFileDataDesc { range, data_type }) = self.files_index.get(file) else {
            return Ok(None);
        };

        let data = self.data.slice(range.clone());

        if matches!(
            (line_range.start_bound(), line_range.end_bound()),
            (Bound::Unbounded, Bound::Unbounded)
        ) {
            return Ok(Some(CrateFileContent {
                data,
                data_type: *data_type,
            }));
        }

        if let CrateFileDataType::NonUtf8 = data_type {
            anyhow::bail!("Non-UTF8 formatted files do not support line-range querying.");
        }

        let s = std::str::from_utf8(data.as_ref())?;
        let start_line = match line_range.start_bound() {
            Bound::Included(n) => n.get() - 1,
            Bound::Excluded(n) => n.get(),
            Bound::Unbounded => 0,
        };
        let end_line = match line_range.end_bound() {
            Bound::Included(n) => n.get(),
            Bound::Excluded(n) => n.get() - 1,
            Bound::Unbounded => usize::MAX,
        };

        let mut line_start = 0;
        let mut line_end = s.len();
        let mut current_line = 0;

        // 定位起始行的开始
        for _ in 0..start_line {
            if let Some(pos) = s[line_start..].find('\n') {
                line_start += pos + 1;
                current_line += 1;
            } else {
                // 找不到更多的行
                break;
            }
        }

        // 定位结束行的结束
        if current_line < end_line {
            line_end = line_start;
            for _ in current_line..end_line {
                if let Some(pos) = s[line_end..].find('\n') {
                    line_end += pos + 1;
                } else {
                    break;
                }
            }
        }

        if line_start < line_end {
            let line_bytes_range = range.start + line_start..range.start + line_end;
            return Ok(Some(CrateFileContent {
                data_type: CrateFileDataType::Utf8,
                data: self.data.slice(line_bytes_range),
            }));
        }

        Ok(None)
    }

    /// Reads the content of a specified directory within the crate.
    ///
    pub fn read_directory<P: AsRef<Path>>(&self, path: P) -> Option<&Directory> {
        self.directories_index.get(path.as_ref())
    }

    /// Searches for items in the crate based on a given query.
    ///
    pub fn search_item(&self, query: &ItemQuery) -> Vec<Item> {
        self.item_search_index.search(query)
    }

    /// Searches for lines in the crate's files based on a given query.
    ///
    pub fn search_line(&self, query: &LineQuery) -> anyhow::Result<Vec<Line>> {
        let mut results = Vec::new();
        let file_ext = query
            .file_ext
            .split(",")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        let mut regex_pattern = match query.mode {
            SearchMode::PlainText => regex::escape(&query.query),
            SearchMode::Regex => query.query.clone(),
        };

        // 如果需要全字匹配，则对模式进行相应包装
        if query.whole_word {
            regex_pattern = format!(r"\b{}\b", regex_pattern);
        }

        // 创建正则表达式，考虑大小写敏感设置
        let pattern = RegexBuilder::new(&regex_pattern)
            .case_insensitive(!query.case_sensitive)
            .build()?;

        for (path, file_desc) in self.files_index.iter() {
            if let Some(query_path) = &query.path {
                if !path.starts_with(query_path) {
                    continue;
                }
            };
            if !file_ext.is_empty() {
                if let Some(extension) = path.extension() {
                    if !file_ext
                        .iter()
                        .any(|ext| extension.eq_ignore_ascii_case(ext))
                    {
                        continue;
                    }
                } else {
                    // 如果路径没有扩展名，则跳过
                    continue;
                }
            }

            let content_range = file_desc.range.clone();
            let content = &self.data.slice(content_range);

            let cursor = Cursor::new(content);

            for (line_number, line) in cursor.lines().enumerate() {
                let line = line?;
                let Some(line_number) = NonZeroUsize::new(line_number + 1) else {
                    continue;
                };

                // 使用 pattern 对每一行进行匹配
                if let Some(mat) = pattern.find(&line) {
                    let column_range = NonZeroUsize::new(mat.start() + 1).unwrap()
                        ..NonZeroUsize::new(mat.end() + 1).unwrap();

                    let line_result = Line {
                        line,
                        file: path.clone(),
                        line_number,
                        column_range,
                    };
                    results.push(line_result);

                    if let Some(max_results) = query.max_results {
                        if results.len() >= max_results.get() {
                            break;
                        }
                    }
                }
            }

            if let Some(max_results) = query.max_results {
                if results.len() >= max_results.get() {
                    break;
                }
            }
        }

        Ok(results)
    }
}

impl TryFrom<CrateTar> for Crate {
    type Error = std::io::Error;
    fn try_from(crate_tar: CrateTar) -> std::io::Result<Self> {
        let mut archive = tar::Archive::new(crate_tar.tar_data.as_slice());
        let root_dir = crate_tar.crate_version.root_dir();

        let mut data = BytesMut::new();
        let mut files_index = FnvHashMap::default();
        let mut directories_index = FnvHashMap::default();
        let mut search_index_builder = SearchIndexBuilder::default();

        let mut buffer = Vec::new();
        let entries = archive.entries()?;
        for entry in entries {
            let Ok(mut entry) = entry else {
                continue;
            };

            let Ok(path) = entry.path() else {
                continue;
            };

            let Ok(path) = path.strip_prefix(&root_dir) else {
                continue;
            };

            let Some(last) = path.components().last() else {
                continue;
            };

            let filename = PathBuf::from(last.as_os_str());
            let is_rust_src =
                matches!(filename.extension(), Some(ext) if ext.eq_ignore_ascii_case("rs"));

            let path = path.to_path_buf();
            if let EntryType::Regular = entry.header().entry_type() {
                buffer.clear();
                entry.read_to_end(&mut buffer)?;

                let data_type = match std::str::from_utf8(&buffer) {
                    Ok(utf8_src) => {
                        if is_rust_src {
                            search_index_builder.update(path.as_path(), utf8_src);
                        }
                        CrateFileDataType::Utf8
                    }
                    Err(_) => CrateFileDataType::NonUtf8,
                };

                let range = data.len()..data.len() + buffer.len();

                data.extend_from_slice(buffer.as_slice());
                files_index.insert(path.clone(), CrateFileDataDesc { data_type, range });
                let parent = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
                directories_index
                    .entry(parent)
                    .and_modify(|o: &mut DirectoryMut| {
                        o.files.insert(filename.clone());
                    })
                    .or_insert({
                        let mut set = BTreeSet::default();
                        set.insert(filename);
                        DirectoryMut {
                            files: set,
                            directories: Default::default(),
                        }
                    });
            }
        }

        let mut subdirectories_index = FnvHashMap::default();
        for key in directories_index.keys() {
            let Some(last) = key.components().last() else {
                continue;
            };

            let sub_dir_name = PathBuf::from(last.as_os_str());
            let parent = key.parent().map(|p| p.to_path_buf()).unwrap_or_default();
            subdirectories_index
                .entry(parent)
                .and_modify(|s: &mut BTreeSet<PathBuf>| {
                    s.insert(sub_dir_name.clone());
                })
                .or_insert({
                    let mut set = BTreeSet::default();
                    set.insert(sub_dir_name);
                    set
                });
        }

        for (k, directories) in subdirectories_index {
            directories_index
                .entry(k)
                .and_modify(|directory: &mut DirectoryMut| {
                    directory.directories = directories.clone();
                })
                .or_insert(DirectoryMut {
                    files: Default::default(),
                    directories,
                });
        }

        let directories_index = directories_index
            .into_iter()
            .map(|(k, v)| (k, v.freeze()))
            .collect();

        Ok(Self {
            data: data.freeze(),
            files_index: Arc::new(files_index),
            directories_index: Arc::new(directories_index),
            item_search_index: search_index_builder.finish(),
        })
    }
}

/// A cache for storing and retrieving `Crate` instances to minimize redundant operations.
///
/// This cache uses a least-recently-used (LRU) strategy and is thread-safe.
#[derive(Clone)]
pub struct CrateCache {
    lru: Arc<Mutex<LruCache<CrateVersion, Crate, fnv::FnvBuildHasher>>>,
}

impl Default for CrateCache {
    fn default() -> Self {
        Self::new(unsafe { NonZeroUsize::new_unchecked(2048) })
    }
}

impl CrateCache {
    /// Creates a new `CrateCache` with a specified capacity.
    ///
    pub fn new(capacity: NonZeroUsize) -> Self {
        CrateCache {
            lru: Arc::new(Mutex::new(LruCache::with_hasher(
                capacity,
                fnv::FnvBuildHasher::default(),
            ))),
        }
    }

    /// Retrieves a crate from the cache if it exists.
    ///
    pub fn get_crate(&self, crate_version: &CrateVersion) -> Option<Crate> {
        self.lru.lock().get(crate_version).cloned()
    }

    /// Inserts or updates a crate in the cache.
    ///
    pub fn set_crate(
        &self,
        crate_version: impl Into<CrateVersion>,
        krate: impl Into<Crate>,
    ) -> Option<Crate> {
        self.lru.lock().put(crate_version.into(), krate.into())
    }
}
