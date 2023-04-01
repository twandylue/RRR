use crate::rate_limiter_redis_trait::RateLimiterRedisTrait;
use redis::Connection;
use std::time::{self, SystemTime};

pub struct TokenBucketRateLimiter {
    pub conn: Connection,
    pub limit_per_sec: u64,
}

impl RateLimiterRedisTrait for TokenBucketRateLimiter {
    fn record(
        &mut self,
        key_prefix: &str,
        resource: &str,
        subject: &str,
        size: std::time::Duration,
    ) -> Result<bool, ()> {
        let key = format!("{key_prefix}:{resource}:{subject}");
        let last_set_time_key = format!("{key}:last_set_time");
        let remain_req_key = format!("{key}:remain_requests");
        let (last_set_time,): (Option<u64>,) = redis::pipe()
            .atomic()
            .get(&last_set_time_key)
            .query(&mut self.conn)
            .map_err(|err| eprintln!("Error: could not get the last setting time: {err}"))?;

        let now = SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
        match last_set_time {
            Some(last_time) => {
                if now.as_secs() - last_time >= size.as_secs() {
                    redis::pipe()
                        .atomic()
                        .set(&remain_req_key, &self.limit_per_sec * size.as_secs())
                        .ignore()
                        .set(&last_set_time_key, now.as_secs())
                        .ignore()
                        .query(&mut self.conn)
                        .map_err(|err| {
                            eprintln!("Error: could not re-set the remain request by keys: {remain_req_key} and {last_set_time_key}: {err}")
                        })?;
                } else {
                    let (remain_requests,): (u64,) = redis::pipe()
                        .atomic()
                        .get(&remain_req_key)
                        .query(&mut self.conn)
                        .map_err(|err| {
                            eprintln!("Error: could not get the remain requests by keys: {remain_req_key} and {last_set_time_key}: {err}")
                        })?;

                    if remain_requests <= 0 {
                        return Ok(false);
                    }
                }
            }
            None => {
                redis::pipe()
                    .atomic()
                    .set(&last_set_time_key, now.as_secs())
                    .ignore()
                    .set(&remain_req_key, &self.limit_per_sec * size.as_secs())
                    .ignore()
                    .query(&mut self.conn)
                    .map_err(|err| {
                        eprintln!(
                            "Error: could not initiate the first request in token bucket: {err}"
                        )
                    })?;
            }
        }

        redis::pipe()
            .atomic()
            .decr(remain_req_key, 1)
            .query(&mut self.conn)
            .map_err(|err| eprintln!("Error: could not decrease the value: {err}"))?;

        Ok(true)
    }

    fn fetch(
        &mut self,
        key_prefix: &str,
        resource: &str,
        subject: &str,
        size: std::time::Duration,
    ) -> Result<u64, ()> {
        let key = format!("{key_prefix}:{resource}:{subject}");
        let last_set_time_key = format!("{key}:last_set_time");

        let (last_set_time,): (Option<u64>,) = redis::pipe()
            .atomic()
            .get(&last_set_time_key)
            .query(&mut self.conn)
            .map_err(|err| eprintln!("Error: could not get the last set time: {err}"))?;

        match last_set_time {
            Some(lt) => {
                let now = SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
                if now.as_secs() - lt >= size.as_secs() {
                    return Ok(self.limit_per_sec * size.as_secs());
                } else {
                    let remain_req_key = format!("{key}:remain_requests");
                    let (remain_req,): (Option<u64>,) = redis::pipe()
                        .atomic()
                        .get(&remain_req_key)
                        .query(&mut self.conn)
                        .map_err(|err| {
                            eprintln!(
                                "Error: could not get the remain requests in token bucket: {err}"
                            )
                        })?;

                    return Ok(remain_req.unwrap_or(self.limit_per_sec * size.as_secs()));
                };
            }
            None => Ok(self.limit_per_sec * size.as_secs()),
        }
    }
}
