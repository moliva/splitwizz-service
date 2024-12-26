use bb8_redis::{
    bb8::{self, Pool},
    RedisConnectionManager,
};
use redis::{AsyncCommands, RedisError};

pub type RedisPool = Pool<RedisConnectionManager>;

pub async fn create_redis_pool(connspec: &str) -> Result<RedisPool, RedisError> {
    let manager = bb8_redis::RedisConnectionManager::new(connspec).expect("connectaction mgr");
    bb8::Pool::builder().max_size(15).build(manager).await
}

pub async fn publish_topic(redis: RedisPool, topic: String, payload: String) {
    let mut redis = redis.get().await.expect("pooled conn");

    redis
        .publish::<String, String, ()>(topic, payload)
        .await
        .expect("published topic");
}
