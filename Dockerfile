FROM rust:1.86-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY src src

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/tf-http-pg-backend /usr/local/bin/tf-http-pg-backend

ENV LISTEN_ADDR=0.0.0.0:8080
EXPOSE 8080

CMD ["tf-http-pg-backend"]
