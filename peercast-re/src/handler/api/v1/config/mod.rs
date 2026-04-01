use axum::extract::State;

use crate::AppState;

#[utoipa::path(
    get,
    path = "/config",
    responses(
        (status = 200, description = "get config")
    )
)]
#[allow(dead_code)]
pub(super) async fn get_config(State(store): State<AppState>) -> impl axum::response::IntoResponse {
    let config_path = store.config_path.clone();

    #[derive(Debug, serde::Serialize)]
    struct Config {
        config_path: String,
    }

    axum::Json(Config {
        config_path: config_path.to_string_lossy().to_string(),
    })
}
