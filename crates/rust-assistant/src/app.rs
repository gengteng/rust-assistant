use crate::cache::CrateCache;
use crate::download::CrateDownloader;

#[derive(Clone, Default)]
pub struct RustAssistant {
    downloader: CrateDownloader,
    cache: CrateCache,
}

impl RustAssistant {}
