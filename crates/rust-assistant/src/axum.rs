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

pub async fn get_file_summary(Path(path): Path<CrateVersionPath>) -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED, Json(path))
}

/// Read file in crate.
#[cfg_attr(feature = "utoipa",
    utoipa::path(get, path = "/api/file/{crate}/{version}/{path}", responses(
        (status = 200, description = "Read the file successfully.", body = String),
        (status = 404, description = "The file does not exist."),
        (status = 500, description = "Internal server error.", body = String),
    ),
    params(
        ("crate" = String, Path, description = "The exact name of the crate."),
        ("version" = String, Path, description = "The semantic version number of the crate, following the Semantic versioning specification."),
        ("path" = String, Path, description = "Relative path of a file in crate."),
        ("start" = usize, Path, description = "Start line number of the file (inclusive)."),
        ("end" = usize, Path, description = "End line number of the file (inclusive)."),
    ),
    security(
    ("api_auth" = [])
    )
))]
pub async fn get_file_content(
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

/// Read a subdirectory in crate.
#[cfg_attr(feature = "utoipa",
    utoipa::path(get, path = "/api/directory/{crate}/{version}/{path}", responses(
        (status = 200, description = "Read the subdirectory successfully.", body = Directory),
        (status = 404, description = "The directory does not exist."),
        (status = 500, description = "Internal server error.", body = String),
    ),
    params(
        ("crate" = String, Path, description = "The exact name of the crate."),
        ("version" = String, Path, description = "The semantic version number of the crate, following the Semantic versioning specification."),
        ("path" = String, Path, description = "Relative path of a directory in crate."),
    ),
    security(
        ("api_auth" = [])
    )
))]
pub async fn read_crate_directory(
    Path(path): Path<CrateVersionPath>,
    State(state): State<RustAssistant>,
) -> impl IntoResponse {
    match state.read_directory(path).await {
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Ok(Some(directory)) => Json(directory).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

/// Read crate root directory.
#[cfg_attr(feature = "utoipa", 
    utoipa::path(get, path = "/api/directory/{crate}/{version}", responses(
        (status = 200, description = "Read crate root directory successfully.", body = Directory),
        (status = 404, description = "The root directory does not exist."),
        (status = 500, description = "Internal server error."),
    ),
    params(
        ("crate" = String, Path, description = "The exact name of the crate."),
        ("version" = String, Path, description = "The semantic version number of the crate, following the Semantic versioning specification."),
    ),
    security(
        ("api_auth" = [])
    )
))]
pub async fn read_crate_root_directory(
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
        Ok(Some(directory)) => Json(directory).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

pub async fn health() {}

pub async fn privacy_policy() -> impl IntoResponse {
    include_str!("../../../doc/privacy-policy.md")
}

pub fn router(auth_info: impl Into<Option<AuthInfo>>) -> Router {
    let main = Router::new()
        .route("/", get(health))
        .route("/privacy-policy", get(privacy_policy));

    #[cfg(feature = "utoipa")]
    let main = {
        use utoipa::OpenApi;
        main.merge(
            utoipa_swagger_ui::SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", swagger_ui::ApiDoc::openapi()),
        )
    };

    let api = Router::new()
        .route("/summary/:crate/:version/*path", get(get_file_summary))
        .route("/file/:crate/:version/*path", get(get_file_content))
        .nest(
            "/directory/:crate/:version",
            Router::new()
                .route("/", get(read_crate_root_directory))
                .route("/*path", get(read_crate_directory)),
        )
        .with_state(RustAssistant::default());

    let api = if let Some(auth_info) = auth_info.into() {
        api.layer(axum::middleware::from_extractor::<RequireAuth>())
            .layer(Extension(auth_info))
    } else {
        api
    };

    main.nest("/api", api)
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

#[cfg(feature = "utoipa")]
mod swagger_ui {
    use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};

    #[derive(utoipa::OpenApi)]
    #[openapi(
    info(
        title = "Rust Assistant API",
        description = "API that supports source code browsing of crates on crates.io for Rust Assistant."
    ),
    paths(
        super::get_file_content,
        super::read_crate_directory,
        super::read_crate_root_directory,
    ),
    components(
        schemas(crate::Directory)
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Rust Assistant", description = "Rust Assistant API")
    )
    )]
    pub struct ApiDoc;

    struct SecurityAddon;

    impl utoipa::Modify for SecurityAddon {
        fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
            if let Some(components) = openapi.components.as_mut() {
                components.add_security_scheme(
                    "api_auth",
                    SecurityScheme::Http(Http::new(HttpAuthScheme::Basic)),
                )
            }
        }
    }
}
