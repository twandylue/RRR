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

- Cons

### Sliding Log

- Pros

- Cons

### Sliding Window

- Pros

- Cons

## Conclusions

<!-- TODO: -->

## References

- [Rate Limiting in Rust Using Redis](https://outcrawl.com/rust-redis-rate-limiting)
