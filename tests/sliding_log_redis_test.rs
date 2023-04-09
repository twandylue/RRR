// NOTE: cargo test --all -- --test-threads 1
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

    /// Tests the requests exceed the rate limit.
    #[test]
    fn sliding_log_redis_case1() -> Result<(), ()> {
        // prev
        initialize_redis()?;

        // arrange
        let limit_count = 1;
        let mut client = rate_limiter_redis::RateLimiterRedis::open(CONN, limit_count)?;
        let key_prefix = "test2";
        let resource = "data";
        let subject = "andy";
        let size = Duration::from_secs(1);

        // act && assert
        let actual = client.record_sliding_log(key_prefix, resource, subject, size)?;

        assert!(actual);

        let actual = client.record_sliding_log(key_prefix, resource, subject, size)?;

        assert!(!actual);

        Ok(())
    }

    /// Integration: Initiated -> Throttled -> Cool Down -> Refilled
    #[test]
    fn sliding_log_redis_case2() -> Result<(), ()> {
        // prev
        initialize_redis()?;

        // arrange
        let limit_count = 1;
        let mut client = rate_limiter_redis::RateLimiterRedis::open(CONN, limit_count)?;
        let key_prefix = "test2";
        let resource = "data";
        let subject = "andy";
        let size = Duration::from_secs(1);

        // act && assert
        let actual = client.record_sliding_log(key_prefix, resource, subject, size)?;

        assert!(actual);

        // throttled
        let actual = client.record_sliding_log(key_prefix, resource, subject, size)?;

        assert!(!actual);

        // cool down
        std::thread::sleep(Duration::from_secs(2));

        let count = client.fetch_sliding_log(key_prefix, resource, subject)?;
        assert_eq!(count, 0);

        // refilled
        let actual = client.record_sliding_log(key_prefix, resource, subject, size)?;

        assert!(actual);

        let count = client.fetch_sliding_log(key_prefix, resource, subject)?;
        assert_eq!(count, 1);

        Ok(())
    }
}
