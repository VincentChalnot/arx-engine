# Build stage
FROM rust:1.83-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

# Set working directory
WORKDIR /app

# Copy the Cargo files
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build the server binary
RUN cargo build --bin server

# Runtime stage
FROM alpine:latest

# Copy the binary from builder
COPY --from=builder /app/target/debug/server /usr/local/bin/server

# Expose the server port
EXPOSE 3000

# Run the server
CMD ["server"]
