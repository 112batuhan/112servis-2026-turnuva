# syntax=docker/dockerfile:1

# ---- frontend build ----
FROM node:20-alpine AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package.json ./
RUN npm install
COPY frontend/ ./
# Empty by default: the backend serves the frontend from the same origin in
# production, so requests can use relative paths (see frontend/src/api.js).
ARG VITE_API_URL=""
ENV VITE_API_URL=$VITE_API_URL
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
RUN cargo build --release

# ---- runtime ----
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=backend-builder /app/backend/target/release/backend ./backend
COPY --from=frontend-builder /app/frontend/dist ./static

ENV STATIC_DIR=/app/static
ENV SERVER_ADDR=0.0.0.0:8080
EXPOSE 8080

CMD ["./backend"]
