//! The `axum` module.
//!
use crate::app::RustAssistant;
use crate::cache::{CrateCache, FileContent, FileDataType};
use crate::download::CrateDownloader;
use crate::github::{GithubClient, Repository, RepositoryPath};
use crate::{CrateVersion, CrateVersionPath, FileLineRange, ItemQuery, LineQuery};
use axum::extract::{FromRequestParts, Path, Query, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::{Extension, Json, Router};
use axum_extra::headers::authorization::Basic;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Search for lines in a specific crate.
///
/// This asynchronous function handles GET requests to search for lines within a crate's files.
/// It extracts crate version and line query parameters from the request, performs the search, and returns the results.
///
#[cfg_attr(feature = "utoipa",
utoipa::path(get, path = "/api/lines/{crate}/{version}", responses(
        (status = 200, description = "Search the crate for lines successfully.", body = [Line]),
        (status = 500, description = "Internal server error.", body = String),
    ),
    params(
        ("crate" = String, Path, description = "The exact name of the crate."),
        ("version" = String, Path, description = "The semantic version number of the crate, following the Semantic versioning specification."),
        ("query" = String, Query, description = "Query string."),
        ("mode" = SearchMode, Query, description = "Search mode."),
        ("case_sensitive" = Option<bool>, Query, description = "Case sensitive."),
        ("whole_word" = Option<bool>, Query, description = "Whole word."),
        ("max_results" = Option<usize>, Query, description = "Max results count."),
        ("file_ext" = Option<usize>, Query, description = "The extensions of files to search."),
        ("path" = Option<String>, Query, description = "Directory containing the lines to search."),
    ),
    security(
        ("api_auth" = [])
    )
))]
pub async fn search_crate_for_lines(
    Path(crate_version): Path<CrateVersion>,
    Query(query): Query<LineQuery>,
    State(state): State<RustAssistant>,
) -> impl IntoResponse {
    match state.search_line(&crate_version, query).await {
        Ok(lines) => Json(lines).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

/// Search for items in a specific crate.
///
/// This function provides an API endpoint to search for various items like structs, enums,
/// functions, etc., within a crate. It uses query parameters to filter search results.
///
#[cfg_attr(feature = "utoipa",
    utoipa::path(get, path = "/api/items/{crate}/{version}", responses(
        (status = 200, description = "Search the crate for items successfully.", body = [Item]),
        (status = 500, description = "Internal server error.", body = String),
    ),
    params(
        ("crate" = String, Path, description = "The exact name of the crate."),
        ("version" = String, Path, description = "The semantic version number of the crate, following the Semantic versioning specification."),
        ("type" = ItemType, Query, description = "The type of the item."),
        ("query" = String, Query, description = "Query string."),
        ("path" = String, Query, description = "Directory containing the items to search."),
    ),
    security(
        ("api_auth" = [])
    )
))]
pub async fn search_crate_for_items(
    Path(crate_version): Path<CrateVersion>,
    Query(query): Query<ItemQuery>,
    State(state): State<RustAssistant>,
) -> impl IntoResponse {
    match state.search_item(&crate_version, query).await {
        Ok(items) => Json(items).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

/// Get the content of a file in a crate.
///
/// This function serves an endpoint to retrieve the content of a specific file from a crate,
/// potentially within a specified range of lines.
///
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

/// Read a subdirectory in a crate.
///
/// This endpoint provides access to the contents of a subdirectory within a crate,
/// including files and other subdirectories.
///
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

/// Read the root directory of a GitHub repository.
///
/// This endpoint provides access to the contents of the root directory within a GitHub repository,
///
#[cfg_attr(feature = "utoipa",
    utoipa::path(get, path = "/api/github/directory/{owner}/{repo}", responses(
        (status = 200, description = "Read repository root directory successfully.", body = Directory),
        (status = 404, description = "The root directory does not exist."),
        (status = 500, description = "Internal server error."),
    ),
        params(
            ("owner" = String, Path, description = "The owner of the GitHub repository."),
            ("repo" = String, Path, description = "The name of the GitHub repository."),
        ),
        security(
            ("api_auth" = [])
        )
    ))]
pub async fn read_github_repository_root_directory(
    Path(repository): Path<Repository>,
    State(state): State<RustAssistant>,
) -> impl IntoResponse {
    match state
        .read_github_repository_directory(&repository, "")
        .await
    {
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Ok(Some(directory)) => Json(directory).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

/// Read a subdirectory in a GitHub repository.
///
/// This endpoint provides access to the contents of a subdirectory within a GitHub repository,
/// including files and other subdirectories.
#[cfg_attr(feature = "utoipa",
    utoipa::path(get, path = "/api/github/directory/{owner}/{repo}/{path}", responses(
        (status = 200, description = "Read the subdirectory successfully.", body = Directory),
        (status = 404, description = "The directory does not exist."),
        (status = 500, description = "Internal server error.", body = String),
    ),
        params(
            ("owner" = String, Path, description = "The owner of the GitHub repository."),
            ("repo" = String, Path, description = "The name of the GitHub repository."),
            ("path" = String, Path, description = "Relative path of a directory in repository."),
        ),
        security(
            ("api_auth" = [])
        )
    ))]
