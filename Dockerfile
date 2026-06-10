# --- builder ---
FROM rust:1.96-slim AS builder

WORKDIR /app

# Cache dependencies before copying source
COPY Cargo.toml Cargo.lock ./
COPY crates/types/Cargo.toml crates/types/
COPY crates/config/Cargo.toml crates/config/
COPY crates/embedding/Cargo.toml crates/embedding/
COPY crates/vector/Cargo.toml crates/vector/
COPY crates/cache/Cargo.toml crates/cache/
COPY crates/admission/Cargo.toml crates/admission/
COPY crates/monitoring/Cargo.toml crates/monitoring/
COPY crates/proxy/Cargo.toml crates/proxy/

# Stub out all lib/main files so cargo fetch can resolve the dep graph
RUN for crate in types config embedding vector cache admission monitoring; do \
      mkdir -p crates/$crate/src && echo "pub fn _stub() {}" > crates/$crate/src/lib.rs; \
    done && \
    mkdir -p crates/proxy/src && echo "fn main() {}" > crates/proxy/src/main.rs

RUN cargo fetch

# Now copy real source and build
COPY crates/ crates/
COPY migrations/ migrations/

RUN cargo build --release -p semantiq-proxy

# --- runtime ---
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/semantiq /usr/local/bin/semantiq

EXPOSE 8080

CMD ["semantiq"]
