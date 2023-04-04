use redis::{Commands, Connection, LposOptions};
use std::time::{self, Duration, SystemTime};

pub struct RateLimiterRedis {
    pub conn: Connection,
    pub limit_per_sec: u64,
}

impl RateLimiterRedis {
    pub fn open(redis_address: &str, limit_per_sec: u64) -> Result<Self, ()> {
        let client = redis::Client::open(redis_address).map_err(|err| {
            eprintln!("Error: could not open the connection to the Redis({redis_address}): {err}")
        })?;

        let conn = client.get_connection().map_err(|err| {
            eprintln!("Error: client could not get the connection to the Redis: {err}")
        })?;

        Ok(RateLimiterRedis {
            conn,
            limit_per_sec,
        })
    }

    pub fn record_fixed_window(
        &mut self,
        key_prefix: &str,
        resource: &str,
        subject: &str,
        size: Duration,
    ) -> Result<bool, ()> {
        let now = SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
        let window = (now.as_secs() / size.as_secs()) * size.as_secs();
        let key = format!("{}:{}:{}:{}", key_prefix, resource, subject, window);

        let (curr_count,): (Option<u64>,) = redis::pipe()
            .atomic()
            .get(&key)
            .query(&mut self.conn)
            .map_err(|err| {
                eprintln!("Error: could not get the current requests number in fixed window: {err}")
            })?;

        match curr_count {
            Some(c) => {
                if c >= self.limit_per_sec * size.as_secs() {
                    return Ok(false);
                }
            }
            None => {
                redis::pipe()
                    .atomic()
                    .set(&key, 0)
                    .query(&mut self.conn)
                    .map_err(|err| {
                        eprintln!("Error: could not initiate the fixed window: {err}")
                    })?;
            }
        };

        redis::pipe()
            .atomic()
            .incr(&key, 1)
            .expire(&key, size.as_secs() as usize)
            .ignore()
            .query(&mut self.conn)
            .map_err(|err| eprintln!("Error: could not set the key-value into Redis when using fixed window method: {err}"))?;

        Ok(true)
    }

