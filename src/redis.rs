use bb8_redis::{
    bb8::{self, Pool},
    RedisConnectionManager,
};
use redis::RedisError;

pub type RedisPool = Pool<RedisConnectionManager>;

pub async fn create_redis_pool(connspec: &str) -> Result<RedisPool, RedisError> {
    let manager = bb8_redis::RedisConnectionManager::new(connspec).expect("connectaction mgr");
    bb8::Pool::builder().max_size(15).build(manager).await
}
