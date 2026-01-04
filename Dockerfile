# Build stage
FROM rust:slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Install nightly toolchain for edition 2024
RUN rustup toolchain install nightly --profile minimal && \
    rustup default nightly

WORKDIR /app

# Copy manifests first for better Docker layer caching
COPY Cargo.toml Cargo.lock ./

COPY src/main.rs ./src/main.rs

# Pre-fetch dependencies (keeps rebuilds fast when only src changes)
RUN cargo fetch

# Copy source code
COPY src ./src

# Build the application (debug by default; can be overridden to release)
ARG BUILD_PROFILE=debug
ENV CARGO_BUILD_JOBS=1
RUN if [ "$BUILD_PROFILE" = "release" ]; then \
        cargo build --locked --release; \
    else \
        cargo build --locked; \
    fi && \
    mkdir -p /out && \
    cp "/app/target/${BUILD_PROFILE}/oscbot" /out/oscbot

# Runtime stage
FROM git.sulej.net/osc/skins-image:latest

ENV OSC_BOT_DANSER_PATH=/app/danser
ENV PATH="/app/danser:${PATH}"
WORKDIR /app/oscbot

# Copy the binary from builder
COPY --from=builder /out/oscbot /app/oscbot/oscbot

# Copy configuration files (non-secret)
COPY default-danser.json /app/oscbot/default-danser.json

# Seed danser settings (danser reads /app/danser/settings/default.json)
COPY default-danser.json /app/danser/settings/default.json

# Copy generate data directory
COPY src/generate/data /app/oscbot/src/generate/data

# Create necessary runtime directories
RUN mkdir -p /app/oscbot/Songs /app/oscbot/Skins /app/oscbot/Replays /app/oscbot/videos /app/oscbot/videoForRegen && \
    chmod +x /app/oscbot/oscbot

CMD ["/app/oscbot/oscbot"]
