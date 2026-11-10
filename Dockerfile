# syntax=docker/dockerfile:1

# ---- frontend build ----
FROM node:20-alpine AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN --mount=type=cache,target=/root/.npm npm ci
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
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
# sqlx::migrate! embeds these into the binary at compile time.
COPY backend/migrations ./migrations
# The dev seed is embedded too (include_str!), even when DEV_LOGIN is off.
COPY backend/seed ./seed
# The registry and target directory are cache mounts, so a change only rebuilds what it
# touches instead of every dependency. The binary is copied out of the cache, which
# isn't part of the image.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/backend/target \
    cargo build --release && cp target/release/backend /app/backend-bin

# ---- runtime ----
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --no-create-home --shell /usr/sbin/nologin app
WORKDIR /app
COPY --from=backend-builder /app/backend-bin ./backend
COPY --from=frontend-builder /app/frontend/dist ./static
# Nothing is written to disk (the database holds everything), so it runs unprivileged.
USER app

ENV STATIC_DIR=/app/static
ENV SERVER_ADDR=0.0.0.0:8080
EXPOSE 8080

CMD ["./backend"]
