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

RUN mkdir -p /data/roms /data/screenshots

VOLUME /data
EXPOSE 8080

# FC_HUB_API_KEY must be provided at runtime; the release binary refuses the
# default "changeme" key.

ENTRYPOINT ["fc-hub"]
CMD ["--data-dir", "/data", "--address", "0.0.0.0", "--port", "8080"]
