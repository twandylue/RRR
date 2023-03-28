use rate_limiter_redis::RateLimiterRedis;
use std::time::Duration;

mod rate_limiter_redis;

#[tokio::main]
async fn main() -> Result<(), ()> {
    let conn = "redis://127.0.0.1:6379/";
    let mut redis_client = RateLimiterRedis::open(&conn, 10).await?;
    redis_client
        .record_fixed_window("test", "data", "andy", Duration::from_secs(10))
        .await?;

    Ok(())
}
