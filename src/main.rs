use rate_limiter_redis::RateLimiterRedis;
use std::{
    sync::mpsc::{self, TryRecvError},
    time::Duration,
};

mod rate_limiter_redis;

fn main() -> Result<(), ()> {
    let conn = "redis://127.0.0.1:6379/";
    let limit_count = 1;
    let size = Duration::from_secs(1);
    let mut client = rate_limiter_redis::RateLimiterRedis::open(conn, limit_count)?;
    let key_prefix = "test4";
    let resource = "data";
    let subject = "andy";

    // act && assert
    let (tx, rx) = mpsc::channel::<()>();

    let _consumer = std::thread::spawn(move || -> Result<(), ()> {
        loop {
            println!("Consuming...");
            client.consume_leaky_bucket(key_prefix, resource, subject, size)?;
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

    std::thread::sleep(Duration::from_secs(1));

    let _ = tx.send(());
    let _ = _consumer.join();
    Ok(())
}
