# Web build stage — context must be the workspace root
FROM node:22-slim AS web

WORKDIR /app/crates/caiven-port/web
COPY crates/caiven-port/web/package.json crates/caiven-port/web/package-lock.json ./
COPY crates/caiven-ui/package.json /app/crates/caiven-ui/package.json
COPY crates/caiven-ui/src/ /app/crates/caiven-ui/src/
RUN npm ci
COPY crates/caiven-port/web/ ./
RUN npm run build

# Build stage — context must be the workspace root
FROM rust:latest AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --release -p caiven-port

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/caiven-port /usr/local/bin/caiven-port
COPY --from=web /app/crates/caiven-port/web/dist /app/web

EXPOSE 8080

ENTRYPOINT ["caiven-port"]
CMD ["--web-dir", "/app/web", "--address", "0.0.0.0", "--port", "8080"]
