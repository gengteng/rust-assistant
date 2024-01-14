use axum::extract::{Path, Query};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CrateVersion {
    #[serde(rename = "crate")]
    pub krate: Arc<str>,
    pub version: Arc<str>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CrateVersionPath {
    #[serde(flatten)]
    pub crate_version: CrateVersion,
    pub path: Arc<str>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileLineRange {
    pub start: Option<u64>,
    pub end: Option<u64>,
}

async fn read_ast(Path(path): Path<CrateVersionPath>) -> Json<CrateVersionPath> {
    Json(path)
}

async fn read_file(
    Path(path): Path<CrateVersionPath>,
    Query(range): Query<FileLineRange>,
) -> Json<(CrateVersionPath, FileLineRange)> {
    Json((path, range))
}

async fn read_directory(Path(path): Path<CrateVersionPath>) -> Json<CrateVersionPath> {
    Json(path)
}

async fn read_crate_directory(Path(path): Path<CrateVersion>) -> Json<CrateVersion> {
    Json(path)
}

pub fn router() -> Router {
    let directory_app = Router::new()
        .route("/", get(read_crate_directory))
        .route("/*path", get(read_directory));

    Router::new()
        .route("/api/ast/:crate/:version/*path", get(read_ast))
        .route("/api/file/:crate/:version/*path", get(read_file))
        .nest("/api/directory/:crate/:version", directory_app)
}
