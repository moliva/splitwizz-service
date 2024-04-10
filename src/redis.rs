use r2d2::Pool;
use redis::Client;

pub type RedisPool = Pool<Client>;

pub fn create_redis_pool(connspec: &str) -> Result<RedisPool, r2d2::Error> {
    let redis_connection = redis::Client::open(connspec).expect("redis connected successfully");
    r2d2::Pool::builder()
        .min_idle(Some(5))
        .max_size(10)
        .build(redis_connection)
}
