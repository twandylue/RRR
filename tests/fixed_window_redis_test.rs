#[cfg(test)]
mod tests {
    use rrr::rate_limiter_redis;
    use std::time::Duration;

    const CONN: &str = "redis://127.0.0.1:6379/";

    #[tokio::test]
    async fn fixed_window_redis_case1() -> Result<(), ()> {
        // arrange
        let limit_count_per_min = 10;
        let mut client =
            rate_limiter_redis::RateLimiterRedis::open(CONN, limit_count_per_min).await?;
        let size = Duration::from_secs(1);

        // act
        for _ in 0..20 {
            client
                .record_fixed_window("test", "data", "andy", size)
                .await?;
        }

        let actual = client.canMakeRequest().await?;

        // assert
        // assert!(!actual);
        assert!(false);

        Ok(())
    }
}
