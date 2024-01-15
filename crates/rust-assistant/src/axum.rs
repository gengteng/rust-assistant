use crate::app::RustAssistant;
use crate::{CrateVersion, CrateVersionPath, FileLineRange};
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};

async fn get_file_summary(Path(path): Path<CrateVersionPath>) -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED, Json(path))
}

async fn get_file_content(
    Path(path): Path<CrateVersionPath>,
    Query(range): Query<FileLineRange>,
) -> Json<(CrateVersionPath, FileLineRange)> {
    Json((path, range))
}

async fn get_crate_file_list(Path(path): Path<CrateVersionPath>) -> Json<CrateVersionPath> {
    Json(path)
}

async fn read_crate_directory(Path(path): Path<CrateVersion>) -> Json<CrateVersion> {
    Json(path)
}

pub fn router() -> Router {
    let directory_app = Router::new()
        .route("/", get(read_crate_directory))
        .route("/*path", get(get_crate_file_list));

    Router::new()
        .route("/api/summary/:crate/:version/*path", get(get_file_summary))
        .route("/api/file/:crate/:version/*path", get(get_file_content))
        .nest("/api/directory/:crate/:version", directory_app)
        .with_state(RustAssistant::default())
}
