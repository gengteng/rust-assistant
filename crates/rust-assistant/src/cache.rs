use crate::{CrateVersion, Directory};
use lru::LruCache;
use parking_lot::Mutex;
use std::collections::BTreeSet;
use std::io::Read;
use std::num::{NonZeroU64, NonZeroUsize};
use std::ops::RangeBounds;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
pub struct Crate {
    pub crate_version: CrateVersion,
    pub data: Arc<[u8]>,
}

impl<C, D> From<(C, D)> for Crate
where
    C: Into<CrateVersion>,
    D: Into<Arc<[u8]>>,
{
    fn from((c, d): (C, D)) -> Self {
        Crate {
            crate_version: c.into(),
            data: d.into(),
        }
    }
}

impl Crate {
    /// Get file content
    pub fn get_file(&self, file: &str) -> anyhow::Result<Option<String>> {
        let mut archive = tar::Archive::new(self.data.as_ref());
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
        start: impl Into<Option<NonZeroU64>>,
        end: impl Into<Option<NonZeroU64>>,
    ) -> anyhow::Result<Option<String>> {
        let mut archive = tar::Archive::new(self.data.as_ref());
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

                let start_line = start.map_or(0, |n| n.get() as usize - 1);
                let end_line = end.map_or(lines.len(), |n| n.get() as usize);

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
        let mut archive = tar::Archive::new(self.data.as_ref());
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
        let mut archive = tar::Archive::new(self.data.as_ref());
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
    pub fn get(&self, crate_version: impl Into<CrateVersion>) -> Option<Crate> {
        let crate_version = crate_version.into();
        let data = self.lru.lock().get(&crate_version).cloned()?;
        Some(Crate {
            crate_version,
            data,
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
