use crate::app::RustAssistant;
use crate::cache::{CrateFileContent, CrateFileDataType};
use crate::{CrateVersion, CrateVersionPath, FileLineRange};
use axum::extract::{FromRequestParts, Path, Query, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Extension, Json, Router};
use axum_extra::headers::authorization::Basic;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

async fn health() {}

async fn privacy_policy() -> impl IntoResponse {
    include_str!("../../../doc/privacy-policy.md")
}

pub fn router(auth_info: impl Into<Option<AuthInfo>>) -> Router {
    let directory_app = Router::new()
        .route("/", get(read_crate_root_directory))
        .route("/*path", get(read_crate_directory));

    let router = Router::new()
        .route("/", get(health))
        .route("/api/summary/:crate/:version/*path", get(get_file_summary))
        .route("/api/file/:crate/:version/*path", get(get_file_content))
        .nest("/api/directory/:crate/:version", directory_app)
        .route("/privacy-policy", get(privacy_policy))
        .with_state(RustAssistant::default());

    if let Some(auth_info) = auth_info.into() {
        router
            .layer(axum::middleware::from_extractor::<RequireAuth>())
            .layer(Extension(auth_info))
    } else {
        router
    }
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthInfo {
    pub username: Arc<str>,
    pub password: Arc<str>,
}

impl AuthInfo {
    pub fn check(&self, basic: &Basic) -> bool {
        self.username.as_ref().eq(basic.username()) && self.password.as_ref().eq(basic.password())
    }
}

impl<U, P> From<(U, P)> for AuthInfo
where
    U: AsRef<str>,
    P: AsRef<str>,
{
    fn from((u, p): (U, P)) -> Self {
        Self {
            username: Arc::from(u.as_ref()),
            password: Arc::from(p.as_ref()),
        }
    }
}

pub struct RequireAuth;

#[axum::async_trait]
impl FromRequestParts<()> for RequireAuth {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &()) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(basic)) =
            TypedHeader::<Authorization<Basic>>::from_request_parts(parts, state)
                .await
                .map_err(IntoResponse::into_response)?;
        let auth_info = Extension::<AuthInfo>::from_request_parts(parts, state)
            .await
            .map_err(IntoResponse::into_response)?;
        if auth_info.check(&basic) {
            Ok(RequireAuth)
        } else {
            Err(StatusCode::UNAUTHORIZED.into_response())
        }
    }
}
