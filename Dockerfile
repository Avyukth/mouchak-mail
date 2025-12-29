# Build Stage
FROM rust:1.83-bookworm as builder

WORKDIR /app
COPY . .

# Build specific package
RUN cargo build --release -p mouchak-mail

# Runtime Stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies (sqlite3, ca-certificates)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /app/target/release/mouchak-mail /usr/local/bin/mouchak-mail

# Create data directory
RUN mkdir -p /app/data
VOLUME /app/data

# Environment variables
ENV PORT=3000
ENV HOST=0.0.0.0

# Expose port
EXPOSE 3000

# Run
CMD ["mouchak-mail", "serve"]
