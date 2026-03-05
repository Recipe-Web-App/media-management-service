# Rust version must match or exceed what generated Cargo.lock (currently 1.93)
ARG RUST_VERSION=1.93

# Stage 1: Compute dependency recipe
FROM rust:${RUST_VERSION}-bookworm AS chef
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall cargo-chef --no-confirm
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Build dependencies only (this layer is cached)
FROM rust:${RUST_VERSION}-bookworm AS cook
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
RUN apt-get update && apt-get install -y --no-install-recommends cmake && rm -rf /var/lib/apt/lists/*
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall cargo-chef --no-confirm
WORKDIR /app
COPY --from=chef /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Stage 3: Build application (only this rebuilds on code changes)
FROM cook AS builder
COPY . .
RUN cargo build --release

# Stage 4: Minimal runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
RUN useradd -r -s /bin/false media
COPY --from=builder /app/target/release/media-management-service /usr/local/bin/
USER media
EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/api/v1/media-management/health || exit 1
CMD ["media-management-service"]
