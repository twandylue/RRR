use redis::{Commands, Connection};
use std::time::{self, Duration, SystemTime};

pub struct RateLimiterRedis {
    pub conn: Connection,
    pub limit: u64,
}

impl RateLimiterRedis {
    pub async fn open(redis_address: &str, limit: u64) -> Result<Self, ()> {
        let client = redis::Client::open(redis_address).map_err(|err| {
            eprintln!("Error: could not open the connection to the Redis({redis_address}): {err}")
        })?;

        let conn = client.get_connection().map_err(|err| {
            eprintln!("Error: client could not get the connection to the Redis: {err}")
        })?;

        Ok(RateLimiterRedis { conn, limit })
    }

    pub async fn record_fixed_window(
        &mut self,
        key_prefix: &str,
        resource: &str,
        subject: &str,
        size: Duration,
    ) -> Result<u64, ()> {
        let now = SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
        let window = (now.as_secs() / size.as_secs()) * size.as_secs();
        let key = format!("{}:{}:{}:{}", key_prefix, resource, subject, window);

        let (count,) : (u64,)= redis::pipe()
            .atomic()
            .incr(&key, 1)
            .expire(&key, size.as_secs() as usize)
            .ignore()
            // .query_async(&mut self.conn)
            // .await
            .query(&mut self.conn)
            .map_err(|err| eprintln!("Error: could not set the key-value into Redis when using fixed window method: {err}"))?;

        Ok(count)
    }

    pub async fn fetch_fixed_window(
        &mut self,
        key_prefix: &str,
        resource: &str,
        subject: &str,
        size: Duration,
    ) -> Result<u64, ()> {
        let now = SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
        let window = (now.as_secs() / size.as_secs()) * size.as_secs();
        let key = format!("{key_prefix}:{resource}:{subject}:{window}");

        let count: u64 = self
            .conn
            .get(key)
            .map_err(|err| eprintln!("Error: could not get the key from Redis: {err}"))?;

        Ok(count)
    }

    pub async fn canMakeRequest_fixed_window(
        &mut self,
        key_prefix: &str,
        resource: &str,
        subject: &str,
        size: Duration,
    ) -> Result<bool, ()> {
        let count = Self::fetch_fixed_window(self, key_prefix, resource, subject, size).await?;

        Ok(count < self.limit)
    }

    pub async fn record_sliding_log(
        &mut self,
        key_prefix: &str,
        resource: &str,
        subject: &str,
        size: Duration,
    ) -> Result<u64, ()> {
        let now = SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
        let key = format!("{key_prefix}:{resource}:{subject}");
        let (count,): (u64,) = redis::pipe()
            .atomic()
            .zrembyscore(&key, 0, (now.as_millis() - size.as_millis()) as u64)
            .ignore()
            .zadd(&key, now.as_millis() as u64, now.as_millis() as u64)
            .ignore()
            .zcard(&key)
            .expire(&key, size.as_secs() as usize)
            .ignore()
            .query(&mut self.conn)
            .map_err(|err| {
                eprintln!("Error: could not set the key-value by sliding log method: {err}")
            })?;

        Ok(count)
    }

    pub async fn fetch_sliding_log(
        &mut self,
        key_prefix: &str,
        resource: &str,
        subject: &str,
    ) -> Result<u64, ()> {
        let key = format!("{key_prefix}:{resource}:{subject}");
        let count: u64 = self.conn.zcard(&key).map_err(|err| {
            eprintln!("Error: could not fetch the value of key: {key}: {err}");
        })?;

        Ok(count)
    }

    pub async fn canMakeRequest_sliding_log(
        &mut self,
        key_prefix: &str,
        resource: &str,
        subject: &str,
    ) -> Result<bool, ()> {
        let count = self
            .fetch_sliding_log(key_prefix, resource, subject)
            .await?;

        Ok(count < self.limit)
    }
}
