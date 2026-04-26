
FROM rust:bookworm AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev libpq-dev ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && printf 'fn main(){println!("dummy");}\n' > src/main.rs
RUN cargo build --release --locked
RUN rm -rf src

RUN cargo install diesel_cli --no-default-features --features postgres

COPY . .

RUN touch src/main.rs && cargo build --release --locked


FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libpq5 libssl3 \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/rtc_backend /usr/local/bin/rtc_backend
COPY --from=builder /usr/local/cargo/bin/diesel /usr/local/bin/diesel

COPY --from=builder /app/migrations /app/migrations
COPY --from=builder /app/diesel.toml /app/diesel.toml

COPY docker/entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

ENV RUST_LOG=info
EXPOSE 3000
ENTRYPOINT ["/bin/sh", "/app/entrypoint.sh"]
