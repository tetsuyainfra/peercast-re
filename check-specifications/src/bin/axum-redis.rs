
use axum::{extract::{FromRef, FromRequestParts, State}, routing::get, Router};
use http::{request::Parts, StatusCode};
use redis::AsyncCommands;

pub struct AppState {
}


#[tokio::main]
async fn main(){
    let manager = bb8_redis::RedisConnectionManager::new("redis://localhost").unwrap();
    let pool = bb8::Pool::builder().build(manager).await.unwrap();
    {
        // ping the database before starting
        let mut conn = pool.get().await.unwrap();
        conn.set::<&str, &str, ()>("foo", "bar").await.unwrap();
        let result: String = conn.get("foo").await.unwrap();
        assert_eq!(result, "bar");
    }
    println!("successfully connected to redis and pinged it");


    let router = Router::new()
        .route("/", get(root_handler))
        .route("/send", get(send_handler)).with_state(pool);

    let addr = "127.0.0.1:3000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("http://{}/",addr);
    println!("http://{}/send",addr);

    axum::serve(listener, router).await.unwrap();
}


type ConnectionPool = bb8::Pool<bb8_redis::RedisConnectionManager>;

async fn root_handler(
    State(pool): State<ConnectionPool>,
) ->Result<String, (StatusCode, String)> {
    let mut conn = pool.get().await.map_err(internal_error)?;
    let result: String = conn.get("foo").await.map_err(internal_error)?;

    let html =    format!(
    r#"
we are here -> /
foo: {val}
"#,
val = result
);

    Ok(html)
}

// we can also write a custom extractor that grabs a connection from the pool
// which setup is appropriate depends on your application
struct DatabaseConnection(bb8::PooledConnection<'static, bb8_redis::RedisConnectionManager>);

impl<S> FromRequestParts<S> for DatabaseConnection
where
    ConnectionPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = ConnectionPool::from_ref(state);

        let conn = pool.get_owned().await.map_err(internal_error)?;

        Ok(Self(conn))
    }
}

async fn send_handler(
    DatabaseConnection(mut conn): DatabaseConnection,
) -> Result<String, (StatusCode, String)> {
    let result: String = conn.get("foo").await.map_err(internal_error)?;

    Ok(result)
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}