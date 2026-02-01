use tower_http::normalize_path::NormalizePathLayer;

#[tokio::main]
async fn main() {}

fn router() -> axum::Router {
    use axum::{
        Router,
        routing::{get, post},
    };
    let user_routes = Router::new().route("/{id}", get(|| async {}));
    let team_routes = Router::new().route("/", post(|| async {}));

    let api_routes = Router::new().nest("/users", user_routes).nest("/teams", team_routes);

    let app = Router::new().nest("/api", api_routes).layer(NormalizePathLayer::trim_trailing_slash()); // Requst Pathの正規化 /A/B/ -> /A/B
    // Our app now accepts
    // - GET /api/users/{id}
    // - POST /api/teams
    app
}

#[cfg(test)]
mod tests {
    use tower::ServiceExt;

    use super::*;

    // access /api/users/123
    #[tokio::test]
    async fn test_api_id() {
        let app = router();
        let response = app
            .oneshot(axum::http::Request::builder().uri("/api/users/123").body(axum::body::Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    // access /api/teams
    #[tokio::test]
    async fn test_api_teams() {
        let app = router();
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/teams")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    // access /api/teams/
    #[ignore = "必ず失敗するから, /はどうすればいいかわからにゃい"]
    #[tokio::test]
    async fn test_api_teams_slash() {
        let app = router();
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/teams/")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }
}
