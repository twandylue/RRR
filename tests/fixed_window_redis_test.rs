#[cfg(test)]
mod tests {
    use rrr::rate_limiter_redis;
    use std::time::Duration;

    const CONN: &str = "redis://127.0.0.1:6379/";

    #[tokio::test]
    async fn fixed_window_redis_case1() -> Result<(), ()> {
        // arrange
        let limit_count = 10;
        let mut client = rate_limiter_redis::RateLimiterRedis::open(CONN, limit_count).await?;
        let key_prefix = "test";
        let resource = "data";
        let subject = "andy";
        let size = Duration::from_secs(1);

        // act
        for c in 0..11 {
            client
                .record_fixed_window("test", "data", "andy", size)
                .await?;

            let count = client
                .fetch_fixed_window(key_prefix, resource, subject, size)
                .await?;

            assert_eq!(count, c + 1);
        }

        let actual = client
            .canMakeRequest(key_prefix, resource, subject, size)
            .await?;

        // assert
        assert!(!actual);

        Ok(())
    }
}
