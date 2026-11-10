# syntax=docker/dockerfile:1

# ---- frontend build ----
FROM node:20-alpine AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
# Empty by default: the backend serves the frontend from the same origin in
# production, so requests can use relative paths (see frontend/src/api/client.ts).
ARG VITE_API_URL=""
ENV VITE_API_URL=$VITE_API_URL
# Leave false for production; true bundles the dev account switcher.
ARG DEV_LOGIN=false
ENV DEV_LOGIN=$DEV_LOGIN
RUN npm run build

# ---- backend build ----
FROM rust:1-bookworm AS backend-builder
WORKDIR /app/backend
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
# Dependencies are built first against a stub main.rs, so that layer is reused until
# Cargo.toml or Cargo.lock changes. (No cache mounts: Railway rejects them without a
# service-specific id.)
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && cargo build --release && rm -rf src
COPY backend/src ./src
# sqlx::migrate! embeds these into the binary at compile time.
COPY backend/migrations ./migrations
# The dev seed is embedded too (include_str!), even when DEV_LOGIN is off.
COPY backend/seed ./seed
# COPY keeps the files' original mtimes, which can be older than the stub build; touch
# makes cargo rebuild the crate itself.
RUN touch src/main.rs && cargo build --release

# ---- runtime ----
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --no-create-home --shell /usr/sbin/nologin app
WORKDIR /app
COPY --from=backend-builder /app/backend/target/release/backend ./backend
COPY --from=frontend-builder /app/frontend/dist ./static
# Nothing is written to disk (the database holds everything), so it runs unprivileged.
USER app

ENV STATIC_DIR=/app/static
ENV SERVER_ADDR=0.0.0.0:8080
EXPOSE 8080

CMD ["./backend"]
