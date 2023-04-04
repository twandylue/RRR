// WARN: cargo test --all -- --test-threads 1
fn initialize_redis() -> Result<(), ()> {
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

    /// The key is not expired.
    #[ignore = "skip"]
    #[test]
    fn leaky_bucket_redis_case1() -> Result<(), ()> {
        // prev
        initialize_redis()?;

        // arrange
        let limit_count = 5;
        let size = Duration::from_secs(1);
        let mut client = rate_limiter_redis::RateLimiterRedis::open(CONN, limit_count)?;
        let key_prefix = "test4";
        let resource = "data";
        let subject = "andy";

        // act && assert
        for c in 0..11 {
            client.record_leaky_bucket(key_prefix, resource, subject, size)?;

            let count = client.fetch_leaky_bucket(key_prefix, resource, subject, size)?;

            assert_eq!(count, c + 1);

            let permission =
                client.allow_request_leaky_bucket(key_prefix, resource, subject, size)?;

            if c + 1 <= limit_count {
                assert!(permission)
            } else {
                assert!(!permission)
            }
        }

        Ok(())
    }

    /// The key is expired
    #[ignore = "skip"]
    #[test]
    fn leaky_bucket_redis_case2() -> Result<(), ()> {
        // prev
        initialize_redis()?;

        // arrange
        let limit_rate_per_sec = 1000;
        let size = Duration::from_secs(1);
        let mut client = rate_limiter_redis::RateLimiterRedis::open(CONN, limit_rate_per_sec)?;
        let key_prefix = "test4";
        let resource = "data";
        let subject = "andy";

        // act && assert
        for _ in 0..1500 {
            client.record_leaky_bucket(key_prefix, resource, subject, size)?;
        }

        std::thread::sleep(Duration::from_secs(2));
        let count = client.fetch_leaky_bucket(key_prefix, resource, subject, size)?;

        assert_eq!(count, 0);

        Ok(())
    }

    /// The partial keys are expired
    #[ignore = "skip"]
    #[test]
    fn leaky_bucket_redis_case3() -> Result<(), ()> {
        // prev
        initialize_redis()?;

        // arrange
        let limit_per_sec = 1;
        let size = Duration::from_secs(1);
        let mut client = rate_limiter_redis::RateLimiterRedis::open(CONN, limit_per_sec)?;
        let key_prefix = "test4";
        let resource = "data";
        let subject = "andy";

        // act && assert
        client.record_leaky_bucket(key_prefix, resource, subject, size)?;

        assert!(client.allow_request_leaky_bucket(key_prefix, resource, subject, size)?);

        client.record_leaky_bucket(key_prefix, resource, subject, size)?;

        assert!(!client.allow_request_leaky_bucket(key_prefix, resource, subject, size)?);

        std::thread::sleep(Duration::from_secs(1));
        assert!(client.allow_request_leaky_bucket(key_prefix, resource, subject, size)?);

        let count = client.fetch_leaky_bucket(key_prefix, resource, subject, size)?;

        assert_eq!(count, 0);

        Ok(())
    }

    /// The partial keys are expired
    #[ignore = "skip"]
    #[test]
    fn leaky_bucket_redis_case4() -> Result<(), ()> {
        // prev
        initialize_redis()?;

        // arrange
        let limit_per_sec = 10;
        let size = Duration::from_secs(1);
        let mut client = rate_limiter_redis::RateLimiterRedis::open(CONN, limit_per_sec)?;
        let key_prefix = "test4";
        let resource = "data";
        let subject = "andy";

        // act && assert
        for _ in 0..6 {
            client.record_leaky_bucket(key_prefix, resource, subject, size)?;
            std::thread::sleep(Duration::from_millis(1));
        }

        std::thread::sleep(Duration::from_millis(500));

        for _ in 0..4 {
            client.record_leaky_bucket(key_prefix, resource, subject, size)?;
            std::thread::sleep(Duration::from_millis(1));
        }

        std::thread::sleep(Duration::from_millis(500));

        assert!(client.allow_request_leaky_bucket(key_prefix, resource, subject, size)?);

        let count = client.fetch_leaky_bucket(key_prefix, resource, subject, size)?;

        let key = format!("{key_prefix}:{resource}:{subject}");
        print_the_value_in_redis(&key)?;

        assert_eq!(count, 4);

        Ok(())
    }

    /// Test helper
    fn print_the_value_in_redis(key: &str) -> Result<(), ()> {
        let redis_address: &str = "redis://127.0.0.1:6379/";
        let client = redis::Client::open(redis_address).map_err(|err| {
            eprintln!("Error: could not open the connection to the Redis({redis_address}): {err}")
        })?;

        let mut conn = client.get_connection().map_err(|err| {
            eprintln!("Error: client could not get the connection to the Redis: {err}")
        })?;

        let (r,): (Vec<u64>,) = redis::cmd("LRANGE")
            .arg(&key)
            .arg(0)
            .arg(-1)
            .query(&mut conn)
            .map_err(|err| eprintln!("Error: could not get all data in Redis: {err}"))
            .ok()
            .unwrap_or_default();

        println!("values in the Redis: {r:?}");

        Ok(())
    }
}
