use bb8_redis::RedisConnectionManager;

pub type ConnectionPool = bb8::Pool<RedisConnectionManager>;
pub struct DatabaseConnection(pub bb8::PooledConnection<'static, RedisConnectionManager>);


mod model;
mod repository;

pub use model::*;
pub use repository::*;
