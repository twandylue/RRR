# RRR

Redis Rate limiter in Rust.

## Running Tests Locally

### Set up Redis by Docker

```console
$ docker-compose up -d
...
```

### Running Tests

```console
$ cargo test --all -- --test-threads 1
...
```

## Introduction of Different Methods about Rate Limiting

### Fixed Window

- Pros
  - Easy to implement.

- Cons
  - Accumulating is not distributed uniformly.
  - Not accurate if there is a burst of requests at the end of window. Or rate limit quota can be "starved" if there is a traffic spike at the beginning of window.

### Sliding Log

- Pros
  - Unaffected by burst of requests at the end of window, which means it could count requests more accurately at windows' boundaries.

- Cons
  - It is very expensive because we count(store) user's last requests in each request, which doesn't scale well when large burst of requests happen.

### Sliding Window

- Pros
  - Avoids the **starvation problem** of leaky bucket and **bursting problem** of fixed window.
  - Unaffected by burst of requests at the end of window, which means it could count requests more accurately at windows' boundaries.
  - Overcome the cons of sliding logs by not storing all the requests avoiding counting for every request and thus it is more memory and performance efficient.

- Cons
  - Not a con, but have to delete expired window keys, an extra command/load on Redis.

### Leaky Bucket

- Pros
  - Unaffected by burst of requests at the end of window, which means it could count requests more accurately at windows' boundaries.
  - Do not have to store all requests(only the requests limited to queue size) and thus more memory efficient.

- Cons
  - Burst of requests can fill up the queue with old requests and most recent requests are slowed from being processed and thus give no guarantee that requests are processed in a fixed amount time.
  - This method causes traffic shaping(handling requests at a constant rate), which may slow user's requests.

### Token Bucket

- Pros
  - No traffic shaping, no inaccurate counting at the windows' boundaries, more memory and performance efficient.
  - No need for background code to check and delete the expired keys.

- Cons
  - Can cause race condition in a distributed environment.

## Conclusion

Without considering distributed environment(race condition), `Token bucket` is likely the best method because there are no traffic shaping issue, window boundary issue and memory efficiency issue in this method.

## References

- [Rate Limiting in Rust Using Redis](https://outcrawl.com/rust-redis-rate-limiting)
- [Rate Limiting Algorithms using Redis](https://medium.com/@SaiRahulAkarapu/rate-limiting-algorithms-using-redis-eb4427b47e33)
- [Rate Limiting Algorithms](https://codeminion.hashnode.dev/rate-limiting-algorithms)
