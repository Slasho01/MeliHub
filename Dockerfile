# ---------- Build ----------
FROM rust:1-slim-bookworm AS builder
WORKDIR /app
# libpq-dev: Diesel (feature postgres) enlaza contra libpq
RUN apt-get update && apt-get install -y --no-install-recommends libpq-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release --locked

# ---------- Runtime ----------
FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends libpq5 ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/melihub /usr/local/bin/melihub
ENV HOST=0.0.0.0
EXPOSE 8090
CMD ["melihub"]
