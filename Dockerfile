# Use a Rust base image for compilation
FROM rust:latest as builder

# Set the working directory
WORKDIR /app

# Copy the source code
COPY ./src ./src
COPY ./Cargo.toml ./Cargo.toml

# Build the server in release mode
RUN cargo build --release

# Production stage
FROM debian:trixie

# Create a directory for the binary
RUN mkdir -p /app

# Copy the binary from the builder stage to the production image
COPY --from=builder /app/target/release/benchclient /app/benchclient

# Set the binary as executable (if needed)
RUN chmod +x /app/benchclient

# Command to run the server
WORKDIR /app
CMD ["./benchclient", "$ADDR", "$PORT", "$NUM_CLIENTS", "$NUM_REQUESTS"]
