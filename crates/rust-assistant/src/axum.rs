use crate::app::RustAssistant;
use crate::cache::{CrateFileContent, CrateFileDataType};
use crate::{CrateVersion, CrateVersionPath, FileLineRange};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};

async fn get_file_summary(Path(path): Path<CrateVersionPath>) -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED, Json(path))
}

async fn get_file_content(
    Path(path): Path<CrateVersionPath>,
    Query(range): Query<FileLineRange>,
    State(state): State<RustAssistant>,
) -> impl IntoResponse {
    match state.get_file_content(&path, range).await {
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Ok(Some(file)) => file.into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

async fn read_crate_directory(
    Path(path): Path<CrateVersionPath>,
    State(state): State<RustAssistant>,
) -> impl IntoResponse {
    match state.read_directory(path).await {
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Ok(Some(directory)) => {
            if directory.is_empty() {
                (StatusCode::BAD_REQUEST, "Not a directory").into_response()
            } else {
                Json(directory).into_response()
            }
        }
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

async fn read_crate_root_directory(
    Path(crate_version): Path<CrateVersion>,
    State(state): State<RustAssistant>,
) -> impl IntoResponse {
    match state
        .read_directory(CrateVersionPath {
            crate_version,
            path: "".into(),
        })
        .await
    {
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Ok(Some(directory)) => {
            if directory.is_empty() {
                (StatusCode::BAD_REQUEST, "Not a directory").into_response()
            } else {
                Json(directory).into_response()
            }
        }
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

async fn privacy_policy() -> impl IntoResponse {
    include_str!("../../../doc/privacy-policy.md")
}

pub fn router() -> Router {
    let directory_app = Router::new()
        .route("/", get(read_crate_root_directory))
        .route("/*path", get(read_crate_directory));

    Router::new()
        .route("/api/summary/:crate/:version/*path", get(get_file_summary))
        .route("/api/file/:crate/:version/*path", get(get_file_content))
        .nest("/api/directory/:crate/:version", directory_app)
        .route("/privacy-policy", get(privacy_policy))
        .with_state(RustAssistant::default())
}

impl IntoResponse for CrateFileContent {
    fn into_response(self) -> Response {
        let content_type = match self.data_type {
            CrateFileDataType::Utf8 => "text/plain; charset=utf-8",
            CrateFileDataType::NonUtf8 => "application/octet-stream",
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            HeaderValue::from_static(content_type),
        );
        (headers, self.data).into_response()
    }
}
