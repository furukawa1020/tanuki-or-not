### Multi-stage Dockerfile: build React frontend then Rust backend, produce single runtime image

# 1) Build frontend
FROM node:18-bullseye as node-builder
WORKDIR /workspace/frontend

# copy only what we need for npm install first (cache)
COPY package.json package-lock.json* ./
# copy optional package files from rust subfolder if present
RUN if [ -f tanuki-quiz-rust/package.json ]; then mkdir -p ./tanuki-quiz-rust && cp tanuki-quiz-rust/package.json ./tanuki-quiz-rust/ || true; fi
# install dependencies reliably (use npm install so missing lockfile won't fail)
RUN npm install --legacy-peer-deps --silent

# copy frontend sources (root-level CRA)
COPY public ./public
COPY src ./src
RUN npm run build --silent

# 2) Build Rust backend
FROM --platform=linux/amd64 rust:1.72-slim as rust-builder
WORKDIR /usr/src/app
RUN apt-get update && apt-get install -y pkg-config libssl-dev libpq-dev libjpeg-dev libpng-dev ca-certificates && rm -rf /var/lib/apt/lists/*

# copy cargo files and source for rust app
COPY tanuki-quiz-rust/Cargo.toml ./
COPY tanuki-quiz-rust/Cargo.lock ./
COPY tanuki-quiz-rust/src ./src

RUN cargo build --release

# 3) Final runtime image
FROM debian:bookworm-slim
WORKDIR /app

# copy binary
COPY --from=rust-builder /usr/src/app/target/release/tanuki-quiz-rust /usr/local/bin/tanuki-quiz-rust

# copy frontend build into public
COPY --from=node-builder /workspace/frontend/build /app/public

ENV RUST_LOG=info
EXPOSE 8080
CMD ["/usr/local/bin/tanuki-quiz-rust"]
