# syntax=docker/dockerfile:1

FROM rust:1.70-buster

WORKDIR /app

# Copy source code.
COPY . .

# Set `cargo test` as executable.
ENTRYPOINT ["/usr/local/cargo/bin/cargo", "test"]
