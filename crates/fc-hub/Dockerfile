# Web build stage — context must be the workspace root
FROM node:22-slim AS web

WORKDIR /app/web
COPY crates/fc-hub/web/package.json crates/fc-hub/web/package-lock.json ./
RUN npm ci
COPY crates/fc-hub/web/ ./
RUN npm run build

# Build stage — context must be the workspace root
FROM rust:latest AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --release -p fc-hub

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/fc-hub /usr/local/bin/fc-hub
COPY --from=web /app/web/dist /app/web

RUN mkdir -p /data/roms /data/screenshots

VOLUME /data
EXPOSE 8080

ENTRYPOINT ["fc-hub"]
CMD ["--data-dir", "/data", "--web-dir", "/app/web", "--address", "0.0.0.0", "--port", "8080"]
