# Multi-stage Dockerfile for Rust binary
FROM --platform=linux/amd64 rust:1.72-slim as builder
WORKDIR /usr/src/app
# install needed system deps for image crate (libpng, libjpeg)
RUN apt-get update && apt-get install -y pkg-config libssl-dev libpq-dev libjpeg-dev libpng-dev && rm -rf /var/lib/apt/lists/*

# copy manifests and fetch deps
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build release
RUN cargo build --release

# runtime image
FROM debian:bookworm-slim
WORKDIR /usr/local/bin
COPY --from=builder /usr/src/app/target/release/tanuki-quiz-rust ./tanuki-quiz-rust
# copy public assets
COPY public /app/public
ENV RUST_LOG=info
EXPOSE 3000
CMD ["/usr/local/bin/tanuki-quiz-rust"]
