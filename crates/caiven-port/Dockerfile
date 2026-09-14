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

RUN cargo build --locked --release -p caiven-port

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/caiven-port /usr/local/bin/caiven-port
COPY --from=web /app/crates/caiven-port/web/dist /app/web

RUN groupadd --gid 10001 caiven \
    && useradd --uid 10001 --gid caiven --no-create-home --shell /usr/sbin/nologin caiven \
    && mkdir -p /app/data \
    && chown caiven:caiven /app/data

WORKDIR /app
USER 10001:10001

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=30s --retries=3 \
    CMD curl --fail --silent --max-time 2.5 http://127.0.0.1:8080/readyz || exit 1

ENTRYPOINT ["caiven-port"]
CMD ["--web-dir", "/app/web", "--address", "0.0.0.0", "--port", "8080"]
