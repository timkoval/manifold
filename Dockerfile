# Multi-stage build for minimal final image
FROM rust:1.82 as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source
COPY src ./src
COPY schemas ./schemas

# Build release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

# Create manifold user
RUN useradd -m -u 1000 manifold

# Copy binary from builder
COPY --from=builder /app/target/release/manifold /usr/local/bin/manifold

# Create data directory
RUN mkdir -p /home/manifold/.manifold && \
    chown -R manifold:manifold /home/manifold

USER manifold
WORKDIR /home/manifold

# Initialize manifold on first run
RUN manifold init

# Expose MCP server port (if running as HTTP in future)
EXPOSE 8080

# Default command runs MCP server
CMD ["manifold", "serve"]
