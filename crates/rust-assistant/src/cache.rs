use crate::{CrateVersion, Directory};
use bytes::{Bytes, BytesMut};
use fnv::FnvHashMap;
use lru::LruCache;
use parking_lot::Mutex;
use std::collections::BTreeSet;
use std::io::Read;
use std::num::NonZeroUsize;
use std::ops::{Bound, Range, RangeBounds};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tar::EntryType;

#[derive(Clone)]
pub struct CrateTar {
    pub crate_version: CrateVersion,
    pub tar_data: Arc<[u8]>,
}

impl<C, D> From<(C, D)> for CrateTar
where
    C: Into<CrateVersion>,
    D: Into<Arc<[u8]>>,
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
        let mut archive = tar::Archive::new(self.tar_data.as_ref());
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
        let mut archive = tar::Archive::new(self.tar_data.as_ref());
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
    ) -> anyhow::Result<Option<BTreeSet<PathBuf>>> {
        let mut archive = tar::Archive::new(self.tar_data.as_ref());
        let root_dir = self.crate_version.root_dir();
        let entries = archive.entries()?;
        let mut list = BTreeSet::new();
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
        let mut archive = tar::Archive::new(self.tar_data.as_ref());
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
pub struct CrateFile {
    pub data_type: CrateFileDataType,
    pub data: Bytes,
}

#[derive(Debug, Clone)]
pub struct Crate {
    pub data: Bytes,
    pub files_index: FnvHashMap<PathBuf, CrateFileDataDesc>,
    pub directories_index: FnvHashMap<PathBuf, Directory>,
}

impl Crate {
    pub fn get_file_by_line_range<P: AsRef<Path>>(
        &self,
        file: P,
        line_range: impl RangeBounds<NonZeroUsize>,
    ) -> anyhow::Result<Option<CrateFile>> {
        let file = file.as_ref();
        let Some(CrateFileDataDesc { range, data_type }) = self.files_index.get(file) else {
            return Ok(None);
        };

        let data = self.data.slice(range.clone());

        if matches!(
            (line_range.start_bound(), line_range.end_bound()),
            (Bound::Unbounded, Bound::Unbounded)
        ) {
            return Ok(Some(CrateFile {
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
            Bound::Included(n) => n.get() - 1,
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
            return Ok(Some(CrateFile {
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
        let mut archive = tar::Archive::new(crate_tar.tar_data.as_ref());
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
                let parent = match path.parent() {
                    None => PathBuf::from("."),
                    Some(parent) => parent.to_path_buf(),
                };
                directories_index
                    .entry(parent)
                    .and_modify(|o: &mut Directory| {
                        o.files.insert(filename.clone());
                    })
                    .or_insert({
                        let mut set = BTreeSet::new();
                        set.insert(filename);
                        Directory {
                            files: set,
                            directories: Default::default(),
                        }
                    });
            }
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
    lru: Arc<Mutex<LruCache<CrateVersion, Arc<[u8]>>>>,
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
            lru: Arc::new(Mutex::new(LruCache::new(capacity))),
        }
    }

    /// Get raw crate file data
    pub fn get_data(&self, crate_version: &CrateVersion) -> Option<Arc<[u8]>> {
        self.lru.lock().get(crate_version).cloned()
    }

    /// Get crate
    pub fn get(&self, crate_version: impl Into<CrateVersion>) -> Option<CrateTar> {
        let crate_version = crate_version.into();
        let data = self.lru.lock().get(&crate_version).cloned()?;
        Some(CrateTar {
            crate_version,
            tar_data: data,
        })
    }

    /// Set crate file data.
    pub fn set_data(
        &self,
        crate_version: impl Into<CrateVersion>,
        data: impl Into<Arc<[u8]>>,
    ) -> Option<Arc<[u8]>> {
        self.lru.lock().put(crate_version.into(), data.into())
    }
}
