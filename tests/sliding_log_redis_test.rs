// WARN: cargo test -- --test-threads 1
async fn initialize_redis() -> Result<(), ()> {
    let redis_address: &str = "redis://127.0.0.1:6379/";
    let client = redis::Client::open(redis_address).map_err(|err| {
        eprintln!("Error: could not open the connection to the Redis({redis_address}): {err}")
    })?;

    let mut conn = client.get_connection().map_err(|err| {
        eprintln!("Error: client could not get the connection to the Redis: {err}")
    })?;

    let _: String = redis::cmd("FLUSHALL")
        .query(&mut conn)
        .map_err(|err| eprintln!("Error: could not flush all data in Redis: {err}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rrr::rate_limiter_redis;
    use std::time::Duration;

    const CONN: &str = "redis://127.0.0.1:6379/";

    /// Tests the requests exceed the rate limit.
    #[tokio::test]
    async fn sliding_log_redis_case1() -> Result<(), ()> {
        // prev
        initialize_redis().await?;

        // arrange
        let limit_count = 5;
        let mut client = rate_limiter_redis::RateLimiterRedis::open(CONN, limit_count).await?;
        let key_prefix = "test2";
        let resource = "data";
        let subject = "andy";
        let size = Duration::from_secs(1);

        // act
        for c in 0..11 {
            client
                .record_sliding_log(key_prefix, resource, subject, size)
                .await?;

            let count = client
                .fetch_sliding_log(key_prefix, resource, subject)
                .await?;

            assert_eq!(count, c + 1);

            tokio::time::sleep(Duration::from_millis(1)).await;
        }

        let actual = client
            .canMakeRequest_sliding_log(key_prefix, resource, subject)
            .await?;

        // assert
        assert!(!actual);

        Ok(())
    }

    /// Tests the requests do not exceed the rate limit.
    #[tokio::test]
    async fn sliding_log_redis_case2() -> Result<(), ()> {
        // prev
        initialize_redis().await?;

        // arrange
        let limit_count = 20;
        let mut client = rate_limiter_redis::RateLimiterRedis::open(CONN, limit_count).await?;
        let key_prefix = "test2";
        let resource = "data";
        let subject = "andy";
        let size = Duration::from_secs(1);

        // act
        for c in 0..11 {
            client
                .record_sliding_log(key_prefix, resource, subject, size)
                .await?;

            let count = client
                .fetch_sliding_log(key_prefix, resource, subject)
                .await?;

            assert_eq!(count, c + 1);

            tokio::time::sleep(Duration::from_millis(1)).await;
        }

        let actual = client
            .canMakeRequest_sliding_log(key_prefix, resource, subject)
            .await?;

        // assert
        assert!(actual);

        Ok(())
    }
}
