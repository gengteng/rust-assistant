#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    Ok(rust_assistant::axum::router().into())
}
