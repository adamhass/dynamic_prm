# Use a Rust base image for compilation
FROM rust:latest as builder

# Set the working directory
WORKDIR /app

# Copy the source code
COPY ./src ./src
COPY ./Cargo.toml ./Cargo.toml
COPY ./benches ./benches

# Build the server in release mode
RUN cargo build --release

# Production stage
FROM debian:trixie
RUN apt-get update && apt-get install -y libfontconfig1 fontconfig libfontconfig1-dev
# Create a directory for the binary
RUN mkdir -p /app

# Copy the binary from the builder stage to the production image
COPY --from=builder /app/target/release/dynamic_prm /app/dynamic_prm

# Set the binary as executable (if needed)
RUN chmod +x /app/dynamic_prm

# Command to run the server
WORKDIR /app
#, "$ADDR", "$PORT", "$NUM_CLIENTS", "$NUM_REQUESTS"]
CMD ["./dynamic_prm"] 
