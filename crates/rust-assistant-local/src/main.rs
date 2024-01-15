use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind(SocketAddr::from(([0u8, 0, 0, 0], 3000))).await?;
    Ok(axum::serve(listener, rust_assistant::axum::router().into_make_service()).await?)
}
