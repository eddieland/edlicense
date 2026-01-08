# Dockerfile for edlicense with optimized BuildKit features
# Build with:
#   - Default (production): docker build .
#   - Distroless: docker build --build-arg MODE=distroless .
#   - Debug: docker build --build-arg MODE=debug .

# Define build argument for mode
ARG MODE=production
ARG RUST_VERSION=1.92
# Define build arguments for labels with defaults
ARG BUILD_DATE=unknown
ARG BUILD_REVISION=unknown
ARG BUILD_VERSION=dev

# Base build stage
FROM rust:${RUST_VERSION}-slim AS builder

ARG MUSL_TARGET=x86_64-unknown-linux-musl

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    musl-tools \
    pkg-config \
    libssl-dev \
    git \
    && rustup target add ${MUSL_TARGET} \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/edlicense

# Copy only the files needed for dependency resolution first
COPY Cargo.toml Cargo.lock* rust-toolchain.toml* ./

# Create a dummy main.rs to build dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release --target ${MUSL_TARGET} && \
    rm -rf src

# Copy the actual source code
COPY . .

# Build the application
RUN cargo build --release --target ${MUSL_TARGET} && \
    cp target/${MUSL_TARGET}/release/edlicense /usr/local/bin/

# Debug image with full toolchain
FROM rust:${RUST_VERSION} AS debug

# Install additional development tools
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    git \
    libssl-dev \
    pkg-config \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/edlicense

# Copy the entire project
COPY . .

# Build the project in debug mode
RUN cargo build

# Install development tools
RUN cargo install cargo-nextest

# Set environment variables
ENV RUST_BACKTRACE=1

# Default command for debug mode
CMD ["cargo", "nextest", "run"]

# Production image (minimal Debian-based)
FROM debian:bookworm-slim AS production

# Install minimal runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends libssl3 ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Create a non-root user for runtime
RUN useradd --system --uid 10001 --home-dir /app --shell /usr/sbin/nologin edlicense && \
    chown -R edlicense:edlicense /app

# Copy the binary from the builder stage - already in /usr/local/bin from our optimized build
COPY --from=builder /usr/local/bin/edlicense /usr/local/bin/edlicense

# Drop privileges
USER edlicense

# Set the entrypoint
ENTRYPOINT ["edlicense"]

# Default command (can be overridden)
CMD ["--help"]

# Intermediate stage for certs
FROM debian:bookworm-slim AS cert-stage

# Install CA certificates package to ensure /etc/ssl/certs exists
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Distroless image (even more minimal)
FROM gcr.io/distroless/static-debian12 AS distroless

# Copy SSL certificates for git operations
COPY --from=cert-stage /etc/ssl/certs /etc/ssl/certs

WORKDIR /app

# Copy the binary from the builder stage - already in /usr/local/bin from our optimized build
COPY --from=builder --chown=nonroot:nonroot /usr/local/bin/edlicense /usr/bin/edlicense

# Drop privileges
USER nonroot

# Set the entrypoint
ENTRYPOINT ["/usr/bin/edlicense"]

# Default command (can be overridden)
CMD ["--help"]

# Final stage - determined by build arg
FROM ${MODE}

# Re-declare build arguments for the final stage
ARG BUILD_DATE
ARG BUILD_REVISION
ARG BUILD_VERSION

# Add standardized OCI labels
# https://github.com/opencontainers/image-spec/blob/main/annotations.md
LABEL org.opencontainers.image.title="edlicense"
LABEL org.opencontainers.image.description="A tool that ensures source code files have copyright license headers"
LABEL org.opencontainers.image.source="https://github.com/eddieland/edlicense"
LABEL org.opencontainers.image.created="${BUILD_DATE}"
LABEL org.opencontainers.image.revision="${BUILD_REVISION}"
LABEL org.opencontainers.image.version="${BUILD_VERSION}"
