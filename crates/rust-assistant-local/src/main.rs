use rust_assistant::axum::AuthInfo;
use std::net::{Ipv4Addr, SocketAddr};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Some(username) = dotenv::var("API_USERNAME").ok() else {
        return Err(anyhow::anyhow!("'API_USERNAME' must be provided",));
    };
    let Some(password) = dotenv::var("API_PASSWORD").ok() else {
        return Err(anyhow::anyhow!("'API_PASSWORD' must be provided",));
    };
    let Some(github_token) = dotenv::var("GITHUB_ACCESS_TOKEN").ok() else {
        return Err(anyhow::anyhow!("'GITHUB_ACCESS_TOKEN' must be provided",));
    };
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 3000))).await?;
    Ok(axum::serve(
        listener,
        rust_assistant::axum::router(AuthInfo::from((username, password)), &github_token)?
            .into_make_service(),
    )
    .await?)
}