    pub fn fetch_fixed_window(
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

    pub fn record_sliding_log(
        &mut self,
        key_prefix: &str,
        resource: &str,
        subject: &str,
        size: Duration,
    ) -> Result<bool, ()> {
        let now = SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
        let key = format!("{key_prefix}:{resource}:{subject}");

        let count: u64 = self.conn.zcard(&key).map_err(|err| {
            eprintln!("Error: could not fetch the value of key: {key}: {err}");
        })?;

        if count >= self.limit_per_sec * size.as_secs() {
            return Ok(false);
        }

        let (_count,): (u64,) = redis::pipe()
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

        Ok(true)
    }

    pub fn fetch_sliding_log(
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

    pub fn record_sliding_window(
        &mut self,
        key_prefix: &str,
        resource: &str,
        subject: &str,
        size: Duration,
    ) -> Result<bool, ()> {
        let now = SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
        let current_window = (now.as_secs() / size.as_secs()) * size.as_secs();
        let current_key = format!("{key_prefix}:{resource}:{subject}:{current_window}");
        let previous_window = (now.as_secs() / size.as_secs()) * size.as_secs() - size.as_secs();
        let previous_key = format!("{key_prefix}:{resource}:{subject}:{previous_window}");

        let (previous_count, current_count): (Option<u64>, Option<u64>) = self
            .conn
            .get(vec![&previous_key, &current_key])
            .map_err(|err| {
                eprintln!("Error: could not fetch the key-value in fetch sliding window: {err}")
            })?;

        let count = Self::sliding_window_counter(previous_count, current_count, now, size);

        if count >= self.limit_per_sec * size.as_secs() {
            return Ok(false);
        }

        let (_previous_count, _current_count): (Option<u64>, Option<u64>) = redis::pipe()
            .atomic()
            .get(&previous_key)
            .incr(&current_key, 1)
            .expire(&current_key, (size.as_secs() * 2) as usize)
            .ignore()
            .query(&mut self.conn)
            .map_err(|err| {
                eprintln!("Error: could not set the key-value in record sliding window: {err}")
            })?;

        return Ok(true);
    }

    fn sliding_window_counter(
        previous_count: Option<u64>,
        current_count: Option<u64>,
        now: Duration,
        size: Duration,
    ) -> u64 {
        let current_window = (now.as_secs() / size.as_secs()) * size.as_secs();
        let next_window = current_window + size.as_secs();
        let weight = (Duration::from_secs(next_window).as_millis() - now.as_millis()) as f64
            / size.as_millis() as f64;

        current_count.unwrap_or(0) + (previous_count.unwrap_or(0) as f64 * weight).round() as u64
    }

    pub fn fetch_sliding_window(
        &mut self,
        key_prefix: &str,
        resource: &str,
        subject: &str,
        size: Duration,
    ) -> Result<u64, ()> {
        let now = SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
        let current_window = (now.as_secs() / size.as_secs()) * size.as_secs();
        let current_key = format!("{key_prefix}:{resource}:{subject}:{current_window}");
        let previous_window = (now.as_secs() / size.as_secs()) * size.as_secs() - size.as_secs();
        let previous_key = format!("{key_prefix}:{resource}:{subject}:{previous_window}");

        let (previous_count, current_count): (Option<u64>, Option<u64>) = self
            .conn
            .get(vec![previous_key, current_key])
            .map_err(|err| {
                eprintln!("Error: could not fetch the key-value in fetch sliding window: {err}")
            })?;

        Ok(Self::sliding_window_counter(
            previous_count,
            current_count,
            now,
            size,
        ))
    }

    // TODO:
    pub fn record_leaky_bucket(
        &mut self,
        key_prefix: &str,
        resource: &str,
        subject: &str,
        size: Duration,
    ) -> Result<u64, ()> {
        let now = SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
        let curr_window = (now.as_secs() / size.as_secs()) * size.as_secs();
        let key = format!("{key_prefix}:{resource}:{subject}");

        let pos = Self::find_pos_in_list(self, &key, curr_window, 1)?;
        let count = match pos {
            Some(n) => {
                let (c,): (u64,) = redis::pipe()
                    .atomic()
                    .ltrim(&key, n, -1)
                    .ignore()
                    .rpush(&key, curr_window)
                    .query(&mut self.conn)
                    .map_err(|err| {
                        eprintln!("Error: could not insert the key-value into Redis: {err}")
                    })?;
                c
            }
            None => {
                let (c,): (u64,) = redis::pipe()
                    .atomic()
                    .ltrim(&key, 0, -1)
                    .ignore()
                    .rpush(&key, curr_window)
                    .query(&mut self.conn)
                    .map_err(|err| {
                        eprintln!("Error: could not insert the key-value into Redis: {err}")
                    })?;
                c
            }
        };

        Ok(count)
    }

    // TODO:
    pub fn fetch_leaky_bucket(
        &mut self,
        key_prefix: &str,
        resource: &str,
        subject: &str,
        size: Duration,
    ) -> Result<u64, ()> {
        let now = SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
        let curr_window = (now.as_secs() / size.as_secs()) * size.as_secs();
        let prev_window = (now.as_secs() / size.as_secs()) * size.as_secs() - size.as_secs();
        let key = format!("{key_prefix}:{resource}:{subject}");

        println!("curr_window: {curr_window}");
        println!("prev_window: {prev_window}");

        let pos = Self::find_pos_in_list(self, &key, curr_window, 1)?;
        let prev_pos = Self::find_pos_in_list(self, &key, prev_window, 1)?;

        // TODO: discrete problem
        let count = match pos {
            // let count = match prev_pos {
            Some(n) => {
                let (c,): (Vec<u64>,) = redis::pipe()
                    .atomic()
                    .lrange(&key, n, -1)
                    // .ignore()
                    // .llen(&key)
                    .query(&mut self.conn)
                    .map_err(|err| {
                        eprintln!("Error: could not fetch the key-value in Redis: {err}")
                    })?;
                c.len() as u64
            }
            None => 0,
        };

        Ok(count)
    }

    fn find_pos_in_list(&mut self, key: &str, ele: u64, rank: isize) -> Result<Option<isize>, ()> {
        let (start_pos,): (Option<isize>,) = redis::pipe()
            .atomic()
            .lpos(&key, ele, LposOptions::default().rank(rank))
            .query(&mut self.conn)
            .map_err(|err| {
                eprintln!("Error: could not find the position of element({ele} in the list: {err})")
            })?;

        Ok(start_pos)
    }

    // TODO:
    pub fn allow_request_leaky_bucket(
        &mut self,
        key_prefix: &str,
        resource: &str,
        subject: &str,
        size: Duration,
    ) -> Result<bool, ()> {
        let count = Self::fetch_leaky_bucket(self, key_prefix, resource, subject, size)?;

        Ok(count <= self.limit_per_sec)
    }

    pub fn record_token_bucket(
        &mut self,
        key_prefix: &str,
        resource: &str,
        subject: &str,
        size: Duration,
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
                        .expire(&remain_req_key, size.as_secs() as usize)
                        .set(&last_set_time_key, now.as_secs())
                        .expire(&last_set_time_key, size.as_secs() as usize)
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
                    .expire(&last_set_time_key, size.as_secs() as usize)
                    .set(&remain_req_key, &self.limit_per_sec * size.as_secs())
                    .expire(&remain_req_key, size.as_secs() as usize)
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
}
