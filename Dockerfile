# ---------- Panel (React) ----------
FROM node:20-slim AS panel
WORKDIR /panel
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web ./
# Mismo origen que la API (el backend sirve el panel): base = /api
ENV VITE_API_URL=""
RUN npm run build   # -> /panel/dist

# ---------- Build (Rust) ----------
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
# El binario sirve el panel desde ./web/dist (mismo origen que /api)
COPY --from=panel /panel/dist ./web/dist
ENV HOST=0.0.0.0
EXPOSE 8090
CMD ["melihub"]
