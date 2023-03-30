// WARN: cargo test --all -- --test-threads 1
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
    async fn leaky_bucket_redis_case1() -> Result<(), ()> {
        // prev
        initialize_redis().await?;

        // arrange
        let limit_count = 5;
        let size = Duration::from_secs(1);
        let mut client = rate_limiter_redis::RateLimiterRedis::open(CONN, limit_count).await?;
        let key_prefix = "test4";
        let resource = "data";
        let subject = "andy";

        // act && assert
        for c in 0..11 {
            client
                .record_leaky_bucket(key_prefix, resource, subject, size)
                .await?;

            let count = client
                .fetch_leaky_bucket(key_prefix, resource, subject, size)
                .await?;

            assert_eq!(count, c + 1);

            let actual = client
                .can_make_request_leaky_bucket(key_prefix, resource, subject, size)
                .await?;

            if c + 1 >= limit_count {
                assert!(!actual)
            } else {
                assert!(actual)
            }
        }

        Ok(())
    }

    /// Tests the requests do not exceed the rate limit.
    #[tokio::test]
    async fn leaky_bucket_redis_case2() -> Result<(), ()> {
        // prev
        initialize_redis().await?;

        // arrange
        let limit_rate_per_sec = 1000;
        let size = Duration::from_secs(1);
        let mut client =
            rate_limiter_redis::RateLimiterRedis::open(CONN, limit_rate_per_sec).await?;
        let key_prefix = "test4";
        let resource = "data";
        let subject = "andy";

        // act && assert
        for _ in 0..1500 {
            client
                .record_leaky_bucket(key_prefix, resource, subject, size)
                .await?;

            let count = client
                .fetch_leaky_bucket(key_prefix, resource, subject, size)
                .await?;

            let permission = client
                .can_make_request_leaky_bucket(key_prefix, resource, subject, size)
                .await?;
        }

        // tokio::time::sleep(Duration::from_secs(1)).await;
        std::thread::sleep(Duration::from_secs(2));
        let count = client
            .fetch_leaky_bucket(key_prefix, resource, subject, size)
            .await?;

        let key = format!("{key_prefix}:{resource}:{subject}");
        // print_the_value(&key)?;

        assert_eq!(count, 0);

        Ok(())
    }

    // NOTE: for test
    fn print_the_value(key: &str) -> Result<(), ()> {
        let redis_address: &str = "redis://127.0.0.1:6379/";
        let client = redis::Client::open(redis_address).map_err(|err| {
            eprintln!("Error: could not open the connection to the Redis({redis_address}): {err}")
        })?;

        let mut conn = client.get_connection().map_err(|err| {
            eprintln!("Error: client could not get the connection to the Redis: {err}")
        })?;

        let (r,): (Vec<String>,) = redis::cmd("LRANGE")
            .arg(&key)
            .arg(0)
            .arg(-1)
            .query(&mut conn)
            .map_err(|err| eprintln!("Error: could not get all data in Redis: {err}"))?;

        println!("values in the Redis: {r:?}");

        Ok(())
    }
}