pub async fn read_github_repository_directory(
    Path(repository_path): Path<RepositoryPath>,
    State(state): State<RustAssistant>,
) -> impl IntoResponse {
    match state
        .read_github_repository_directory(&repository_path.repo, repository_path.path.as_ref())
        .await
    {
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Ok(Some(directory)) => Json(directory).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

/// Read the content of a file in a GitHub repository.
///
/// This function serves an endpoint to retrieve the content of a specific file from a GitHub repository.
///
#[cfg_attr(feature = "utoipa",
    utoipa::path(get, path = "/api/github/file/{owner}/{repo}/{path}", responses(
        (status = 200, description = "Read the file successfully.", body = String),
        (status = 404, description = "The file does not exist."),
        (status = 500, description = "Internal server error.", body = String),
    ),
        params(
            ("owner" = String, Path, description = "The owner of the GitHub repository."),
            ("repo" = String, Path, description = "The name of the GitHub repository."),
            ("path" = String, Path, description = "Relative path of a file in repository."),
        ),
        security(
            ("api_auth" = [])
        )
    ))]
pub async fn read_github_repository_file_content(
    Path(repository_path): Path<RepositoryPath>,
    State(state): State<RustAssistant>,
) -> impl IntoResponse {
    match state
        .read_github_repository_file(&repository_path.repo, repository_path.path.as_ref())
        .await
    {
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Ok(Some(file)) => file.into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

/// Health check endpoint.
///
/// This endpoint is used to perform a health check of the API, ensuring that it is running and responsive.
///
pub async fn health() {}

/// Redirect the client to "https://rustassistant.com".
///
pub async fn redirect() -> impl IntoResponse {
    Redirect::to("https://rustassistant.com")
}

/// Privacy policy endpoint.
///
/// This endpoint provides access to the privacy policy of the Rust Assistant application.
///
pub async fn privacy_policy() -> impl IntoResponse {
    include_str!("../../../doc/privacy-policy.md")
}

/// Configures and returns the axum router for the API.
///
/// This function sets up the routing for the API, including all the endpoints for searching crates,
/// reading file contents, and accessing directory information. It also configures any necessary middleware.
///
pub fn router(
    auth_info: impl Into<Option<AuthInfo>>,
    github_token: &str,
) -> anyhow::Result<Router> {
    let main = Router::new()
        .route("/", get(redirect))
        .route("/health", get(health))
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
        .route("/lines/:crate/:version", get(search_crate_for_lines))
        .route("/items/:crate/:version", get(search_crate_for_items))
        .route("/file/:crate/:version/*path", get(get_file_content))
        .nest(
            "/directory/:crate/:version",
            Router::new()
                .route("/", get(read_crate_root_directory))
                .route("/*path", get(read_crate_directory)),
        )
        .nest(
            "/github",
            Router::new()
                .nest(
                    "/directory/:owner/:repo",
                    Router::new()
                        .route("/", get(read_github_repository_root_directory))
                        .route("/*path", get(read_github_repository_directory)),
                )
                .route(
                    "/file/:owner/:repo/*path",
                    get(read_github_repository_file_content),
                ),
        )
        .with_state(RustAssistant::from((
            CrateDownloader::default(),
            CrateCache::default(),
            GithubClient::new(github_token, None)?,
        )));

    let api = if let Some(auth_info) = auth_info.into() {
        api.layer(axum::middleware::from_extractor::<RequireAuth>())
            .layer(Extension(auth_info))
    } else {
        api
    };

    Ok(main.nest("/api", api))
}

impl IntoResponse for FileContent {
    fn into_response(self) -> Response {
        let content_type = match self.data_type {
            FileDataType::Utf8 => "text/plain; charset=utf-8",
            FileDataType::NonUtf8 => "application/octet-stream",
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            HeaderValue::from_static(content_type),
        );
        (headers, self.data).into_response()
    }
}

/// Authentication information structure.
///
/// This struct holds authentication credentials, such as username and password, used for API access.
///
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthInfo {
    /// Username for authentication.
    pub username: Arc<str>,
    /// Password for authentication.
    pub password: Arc<str>,
}

impl AuthInfo {
    /// Validates the provided basic authentication against the stored credentials.
    ///
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

/// Middleware for API authentication.
///
/// This struct is used as a middleware in Axum routes to require authentication
/// for accessing certain endpoints.
///
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
        super::search_crate_for_items,
        super::search_crate_for_lines,
        super::read_github_repository_root_directory,
        super::read_github_repository_directory,
        super::read_github_repository_file_content,
    ),
    components(
        schemas(crate::Directory, crate::Item, crate::ItemType, crate::SearchMode, crate::Line, crate::RangeSchema)
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
