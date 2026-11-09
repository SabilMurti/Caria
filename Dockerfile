# ==============================================================================
# CARIA NATIVE BACKEND - MULTI-STAGE HIGH-VELOCITY DOCKERFILE
# ==============================================================================
# Produces an ultra-lightweight (< 15MB) container for Caria native microservices.
# Cold-start: < 2ms | RAM: < 10MB | Zero VM overhead
# ==============================================================================

# Stage 1: Build the Caria Compiler & Native Toolchain
FROM rust:1.80-alpine AS builder

RUN apk add --no-cache musl-dev gcc make

WORKDIR /usr/src/caria

# Cache dependency builds
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release --bin caria

# Stage 2: Production Minimal Runtime
FROM alpine:3.20

RUN apk add --no-cache ca-certificates libgcc

WORKDIR /app

# Copy the native compiler CLI
COPY --from=builder /usr/src/caria/target/release/caria /usr/local/bin/caria
COPY std /usr/local/lib/caria/std
COPY examples /app/examples

EXPOSE 8080

# Default command to run a native Caria microservice
ENTRYPOINT ["caria"]
CMD ["run", "examples/backend_api.cria"]
