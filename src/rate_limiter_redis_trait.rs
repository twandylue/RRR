use std::time::Duration;

pub trait RateLimiterRedisTrait {
    fn record(
        &mut self,
        key_prefix: &str,
        resource: &str,
        subject: &str,
        size: Duration,
    ) -> Result<bool, ()>;

    fn fetch(
        &mut self,
        key_prefix: &str,
        resource: &str,
        subject: &str,
        size: Duration,
    ) -> Result<u64, ()>;
}
