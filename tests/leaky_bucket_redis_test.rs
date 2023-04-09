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
    use std::{sync::mpsc::TryRecvError, time::Duration};

    const CONN: &str = "redis://127.0.0.1:6379/";

    /// Integration: Initiated -> Throttled -> Cool Down -> Refilled
    #[test]
    fn leaky_bucket_redis_case1() -> Result<(), ()> {
        // prev
        initialize_redis()?;

        // arrange
        let limit_count = 1;
        let size = Duration::from_secs(1);
        let mut client_consumer = rate_limiter_redis::RateLimiterRedis::open(CONN, limit_count)?;
        let mut client_requester = rate_limiter_redis::RateLimiterRedis::open(CONN, limit_count)?;
        let key_prefix = "test4";
        let resource = "data";
        let subject = "andy";

        let (tx, rx) = std::sync::mpsc::channel::<()>();
        let consumer = std::thread::spawn(move || -> Result<(), ()> {
            loop {
                println!("Consuming...");
                client_consumer.consume_leaky_bucket(key_prefix, resource, subject, size)?;
                std::thread::sleep(Duration::from_secs(size.as_secs()));
                match rx.try_recv() {
                    Ok(_) | Err(TryRecvError::Disconnected) => {
                        println!("The consumer is terminating...");
                        break;
                    }
                    Err(TryRecvError::Empty) => {}
                }
            }
            Ok(())
        });

        // act && assert
        let actual = client_requester.record_leaky_bucket(key_prefix, resource, subject, size)?;
        assert!(actual);

        // throttled
        let actual = client_requester.record_leaky_bucket(key_prefix, resource, subject, size)?;
        assert!(!actual);

        // cool down
        std::thread::sleep(Duration::from_secs(1));
        let count = client_requester.fetch_leaky_bucket(key_prefix, resource, subject, size)?;
        assert_eq!(count, 0);

        // refilled
        let actual = client_requester.record_leaky_bucket(key_prefix, resource, subject, size)?;
        assert!(actual);

        let count = client_requester.fetch_leaky_bucket(key_prefix, resource, subject, size)?;
        assert_eq!(count, 1);

        // final
        let _ = tx.send(());
        let _ = consumer.join();
        Ok(())
    }
}
