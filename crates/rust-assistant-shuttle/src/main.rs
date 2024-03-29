use rust_assistant::axum::AuthInfo;
use shuttle_runtime::CustomError;

#[shuttle_runtime::main]
async fn main(
    #[shuttle_secrets::Secrets] secret_store: shuttle_secrets::SecretStore,
) -> shuttle_axum::ShuttleAxum {
    let Some(username) = secret_store.get("API_USERNAME") else {
        return Err(shuttle_runtime::Error::Custom(CustomError::msg(
            "'API_USERNAME' must be provided",
        )));
    };
    let Some(password) = secret_store.get("API_PASSWORD") else {
        return Err(shuttle_runtime::Error::Custom(CustomError::msg(
            "'API_PASSWORD' must be provided",
        )));
    };
    Ok(rust_assistant::axum::router(AuthInfo::from((username, password))).into())
}
