use crate::{CrateVersion, Directory, FileLineRange};
use bytes::{Bytes, BytesMut};
use fnv::{FnvHashMap, FnvHashSet};
use lru::LruCache;
use parking_lot::Mutex;
use std::io::Read;
use std::num::NonZeroUsize;
use std::ops::{Bound, Range, RangeBounds};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tar::EntryType;

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
    /// Get file content
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
        return Ok(None);
    }

    /// Get file content by range
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
        return Ok(None);
    }

    /// List all files in a crate
    pub fn get_all_file_list(
        &self,
        range: impl RangeBounds<usize>,
    ) -> anyhow::Result<Option<FnvHashSet<PathBuf>>> {
        let mut archive = tar::Archive::new(self.tar_data.as_slice());
        let root_dir = self.crate_version.root_dir();
        let entries = archive.entries()?;
        let mut list = FnvHashSet::default();
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

    pub fn read_directory<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<Option<Directory>> {
        let mut archive = tar::Archive::new(self.tar_data.as_slice());
        let base_dir = self.crate_version.root_dir().join(path);
        let entries = archive.entries()?;
        let mut dir = Directory::default();
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

        Ok(Some(dir))
    }
}

#[derive(Debug, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum CrateFileDataType {
    Utf8,
    #[default]
    NonUtf8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct CrateFileDataDesc {
    pub data_type: CrateFileDataType,
    pub range: Range<usize>,
}

#[derive(Debug, Clone)]
pub struct CrateFileContent {
    pub data_type: CrateFileDataType,
    pub data: Bytes,
}

#[derive(Debug, Clone)]
pub struct Crate {
    data: Bytes,
    files_index: FnvHashMap<PathBuf, CrateFileDataDesc>,
    directories_index: FnvHashMap<PathBuf, Directory>,
}

impl Crate {
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

    pub fn read_directory<P: AsRef<Path>>(&self, path: P) -> Option<&Directory> {
        self.directories_index.get(path.as_ref())
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

            let path = path.to_path_buf();
            if let EntryType::Regular = entry.header().entry_type() {
                buffer.clear();
                entry.read_to_end(&mut buffer)?;

                let data_type = match std::str::from_utf8(&buffer) {
                    Ok(_) => CrateFileDataType::Utf8,
                    Err(_) => CrateFileDataType::NonUtf8,
                };

                let range = data.len()..data.len() + buffer.len();

                data.extend_from_slice(buffer.as_slice());
                files_index.insert(path.clone(), CrateFileDataDesc { data_type, range });
                let parent = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
                directories_index
                    .entry(parent)
                    .and_modify(|o: &mut Directory| {
                        o.files.insert(filename.clone());
                    })
                    .or_insert({
                        let mut set = FnvHashSet::default();
                        set.insert(filename);
                        Directory {
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
                .and_modify(|s: &mut FnvHashSet<PathBuf>| {
                    s.insert(sub_dir_name.clone());
                })
                .or_insert({
                    let mut set = FnvHashSet::default();
                    set.insert(sub_dir_name);
                    set
                });
        }

        for (k, directories) in subdirectories_index {
            directories_index
                .entry(k)
                .and_modify(|directory: &mut Directory| {
                    directory.directories = directories.clone();
                })
                .or_insert(Directory {
                    files: Default::default(),
                    directories,
                });
        }

        Ok(Self {
            data: data.freeze(),
            files_index,
            directories_index,
        })
    }
}

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
    /// Create a new cache instance.
    pub fn new(capacity: NonZeroUsize) -> Self {
        CrateCache {
            lru: Arc::new(Mutex::new(LruCache::with_hasher(
                capacity,
                fnv::FnvBuildHasher::default(),
            ))),
        }
    }

    /// Get crate
    pub fn get_crate(&self, crate_version: &CrateVersion) -> Option<Crate> {
        self.lru.lock().get(crate_version).cloned()
    }

    /// Set crate
    pub fn set_crate(
        &self,
        crate_version: impl Into<CrateVersion>,
        krate: impl Into<Crate>,
    ) -> Option<Crate> {
        self.lru.lock().put(crate_version.into(), krate.into())
    }
}
