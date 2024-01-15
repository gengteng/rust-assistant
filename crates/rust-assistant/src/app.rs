use crate::cache::CrateCache;
use crate::download::CrateDownloader;

#[derive(Clone, Default)]
pub struct RustAssistantApplication {
    downloader: CrateDownloader,
    cache: CrateCache,
}

impl RustAssistantApplication {}
