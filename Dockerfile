# Dockerfile for edlicense with support for production, distroless, and debug modes
# Build with:
#   - Default (production): docker build .
#   - Distroless: docker build --build-arg MODE=distroless .
#   - Debug: docker build --build-arg MODE=debug .

# Define build argument for mode
ARG MODE=production

# Base build stage
FROM rust:1.85-slim AS builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    git \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/edlicense

# Copy only the files needed for dependency resolution first
COPY Cargo.toml Cargo.lock* rust-toolchain.toml* ./

# Create a dummy main.rs to build dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy the actual source code
COPY . .

# Build the application
RUN cargo build --release

# Debug image with full toolchain
FROM rust:1.85 AS debug

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
RUN cargo install cargo-watch cargo-audit cargo-outdated

# Set environment variables
ENV RUST_BACKTRACE=1

# Default command for debug mode
CMD ["cargo", "test"]

# Production image (minimal Debian-based)
FROM debian:bookworm-slim AS production

# Install minimal runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends libssl-dev ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /usr/src/edlicense/target/release/edlicense /usr/local/bin/edlicense

# Set the entrypoint
ENTRYPOINT ["edlicense"]

# Default command (can be overridden)
CMD ["--help"]

# Distroless image (even more minimal)
FROM gcr.io/distroless/cc-debian12 AS distroless

# Copy SSL certificates for git operations
COPY --from=debian:bookworm-slim /etc/ssl/certs /etc/ssl/certs

# Copy necessary shared libraries from the builder
COPY --from=builder /usr/lib/*/libgit2.so* /usr/lib/
COPY --from=builder /usr/lib/*/libssl.so* /usr/lib/
COPY --from=builder /usr/lib/*/libcrypto.so* /usr/lib/
COPY --from=builder /lib/*/libz.so* /lib/

WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /usr/src/edlicense/target/release/edlicense /usr/bin/edlicense

# Set the entrypoint
ENTRYPOINT ["/usr/bin/edlicense"]

# Default command (can be overridden)
CMD ["--help"]

# Final stage - determined by build arg
FROM ${MODE}

# Label the image
LABEL org.opencontainers.image.title="edlicense"
LABEL org.opencontainers.image.description="A tool that ensures source code files have copyright license headers"
LABEL org.opencontainers.image.source="https://github.com/eddieland/edlicense"