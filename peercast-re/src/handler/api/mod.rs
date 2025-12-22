// src/handler/api/mod.rs
// API handler
//
use crate::{AppState, SWAGGER_PATH};

pub mod v1;
pub mod v2;

#[derive(utoipa::OpenApi)]
#[openapi(
    nest(
        (path = "/api/v1", api = v1::ApiV1),
    )
)]
pub struct ApiSetV1 {}

#[derive(utoipa::OpenApi)]
#[openapi(
    nest(
        (path = "/api/v2", api = v2::ApiV2),
    )
)]
pub struct ApiSetV2 {}

pub fn build_swagger() -> utoipa_swagger_ui::SwaggerUi {
    use utoipa::OpenApi;
    use utoipa_swagger_ui::SwaggerUi;
    use utoipa_swagger_ui::Url;

    let swagger_ui = SwaggerUi::new(SWAGGER_PATH).urls(vec![
        (
            Url::with_primary("API v1", "/api-docs/openapi.json", true),
            ApiSetV1::openapi(), // to suppress unused warning
        ),
        (Url::new("API v2", "/api-docs/openapi2.json"), ApiSetV2::openapi()),
    ]);

    return swagger_ui;
}

pub fn build_router() -> axum::Router<AppState> {
    axum::Router::new()
        //
        .nest("/v1", v1::router())
        .nest("/v2", v2::router())
}
