use crate::CrateVersion;
use bytes::Bytes;
use lru::LruCache;
use parking_lot::Mutex;
use std::io::Read;
use std::num::{NonZeroU64, NonZeroUsize};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct CrateCache {
    lru: Arc<Mutex<LruCache<CrateVersion, Bytes>>>,
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
    pub fn get_raw(&self, crate_version: &CrateVersion) -> Option<Bytes> {
        self.lru.lock().get(crate_version).cloned()
    }

    /// Set raw crate file data.
    pub fn set_raw(
        &self,
        crate_version: impl Into<CrateVersion>,
        data: impl Into<Bytes>,
    ) -> Option<Bytes> {
        self.lru.lock().put(crate_version.into(), data.into())
    }

    /// Get file content
    pub fn get_file(
        &self,
        crate_version: &CrateVersion,
        file: &str,
    ) -> anyhow::Result<Option<String>> {
        let Some(data) = self.lru.lock().get(crate_version).cloned() else {
            return Ok(None);
        };
        let mut archive = tar::Archive::new(data.as_ref());
        let entries = archive.entries()?;
        for entry in entries {
            let Ok(mut entry) = entry else {
                continue;
            };

            let Ok(path) = entry.path() else {
                continue;
            };

            if crate_version.root_dir().join(file).eq(path.as_ref()) {
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
        crate_version: &CrateVersion,
        file: &str,
        start: impl Into<Option<NonZeroU64>>,
        end: impl Into<Option<NonZeroU64>>,
    ) -> anyhow::Result<Option<String>> {
        let Some(data) = self.lru.lock().get(crate_version).cloned() else {
            return Ok(None);
        };
        let mut archive = tar::Archive::new(data.as_ref());
        let entries = archive.entries()?;
        for entry in entries {
            let Ok(mut entry) = entry else {
                continue;
            };

            let Ok(path) = entry.path() else {
                continue;
            };

            if crate_version.root_dir().join(file).eq(path.as_ref()) {
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
    pub fn list_files(&self, crate_version: &CrateVersion) -> anyhow::Result<Option<Vec<PathBuf>>> {
        let Some(data) = self.lru.lock().get(crate_version).cloned() else {
            return Ok(None);
        };
        let mut archive = tar::Archive::new(data.as_ref());
        let root_dir = crate_version.root_dir();
        let entries = archive.entries()?;
        let mut vec = Vec::new();
        for entry in entries {
            let Ok(entry) = entry else {
                continue;
            };

            let Ok(path) = entry.path() else {
                continue;
            };

            let Ok(path) = path.strip_prefix(&root_dir) else {
                continue;
            };
            vec.push(path.to_path_buf());
        }
        Ok(Some(vec))
    }
}
