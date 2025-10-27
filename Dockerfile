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
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --bin server

# Runtime stage
FROM alpine:latest

# Install Vulkan loader and tools for runtime GPU access
RUN apk add --no-cache vulkan-loader vulkan-tools mesa-vulkan-ati

# Copy the binary from builder
COPY --from=builder /app/target/debug/server /usr/local/bin/server

# Expose the server port
EXPOSE 3000

# Run the server
CMD ["server"]
