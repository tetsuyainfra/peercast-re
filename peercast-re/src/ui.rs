use axum::{routing, Router};


pub fn router() -> axum::Router {
    Router::new().route("/ui", routing::get(ui_root))
}

async fn  ui_root() -> String{
    "/ui".into()
}
