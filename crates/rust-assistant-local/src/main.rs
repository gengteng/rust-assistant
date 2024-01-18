use rust_assistant::axum::AuthInfo;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (Some(username), Some(password)) = (
        dotenv::var("API_USERNAME").ok(),
        dotenv::var("API_PASSWORD").ok(),
    ) else {
        return Err(anyhow::anyhow!(
            "'API_USERNAME' and 'API_PASSWORD' must be provided",
        ));
    };
    let listener = TcpListener::bind(SocketAddr::from(([0u8, 0, 0, 0], 3000))).await?;
    Ok(axum::serve(
        listener,
        rust_assistant::axum::router(AuthInfo::from((username, password))).into_make_service(),
    )
    .await?)
}
